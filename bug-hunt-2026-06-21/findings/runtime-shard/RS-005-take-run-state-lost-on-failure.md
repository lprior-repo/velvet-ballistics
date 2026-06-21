# RS-005: `take_run_state` / `drive_run` / `handle_timer` lose the run on intermediate failure

- **Severity**: High
- **Category**: correctness / resource leak / lost write
- **Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:81-116, 163-169`; `crates/vb_runtime/src/shard/transitions/continuation.rs:60-89, 99-152`
- **Confidence**: confirmed

## Description

Multiple lifecycle paths take a `RunState` out of `self.runs` by value, then call fallible operations (journal append, evidence flush, timer-fire advance) before re-inserting. If any of those intermediate calls returns `Err`, the state is dropped on the floor — never re-inserted into `runs`, never moved to `terminal_runs`, never given a terminal outcome. The run vanishes from shard bookkeeping while its journal entries, frame-pool slot, and any pending timer references may persist.

## Evidence

`drive_run`:

```rust
// lifecycle/chunk_002.rs:163-169
fn drive_run(&mut self, run: RunId) -> RuntimeResult<()> {
    let mut state = self.take_run_state(run)?;       // state REMOVED from runs
    let mut evidence = EvidenceCollector::new();
    let result = Self::drive_state(&mut state, …);   // ok: cannot fail (no journal)
    self.flush_evidence(run, &mut evidence)?;        // FAIL → state dropped, no restore
    self.apply_drive_result(run, state, result)      // consumes state
}
```

`handle_timer`:

```rust
// lifecycle/chunk_002.rs:81-116
pub(crate) fn handle_timer(…) -> RuntimeResult<()> {
    let Some(current_timer) = self.pending_timer_get(run) else { … };
    if !current_timer.matches_authority(…) { … }
    let mut state = self.take_run_state(run)?;        // REMOVED
    let timer = match self.pending_timer_remove(run) {
        Some(timer) => timer,
        None => { self.run_state_insert(run, state); return Err(…); }  // ← restores
    };
    crate::shard::helpers::advance_after_timer_fire(&mut state, timer)?;
        // FAIL → state dropped, NOT restored, no terminal transition
    match timer.kind {
        PendingTimerKind::Wait => {
            self.append_journal_event(…)?;            // FAIL → state dropped
        }
        PendingTimerKind::Ask => {}
    }
    let mut evidence = EvidenceCollector::new();
    let result = Self::drive_state(&mut state, …);
    self.flush_evidence(run, &mut evidence)?;         // FAIL → state dropped
    self.apply_drive_result(run, state, result)
}
```

`await_action` (continuation.rs:61-89) and `await_timer` (continuation.rs:99-152) consume `state` by value and only re-insert on the success path; on journal-append failure the state is dropped.

`handle_submit_with_inputs_contracts_and_header_mode` (chunk_001_submit.rs:198-206) is aware of this and tries to clean up:

```rust
match self.drive_run(run) {
    Ok(()) => Ok(()),
    Err(error) => {
        if !self.run_state_contains(run) {
            self.discard_journal_sequence(run);   // ← acknowledges state is gone
        }
        Err(error)
    }
}
```

But this cleanup only discards the sequence; it does **not** add the run to `terminal_runs`, does **not** record a `TerminalOutcome`, does **not** drain buffered coalesce events for the run (see RS-022), and does **not** release the frame back to the pool (the frame is dropped, leaking the pool capacity). The run is now invisible to `snapshot_run` (returns `NotFound`) but still referenced by any pending timer, by buffered journal events, and possibly by introspection handles.

`handle_action_completion` (chunk_001_action.rs:7-45) calls `drive_run(run)` at line 44 without the `discard_journal_sequence` cleanup, so its failure path leaks the sequence too.

## Adversarial Check

A defender might argue "any failure here is unrecoverable, the runtime is going down". But `RuntimeResult` is a typed error, not a panic — these are returned through `dispatch_command` and `tick()` to the caller, which observes `Ok(true)` on subsequent ticks. The shard keeps running with a hole in its state. The frame-pool capacity is consumed permanently (the `FramePool` capacity was set at construction and the dropped frame's slot is never returned). The pending timer for the run (if any) is still in `pending_timers` and a future `TimerFired` command will hit `take_run_state` → `RunNotFound`. The `handle_submit` cleanup at line 200-205 proves the authors knew state could be lost — but the cleanup is incomplete and only `handle_submit` does it; `handle_action_completion`, `handle_ask_answer`, `handle_resume`, `handle_timer` do not.

## Suggested Fix

Restructure so that the state is *owned* by `runs` for the duration of the operation (use `IndexMap::get_mut` rather than `swap_remove` + re-insert), or wrap the fallible middle section in a closure that, on `Err`, restores the state into `runs` (or transitions it through `terminal_runs` with a `Failed` outcome). The cleanest fix is to make `drive_state`, `flush_evidence`, and `apply_drive_result` operate on `&mut RunState` borrowed from `self.runs`, eliminating the take/re-insert pattern entirely.
