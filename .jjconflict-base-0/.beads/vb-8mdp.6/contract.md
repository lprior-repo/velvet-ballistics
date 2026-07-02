# Contract — Fowler/Wlaschin Style
## Idempotency Hydration for ActionTicket

---

## Contract: `compute_action_idempotency_key`

### Signature
```rust
pub fn compute_action_idempotency_key(run: RunId, seq: SeqNo, action: ActionId) -> u128
```

### Preconditions
- `run.get()` is a valid `u64` (always true — internal type)
- `seq.get()` is a valid `u64` (always true — internal type)
- `action.get()` is a valid `u32` (always true — internal type)

### Postconditions
- Returns a `u128` computed via deterministic wrapping arithmetic
- Given same `(run, seq, action)`, always returns same `u128`
- The function is NOT injective (see H1 in hazard-analysis.md)

### Invariants
- Function is pure: no side effects, no I/O, deterministic
- No panics, unwraps, or bounds checks on inputs

---

## Contract: `action_ticket_has_valid_key`

### Signature
```rust
pub fn action_ticket_has_valid_key(ticket: ActionTicket) -> bool
```

### Preconditions
- `ticket.run`, `ticket.seq`, `ticket.action` are well-formed (internal validation)

### Postconditions
- Returns `true` iff `ticket.idempotency_key == compute_action_idempotency_key(ticket.run, ticket.seq, ticket.action)`
- Returns `false` otherwise

### Invariants
- Pure function, no side effects
- No panics

---

## Contract: `validate_idempotency_key_ingredients`

### Signature
```rust
pub fn validate_idempotency_key_ingredients(
    key_slots: &[SlotIdx],
    frame: &RunFrame,
) -> Result<(), IdempotencyViolation>
```

### Preconditions
- `key_slots` is a valid slice of slot indices
- `frame` is a valid `RunFrame` with readable slots

### Postconditions
- Returns `Ok(())` if all slots in `key_slots` have `Taint::Clean`
- Returns `Err(SecretInKey(slot))` if any slot has `Taint::Secret | Taint::DerivedFromSecret`
- Returns `Err(RandomInKey(slot))` if any slot has `Taint::Random`
- Returns `Err(TimeInKey(slot))` if any slot has `Taint::TimeDependent`

### Invariants
- Pure function: only reads slot taint, does not modify state
- No panics on empty `key_slots` (returns `Ok(())`)

---

## Contract: `verify_idempotency`

### Signature
```rust
pub fn verify_idempotency(
    action: &ActionContract,
    key_slots: &[SlotIdx],
    frame: &RunFrame,
) -> Result<(), IdempotencyViolation>
```

### Preconditions
- `action` is a valid `ActionContract`
- `key_slots` is a valid slice of slot indices
- `frame` is a valid `RunFrame`

### Postconditions
- Returns `Ok(())` if:
  - `action.side_effect == SideEffect::None`, OR
  - `action.retry_safety == RetrySafety::Safe`, OR
  - `action.retry_safety == RetrySafety::KeyRequired` AND `!key_slots.is_empty()` AND all slots pass `validate_idempotency_key_ingredients`
- Returns `Err(MissingKey(...))` if:
  - `action.retry_safety == RetrySafety::KeyRequired` AND `key_slots.is_empty()`, OR
  - `action.retry_safety == RetrySafety::Unsafe`
- Returns specific `IdempotencyViolation` errors for secret/random/time-in-key

### Invariants
- Pure function: only reads, no side effects
- Does not call external services

---

## Contract: `ActionReplayTracker::new`

### Signature
```rust
pub fn new() -> Self
```

### Postconditions
- Returns `ActionReplayTracker` with empty `scheduled_tickets`, `completed`, `failed`, `completed_envelopes`
- No actions are marked as scheduled, completed, or failed

### Invariants
- All HashMaps/HashSets are empty
- Function is total: always returns valid tracker

---

## Contract: `ActionReplayTracker::mark_scheduled_ticket_effect`

### Signature
```rust
pub(crate) fn mark_scheduled_ticket_effect(
    &mut self,
    ticket: ActionTicket,
    input: SlotIdx,
    output: SlotIdx,
) -> RecoveryResult<ActionReplayEffect>
```

### Preconditions
- `ticket` is a valid `ActionTicket`
- `input` and `output` are valid `SlotIdx`

### Postconditions
- If `is_resolved(ticket.action, ticket.step)` returns `Ok(Apply)` (must NOT have `Err`)
- If `scheduled_tickets.get(&(ticket.action, ticket.step))` returns `None`: inserts evidence, returns `Ok(Apply)`
- If `scheduled_tickets.get(&(ticket.action, ticket.step))` returns `Some(existing)` AND `existing == evidence`: returns `Ok(Duplicate)`
- If `scheduled_tickets.get(&(ticket.action, ticket.step))` returns `Some(existing)` AND `existing != evidence`: returns `Err(ReplayDivergence)`

### Invariants
- `is_resolved` check is applied BEFORE duplicate detection
- `Err(NonIdempotentActionBlocked)` takes precedence over duplicate detection

---

## Contract: `ActionReplayTracker::mark_completed_envelope_effect`

### Signature
```rust
pub(crate) fn mark_completed_envelope_effect(
    &mut self,
    ticket: ActionTicket,
    output: SlotIdx,
    encoded_len: u32,
    taint: Taint,
    value_digest: [u8; 32],
) -> RecoveryResult<ActionReplayEffect>
```

### Preconditions
- `ticket` is a valid `ActionTicket`
- `output` is a valid `SlotIdx`
- `encoded_len > 0`
- `value_digest` is a valid BLAKE3 digest

### Postconditions
- If `completed_envelopes.get(&(ticket.action, ticket.step))` returns `None` AND NOT `completed.contains(key)` AND NOT `failed.contains(key)`: inserts evidence, returns `Ok(Apply)`
- If `completed_envelopes.get(&(ticket.action, ticket.step))` returns `Some(existing)` AND `existing == evidence`: returns `Ok(Duplicate)`
- If `completed_envelopes.get(&(ticket.action, ticket.step))` returns `Some(existing)` AND `existing != evidence`: returns `Err(ReplayDivergence)`
- If `completed.contains(key)` OR `failed.contains(key)`: returns `Ok(Duplicate)` (already resolved)

### Invariants
- Cannot insert evidence if already `completed` or `failed`
- `Err(ReplayDivergence)` takes precedence over `Ok(Duplicate)` when evidence differs

---

## Contract: `hydrate_snapshot_tail_preconditions`

### Signature
```rust
pub fn hydrate_snapshot_tail_preconditions(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> bool
```

### Preconditions
- `snapshot` is a valid `RunSnapshot`
- `tail_events` is a valid slice of `JournalEvent`
- `run_id` is a valid `RunId`

### Postconditions
- Returns `true` iff ALL of:
  - `hydrate_snapshot_tail_run_matches(snapshot, tail_events, run_id)` — all run IDs match
  - `hydrate_snapshot_tail_seq_after_snapshot(snapshot, tail_events)` — all tail seqs > snapshot seq
  - `hydrate_snapshot_tail_has_evidence(snapshot, tail_events)` — at least one has data

### Invariants
- Pure function, no side effects
- No panics

---

## Contract: `hydrate_run_frame`

### Signature
```rust
pub fn hydrate_run_frame(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> RecoveryResult<vb_core::RunFrame>
```

### Preconditions
- `hydrate_snapshot_tail_preconditions(snapshot, tail_events, run_id) == true`, OR
- Callers may call directly and handle `RecoveryError` for preconditions

### Postconditions
- Returns `Ok(frame)` if ALL of:
  - `validate_snapshot_metadata(snapshot.run, snapshot.seq, run_id) == Ok`
  - Snapshot bytes decode successfully
  - Dimensions derived are positive (non-zero)
  - All tail events apply successfully via `apply_tail_events`
  - `increment_executed` succeeds
- Returns `Err(RecoveryError)` for any failure:
  - `SnapshotRunMismatch` — run IDs don't match
  - `CorruptSnapshot` — decode failure
  - `FrameDimensionOverflow` — dimensions exceed u16
  - `NonIdempotentActionBlocked` — non-idempotent action replay attempted
  - `ReplayDivergence` — ticket/envelope divergence detected

### Invariants
- Atomic: either fully succeeds with valid `RunFrame` OR returns typed `RecoveryError`
- No partial state on error
- All errors are typed, no String errors

---

## Contract: `hydrate_events_preconditions`

### Signature
```rust
pub const fn hydrate_events_preconditions(events: &[JournalEvent]) -> bool
```

### Postconditions
- Returns `true` iff `!events.is_empty()`
- Returns `false` if `events` is empty

### Invariants
- Pure, total function
- No panics

---

## Global Invariants

### GI1: ActionTicket Identity
For any valid `ActionTicket`, identity is `(action, step)` — attempt number and idempotency_key are properties, not identity components.

### GI2: Tracker Key Independence
`ActionReplayTracker` keys on `(ActionId, StepIdx)` only. Two tickets with same `(action, step)` but different `attempt` or `idempotency_key` are treated as same action (potential divergence if evidence differs).

### GI3: Idempotency Key Determinism
`compute_action_idempotency_key(run, seq, action)` is deterministic: same inputs always produce same output. No time, randomness, or external state.

### GI4: Journal Sequence Strictness
For any run, all `JournalEvent` sequence numbers must be strictly monotonically increasing. No two events share the same `(run_id, seq)`.

### GI5: No Secret in Key
No `ActionTicket` with `RetrySafety::KeyRequired` may have `idempotency_key` derived from slots containing `Taint::Secret | Taint::DerivedFromSecret | Taint::Random | Taint::TimeDependent`.

### GI6: Hydration Atomicity
`hydrate_run_frame` is atomic: either returns `Ok(RunFrame)` with all state correctly reconstructed OR returns typed `Err(RecoveryError)` with no side effects.

### GI7: Value Digest Binding
`ActionCompletedEnvelope.value_digest` is the BLAKE3 digest of `value` bytes. Any mismatch in digest indicates divergence or corruption.

---

## Anti-Contracts (Forbidden Behaviors)

### AC1: Forbidden — Non-Idempotent Replay
`ActionReplayTracker` MUST block replay of `RetrySafety::Unsafe` actions. No mechanism exists to bypass this.

### AC2: Forbidden — String Error Returns
No function in the hydration path may return `Err(String)` or `Err(&str)`. All errors are typed `RecoveryError` or domain-specific error enums.

### AC3: Forbidden — Unchecked Key Derivation
`validate_idempotency_key_ingredients` MUST be called before issuing any `KeyRequired` action ticket. There is no bypass.

### AC4: Forbidden — Cross-Boundary Knowledge
`vb_core` MUST NOT contain references to `vb_storage` types. `vb_storage` MUST NOT contain logic that depends on vb_storage being the only caller of vb_core.