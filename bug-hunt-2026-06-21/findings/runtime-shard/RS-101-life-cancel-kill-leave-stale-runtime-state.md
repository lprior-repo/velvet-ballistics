# RS-101-life: Cancel/kill terminalize runs without clearing runtime state

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:127`
- **Confidence**: confirmed

## Description

`handle_cancel` and `handle_kill` remove the active `RunState` and record terminal outcomes, but they never remove or terminalize the corresponding `runtime_states` entry. A cancelled or killed run can remain recorded as `Initial`, `Running`, `Resuming`, or `Resumable` in the FSM map after it is terminal in `terminal_runs`.

## Evidence

`handle_cancel` clears the timer and removes the run state, then records terminal data:

```rust
self.pending_timer_remove(run);
if let Some(state) = self.run_state_remove(run) {
    self.append_journal_event(RuntimeJournalEvent::RunCancelled { run, reason })?;
    self.release_frame(state.frame);
    self.terminal_runs_insert(run);
    self.terminal_outcome_record(run, TerminalOutcome::Cancelled);
```

`handle_kill` has the same shape at `chunk_002.rs:145-152`. The FSM already has a terminal removal event in `transitions/fsm.rs:66-68`:

```rust
RuntimeEvent::TerminalRemove | RuntimeEvent::DriveFinished => {
    self.runtime_states.swap_remove(&run);
}
```

Neither cancel nor kill calls that transition.

## Adversarial Check

This is not just an alternate terminal representation. `terminal_runs` and `runtime_states` are separate stores, and `RuntimeEvent::TerminalRemove` exists specifically to remove FSM state for terminalization. The cancel/kill paths mutate `run_state` and terminal outcome stores directly, bypassing the FSM route that other terminal paths use.

## Suggested Fix

Route cancel and kill through a transactional terminal helper that also applies `RuntimeEvent::TerminalRemove`, or explicitly remove `runtime_states` after the terminal journal event succeeds. Keep the `run_state`, `terminal_runs`, `terminal_outcomes`, and `runtime_states` mutations in one ordered helper so all terminal paths share the same FSM cleanup.
