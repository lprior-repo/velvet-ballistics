P0-5b2 recover-pending-actions: Add pub fn pending_actions_from_events in vb_storage::recovery::replay::summary that delegates to the existing private recovered_pending_actions

# Verification excerpts (read-before-write)

## crates/vb_storage/src/recovery/recover.rs (296 lines)
- Line 251-260: `pub fn recover_runtime_frame_seed(journal: &FjallJournal, run: RunId) -> RecoveryResult<RecoveryFrameSeed>` — PUBLIC. Real signature: `(journal, run)` NOT `(events)`.

## crates/vb_storage/src/recovery/replay/summary.rs (985 lines)
- Line 814-821: PRIVATE `fn recovered_pending_actions(pending_actions: HashSet<(ActionId, StepIdx)>) -> Vec<RecoveredPendingAction>`. Real signature: `(HashSet<(ActionId, StepIdx)>) -> Vec<RecoveredPendingAction>` NOT `(events: &[JournalEvent]) -> Vec<ActionTicket>`.

## crates/vb_storage/src/recovery/types.rs (657 lines)
- Line 290-297: `pub struct RecoveredPendingAction { pub step: StepIdx, pub action: ActionId }` — 2 fields, NOT an ActionTicket (which has run, step, seq, action, attempt, idempotency_key, capacity).
- Line 377-396: `pub struct RecoveryFrameSeed { ..., pub pending_actions: Vec<RecoveredPendingAction>, ... }` — the pending_actions are ALREADY exposed on the seed returned by `recover_runtime_frame_seed`.

# Scope (verified, no fabrication)

The data is already exposed. The actual gap is that `Runtime::recover` (at `crates/vb_runtime/src/runtime/mod.rs:343-362`, `#[cfg(feature = "test-util")]`) does NOT call `recover_runtime_frame_seed` — it calls `recover_all_incomplete_runs` which uses the seed internally. So pending_actions are not surfaced to the runtime.

This bead adds: `pub fn pending_actions_from_events(events: &[JournalEvent]) -> Vec<RecoveredPendingAction>` in `vb_storage::recovery::replay::summary` (NOT a new trait method). It delegates to the existing private `recovered_pending_actions` for test/observability use.

# Implementation

In `crates/vb_storage/src/recovery/replay/summary.rs` after line 821:
```rust
/// Public accessor for tests and observability.
/// Delegates to the private `recovered_pending_actions`.
/// Use `recover_runtime_frame_seed` to get the full seed including pending_actions.
pub fn pending_actions_from_events(events: &[JournalEvent]) -> Vec<RecoveredPendingAction> {
    let accumulator = recover_pending_actions_from_events_inner(events);
    recovered_pending_actions(accumulator.pending_actions)
}
```

# Acceptance test

In `crates/vb_storage/src/recovery_unit_tests.rs`:
```rust
#[test]
fn pending_actions_from_events_returns_collected_actions_in_set_order() {
    // Build a journal with 5 ActionScheduled events and 3 ActionCompleted events.
    // Call pending_actions_from_events(&events).
    // Assert the result has 2 entries (the uncompleted ones).
}
```

# Anti-hallucination guards

- DO NOT add a new function `recover_pending_actions(events: &[JournalEvent]) -> Vec<ActionTicket>` — the real signature is different.
- DO NOT add a new trait method.
- DO NOT use `ActionTicket` as the result type — use `RecoveredPendingAction` (the real 2-field struct).
- The public accessor is `pending_actions_from_events` (with `_from_events` suffix to make the input clear).

# Kani harness (skipped — no arithmetic; recovery is event-driven)

This is an observability helper. The hard arithmetic bounds are already proven in the existing `recover_runtime_frame_seed` harnesses. No new Kani needed.
