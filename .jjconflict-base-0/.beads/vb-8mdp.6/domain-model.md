# Domain Model — Idempotency Hydration for ActionTicket

## Ubiquitous Language

| Term | Type | Definition |
|------|------|------------|
| `ActionTicket` | Entity (keyed by `(run, step, action, idempotency_key)`) | Unique ticket tracking one action invocation across suspension boundaries. Carries `idempotency_key: u128` for deduplication. |
| `idempotency_key` | Value Object (`u128`) | Deterministic 128-bit key derived from `(run, seq, action)` via `compute_action_idempotency_key`. Must be canonical or explicitly provided for KeyRequired actions. |
| `KeyRequired` | `RetrySafety` variant | Actions that require an idempotency key to be safe for retry. |
| `Idempotency` | Enum (`DeterministicPure \| IdempotentExternal \| AtLeastOnceExternal`) | Classifies how an action behaves with repeated execution. |
| `SideEffect` | Enum (`None \| Writes \| Sends \| Creates \| Destroys`) | Classifies observable side effects of an action. |
| `ActionReplayTracker` | Entity (vb_storage) | Tracks scheduled/completed/failed actions during hydration to prevent re-execution of non-idempotent actions. |
| `ActionReplayEffect` | Enum (`Apply \| Duplicate`) | Result of checking a scheduled ticket: apply the action or skip as duplicate. |
| `JournalEvent` | Enum (vb_storage events.rs) | Durable event recorded to journal: `ActionScheduledTicket`, `ActionCompletedEnvelope`, `ActionFailedEvent`. |
| `Hydration` | Process | Reconstructing a live `RunFrame` from snapshot + tail events via `hydrate_run_frame`. |
| `RecoveryFrameSeed` | Aggregate | Minimal live-frame seed recovered from durable journal headers/events. |

## Value Objects

### `IdempotencyKey` (wrapper around `u128`)
- **Invariant**: For KeyRequired actions, key must equal `compute_action_idempotency_key(run, seq, action)` OR be a caller-supplied deterministic key derived from input slots.
- **Forbidden**: Secret-tainted values, random values, time-dependent values in key ingredients.
- **Construction**: Via `compute_action_idempotency_key(run, seq, action)` or explicit key-from-slots derivation.

### `ActionTicket`
```rust
pub struct ActionTicket {
    pub run: RunId,
    pub step: StepIdx,
    pub seq: SeqNo,
    pub action: ActionId,
    pub attempt: u16,           // 1-indexed attempt counter
    pub idempotency_key: u128, // deduplication key
    pub capacity: u16,          // max attempts allowed
}
```
- **Identity**: `(action, step)` uniquely identifies a scheduled action within a run.
- **Canonical key**: `ticket.idempotency_key == compute_action_idempotency_key(ticket.run, ticket.seq, ticket.action)`.
- **KeyRequired constraint**: If `RetrySafety::KeyRequired`, `idempotency_key` must be canonical or explicitly provided.

### `RetrySafety`
```rust
pub enum RetrySafety {
    Safe = 0,           // Always safe to retry
    KeyRequired = 1,    // Safe ONLY if idempotency key present
    Unsafe = 2,          // Never safe to retry
}
```

## Entities

### `ActionReplayTracker` (vb_storage/recovery/types.rs)
```rust
pub struct ActionReplayTracker {
    scheduled_tickets: HashMap<(ActionId, StepIdx), ActionScheduleEvidence>,
    completed: HashSet<(ActionId, StepIdx)>,
    failed: HashSet<(ActionId, StepIdx)>,
    completed_envelopes: HashMap<(ActionId, StepIdx), ActionCompletionEvidence>,
}
```
- **Purpose**: Prevents re-execution of non-idempotent actions during hydration.
- **Key**: `(action, step)` — NOT idempotency_key alone.
- **Duplicate detection**: If a scheduled ticket matches evidence exactly, returns `Duplicate`. If ticket differs, returns `ReplayDivergence`.

### `ActionScheduleEvidence`
```rust
struct ActionScheduleEvidence {
    ticket: ActionTicket,
    input: SlotIdx,
    output: SlotIdx,
}
```

### `ActionCompletionEvidence`
```rust
struct ActionCompletionEvidence {
    ticket: ActionTicket,
    output: SlotIdx,
    encoded_len: u32,
    taint: Taint,
    value_digest: [u8; 32],
}
```

## Domain Events (Durable Journal)

| Event | Fields | Purpose |
|-------|--------|---------|
| `ActionScheduledTicket` | `run`, `seq`, `ticket: ActionTicket`, `input`, `output` | Records full ticket at suspension time |
| `ActionCompletedEnvelope` | `run`, `seq`, `ticket`, `output`, `outcome`, `value`, `encoded_len`, `taint`, `value_digest` | Durable completion with digest for duplicate detection |
| `ActionFailedEvent` | `run`, `seq`, `step`, `action`, `attempt`, `code`, `retry_policy` | Terminal failure recorded |

## Invariants

1. **I1**: `ActionTicket` identity is `(run, step, action)` — idempotency_key is a property, not an identity component.
2. **I2**: `ActionReplayTracker` keys on `(action, step)` only — matching duplicate requires identical ticket evidence.
3. **I3**: A `KeyRequired` action MUST have either a canonical idempotency_key or an explicitly-provided deterministic key validated against slot ingredients.
4. **I4**: During hydration, encountering an already-completed `KeyRequired` action with matching evidence produces `Duplicate` effect, not `Apply`.
5. **I5**: `value_digest` in `ActionCompletionEnvelope` must match the BLAKE3 digest of the encoded output value.
6. **I6**: Sequence numbers in journal events must be strictly monotonically increasing within a run.

## Forbidden States

- **F1**: `ActionTicket` with `RetrySafety::KeyRequired` and `idempotency_key == 0` (unset key).
- **F2**: `ActionReplayTracker` receiving a second `ActionScheduledTicket` for same `(action, step)` with different `ticket` (divergence).
- **F3**: Duplicate `ActionCompletedEnvelope` with same `(action, step)` but different `value_digest` (divergent completion).
- **F4**: Hydration of `KeyRequired` action without corresponding `ActionScheduledTicket` evidence.
- **F5**: `idempotency_key` derived from slots containing `Taint::Secret`, `Taint::Random`, or `Taint::TimeDependent`.

## Aggregate Boundary

**vb_storage/recovery** owns the `ActionReplayTracker` and all hydration logic. **vb_core** owns `ActionTicket`, `Idempotency`, `RetrySafety`, and the pure `compute_action_idempotency_key` function. The boundary is enforced by `vb_storage` importing `vb_core::ActionTicket` but not vice versa.