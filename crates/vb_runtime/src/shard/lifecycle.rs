//! Run lifecycle management: submit, resume, cancel, action completion, timers.

use vb_core::action::{
    ActionFailure, ActionOutputReady, ActionTicket, RetryPolicy as VbCoreRetryPolicy,
};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::workflow::CompiledWorkflow;
use vb_core::ValueStore;

use crate::engine::{
    EvidenceCollector, RetryPolicy, RuntimeEngineResult, RuntimeSignal, drive_deterministic_full,
};
use crate::journal::RuntimeJournalEvent;
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

use crate::primitives::collect::CollectStates;
use crate::shard::types::{AskAnswer, PendingTimerKind, RunState, Shard};

impl Shard {
    pub(crate) fn handle_submit(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.handle_submit_with_inputs(run, workflow, &[], caps)
    }

    pub(crate) fn handle_submit_with_inputs(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: &[(SlotIdx, SlotValue)],
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        if self.runs.contains_key(&run) {
            return Err(RuntimeError::RunAlreadyExists);
        }
        if self.runs.len() >= self.max_active_runs {
            return Err(RuntimeError::ActiveRunCapacityExceeded {
                capacity: self.max_active_runs,
            });
        }
        let digest = workflow.digest();
        let admission = self.build_admission(run, digest, caps)?;
        let mut frame = self.take_frame_for(run, &workflow)?;
        crate::shard::helpers::seed_input_slots(&mut frame, inputs)?;
        self.trace_ring.push(TraceEvent::RunSubmitted { run });
        self.journal.append(RuntimeJournalEvent::RunSubmitted {
            run,
            workflow: digest,
        })?;
        self.counters.inc_submitted();
        let frame_step_count = frame.step_count();
        let state = RunState {
            frame,
            workflow,
            store: ValueStore::with_max_slots(workflow.resource_contract().max_slots),
            action_attempts: crate::shard::helpers::new_action_attempts(frame_step_count),
            admission,
            collect_states: CollectStates::new(),
        };
        self.runs.insert(run, state);
        self.drive_run(run)?;
        Ok(())
    }

    fn build_admission(
        &self,
        run: RunId,
        digest: vb_core::ids::WorkflowDigest,
        caps: CapabilitySet,
    ) -> RuntimeResult<Option<crate::admission::RunAdmission>> {
        use crate::admission::{AdmissionError, admit_run};

        match admit_run(self.artifact_store.as_ref(), self.policy, digest, run, caps) {
            Ok(admission) => Ok(Some(admission)),
            Err(AdmissionError::ArtifactNotFound { digest }) => {
                Err(RuntimeError::AdmissionArtifactNotFound { digest })
            }
            Err(AdmissionError::CapabilityDenied {
                action,
                required,
                granted,
            }) => Err(RuntimeError::AdmissionCapabilityDenied {
                action,
                required,
                granted,
            }),
        }
    }

    pub(crate) fn handle_resume(&mut self, run: RunId) -> RuntimeResult<()> {
        self.drive_run(run)
    }

    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn handle_action_completion(
        &mut self,
        ticket: ActionTicket,
        output: ActionOutputReady,
    ) -> RuntimeResult<()> {
        let run = ticket.run;
        let state = self.runs.get_mut(&run).ok_or(RuntimeError::RunNotFound)?;
        crate::shard::helpers::validate_action_completion(state, ticket)?;
        state
            .frame
            .write_slot_with_taint(output.output_slot, output.value, output.taint)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        state
            .frame
            .mark_succeeded(ticket.step)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        crate::shard::helpers::advance_after_action_completion(state, ticket.step)?;
        self.trace_ring.push(TraceEvent::SlotWritten {
            run,
            slot: output.output_slot,
        });
        self.trace_ring.push(TraceEvent::ActionCompleted {
            run,
            step: ticket.step,
        });
        let encoded_value =
            postcard::to_allocvec(&output.value).map_err(|_| RuntimeError::EncodeFailed)?;
        self.journal.append(RuntimeJournalEvent::SlotWritten {
            run,
            slot: output.output_slot,
            value: encoded_value,
        })?;
        self.journal.append(RuntimeJournalEvent::StepSucceeded {
            run,
            step: ticket.step,
            output: output.output_slot,
        })?;
        self.journal.append(RuntimeJournalEvent::ActionCompleted {
            run,
            step: ticket.step,
            action: ticket.action,
        })?;
        self.drive_run(run)
    }

    pub(crate) fn handle_legacy_action_completion(
        &mut self,
        run: RunId,
        step: StepIdx,
    ) -> RuntimeResult<()> {
        let state = self.runs.get_mut(&run).ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_succeeded(step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        self.trace_ring
            .push(TraceEvent::ActionCompleted { run, step });
        // Evidence chain: emit StepSucceeded for legacy action completion.
        // Legacy path has no output slot information.
        self.journal.append(RuntimeJournalEvent::StepSucceeded {
            run,
            step,
            output: SlotIdx::ZERO,
        })?;
        self.drive_run(run)
    }

    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn handle_action_failure(
        &mut self,
        ticket: ActionTicket,
        failure: ActionFailure,
    ) -> RuntimeResult<()> {
        let run = ticket.run;
        let mut retry_now = false;
        let mut fail_without_handler = false;
        {
            let state = self.runs.get_mut(&run).ok_or(RuntimeError::RunNotFound)?;
            crate::shard::helpers::validate_action_completion(state, ticket)?;
            if failure.retry_policy == VbCoreRetryPolicy::Retryable
                && crate::shard::helpers::retry_metadata_exists(state, ticket.step)
            {
                let policy = crate::shard::helpers::retry_policy_after_action(state, ticket.step)?;
                self.trace_ring.push(TraceEvent::ActionFailed {
                    run,
                    step: ticket.step,
                    code: failure.code,
                });
                if crate::shard::helpers::record_retry_attempt(state, ticket, policy)? {
                    state
                        .frame
                        .set_pc(ticket.step)
                        .map_err(|_| RuntimeError::InvalidActionCompletion)?;
                    retry_now = true;
                }
            }
            if !retry_now {
                match crate::shard::helpers::find_error_handler_for_failure(
                    &state.workflow,
                    ticket.step,
                ) {
                    Some((handler, error_slot)) => {
                        state
                            .frame
                            .mark_failed(ticket.step)
                            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
                        // Write failed step index to error slot if configured.
                        if let Some(slot) = error_slot {
                            let failed_step_i64 = i64::from(ticket.step.get());
                            let slot_value = vb_core::value::SlotValue::I64(failed_step_i64);
                            if state.frame.write_slot(slot, slot_value).is_err() {
                                // Slot write failure - continue without error slot
                            }
                        }
                        state
                            .frame
                            .set_pc(handler)
                            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
                    }
                    None => {
                        fail_without_handler = true;
                    }
                }
            }
        }
        if retry_now {
            return self.drive_run(run);
        }
        if fail_without_handler {
            self.trace_ring.push(TraceEvent::ActionFailed {
                run,
                step: ticket.step,
                code: failure.code,
            });
            let state = self.take_run_state(run)?;
            self.fail_run_state(run, state)?;
            return Ok(());
        }
        self.trace_ring.push(TraceEvent::ActionFailed {
            run,
            step: ticket.step,
            code: failure.code,
        });
        self.drive_run(run)
    }

    pub(crate) fn handle_ask_answer(&mut self, answer: AskAnswer) -> RuntimeResult<()> {
        let run = answer.ticket.run;
        let state = self.runs.get_mut(&run).ok_or(RuntimeError::RunNotFound)?;
        if let Some(timer) = self.pending_timers.get(&run)
            && timer.step == answer.ticket.ask_step
        {
            self.pending_timers.swap_remove(&run);
        }
        state
            .frame
            .write_slot_with_taint(answer.answer_slot, answer.value, answer.taint)
            .map_err(|_| RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_running(answer.ticket.ask_step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_succeeded(answer.ticket.ask_step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        state
            .frame
            .set_pc(answer.ticket.resume_step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        self.trace_ring.push(TraceEvent::AskAnswered {
            run,
            step: answer.ticket.ask_step,
            slot: answer.answer_slot,
        });
        self.journal.append(RuntimeJournalEvent::AskAnswered {
            run,
            step: answer.ticket.ask_step,
            slot: answer.answer_slot,
        })?;
        let encoded_answer_value =
            postcard::to_allocvec(&answer.value).map_err(|_| RuntimeError::EncodeFailed)?;
        self.journal.append(RuntimeJournalEvent::SlotWritten {
            run,
            slot: answer.answer_slot,
            value: encoded_answer_value,
        })?;
        self.journal.append(RuntimeJournalEvent::StepSucceeded {
            run,
            step: answer.ticket.ask_step,
            output: answer.answer_slot,
        })?;
        self.drive_run(run)
    }

    pub(crate) fn handle_timer(&mut self, run: RunId) -> RuntimeResult<()> {
        let mut state = self.take_run_state(run)?;
        let Some(timer) = self.pending_timers.swap_remove(&run) else {
            self.runs.insert(run, state);
            return Err(RuntimeError::InvalidTimerFire);
        };
        crate::shard::helpers::advance_after_timer_fire(&mut state, timer)?;
        match timer.kind {
            PendingTimerKind::Wait => {
                self.journal.append(RuntimeJournalEvent::WaitResolved {
                    run,
                    step: timer.step,
                })?;
            }
            PendingTimerKind::Ask => {}
        }
        let mut evidence = EvidenceCollector::new();
        let result = Self::drive_state(&mut state, self.step_budget_per_tick, &mut evidence);
        self.flush_evidence(run, &mut evidence)?;
        self.apply_drive_result(run, state, result)
    }

    pub(crate) fn handle_cancel(&mut self, run: RunId) -> RuntimeResult<()> {
        self.pending_timers.swap_remove(&run);
        if self.runs.contains_key(&run) {
            self.journal
                .append(RuntimeJournalEvent::RunCancelled { run })?;
        }
        if let Some(state) = self.runs.swap_remove(&run) {
            self.release_frame(state.frame);
            self.counters.inc_failed();
            self.trace_ring.push(TraceEvent::RunCancelled { run });
        }
        Ok(())
    }

    pub(crate) fn handle_inspect(&mut self, run: RunId, correlation: u64) {
        self.inspect_response = Some(self.snapshot_run(run, correlation));
    }

    fn drive_run(&mut self, run: RunId) -> RuntimeResult<()> {
        let mut state = self.take_run_state(run)?;
        let mut evidence = EvidenceCollector::new();
        let result = Self::drive_state(&mut state, self.step_budget_per_tick, &mut evidence);
        self.flush_evidence(run, &mut evidence)?;
        self.apply_drive_result(run, state, result)
    }

    fn take_run_state(&mut self, run: RunId) -> RuntimeResult<RunState> {
        match self.runs.swap_remove(&run) {
            Some(state) => Ok(state),
            None => Err(RuntimeError::RunNotFound),
        }
    }

    fn drive_state(
        state: &mut RunState,
        step_budget_per_tick: u64,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeEngineResult<RuntimeSignal> {
        let mut budget = vb_core::engine::StepBudget::new(step_budget_per_tick);
        let granted = state
            .admission
            .as_ref()
            .map(|a| a.granted_capabilities())
            .unwrap_or_else(CapabilitySet::empty);
        drive_deterministic_full(
            &state.workflow,
            &mut state.frame,
            &mut budget,
            &mut state.store,
            &[],
            RetryPolicy::NEVER,
            evidence,
            &mut state.collect_states,
            granted,
        )
    }

    #[allow(clippy::needless_pass_by_value)]
    fn apply_drive_result(
        &mut self,
        run: RunId,
        state: RunState,
        result: RuntimeEngineResult<RuntimeSignal>,
    ) -> RuntimeResult<()> {
        match result {
            Ok(RuntimeSignal::Continue | RuntimeSignal::StepBudgetExhausted) => {
                self.keep_run(run, state);
                Ok(())
            }
            Ok(RuntimeSignal::Finished(_)) => self.finish_run(run, state),
            Ok(RuntimeSignal::AwaitingAction(ticket)) => self.await_action(run, state, ticket),
            Ok(RuntimeSignal::AwaitingWait) => self.await_timer(run, state, PendingTimerKind::Wait),
            Ok(RuntimeSignal::AwaitingAsk) => self.await_timer(run, state, PendingTimerKind::Ask),
            Err(_) => self.fail_run_state(run, state),
        }
    }
}
