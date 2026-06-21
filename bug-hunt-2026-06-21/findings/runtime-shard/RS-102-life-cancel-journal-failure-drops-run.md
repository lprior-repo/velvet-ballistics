# RS-102-life: Cancel drops the active run before the cancel event is durable

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:127`
- **Confidence**: confirmed

## Description

`handle_cancel` removes the pending timer and active run state before appending `RunCancelled`. If the journal append fails, `?` returns immediately and the run is neither reinserted nor terminalized.

## Evidence

The destructive mutations happen before the fallible journal append:

```rust
self.pending_timer_remove(run);
if let Some(state) = self.run_state_remove(run) {
    self.append_journal_event(RuntimeJournalEvent::RunCancelled { run, reason })?;
    self.release_frame(state.frame);
    self.terminal_runs_insert(run);
```

The cleanup and terminal recording are all after the `?`, and `discard_journal_sequence(run)` at `chunk_002.rs:136` is also skipped on append failure.

## Adversarial Check

This is not a harmless error return. The function has already removed both the timer authority and the `RunState` before it can return the append error. The local `state` is dropped on the error path, and there is no rollback or `release_frame` call before returning.

## Suggested Fix

Make cancel terminalization transactional. Either append the durable cancel record before removing live state, or use a guard that restores the pending timer and `RunState` on append failure. Only release the frame, remove FSM state, and record terminal outcome after the terminal event is durable.
