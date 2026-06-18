#![forbid(unsafe_code)]
//! Terminal transitions: finish run, fail run.
//!
//! - `finish_run` — marks a run as completed, releases frame, updates counters
//! - `fail_run_state` — marks a run as failed, releases frame, updates counters
//!
//! NOTE: `runtime_states` mutation (inserting Failed/removed) is handled by
//! `apply()` before these are called. These only handle cleanup.

use vb_core::ids::{RunId, SlotIdx};

use super::SnapshotWriteOutcome;
use crate::RuntimeResult;
use crate::journal::RuntimeJournalEvent;
use crate::shard::helpers::result_slot_for_finished_run;
use crate::shard::types::{RunState, Shard, TerminalOutcome};
use crate::trace::TraceEvent;

impl Shard {
    /// Marks a run as finished, releases its frame, and updates counters.
    pub(crate) fn finish_run(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        self.pending_timer_remove(run);
        self.terminal_runs_insert(run);
        self.terminal_outcome_record(run, TerminalOutcome::Completed);
        self.counters.inc_completed();
        self.counters.add_steps(state.frame.executed());
        self.trace_ring.push(TraceEvent::RunFinished { run });

        // Best-effort terminal snapshot: write before discarding the state.
        // Per C-3: snapshot failure does NOT block run completion.
        if self.snapshot_interval_steps > 0 {
            let outcome = self.write_snapshot_for_run(
                run,
                &state,
                self.snapshot_interval_steps,
                state.frame.executed(),
                state.last_snapshot_executed,
            );
            match outcome {
                SnapshotWriteOutcome::Written => {
                    // Snapshot succeeded; state is retained above for the terminal transition.
                }
                _ => {
                    // Snapshot skipped or failed — terminal transition proceeds anyway.
                }
            }
        }

        let result = match result_slot_for_finished_run(&state) {
            Some(slot) => slot,
            None => SlotIdx::ZERO,
        };
        // Note: StepSucceeded for the Finish step is now emitted by the evidence
        // collector during flush_evidence, before apply_drive_result is called.
        self.append_journal_event(RuntimeJournalEvent::RunFinished { run, result })?;
        self.release_frame(state.frame);
        self.discard_journal_sequence(run);
        Ok(())
    }

    /// Marks a run as failed, releases its frame, and updates counters.
    /// NOTE: runtime_states mutation (inserting Failed) is handled by apply() before this is called.
    /// This function only handles cleanup: pending_timers, counters, trace, journal, frame, sequence.
    pub(crate) fn fail_run_state(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        self.pending_timer_remove(run);
        self.terminal_runs_insert(run);
        self.terminal_outcome_record(run, TerminalOutcome::Failed);
        self.counters.inc_failed();
        self.trace_ring.push(TraceEvent::RunFailed { run });

        // Best-effort terminal snapshot: write before discarding the state.
        // Per C-3: snapshot failure does NOT block run completion.
        if self.snapshot_interval_steps > 0 {
            let outcome = self.write_snapshot_for_run(
                run,
                &state,
                self.snapshot_interval_steps,
                state.frame.executed(),
                state.last_snapshot_executed,
            );
            match outcome {
                SnapshotWriteOutcome::Written => {
                    // Snapshot succeeded; state is retained above for the terminal transition.
                }
                _ => {
                    // Snapshot skipped or failed — terminal transition proceeds anyway.
                }
            }
        }

        self.append_journal_event(RuntimeJournalEvent::RunFailed { run })?;
        self.release_frame(state.frame);
        self.discard_journal_sequence(run);
        Ok(())
    }
}
