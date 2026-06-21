# RS-010: `handle_legacy_action_completion` mutates frame before journal append — on journal failure state and journal diverge

- **Severity**: Medium
- **Category**: correctness / durability ordering
- **Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_001_action.rs:47-68`
- **Confidence**: confirmed

## Description

`handle_legacy_action_completion` calls `frame.mark_succeeded(step)` *before* appending the `StepSucceeded` journal event. If the journal append fails, the in-memory frame shows the step as Succeeded but the durable journal has no record of the transition. On recovery the step appears not-succeeded.

## Evidence

```rust
// lifecycle/chunk_001_action.rs:47-68
pub(crate) fn handle_legacy_action_completion(
    &mut self, run: RunId, step: StepIdx,
) -> RuntimeResult<()> {
    let state = self.run_state_get_mut(run).ok_or(RuntimeError::RunNotFound)?;
    state
        .frame
        .mark_succeeded(step)                                // (1) state mutation
        .map_err(|_| RuntimeError::RunNotFound)?;
    self.trace_ring
        .push(TraceEvent::ActionCompleted { run, step });    // (2) trace
    self.append_journal_event(RuntimeJournalEvent::StepSucceeded {
        run, step,
        output: SlotIdx::ZERO,
        attempt: 1,
    })?;                                                      // (3) durable — may fail
    self.drive_run(run)
}
```

If line (3) returns `Err`:
- The frame is already marked Succeeded in memory.
- The trace ring already has `ActionCompleted`.
- The journal has no `StepSucceeded` event.
- The error propagates up; if it reaches `drive_run`'s caller, the run is left in an inconsistent state (frame says Succeeded, journal does not).

Compare with `handle_action_completion` (same file, lines 7-45) which correctly journals the `ActionCompletedEnvelope` *before* calling `mark_succeeded`. The legacy path was missed when the ordering was fixed for the new path.

## Adversarial Check

A defender might argue "legacy" means this code path is deprecated and not used in production. But the variant `ShardCommand::ActionCompletedLegacy { run, step }` is part of the public `ShardCommand` enum (`command.rs:84-89`), dispatched through `dispatch_command` (`impl_parts/dispatch.rs:111-113`), and has no `#[deprecated]` annotation. It is a live production code path that callers can use.

The "legacy" in the name refers to the wire-format compatibility (no typed output payload), not to the function being obsolete. The ordering bug is therefore reachable by any caller that submits a legacy action-completion command.

## Suggested Fix

Reorder: append journal first, then mutate frame. If the journal append fails, the frame is untouched and the run remains in its prior (Running) state, consistent with the journal:

```rust
pub(crate) fn handle_legacy_action_completion(
    &mut self, run: RunId, step: StepIdx,
) -> RuntimeResult<()> {
    // Probe state exists but do not mutate yet.
    let state = self.run_state_get_mut(run).ok_or(RuntimeError::RunNotFound)?;
    if state.frame.step_state(step) != Ok(StepState::Running) {
        return Err(RuntimeError::RunNotFound);
    }
    // Append journal first.
    self.append_journal_event(RuntimeJournalEvent::StepSucceeded {
        run, step, output: SlotIdx::ZERO, attempt: 1,
    })?;
    // Now mutate.
    let state = self.run_state_get_mut(run).ok_or(RuntimeError::RunNotFound)?;
    state.frame.mark_succeeded(step).map_err(|_| RuntimeError::RunNotFound)?;
    self.trace_ring.push(TraceEvent::ActionCompleted { run, step });
    self.drive_run(run)
}
```
