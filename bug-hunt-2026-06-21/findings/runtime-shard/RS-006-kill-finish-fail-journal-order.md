# RS-006: `handle_kill` and `finish_run` / `fail_run_state` mutate state and counters before persisting the journal event

- **Severity**: Medium
- **Category**: correctness / durability ordering
- **Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:140-156`; `crates/vb_runtime/src/shard/transitions/terminal.rs:21-58, 64-95`
- **Confidence**: confirmed

## Description

`handle_kill`, `finish_run`, and `fail_run_state` mutate in-memory structures (`terminal_runs`, `terminal_outcomes`, `counters`, `trace_ring`) **before** appending the durable `RunKilled` / `RunFinished` / `RunFailed` journal event. If the journal append fails, the shard's in-memory state diverges from the durable journal: the run is recorded as terminal in memory but the journal has no record of the terminal transition. On crash-recovery the run appears to still be in its last pre-terminal state.

## Evidence

`handle_kill` (chunk_002.rs:140-156) — note the order:

```rust
if let Some(state) = self.run_state_remove(run) {
    self.release_frame(state.frame);              // (1) frame returned to pool
    self.terminal_runs_insert(run);               // (2) terminal set mutated
    self.terminal_outcome_record(run, TerminalOutcome::Killed);  // (3)
    self.counters.inc_failed();                   // (4)
    self.trace_ring.push(TraceEvent::RunKilled { run });          // (5)
    self.append_journal_event(RuntimeJournalEvent::RunKilled { run, reason })?;  // (6) durable
}
```

If line (6) returns `Err`, all of (1)-(5) have already happened. The journal has no `RunKilled` event. The run is in `terminal_runs` (memory says killed) but the durable journal says the run was last seen active. Contrast with `handle_cancel` (chunk_002.rs:118-138), which correctly journals first:

```rust
if let Some(state) = self.run_state_remove(run) {
    self.append_journal_event(RuntimeJournalEvent::RunCancelled { run, reason })?;  // durable FIRST
    self.release_frame(state.frame);
    self.terminal_runs_insert(run);
    …
}
```

`finish_run` (transitions/terminal.rs:21-58):

```rust
fn finish_run(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
    self.pending_timer_remove(run);
    self.terminal_runs_insert(run);               // (1)
    self.terminal_outcome_record(run, TerminalOutcome::Completed);  // (2)
    self.counters.inc_completed();                // (3)
    self.counters.add_steps(state.frame.executed());
    self.trace_ring.push(TraceEvent::RunFinished { run });
    if self.snapshot_interval_steps > 0 {
        let outcome = self.write_snapshot_for_run(…);  // (4) writes a snapshot with seq
        …
    }
    let result = match result_slot_for_finished_run(&state) { … };
    self.append_journal_event(RuntimeJournalEvent::RunFinished { run, result })?;  // (5) durable
    self.release_frame(state.frame);
    self.discard_journal_sequence(run);
    Ok(())
}
```

`fail_run_state` (terminal.rs:64-95) follows the same anti-pattern.

## Adversarial Check

A defender might argue that `append_journal_event` rarely fails and the failure is unrecoverable anyway. But the journal is the authoritative recovery log per the durability matrix (`durability_matrix.rs:21-29`: "AfterJournalAppend" is the required ack point). The whole point of "ack after journal append" is that *if* the append fails, in-memory state must remain consistent with the (untouched) journal. The ordering here directly violates AckPoint::AfterJournalAppend for kill/finish/fail. The fact that `handle_cancel` gets the order right proves the codebase knows the correct pattern.

A second defender argument: "snapshot before RunFinished is intentional so the snapshot's seq < RunFinished's seq." That part is fine — the issue is the *non-journal* mutations (terminal_runs, terminal_outcomes, counters, trace_ring) that happen before the durable event.

## Suggested Fix

Reorder all three functions to journal first, then mutate:

```rust
fn finish_run(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
    let result = match result_slot_for_finished_run(&state) {
        Some(slot) => slot, None => SlotIdx::ZERO,
    };
    if self.snapshot_interval_steps > 0 {
        let _ = self.write_snapshot_for_run(…);   // best-effort, before RunFinished seq
    }
    self.append_journal_event(RuntimeJournalEvent::RunFinished { run, result })?;
    // Now perform in-memory side effects.
    self.pending_timer_remove(run);
    self.terminal_runs_insert(run);
    self.terminal_outcome_record(run, TerminalOutcome::Completed);
    self.counters.inc_completed();
    self.counters.add_steps(state.frame.executed());
    self.trace_ring.push(TraceEvent::RunFinished { run });
    self.release_frame(state.frame);
    self.discard_journal_sequence(run);
    Ok(())
}
```

Mirror the same change in `handle_kill` and `fail_run_state`.
