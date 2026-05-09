//! Run state transition helpers: keep, finish, await action, await timer, fail.

use vb_core::action::ActionTicket;
use vb_core::ids::{RunId, SlotIdx};

use crate::journal::RuntimeJournalEvent;
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

use crate::shard::types::{PendingTimer, PendingTimerKind, RunState, Shard};

impl Shard {
    /// Re-inserts a run that has remaining work into the active runs map.
    pub(crate) fn keep_run(&mut self, run: RunId, state: RunState) {
        self.counters.add_steps(state.frame.executed());
        self.runs.insert(run, state);
    }

    /// Marks a run as finished, releases its frame, and updates counters.
    pub(crate) fn finish_run(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        self.pending_timers.swap_remove(&run);
        self.counters.inc_completed();
        self.counters.add_steps(state.frame.executed());
        self.trace_ring.push(TraceEvent::RunFinished { run });
        let result = match crate::shard::helpers::result_slot_for_finished_run(&state) {
            Some(slot) => slot,
            None => SlotIdx::ZERO,
        };
        // Note: StepSucceeded for the Finish step is now emitted by the evidence
        // collector during flush_evidence, before apply_drive_result is called.
        self.journal
            .append(RuntimeJournalEvent::RunFinished { run, result })?;
        self.release_frame(state.frame);
        Ok(())
    }

    /// Transitions a run to awaiting an external action response.
    pub(crate) fn await_action(
        &mut self,
        run: RunId,
        mut state: RunState,
        ticket: ActionTicket,
    ) -> RuntimeResult<()> {
        self.counters.add_steps(state.frame.executed());
        let step = state.frame.pc();
        let capacity = match crate::shard::helpers::retry_policy_after_action(&state, ticket.step) {
            Ok(policy) => policy.max_attempts,
            Err(RuntimeError::UnsupportedOperation {
                operation: "retry_metadata_missing",
            }) => ticket.capacity,
            Err(error) => return Err(error),
        };
        let ticket = crate::shard::helpers::normalize_scheduled_ticket(
            &state,
            ActionTicket { capacity, ..ticket },
        )?;
        crate::shard::helpers::record_scheduled_attempt(&mut state, ticket);
        self.trace_ring
            .push(TraceEvent::ActionScheduled { run, step });
        self.journal.append(RuntimeJournalEvent::ActionScheduled {
            run,
            step,
            action: ticket.action,
        })?;
        self.runs.insert(run, state);
        Ok(())
    }

    /// Transitions a run to awaiting a timer (wait or ask timeout).
    pub(crate) fn await_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
    ) -> RuntimeResult<()> {
        self.counters.add_steps(state.frame.executed());
        let step = state.frame.pc();
        if crate::shard::helpers::timer_registration_required(&state, step) {
            self.pending_timers.insert(run, PendingTimer { step, kind });
            match kind {
                PendingTimerKind::Wait => {
                    self.journal
                        .append(RuntimeJournalEvent::WaitScheduled { run, step })?;
                }
                PendingTimerKind::Ask => {
                    self.journal
                        .append(RuntimeJournalEvent::AskScheduled { run, step })?;
                }
            }
        }
        self.runs.insert(run, state);
        Ok(())
    }

    /// Marks a run as failed, releases its frame, and updates counters.
    pub(crate) fn fail_run_state(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        self.pending_timers.swap_remove(&run);
        self.counters.inc_failed();
        self.trace_ring.push(TraceEvent::RunFailed { run });
        self.journal
            .append(RuntimeJournalEvent::RunFailed { run })?;
        self.release_frame(state.frame);
        Ok(())
    }
}
