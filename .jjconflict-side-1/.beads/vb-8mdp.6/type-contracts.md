# Type Contracts — Idempotency Hydration for ActionTicket

## Type-Level Constraints

### `ActionTicket` (vb_core/src/action.rs:138)

```rust
pub struct ActionTicket {
    pub run: RunId,           // 64-bit run identifier
    pub step: StepIdx,        // 16-bit step index
    pub seq: SeqNo,           // 64-bit monotonic sequence
    pub action: ActionId,     // 32-bit action identifier
    pub attempt: u16,         // 1-indexed attempt number, [1, capacity]
    pub idempotency_key: u128, // deduplication key
    pub capacity: u16,        // max attempts, must be > 0
}
```

**Construction preconditions:**
- `attempt >= 1`
- `attempt <= capacity`
- `capacity >= 1`

**Key constraint (type-level via smart constructor):**
- For `RetrySafety::KeyRequired` actions: `idempotency_key` must equal `compute_action_idempotency_key(run, seq, action)` OR be explicitly provided and validated via `validate_idempotency_key_ingredients`.

### `IdempotencyKey` (u128)

**Canonical key computation** (vb_core/src/action.rs:157):
```rust
pub fn compute_action_idempotency_key(run: RunId, seq: SeqNo, action: ActionId) -> u128 {
    let run_part = u128::from(run.get());
    let seq_part = u128::from(seq.get());
    let action_part = u128::from(action.get());
    run_part.wrapping_mul(0x6c62272e07bb0143_u128)
            .wrapping_add(seq_part)
            .wrapping_mul(0x3b4f1a5b6c2d8e7f_u128)
            .wrapping_add(action_part)
            .wrapping_mul(0x5bd1e9956c7b4d3a_u128)
}
```

**Validity predicate** (vb_core/src/action.rs:171):
```rust
pub fn action_ticket_has_valid_key(ticket: ActionTicket) -> bool {
    ticket.idempotency_key == compute_action_idempotency_key(ticket.run, ticket.seq, ticket.action)
}
```

**Key ingredients validation** (vb_core/src/action.rs:347):
```rust
pub fn validate_idempotency_key_ingredients(
    key_slots: &[SlotIdx],
    frame: &RunFrame,
) -> Result<(), IdempotencyViolation>
```
- Rejects slots with `Taint::Secret | Taint::DerivedFromSecret | Taint::Random | Taint::TimeDependent`.

### `ActionReplayTracker` (vb_storage/src/recovery/types.rs:373)

```rust
pub struct ActionReplayTracker {
    scheduled_tickets: HashMap<(ActionId, StepIdx), ActionScheduleEvidence>,
    completed: HashSet<(ActionId, StepIdx)>,
    failed: HashSet<(ActionId, StepIdx)>,
    completed_envelopes: HashMap<(ActionId, StepIdx), ActionCompletionEvidence>,
}
```

**Type-level constraint**: All HashMap/HashSet keys are `(ActionId, StepIdx)` — NOT idempotency_key.

### `ActionReplayEffect` (vb_storage/src/recovery/types.rs:397)

```rust
pub(crate) enum ActionReplayEffect {
    Apply,     // Action should be re-executed during recovery
    Duplicate, // Action already completed, skip re-execution
}
```

**Transition constraint**: `mark_scheduled_ticket_effect` returns:
- `Ok(Apply)` if no prior evidence for `(action, step)`.
- `Ok(Duplicate)` if prior evidence exactly matches new ticket evidence.
- `Err(ReplayDivergence)` if prior evidence exists but differs from new ticket.
- `Err(NonIdempotentActionBlocked)` if action already resolved (completed/failed).

### Hydration Path Constraints (vb_storage/src/recovery/hydrate.rs)

**`hydrate_snapshot_tail_preconditions`** (hydrate.rs:51):
```rust
pub fn hydrate_snapshot_tail_preconditions(
    snapshot: &RunSnapshot,
    tail_events: &[JournalEvent],
    run_id: RunId,
) -> bool {
    hydrate_snapshot_tail_run_matches(snapshot, tail_events, run_id)
        && hydrate_snapshot_tail_seq_after_snapshot(snapshot, tail_events)
        && hydrate_snapshot_tail_has_evidence(snapshot, tail_events)
}
```

**`hydrate_events_preconditions`** (hydrate.rs:63):
```rust
pub const fn hydrate_events_preconditions(events: &[JournalEvent]) -> bool {
    !events.is_empty()
}
```

**`hydrate_dimensions_positive`** (hydrate.rs:69):
```rust
pub const fn hydrate_dimensions_positive(step_count: u16, slot_count: u16) -> bool {
    step_count > 0 && slot_count > 0
}
```

## Typestate Transitions

### `ActionReplayTracker` State Machine

```
                new()
                   |
                   v
            ┌─────────────────────────────────────┐
            │         UNINITIALIZED               │
            │  (no scheduled_tickets entries)      │
            └─────────────────────────────────────┘
                   |
         mark_scheduled_ticket_effect(Apply)
                   |
                   v
            ┌─────────────────────────────────────┐
            │         SCHEDULED                   │
            │  entry: (action, step) -> evidence   │
            └─────────────────────────────────────┘
                   |
      ┌────────────┴────────────┐
      |                         |
mark_completed_envelope(Apply)  mark_completed_envelope(Duplicate)
      |                         |
      v                         v
 ┌─────────┐              ┌──────────────┐
 │COMPLETED│              │  SCHEDULED   │
 │(tracked)│              │  (no change) │
 └─────────┘              └──────────────┘
```

### Recovery Hydration States

```
HYDRATION_START
     │
     │ hydrate_snapshot_tail_preconditions
     │
     ├─ false ────────────────────────────────> HYDRATION_ERROR(RecoveryError)
     │
     │ true
     │
     v
HYDRATING
     │
     │ apply_tail_events + ActionReplayTracker
     │
     ├─ NonIdempotentActionBlocked ───────────> HYDRATION_ERROR(NonIdempotentActionBlocked)
     ├─ ReplayDivergence ──────────────────────> HYDRATION_ERROR(ReplayDivergence)
     │
     v
HYDRATED(RunFrame) or ERROR
```

## Phantom Type Constraints

None — all constraints are enforced via explicit preconditions and error returns, not phantom types.

## Parser/Boundary Contracts

### `verify_idempotency` (vb_core/src/action.rs:391)
```rust
pub fn verify_idempotency(
    action: &ActionContract,
    key_slots: &[SlotIdx],
    frame: &RunFrame,
) -> Result<(), IdempotencyViolation>
```
- Returns `Ok(())` if SideEffect::None.
- Returns `Ok(())` if RetrySafety::Safe.
- Returns `Ok(())` if RetrySafety::KeyRequired AND key_slots pass `validate_idempotency_key_ingredients`.
- Returns `Err(MissingKey)` if RetrySafety::KeyRequired AND key_slots.is_empty().
- Returns `Err(MissingKey)` if RetrySafety::Unsafe.
- Rejects `SecretInKey`, `RandomInKey`, `TimeInKey` violations.

### `validate_action_dispatch` (vb_core/src/action.rs:420)
- Input slot must be readable (populated, within bounds).
- Output slot must be writable (within frame slot_count bounds).
- Contract action ID must match provided ID.