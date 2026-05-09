//! Run lifecycle management: submit, resume, cancel, action completion, timers.

use vb_core::ValueStore;
use vb_core::action::{
    ActionFailure, ActionOutputReady, ActionTicket, RetryPolicy as VbCoreRetryPolicy,
};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::workflow::CompiledWorkflow;

use crate::engine::{
    EvidenceCollector, RetryPolicy, RuntimeEngineResult, RuntimeSignal, drive_deterministic_full,
};
use crate::journal::RuntimeJournalEvent;
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

use crate::primitives::collect::CollectStates;
use crate::shard::types::{AskAnswer, PendingTimerKind, RunState, Shard};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActionFailureOutcome {
    RetryNow,
    DriveHandler,
    FailRun,
}

fn retry_is_available(
    state: &mut RunState,
    ticket: ActionTicket,
    retry_policy: VbCoreRetryPolicy,
) -> RuntimeResult<bool> {
    if retry_policy != VbCoreRetryPolicy::Retryable
        || !crate::shard::helpers::retry_metadata_exists(state, ticket.step)
    {
        return Ok(false);
    }
    let policy = crate::shard::helpers::retry_policy_after_action(state, ticket.step)?;
    crate::shard::helpers::record_retry_attempt(state, ticket, policy)
}

fn apply_error_handler(
    state: &mut RunState,
    ticket: ActionTicket,
) -> RuntimeResult<ActionFailureOutcome> {
    match crate::shard::helpers::find_error_handler_for_failure(&state.workflow, ticket.step) {
        Some((handler, error_slot)) => {
            state
                .frame
                .mark_failed(ticket.step)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?;
            write_failure_slot(state, ticket.step, error_slot)?;
            state
                .frame
                .set_pc(handler)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?;
            Ok(ActionFailureOutcome::DriveHandler)
        }
        None => Ok(ActionFailureOutcome::FailRun),
    }
}

fn write_failure_slot(
    state: &mut RunState,
    step: StepIdx,
    error_slot: Option<SlotIdx>,
) -> RuntimeResult<()> {
    match error_slot {
        Some(slot) => state
            .frame
            .write_slot(slot, vb_core::value::SlotValue::I64(i64::from(step.get())))
            .map_err(|_| RuntimeError::InvalidActionCompletion),
        None => Ok(()),
    }
}

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
        if let Some(admission) = admission.as_ref() {
            self.journal.append(RuntimeJournalEvent::RunAdmission {
                admission: admission.clone(),
            })?;
        }
        self.counters.inc_submitted();
        let frame_step_count = frame.step_count();
        let max_slots = workflow.resource_contract().max_slots;
        let state = RunState {
            frame,
            workflow,
            store: ValueStore::with_max_slots(max_slots),
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
        let encoded_value =
            postcard::to_allocvec(&output.value).map_err(|_| RuntimeError::EncodeFailed)?;
        self.trace_ring.push(TraceEvent::SlotWritten {
            run,
            slot: output.output_slot,
            value: encoded_value.clone(),
        });
        self.trace_ring.push(TraceEvent::ActionCompleted {
            run,
            step: ticket.step,
        });
        self.journal.append(RuntimeJournalEvent::SlotWritten {
            run,
            slot: output.output_slot,
            value: encoded_value,
            extra: None,
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
        let code = failure.code;
        let ticket = self.ticket_with_retry_capacity(ticket, failure.retry_policy)?;
        let outcome = self.apply_action_failure_to_state(ticket, failure)?;
        self.trace_ring.push(TraceEvent::ActionFailed {
            run,
            step: ticket.step,
            code,
        });
        self.journal.append(RuntimeJournalEvent::ActionFailed {
            run,
            step: ticket.step,
            action: ticket.action,
        })?;
        match outcome {
            ActionFailureOutcome::RetryNow | ActionFailureOutcome::DriveHandler => {
                self.drive_run(run)
            }
            ActionFailureOutcome::FailRun => {
                let state = self.take_run_state(run)?;
                self.fail_run_state(run, state)
            }
        }
    }

    fn ticket_with_retry_capacity(
        &self,
        ticket: ActionTicket,
        retry_policy: VbCoreRetryPolicy,
    ) -> RuntimeResult<ActionTicket> {
        let Some(state) = self.runs.get(&ticket.run) else {
            return Err(RuntimeError::RunNotFound);
        };
        if retry_policy != VbCoreRetryPolicy::Retryable
            || !crate::shard::helpers::retry_metadata_exists(state, ticket.step)
        {
            return Ok(ticket);
        }
        let policy = crate::shard::helpers::retry_policy_after_action(state, ticket.step)?;
        Ok(ActionTicket {
            capacity: ticket.capacity.max(policy.max_attempts),
            ..ticket
        })
    }

    fn apply_action_failure_to_state(
        &mut self,
        ticket: ActionTicket,
        failure: ActionFailure,
    ) -> RuntimeResult<ActionFailureOutcome> {
        let state = self
            .runs
            .get_mut(&ticket.run)
            .ok_or(RuntimeError::RunNotFound)?;
        crate::shard::helpers::validate_action_completion(state, ticket)?;
        if retry_is_available(state, ticket, failure.retry_policy)? {
            state
                .frame
                .set_pc(ticket.step)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?;
            return Ok(ActionFailureOutcome::RetryNow);
        }
        apply_error_handler(state, ticket)
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
            extra: None,
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
        let empty_caps = CapabilitySet::empty();
        let granted = state
            .admission
            .as_ref()
            .map(|a| a.granted_capabilities())
            .unwrap_or(&empty_caps);
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

#[cfg(test)]
mod tests {
    use vb_core::action::{
        ActionFailure, ActionFailureCode, ActionOutputReady, ActionTicket,
        RetryPolicy as VbRetryPolicy,
    };
    use vb_core::capability::CapabilitySet;
    use vb_core::ids::{ActionId, ConstIdx, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::value::{ConstValue, SlotValue, Taint};
    use vb_core::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
    };

    use crate::RuntimeError;
    use crate::journal::{RuntimeJournalEvent, SharedRuntimeJournal};
    use crate::trace::TraceEvent;

    use super::super::types::{
        AskAnswer, AskTicket, InspectResponse, Shard, ShardCommand, ShardConfig,
    };

    fn suspended_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("suspended"),
            digest: WorkflowDigest::from_bytes([1; 32]),
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn zero_retry_policy_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_policy = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let action = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::ZERO,
            },
        };
        let retry = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(1),
                body: StepIdx::new(1),
                exhausted: StepIdx::new(3),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("zero_retry_policy"),
            digest: WorkflowDigest::from_bytes([8; 32]),
            nodes: Box::from([set_policy, action, retry, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(0)]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn finished_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_const = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("finished"),
            digest: WorkflowDigest::from_bytes([2; 32]),
            nodes: Box::from([set_const, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::Bool(true)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn error_handler_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let guard = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: None,
            },
        };
        let action = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        };
        let handler = CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let finish_node = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("error_handler"),
            digest: WorkflowDigest::from_bytes([3; 32]),
            nodes: Box::from([guard, action, handler, finish_node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::Bool(false)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn wait_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_deadline = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let wait = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::ZERO,
            },
        };
        let finish_node = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("wait_then_finish"),
            digest: WorkflowDigest::from_bytes([4; 32]),
            nodes: Box::from([set_deadline, wait, finish_node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(10)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn ask_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let set_prompt = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let set_timeout = CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(1),
            },
        };
        let ask = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::ZERO,
                timeout_slot: Some(SlotIdx::new(1)),
            },
        };
        let resume = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: Some(StepIdx::new(4)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::AskResume {
                answer: SlotIdx::new(2),
            },
        };
        let finish_node = CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(2),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("ask_then_finish"),
            digest: WorkflowDigest::from_bytes([5; 32]),
            nodes: Box::from([set_prompt, set_timeout, ask, resume, finish_node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([
                ConstValue::Symbol(vb_core::ids::SymbolId::new(1)),
                ConstValue::I64(10),
            ]),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn small_config() -> ShardConfig {
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        }
    }

    fn make_ticket(run: RunId, step: StepIdx, attempt: u16) -> ActionTicket {
        ActionTicket {
            run,
            step,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            attempt,
            idempotency_key: 0,
            capacity: 1,
        }
    }

    fn non_retryable_failure() -> ActionFailure {
        ActionFailure {
            code: ActionFailureCode::Timeout,
            retry_policy: VbRetryPolicy::NonRetryable,
            taint: Taint::Clean,
            detail: None,
            encoded_len: 0,
        }
    }

    fn retryable_failure() -> ActionFailure {
        ActionFailure {
            retry_policy: VbRetryPolicy::Retryable,
            ..non_retryable_failure()
        }
    }

    fn require_workflow(
        name: &str,
        workflow: Option<vb_core::workflow::CompiledWorkflow>,
    ) -> Result<vb_core::workflow::CompiledWorkflow, String> {
        match workflow {
            Some(wf) => Ok(wf),
            None => Err(format!("{name} fixture workflow must compile")),
        }
    }

    fn require_snapshot(
        journal: &crate::journal::VolatileRuntimeJournal,
    ) -> Result<Vec<RuntimeJournalEvent>, String> {
        journal
            .snapshot()
            .map_err(|error| format!("journal snapshot failed: {error:?}"))
    }

    fn action_failed_count(events: &[RuntimeJournalEvent], run: RunId, step: StepIdx) -> usize {
        events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    RuntimeJournalEvent::ActionFailed { run: event_run, step: event_step, .. }
                        if *event_run == run && *event_step == step
                )
            })
            .count()
    }

    fn retry_workflow() -> Result<vb_core::workflow::CompiledWorkflow, String> {
        let set_policy = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let action = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::ZERO,
            },
        };
        let retry = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(1),
                body: StepIdx::new(1),
                exhausted: StepIdx::new(3),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        };
        let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
            name: Box::from("retry"),
            digest: WorkflowDigest::from_bytes([8; 32]),
            nodes: Box::from([set_policy, action, retry, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(2)]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        });
        workflow.map_err(|error| format!("retry fixture workflow must compile: {error:?}"))
    }

    fn submit_run(shard: &mut Shard, run: RunId, workflow: CompiledWorkflow) {
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }

    fn enqueue_action_failure(shard: &mut Shard, run: RunId, step: StepIdx, attempt: u16) {
        let ticket = make_ticket(run, step, attempt);
        assert_eq!(
            shard.enqueue(ShardCommand::ActionFailed {
                ticket,
                failure: retryable_failure(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
    }

    fn event_position(
        events: &[RuntimeJournalEvent],
        expected: &RuntimeJournalEvent,
    ) -> Option<usize> {
        events.iter().position(|event| event == expected)
    }

    fn assert_event_order(
        events: &[RuntimeJournalEvent],
        first: RuntimeJournalEvent,
        second: RuntimeJournalEvent,
    ) {
        let first_position = event_position(events, &first);
        let second_position = event_position(events, &second);
        assert!(
            matches!((first_position, second_position), (Some(a), Some(b)) if a < b),
            "events out of order: {events:?}"
        );
    }

    fn assert_retry_exhaustion_journal(events: &[RuntimeJournalEvent], run: RunId) {
        assert_eq!(action_failed_count(events, run, StepIdx::new(1)), 2);
        assert_event_order(
            events,
            RuntimeJournalEvent::ActionFailed {
                run,
                step: StepIdx::new(1),
                action: ActionId::new(0),
            },
            RuntimeJournalEvent::RunFailed { run },
        );
    }

    #[test]
    fn submit_finished_workflow_completes_immediately() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run = RunId::new(1);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
        assert_eq!(shard.active_run_count(), 0);
    }

    #[test]
    fn submit_suspended_workflow_suspends_on_action() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(2);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.active_run_count(), 1);
        assert_eq!(shard.counters().snapshot().runs_submitted, 1);
        Ok(())
    }

    #[test]
    fn submit_duplicate_run_returns_run_already_exists() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(10);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf.clone(),
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
        Ok(())
    }

    #[test]
    fn submit_at_capacity_returns_active_run_capacity_exceeded() {
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 1,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        let Some(wf1) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(1),
                workflow: wf1,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let Some(wf2) = suspended_workflow() else {
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(2),
                workflow: wf2,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 1 })
        );
    }

    #[test]
    fn submit_with_inputs_seeds_slots_before_driving() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(20);
        assert_eq!(
            shard.enqueue(ShardCommand::SubmitWithInputs {
                run,
                workflow: wf,
                inputs: Box::from([(SlotIdx::new(0), SlotValue::I64(99))]),
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.active_run_count(), 1);
        Ok(())
    }

    #[test]
    fn submit_with_inputs_rejects_duplicate() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(21);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf.clone(),
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::SubmitWithInputs {
                run,
                workflow: wf,
                inputs: Box::from([(SlotIdx::new(0), SlotValue::I64(1))]),
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunAlreadyExists));
        Ok(())
    }

    #[test]
    fn resume_on_suspended_run_re_drives() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(30);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Resume { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.active_run_count(), 1);
    }

    #[test]
    fn resume_unknown_run_returns_run_not_found() {
        let mut shard = Shard::new(small_config());
        assert_eq!(
            shard.enqueue(ShardCommand::Resume {
                run: RunId::new(9999),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn action_completed_typed_writes_slot_and_advances() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(40);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let ticket = make_ticket(run, StepIdx::ZERO, 1);
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(0),
            value: SlotValue::I64(42),
            taint: Taint::Clean,
            encoded_len: 0,
        };
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let events = shard.trace_ring_mut().drain();
        let found = events.iter().any(|e| {
            *e == TraceEvent::ActionCompleted {
                run,
                step: StepIdx::ZERO,
            }
        });
        assert_eq!(found, true);
    }

    #[test]
    fn action_completed_unknown_run_returns_run_not_found() {
        let mut shard = Shard::new(small_config());
        let ticket = make_ticket(RunId::new(9999), StepIdx::ZERO, 1);
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(0),
            value: SlotValue::I64(1),
            taint: Taint::Clean,
            encoded_len: 0,
        };
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn future_attempt_completion_rejected_when_current_attempt_exists() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            assert_eq!(None::<()>, Some(()), "missing suspended workflow fixture");
            return;
        };
        let run = RunId::new(40_001);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let Some(state) = shard.runs.get_mut(&run) else {
            assert_eq!(None::<()>, Some(()), "run should remain active");
            return;
        };
        assert_eq!(state.action_attempts.get(0).copied(), Some(1));
        let output = ActionOutputReady {
            output_slot: SlotIdx::ZERO,
            value: SlotValue::I64(7),
            taint: Taint::Clean,
            encoded_len: 0,
        };
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompleted {
                ticket: ActionTicket {
                    capacity: 3,
                    ..make_ticket(run, StepIdx::ZERO, 2)
                },
                output,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::InvalidActionCompletion));
    }

    #[test]
    fn future_attempt_completion_beyond_max_is_action_failed_code() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            assert_eq!(None::<()>, Some(()), "missing suspended workflow fixture");
            return;
        };
        let run = RunId::new(40_002);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let output = ActionOutputReady {
            output_slot: SlotIdx::ZERO,
            value: SlotValue::I64(7),
            taint: Taint::Clean,
            encoded_len: 0,
        };
        let error = RuntimeError::AttemptBeyondMax { attempt: 4, max: 3 };
        assert_eq!(error.runtime_code(), Some("ACTION_FAILED"));
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompleted {
                ticket: ActionTicket {
                    capacity: 3,
                    ..make_ticket(run, StepIdx::ZERO, 4)
                },
                output,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(error));
    }

    #[test]
    fn stale_attempt_completion_leaves_run_counters_journal_and_frame_unchanged() {
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let shared: SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(small_config(), shared);
        let Some(wf) = suspended_workflow() else {
            assert_eq!(None::<()>, Some(()), "missing suspended workflow fixture");
            return;
        };
        let run = RunId::new(41);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let Some(state) = shard.runs.get_mut(&run) else {
            assert_eq!(None::<()>, Some(()), "run should remain active");
            return;
        };
        if let Some(attempt) = state.action_attempts.get_mut(0) {
            *attempt = 3;
        }
        let frame_before = state.frame.clone();
        let step_state_before = state.frame.step_state(StepIdx::ZERO);
        let attempts_before = state.action_attempts.clone();
        let counters_before = shard.counters().snapshot();
        let journal_before = journal.snapshot();
        let trace_before = shard
            .trace_ring()
            .snapshot_for_run(run, shard.trace_ring().capacity());
        let output = ActionOutputReady {
            output_slot: SlotIdx::ZERO,
            value: SlotValue::I64(7),
            taint: Taint::Clean,
            encoded_len: 0,
        };
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompleted {
                ticket: ActionTicket {
                    capacity: 3,
                    ..make_ticket(run, StepIdx::ZERO, 2)
                },
                output,
            }),
            Ok(())
        );
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::StaleAttempt {
                incoming: 2,
                current: 3,
            })
        );
        let Some(state_after) = shard.runs.get(&run) else {
            assert_eq!(
                None::<()>,
                Some(()),
                "run should remain active after rejection"
            );
            return;
        };
        assert_eq!(state_after.frame.pc(), frame_before.pc());
        assert_eq!(
            state_after.frame.step_state(StepIdx::ZERO),
            step_state_before
        );
        assert_eq!(state_after.frame, frame_before);
        assert_eq!(state_after.action_attempts, attempts_before);
        assert_eq!(shard.counters().snapshot(), counters_before);
        assert_eq!(journal.snapshot(), journal_before);
        assert_eq!(
            shard
                .trace_ring()
                .snapshot_for_run(run, shard.trace_ring().capacity()),
            trace_before
        );
    }

    #[test]
    fn scheduling_propagates_zero_retry_policy_error() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = zero_retry_policy_workflow() else {
            assert_eq!(
                None::<()>,
                Some(()),
                "missing zero retry policy workflow fixture"
            );
            return;
        };
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(42),
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::UnsupportedOperation {
                operation: "retry_policy_attempts_zero",
            })
        );
    }

    #[test]
    fn legacy_action_completed_on_suspended_run_succeeds() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(50);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompletedLegacy {
                run,
                step: StepIdx::ZERO,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let found = shard.trace_ring_mut().drain().iter().any(|e| {
            *e == TraceEvent::ActionCompleted {
                run,
                step: StepIdx::ZERO,
            }
        });
        assert_eq!(found, true);
    }

    #[test]
    fn legacy_action_completed_unknown_run_returns_run_not_found() {
        let mut shard = Shard::new(small_config());
        assert_eq!(
            shard.enqueue(ShardCommand::ActionCompletedLegacy {
                run: RunId::new(9999),
                step: StepIdx::ZERO,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn action_failure_without_handler_fails_run() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(60);
        submit_run(&mut shard, run, wf);
        let ticket = make_ticket(run, StepIdx::ZERO, 1);
        assert_eq!(
            shard.enqueue(ShardCommand::ActionFailed {
                ticket,
                failure: non_retryable_failure(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_failed, 1);
        assert_eq!(shard.active_run_count(), 0);
        Ok(())
    }

    #[test]
    fn action_failure_without_handler_emits_action_failed_before_run_failed() -> Result<(), String>
    {
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let shared: SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(small_config(), shared);
        let wf = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(600);
        submit_run(&mut shard, run, wf);
        let ticket = make_ticket(run, StepIdx::ZERO, 1);
        assert_eq!(
            shard.enqueue(ShardCommand::ActionFailed {
                ticket,
                failure: non_retryable_failure(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        let events = require_snapshot(&journal)?;
        assert_event_order(
            &events,
            RuntimeJournalEvent::ActionFailed {
                run,
                step: StepIdx::ZERO,
                action: ActionId::new(0),
            },
            RuntimeJournalEvent::RunFailed { run },
        );
        Ok(())
    }

    #[test]
    fn action_failure_routes_to_error_handler() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = require_workflow("error_handler", error_handler_workflow())?;
        let run = RunId::new(61);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let ticket = make_ticket(run, StepIdx::new(1), 1);
        assert_eq!(
            shard.enqueue(ShardCommand::ActionFailed {
                ticket,
                failure: non_retryable_failure(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
        assert_eq!(shard.counters().snapshot().runs_failed, 0);
        Ok(())
    }

    #[test]
    fn action_failure_routed_to_handler_emits_action_failed_before_handler_step()
    -> Result<(), String> {
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let shared: SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(small_config(), shared);
        let wf = require_workflow("error_handler", error_handler_workflow())?;
        let run = RunId::new(610);
        submit_run(&mut shard, run, wf);
        let ticket = make_ticket(run, StepIdx::new(1), 1);
        assert_eq!(
            shard.enqueue(ShardCommand::ActionFailed {
                ticket,
                failure: non_retryable_failure(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));

        let events = require_snapshot(&journal)?;
        assert_event_order(
            &events,
            RuntimeJournalEvent::ActionFailed {
                run,
                step: StepIdx::new(1),
                action: ActionId::new(0),
            },
            RuntimeJournalEvent::StepStarted {
                run,
                step: StepIdx::new(2),
            },
        );
        Ok(())
    }

    #[test]
    fn action_failure_unknown_run_returns_run_not_found() {
        let mut shard = Shard::new(small_config());
        let ticket = make_ticket(RunId::new(9999), StepIdx::ZERO, 1);
        assert_eq!(
            shard.enqueue(ShardCommand::ActionFailed {
                ticket,
                failure: non_retryable_failure(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn retry_exhaustion_emits_single_action_failed() -> Result<(), String> {
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let shared: SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(small_config(), shared);
        let run = RunId::new(620);
        submit_run(&mut shard, run, retry_workflow()?);
        enqueue_action_failure(&mut shard, run, StepIdx::new(1), 1);
        enqueue_action_failure(&mut shard, run, StepIdx::new(1), 2);
        let events = require_snapshot(&journal)?;
        assert_retry_exhaustion_journal(&events, run);
        Ok(())
    }

    #[test]
    fn ask_answer_completes_ask_workflow() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = ask_workflow() else {
            return;
        };
        let run = RunId::new(70);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.pending_timer_count(), 1);
        let answer = AskAnswer {
            ticket: AskTicket {
                run,
                ask_step: StepIdx::new(2),
                resume_step: StepIdx::new(3),
            },
            answer_slot: SlotIdx::new(2),
            value: SlotValue::I64(77),
            taint: Taint::Clean,
        };
        assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
    }

    #[test]
    fn ask_answer_unknown_run_returns_run_not_found() {
        let mut shard = Shard::new(small_config());
        let answer = AskAnswer {
            ticket: AskTicket {
                run: RunId::new(9999),
                ask_step: StepIdx::ZERO,
                resume_step: StepIdx::new(1),
            },
            answer_slot: SlotIdx::ZERO,
            value: SlotValue::I64(0),
            taint: Taint::Clean,
        };
        assert_eq!(shard.enqueue(ShardCommand::AskAnswered { answer }), Ok(()));
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn timer_fire_advances_wait_to_completion() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = wait_workflow() else {
            return;
        };
        let run = RunId::new(80);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.pending_timer_count(), 1);
        assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_completed, 1);
        assert_eq!(shard.pending_timer_count(), 0);
    }

    #[test]
    fn timer_fire_for_non_timer_run_returns_invalid_timer_fire() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(81);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
        assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    }

    #[test]
    fn timer_fire_unknown_run_returns_run_not_found() {
        let mut shard = Shard::new(small_config());
        assert_eq!(
            shard.enqueue(ShardCommand::TimerFired {
                run: RunId::new(9999),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn cancel_removes_active_run_and_increments_failed() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(90);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.active_run_count(), 1);
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(shard.counters().snapshot().runs_failed, 1);
    }

    #[test]
    fn cancel_nonexistent_run_succeeds_without_counter_change() {
        let mut shard = Shard::new(small_config());
        assert_eq!(
            shard.enqueue(ShardCommand::Cancel {
                run: RunId::new(9999),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_failed, 0);
    }

    #[test]
    fn cancel_clears_pending_timer() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = wait_workflow() else {
            return;
        };
        let run = RunId::new(91);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.pending_timer_count(), 1);
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.pending_timer_count(), 0);
    }

    #[test]
    fn inspect_active_run_returns_found() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(100);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run,
                correlation: 42,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        match shard.take_inspect_response() {
            Some(InspectResponse::Found(snap)) => {
                assert_eq!(snap.run, run);
                assert_eq!(snap.correlation, 42);
            }
            other => {
                assert_eq!(
                    format!("{other:?}"),
                    "Some(Found(InspectSnapshot { run: RunId(100), correlation: 42 }))"
                );
            }
        }
    }

    #[test]
    fn inspect_unknown_run_returns_not_found() {
        let mut shard = Shard::new(small_config());
        assert_eq!(
            shard.enqueue(ShardCommand::Inspect {
                run: RunId::new(9999),
                correlation: 1,
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.take_inspect_response(),
            Some(InspectResponse::NotFound {
                run: RunId::new(9999),
                correlation: 1,
            })
        );
    }

    #[test]
    fn submit_produces_run_submitted_trace() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(110);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let found = shard
            .trace_ring_mut()
            .drain()
            .iter()
            .any(|e| *e == TraceEvent::RunSubmitted { run });
        assert_eq!(found, true);
    }

    #[test]
    fn cancel_produces_run_cancelled_trace() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(111);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        let found = shard
            .trace_ring_mut()
            .drain()
            .iter()
            .any(|e| *e == TraceEvent::RunCancelled { run });
        assert_eq!(found, true);
    }

    #[test]
    fn cancel_emits_run_cancelled_journal_event() -> Result<(), String> {
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let shared: SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(small_config(), shared);
        let wf = require_workflow("suspended", suspended_workflow())?;
        let run = RunId::new(112);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        let events = require_snapshot(&journal)?;
        assert!(
            events.contains(&RuntimeJournalEvent::RunCancelled { run }),
            "journal events should contain RunCancelled: {events:?}"
        );
        Ok(())
    }

    #[test]
    fn finish_produces_run_finished_trace() -> Result<(), String> {
        let mut shard = Shard::new(small_config());
        let wf = require_workflow("finished", finished_workflow())?;
        let run = RunId::new(113);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        let found = shard
            .trace_ring_mut()
            .drain()
            .iter()
            .any(|e| *e == TraceEvent::RunFinished { run });
        assert_eq!(found, true);
        Ok(())
    }

    #[test]
    fn finished_workflow_emits_one_slot_written_for_one_output_write() -> Result<(), String> {
        let journal = std::sync::Arc::new(crate::journal::VolatileRuntimeJournal::new());
        let shared: SharedRuntimeJournal = journal.clone();
        let mut shard = Shard::new_with_journal(small_config(), shared);
        let wf = require_workflow("finished", finished_workflow())?;
        let run = RunId::new(1130);
        submit_run(&mut shard, run, wf);

        let events = require_snapshot(&journal)?;
        let slot_written_count = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    RuntimeJournalEvent::SlotWritten {
                        run: event_run,
                        slot: SlotIdx::ZERO,
                        ..
                    } if *event_run == run
                )
            })
            .count();
        assert_eq!(slot_written_count, 1, "events: {events:?}");
        Ok(())
    }

    #[test]
    fn resubmit_after_cancel_succeeds() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run = RunId::new(300);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf.clone(),
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.active_run_count(), 1);
    }

    #[test]
    fn timer_fire_after_cancel_returns_run_not_found() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = wait_workflow() else {
            return;
        };
        let run = RunId::new(400);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.pending_timer_count(), 1);
        assert_eq!(shard.enqueue(ShardCommand::Cancel { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
        assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    }

    #[test]
    fn ask_timer_fire_fails_run_when_no_answer() {
        let mut shard = Shard::new(small_config());
        let Some(wf) = ask_workflow() else {
            return;
        };
        let run = RunId::new(500);
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run,
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.pending_timer_count(), 1);
        assert_eq!(shard.enqueue(ShardCommand::TimerFired { run }), Ok(()));
        assert_eq!(shard.tick(), Ok(true));
        assert_eq!(shard.counters().snapshot().runs_failed, 1);
        assert_eq!(shard.pending_timer_count(), 0);
    }

    #[test]
    fn multiple_submits_fill_to_capacity_then_reject() -> Result<(), String> {
        let config = ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 2,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        };
        let mut shard = Shard::new(config);
        submit_run(
            &mut shard,
            RunId::new(0),
            require_workflow("suspended", suspended_workflow())?,
        );
        submit_run(
            &mut shard,
            RunId::new(1),
            require_workflow("suspended", suspended_workflow())?,
        );
        assert_eq!(shard.active_run_count(), 2);
        let wf = require_workflow("suspended", suspended_workflow())?;
        assert_eq!(
            shard.enqueue(ShardCommand::Submit {
                run: RunId::new(99),
                workflow: wf,
                caps: CapabilitySet::empty(),
            }),
            Ok(())
        );
        assert_eq!(
            shard.tick(),
            Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 2 })
        );
        Ok(())
    }
}
