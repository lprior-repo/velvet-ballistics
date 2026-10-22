#![forbid(unsafe_code)]
//! Run state transition helpers: keep, finish, await action, await timer, fail.

use std::time::Instant;
use vb_core::action::ActionTicket;
use vb_core::ids::{RunId, SlotIdx, StepIdx};

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
    /// * `ResumeRollback` → `runtime_states.insert(run, RuntimeState::Resumable)` (pre-commit journal failure)
    /// * `DriveContinue` → `runtime_states.insert(run, RuntimeState::Running)`
    /// * `AwaitAction` → `runtime_states.insert(run, RuntimeState::Resumable)`
    /// * `AwaitTimer` → `runtime_states.insert(run, RuntimeState::Resumable)`
    /// * `Fail` → `runtime_states.swap_remove(&run)`
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
            RuntimeEvent::Fail | RuntimeEvent::TerminalRemove | RuntimeEvent::DriveFinished => {
                self.runtime_state_terminal_clear(run);
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
    pub(crate) fn finish_run(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        let result = match crate::shard::helpers::result_slot_for_finished_run(&state) {
            Some(slot) => slot,
            None => SlotIdx::ZERO,
        };
        // Note: StepSucceeded for the Finish step is now emitted by the evidence
        // collector during flush_evidence, before apply_drive_result is called.
        if let Err(error) = self.append_journal_event(RuntimeJournalEvent::RunFinished {
            run,
            result,
            attempt: state.run_attempt.get(),
        }) {
            return self.rollback_run_state_preserving_error("finish_run", run, state, error);
        }
        self.finish_run_after_journaled(run, state)
    }

    /// Completes successful-run cleanup after `RunFinished` is already
    /// durably appended by the caller, either as a single event or as part of
    /// a same-run atomic batch with preceding drive evidence.
    pub(crate) fn finish_run_after_journaled(
        &mut self,
        run: RunId,
        state: RunState,
    ) -> RuntimeResult<()> {
        let _removed_timer = self.pending_timer_remove(run);
        let _removed_action = self.pending_action_remove(run);
        self.clear_pending_ask_authority(run);
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
        let output = crate::shard::helpers::action_output_slot(&state, ticket.step)?;
        let input = crate::shard::helpers::action_input_slot(&state, ticket.step)?;
        let action_abi_digest = self.action_abi_digest_for_run_action(run, ticket.action)?;
        if let Err(error) = self.append_journal_event(RuntimeJournalEvent::ActionScheduledTicket {
            ticket,
            input,
            output,
            action_abi_digest,
        }) {
            self.run_state_insert(run, state)?;
            return Err(error);
        }
        self.trace_ring
            .push(TraceEvent::ActionScheduled { run, step });
        if let Err(error) = self.pending_action_insert(run, ticket) {
            self.run_state_insert(run, state)?;
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
        let register_timer = crate::shard::helpers::timer_registration_required(&state, step);
        let ask_boundary_seq = if kind == PendingTimerKind::Ask {
            self.reserve_pending_ask_authority_slot(run)?;
            Some(self.journal_sequence_for(run))
        } else {
            None
        };
        if register_timer {
            self.reserve_pending_timer_slot(run)?;
        }
        if register_timer || kind == PendingTimerKind::Ask {
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
        }
        if register_timer {
            let generation = match self.next_pending_timer_generation(run) {
                Some(generation) => generation,
                None => {
                    self.run_state_insert(run, state)?;
                    return Err(RuntimeError::InvalidTimerFire);
                }
            };
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
        if let Some(boundary_seq) = ask_boundary_seq {
            self.record_pending_ask_authority(run, step, boundary_seq);
        }
        self.run_state_insert(run, state)?;
        Ok(())
    }

    /// Marks a run as failed, releases its frame, and updates counters.
    /// Runtime state mutation is applied after the durable failure event is persisted.
    pub(crate) fn fail_run_state(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        if let Err(error) = self.append_journal_event(RuntimeJournalEvent::RunFailed {
            run,
            attempt: state.run_attempt.get(),
        }) {
            return self.rollback_run_state_preserving_error("fail_run_state", run, state, error);
        }
        let retry_step = state.frame.pc();
        self.fail_run_state_with_retry_checkpoint(run, retry_step, state)
    }

    /// Completes failed-run cleanup after `RunFailed` is already durably
    /// appended by the caller, either as a single event or as part of a
    /// same-run atomic batch with preceding failure evidence.
    pub(crate) fn fail_run_state_after_journaled(
        &mut self,
        run: RunId,
        state: RunState,
    ) -> RuntimeResult<()> {
        let _removed_timer = self.pending_timer_remove(run);
        let _removed_action = self.pending_action_remove(run);
        self.clear_pending_ask_authority(run);
        self.terminal_runs_insert(run)?;
        self.counters.inc_failed();
        self.trace_ring.push(TraceEvent::RunFailed { run });
        self.release_frame(state.frame);
        self.runtime_state_remove(run);
        self.discard_journal_sequence(run);
        self.clear_executed_step_accounting(run);
        Ok(())
    }

    /// Marks a run failed and, when retry authority is intact, captures a full
    /// [`crate::run_checkpoint::RunCheckpointV1`] and appends it as durable
    /// `RetryCheckpoint` evidence.
    ///
    /// The `RunFailed` event must already be durably appended by the caller (this is
    /// the tail of the same-run atomic batch used by `handle_action_failure`). Any
    /// capture or append failure leaves the run durably failed but without a retry
    /// checkpoint, so the run is not retryable — fail-closed, no partial authority.
    pub(crate) fn fail_run_state_with_retry_checkpoint(
        &mut self,
        run: RunId,
        retry_step: StepIdx,
        mut state: RunState,
    ) -> RuntimeResult<()> {
        let checkpoint_result = self.capture_retry_checkpoint(run, retry_step, &mut state);
        self.fail_run_state_after_journaled(run, state)?;
        checkpoint_result
    }

    /// Captures the retry checkpoint from the still-live failure state and appends the
    /// durable `RetryCheckpoint` event. Every failure path returns `Ok(())` without a
    /// checkpoint: the run is already failed and the checkpoint is an optional
    /// retry-enablement, never a fabricated authority.
    fn capture_retry_checkpoint(
        &mut self,
        run: RunId,
        retry_step: StepIdx,
        state: &mut RunState,
    ) -> RuntimeResult<()> {
        if state.frame.mark_failed(retry_step).is_err() {
            return Ok(());
        }
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let artifact_digest = admission.artifact_digest();
        let next_attempt = match state
            .run_attempt
            .get()
            .checked_add(1)
            .and_then(core::num::NonZeroU16::new)
        {
            Some(attempt) => attempt,
            None => return Ok(()),
        };
        // The `RunFailed` event is the immediately preceding durable event for this run.
        let prior_seq = match self.journal_sequence_for(run).get().checked_sub(1) {
            Some(seq) => seq,
            None => return Ok(()),
        };
        let checkpoint = match crate::run_checkpoint::RunCheckpointV1::capture(
            run,
            artifact_digest,
            next_attempt,
            retry_step,
            prior_seq,
            state,
        ) {
            Ok(checkpoint) => checkpoint,
            Err(_) => return Ok(()),
        };
        let payload = match postcard::to_allocvec(&checkpoint) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(()),
        };
        let payload_digest = *blake3::hash(&payload).as_bytes();
        self.append_journal_event(RuntimeJournalEvent::RetryCheckpoint {
            run,
            attempt: next_attempt.get(),
            step: retry_step,
            payload_digest,
            payload,
        })
    }

    fn rollback_run_state_preserving_error(
        &mut self,
        operation: &'static str,
        run: RunId,
        state: RunState,
        original: RuntimeError,
    ) -> RuntimeResult<()> {
        match self.run_state_insert(run, state) {
            Ok(_) => Err(original),
            Err(rollback) => Err(RuntimeError::rollback_failed(operation, original, rollback)),
        }
    }
}
