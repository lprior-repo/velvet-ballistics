# RS-011: `handle_cancel` and `handle_kill` call `discard_journal_sequence` without draining the coalesce buffer for the cancelled run

- **Severity**: Medium
- **Category**: correctness / lost write / ghost events
- **Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:118-156`
- **Confidence**: confirmed

## Description

`handle_cancel` and `handle_kill` both call `self.discard_journal_sequence(run)` at the end, which removes the per-run sequence tracker from `journal_sequences`. But they do **not** remove any events already buffered for that run in `coalesce_buffer`. When the next coalesce flush fires, those buffered events are written to the journal for a run that has been recorded as cancelled/killed — producing "ghost" events in the durable log for a terminal run, and re-using a sequence number that was just discarded.

## Evidence

```rust
// chunk_002.rs:118-138 (handle_cancel)
pub(crate) fn handle_cancel(&mut self, run: RunId, reason: Option<String>) -> RuntimeResult<()> {
    if !self.run_state_contains(run) && !self.terminal_runs_contains(run) {
        return Err(RuntimeError::RunNotFound);
    }
    self.pending_timer_remove(run);
    if let Some(state) = self.run_state_remove(run) {
        self.append_journal_event(RuntimeJournalEvent::RunCancelled { run, reason })?;
        self.release_frame(state.frame);
        self.terminal_runs_insert(run);
        self.terminal_outcome_record(run, TerminalOutcome::Cancelled);
        self.counters.inc_failed();
        self.trace_ring.push(TraceEvent::RunCancelled { run });
    }
    self.discard_journal_sequence(run);    // ← removes journal_sequences[run]
    Ok(())
}
```

Suppose `coalesce_window_ticks = 10` and the run was driven earlier in the current window, accumulating buffered events. Sequence of events:

1. Tick 1: `dispatch_submit(run=A)` → `append_journal_event(RunSubmitted)` buffers `(RunSubmitted, seq=0)`, `journal_sequences[A] = 1`.
2. Tick 2: `dispatch_command(ActionCompleted{A,…})` → `append_journal_event(ActionCompletedEnvelope)` buffers `(ACE, seq=1)`, `journal_sequences[A] = 2`.
3. Tick 3: `dispatch_command(Cancel{A})` → `handle_cancel` appends `RunCancelled` (buffered, seq=2), removes `journal_sequences[A]`.
4. Tick 4 (or end of window): `flush_coalesce_buffer` writes all four events for run A (including post-cancel? No, but: it writes RunCancelled at seq=2 alongside the earlier events). Run A is now recorded in the journal with `RunSubmitted`, `ActionCompletedEnvelope`, `RunCancelled` in the *same* flush, but in memory `journal_sequences[A]` is gone. If a subsequent `Submit{A}` reuses the run id, the new submit starts at seq=0 again — but the durable journal already has seq=0 for the previous incarnation. Sequence clash.

The same applies to `handle_kill` and to the `discard_journal_sequence` call in `handle_submit_with_inputs_contracts_and_header_mode` (`chunk_001_submit.rs:202`) when the run was lost during drive.

## Adversarial Check

A defender might argue "the buffered events were valid at the time they were buffered, so flushing them later is correct." That conflates two issues:

1. **Sequence reuse**: `discard_journal_sequence(run)` removes the in-memory next-seq tracker. A new `Submit{run}` will compute `seq = journal_sequence_for(run) = ZERO`, then write `RunSubmitted` at seq=0 — but the durable journal already has the prior incarnation's `RunSubmitted` at seq=0. Even if the journal keyspace tolerates dup-seq writes (Fjall would overwrite or panic), the replay log is corrupted.

2. **Post-terminal events**: The cancel handler itself appends `RunCancelled` to the buffer (line 129) *before* calling `discard_journal_sequence` (line 136). So the buffer at flush time contains `RunCancelled`, which is correct. But any *future* event for this run that arrives before the flush (e.g. a late `ActionCompleted` for a ticket that was issued before cancel) will be appended to the buffer with a sequence derived from the now-discarded tracker, mixing cancelled-run events with live-run events.

## Suggested Fix

When discarding the sequence for a cancelled/killed run, also drain the coalesce buffer of any pending events for that run, *after* appending the terminal event:

```rust
// After discard_journal_sequence(run):
self.coalesce_buffer.retain(|(ev, _)| ev.run_id() != run);
```

Or, more structurally, give each coalesce-buffer entry an "epoch" tag and reject events from epochs older than the run's terminal epoch on flush.
