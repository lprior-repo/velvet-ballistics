#![forbid(unsafe_code)]
//! Run state transition helpers: keep, finish, await action, await timer, fail.

use std::time::Instant;
use vb_core::action::ActionTicket;
use vb_core::ids::{RunId, SlotIdx};

use crate::journal::RuntimeJournalEvent;
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

use crate::shard::types::{
    PendingTimer, PendingTimerKind, RunState, RuntimeEvent, RuntimeState, Shard,
};

impl Shard {
    /// Applies a RuntimeEvent to mutate runtime_states.
    ///
    /// This is the single routing method for all runtime_states mutations,
    /// replacing direct insert/swap_remove call sites.
    ///
    /// # Arguments
    /// * `run` - The run identifier
    /// * `event` - The runtime event variant
    ///
    /// # State Transitions
    /// * `Submit` → `runtime_states.insert(run, RuntimeState::Initial)`
    /// * `Resume` → `runtime_states.insert(run, RuntimeState::Resuming)`
    /// * `ResumeRollback` → `runtime_states.insert(run, RuntimeState::Resumable)` (journal failure)
    /// * `DriveContinue` → `runtime_states.insert(run, RuntimeState::Running)`
    /// * `AwaitAction` → `runtime_states.insert(run, RuntimeState::Resumable)`
    /// * `AwaitTimer` → `runtime_states.insert(run, RuntimeState::Resumable)`
    /// * `Fail` → `runtime_states.insert(run, RuntimeState::Failed)`
    /// * `TerminalRemove` → `runtime_states.swap_remove(&run)`
    /// * `DriveFinished` → `runtime_states.swap_remove(&run)`
    ///
    /// # Flux refinement (PO-vb282my-RS-FLUX-001):
    /// RuntimeState FSM contract:
    /// - `Resume` transition requires `runtime_states[run] == Resumable`
    /// - `ResumeRollback` transition ensures `runtime_states[run] == Resumable`
    /// - `Running` state rejects repeated `Resume` transitions
    ///
    /// Flux signature (requires flux-rs toolchain):
    /// ```flux
    /// #[flux_rs::sig(fn(&mut Shard, run: RunId, event: RuntimeEvent)
    ///     requires event == Resume => runtime_states[run] == Resumable,
    ///     ensures event == ResumeRollback => runtime_states[run] == Resumable
    /// )]
    /// ```
    pub(crate) fn apply(&mut self, run: RunId, event: RuntimeEvent) -> RuntimeResult<()> {
        match event {
            RuntimeEvent::Submit => {
                self.runtime_state_insert(run, RuntimeState::Initial)?;
            }
            RuntimeEvent::Resume => {
                self.runtime_state_insert(run, RuntimeState::Resuming)?;
            }
            RuntimeEvent::ResumeRollback => {
                // Journal append failed during resume, revert to Resumable
                self.runtime_state_insert(run, RuntimeState::Resumable)?;
            }
            RuntimeEvent::DriveContinue => {
                self.runtime_state_insert(run, RuntimeState::Running)?;
            }
            RuntimeEvent::AwaitAction | RuntimeEvent::AwaitTimer => {
                self.runtime_state_insert(run, RuntimeState::Resumable)?;
            }
            RuntimeEvent::Fail => {
                self.runtime_state_insert(run, RuntimeState::Failed)?;
            }
            RuntimeEvent::TerminalRemove | RuntimeEvent::DriveFinished => {
                self.runtime_state_remove(run);
            }
        }
        Ok(())
    }

    /// Re-inserts a run that has remaining work into the active runs map.
    pub(crate) fn keep_run(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        self.add_executed_step_delta(run, state.frame.executed());
        self.run_state_insert(run, state)?;
        Ok(())
    }

    /// Marks a run as finished, releases its frame, and updates counters.
    #[allow(clippy::let_underscore_must_use)]
    pub(crate) fn finish_run(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        let result = match crate::shard::helpers::result_slot_for_finished_run(&state) {
            Some(slot) => slot,
            None => SlotIdx::ZERO,
        };
        // Note: StepSucceeded for the Finish step is now emitted by the evidence
        // collector during flush_evidence, before apply_drive_result is called.
        if let Err(error) =
            self.append_journal_event(RuntimeJournalEvent::RunFinished { run, result })
        {
            // Best-effort rollback; the original `error` from the journal
            // append is the one to surface. The rollback result is dropped
            // intentionally via `let _` (see the `#[allow]` on this fn).
            let _ = self.run_state_insert(run, state);
            return Err(error);
        }
        self.pending_timer_remove(run);
        self.terminal_runs_insert(run)?;
        self.counters.inc_completed();
        self.add_executed_step_delta(run, state.frame.executed());
        self.trace_ring.push(TraceEvent::RunFinished { run });
        self.release_frame(state.frame);
        self.discard_journal_sequence(run);
        self.clear_executed_step_accounting(run);
        Ok(())
    }

    /// Transitions a run to awaiting an external action response.
    pub(crate) fn await_action(
        &mut self,
        run: RunId,
        mut state: RunState,
        ticket: ActionTicket,
    ) -> RuntimeResult<()> {
        self.add_executed_step_delta(run, state.frame.executed());
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
        let output = crate::shard::helpers::action_output_slot(&state, ticket.step)?;
        let input = crate::shard::helpers::action_input_slot(&state, ticket.step)?;
        self.append_journal_event(RuntimeJournalEvent::ActionScheduledTicket {
            ticket,
            input,
            output,
        })?;
        if let Err(error) = self.pending_action_insert(run, ticket) {
            let _ = self.run_state_insert(run, state);
            return Err(error);
        }
        self.run_state_insert(run, state)?;
        Ok(())
    }

    /// Transitions a run to awaiting a timer (wait or ask timeout).
    pub(crate) fn await_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
    ) -> RuntimeResult<()> {
        self.add_executed_step_delta(run, state.frame.executed());
        let step = state.frame.pc();
        if crate::shard::helpers::timer_registration_required(&state, step) {
            let generation = match self.next_pending_timer_generation(run) {
                Some(generation) => generation,
                None => {
                    self.run_state_insert(run, state)?;
                    return Err(RuntimeError::InvalidTimerFire);
                }
            };
            self.reserve_pending_timer_slot(run)?;
            let append_result = match kind {
                PendingTimerKind::Wait => {
                    self.append_journal_event(RuntimeJournalEvent::WaitScheduled { run, step })
                }
                PendingTimerKind::Ask => {
                    self.append_journal_event(RuntimeJournalEvent::AskScheduled { run, step })
                }
            };
            if let Err(error) = append_result {
                self.run_state_insert(run, state)?;
                return Err(error);
            }
            self.pending_timer_insert(
                run,
                PendingTimer {
                    step,
                    kind,
                    generation,
                    deadline: Instant::now(),
                },
            )?;
        }
        self.run_state_insert(run, state)?;
        Ok(())
    }

    /// Marks a run as failed, releases its frame, and updates counters.
    /// Runtime state mutation is applied after the durable failure event is persisted.
    #[allow(clippy::let_underscore_must_use)]
    pub(crate) fn fail_run_state(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        if let Err(error) = self.append_journal_event(RuntimeJournalEvent::RunFailed { run }) {
            let _ = self.run_state_insert(run, state);
            return Err(error);
        }
        self.pending_timer_remove(run);
        self.terminal_runs_insert(run)?;
        self.counters.inc_failed();
        self.trace_ring.push(TraceEvent::RunFailed { run });
        self.release_frame(state.frame);
        self.runtime_state_remove(run);
        self.discard_journal_sequence(run);
        self.clear_executed_step_accounting(run);
        Ok(())
    }
}
