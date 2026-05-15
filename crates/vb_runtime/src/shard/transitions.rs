#![forbid(unsafe_code)]
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
        self.append_journal_event(RuntimeJournalEvent::RunFinished { run, result })?;
        self.release_frame(state.frame);
        self.discard_journal_sequence(run);
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
        // NOTE: execute_do sets ticket.capacity = retry_policy.max_attempts
        // based on the RetryPolicy passed from drive_deterministic_full.
        // This is RetryPolicy::NEVER (max_attempts = 0) for normal execution.
        // The actual workflow retry policy is in the RetryCheck node's policy_slot,
        // but that slot is uninitialized until the RetryCheck executes AFTER
        // the action completes.
        //
        // If ticket.capacity > 0, the capacity is valid and we trust it.
        // If ticket.capacity = 0, we call retry_policy_after_action to check:
        //   - retry_policy_attempts_zero: propagate (workflow has 0 max attempts)
        //   - retry_policy_slot_unreadable: slot not written yet, use ticket.capacity
        //   - other errors: propagate
        let capacity = if ticket.capacity > 0 {
            ticket.capacity
        } else {
            match crate::shard::helpers::retry_policy_after_action(&state, ticket.step) {
                Ok(policy) => policy.max_attempts,
                Err(RuntimeError::UnsupportedOperation {
                    operation: "retry_metadata_missing",
                }) => ticket.capacity,
                Err(RuntimeError::UnsupportedOperation {
                    operation: "retry_policy_slot_unreadable",
                }) => {
                    // Slot not written yet - RetryCheck hasn't run.
                    // Use ticket.capacity (0) and let RetryCheck enforce
                    // the actual policy after action completion.
                    ticket.capacity
                }
                Err(error) => return Err(error),
            }
        };
        let ticket = crate::shard::helpers::normalize_scheduled_ticket(
            &state,
            ActionTicket { capacity, ..ticket },
        )?;
        crate::shard::helpers::record_scheduled_attempt(&mut state, ticket);
        self.trace_ring
            .push(TraceEvent::ActionScheduled { run, step });
        self.append_journal_event(RuntimeJournalEvent::ActionScheduled {
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
                    self.append_journal_event(RuntimeJournalEvent::WaitScheduled { run, step })?;
                }
                PendingTimerKind::Ask => {
                    self.append_journal_event(RuntimeJournalEvent::AskScheduled { run, step })?;
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
        // Track Failed state so handle_resume can return NotResumable (not RunIdNotFound)
        self.runtime_states
            .insert(run, crate::shard::types::RuntimeState::Failed);
        self.append_journal_event(RuntimeJournalEvent::RunFailed { run })?;
        self.release_frame(state.frame);
        self.discard_journal_sequence(run);
        Ok(())
    }
}
