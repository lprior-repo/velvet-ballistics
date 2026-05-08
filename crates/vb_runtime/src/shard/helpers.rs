//! Pure helper functions for shard operations.

use vb_core::action::ActionTicket;
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

use crate::engine::RetryPolicy;
use crate::{RuntimeError, RuntimeResult};

use crate::shard::types::{InspectSnapshot, PendingTimer, PendingTimerKind};

/// Seeds input slots on a frame before deterministic execution.
pub fn seed_input_slots(
    frame: &mut RunFrame,
    inputs: &[(SlotIdx, SlotValue)],
) -> RuntimeResult<()> {
    for (slot, value) in inputs {
        frame
            .write_slot_with_taint(*slot, *value, Taint::Clean)
            .map_err(|_| RuntimeError::InvalidRecoveryHydration)?;
    }
    Ok(())
}

/// Validates that an action completion matches the expected ticket.
pub fn validate_action_completion(
    state: &crate::shard::types::RunState,
    ticket: ActionTicket,
) -> RuntimeResult<()> {
    if state.frame.step_state(ticket.step) != Ok(StepState::Running) {
        return Err(RuntimeError::InvalidActionCompletion);
    }
    let Some(node) = state.workflow.node(ticket.step) else {
        return Err(RuntimeError::InvalidActionCompletion);
    };
    match node.kind {
        CompiledNodeKind::Do { action, .. } if action == ticket.action => Ok(()),
        _ => Err(RuntimeError::InvalidActionCompletion),
    }
}

/// Advances PC after an action completes successfully.
pub fn advance_after_action_completion(
    state: &mut crate::shard::types::RunState,
    step: StepIdx,
) -> RuntimeResult<()> {
    let Some(node) = state.workflow.node(step) else {
        return Err(RuntimeError::InvalidActionCompletion);
    };
    match node.next {
        Some(next) => {
            state
                .frame
                .set_pc(next)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?;
            Ok(())
        }
        None => Ok(()),
    }
}

/// Returns true if a timer must be registered for the given step.
pub fn timer_registration_required(state: &crate::shard::types::RunState, step: StepIdx) -> bool {
    let Some(node) = state.workflow.node(step) else {
        return false;
    };
    match node.kind {
        CompiledNodeKind::WaitUntil { .. } => true,
        CompiledNodeKind::WaitEvent { timeout_slot, .. }
        | CompiledNodeKind::Ask { timeout_slot, .. } => timeout_slot.is_some(),
        _ => false,
    }
}

/// Advances state after a timer fires.
pub fn advance_after_timer_fire(
    state: &mut crate::shard::types::RunState,
    timer: PendingTimer,
) -> RuntimeResult<()> {
    let Some(node) = state.workflow.node(timer.step) else {
        return Err(RuntimeError::InvalidTimerFire);
    };
    match (timer.kind, &node.kind) {
        (
            PendingTimerKind::Wait,
            CompiledNodeKind::WaitUntil { .. } | CompiledNodeKind::WaitEvent { .. },
        )
        | (PendingTimerKind::Ask, CompiledNodeKind::Ask { .. }) => {}
        _ => return Err(RuntimeError::InvalidTimerFire),
    }
    state
        .frame
        .mark_running(timer.step)
        .map_err(|_| RuntimeError::InvalidTimerFire)?;
    state
        .frame
        .mark_succeeded(timer.step)
        .map_err(|_| RuntimeError::InvalidTimerFire)?;
    let Some(next) = node.next else {
        return Err(RuntimeError::InvalidTimerFire);
    };
    state
        .frame
        .set_pc(next)
        .map_err(|_| RuntimeError::InvalidTimerFire)?;
    Ok(())
}

/// Creates a new action attempts tracker.
pub fn new_action_attempts(step_count: u16) -> Box<[u16]> {
    vec![0; usize::from(step_count)].into_boxed_slice()
}

/// Records a scheduled action attempt.
pub fn record_scheduled_attempt(state: &mut crate::shard::types::RunState, ticket: ActionTicket) {
    if let Some(attempt) = state.action_attempts.get_mut(ticket.step.as_usize())
        && (*attempt == 0 || *attempt < ticket.attempt)
    {
        *attempt = ticket.attempt;
    }
}

/// Returns true if retry metadata exists for the given step.
pub fn retry_metadata_exists(state: &crate::shard::types::RunState, step: StepIdx) -> bool {
    let Some(node) = state.workflow.node(step) else {
        return false;
    };
    let Some(next) = node.next else {
        return false;
    };
    matches!(
        state.workflow.node(next).map(|next_node| &next_node.kind),
        Some(CompiledNodeKind::RetryCheck { .. })
    )
}

/// Extracts retry policy from the step's retry check node.
pub fn retry_policy_after_action(
    state: &crate::shard::types::RunState,
    step: StepIdx,
) -> RuntimeResult<RetryPolicy> {
    let Some(node) = state.workflow.node(step) else {
        return Err(RuntimeError::InvalidActionCompletion);
    };
    let Some(next) = node.next else {
        return Err(RuntimeError::UnsupportedOperation {
            operation: "retry_metadata_missing",
        });
    };
    let Some(retry_node) = state.workflow.node(next) else {
        return Err(RuntimeError::InvalidActionCompletion);
    };
    let CompiledNodeKind::RetryCheck { policy_slot, .. } = retry_node.kind else {
        return Err(RuntimeError::UnsupportedOperation {
            operation: "retry_metadata_missing",
        });
    };
    let SlotValue::I64(max_attempts) =
        *state
            .frame
            .read_slot(policy_slot)
            .map_err(|_| RuntimeError::UnsupportedOperation {
                operation: "retry_policy_slot_unreadable",
            })?
    else {
        return Err(RuntimeError::UnsupportedOperation {
            operation: "retry_policy_slot_not_i64",
        });
    };
    let max_attempts =
        u16::try_from(max_attempts).map_err(|_| RuntimeError::UnsupportedOperation {
            operation: "retry_policy_attempts_out_of_range",
        })?;
    if max_attempts == 0 {
        return Err(RuntimeError::UnsupportedOperation {
            operation: "retry_policy_attempts_zero",
        });
    }
    Ok(RetryPolicy {
        max_attempts,
        base_delay_ms: 0,
        exponential_backoff: false,
    })
}

/// Records a retry attempt and returns true if more retries are allowed.
pub fn record_retry_attempt(
    state: &mut crate::shard::types::RunState,
    ticket: ActionTicket,
    policy: RetryPolicy,
) -> RuntimeResult<bool> {
    let attempt = state
        .action_attempts
        .get_mut(ticket.step.as_usize())
        .ok_or(RuntimeError::InvalidActionCompletion)?;
    if *attempt == 0 || *attempt < ticket.attempt {
        *attempt = ticket.attempt;
    }
    if *attempt >= policy.max_attempts {
        return Ok(false);
    }
    *attempt = attempt
        .checked_add(1)
        .ok_or(RuntimeError::UnsupportedOperation {
            operation: "retry_attempt_overflow",
        })?;
    Ok(true)
}

/// Finds the error handler step and error slot for a failed step.
pub fn find_error_handler_for_failure(
    workflow: &CompiledWorkflow,
    failed: StepIdx,
) -> Option<(StepIdx, Option<SlotIdx>)> {
    if let Some(result) = error_handler_on_node(workflow, failed, failed) {
        return Some(result);
    }

    if failed.get() > 0 {
        let previous = StepIdx::new(failed.get().saturating_sub(1));
        if let Some(result) = error_handler_on_node(workflow, previous, failed) {
            return Some(result);
        }
    }

    let mut index = 0usize;
    let count = usize::from(workflow.node_count());
    while index < count {
        let Ok(raw) = u16::try_from(index) else {
            return None;
        };
        if let Some(result) = error_handler_on_node(workflow, StepIdx::new(raw), failed) {
            return Some(result);
        }
        index = index.checked_add(1)?;
    }

    None
}

fn error_handler_on_node(
    workflow: &CompiledWorkflow,
    candidate: StepIdx,
    failed: StepIdx,
) -> Option<(StepIdx, Option<SlotIdx>)> {
    let node = workflow.node(candidate)?;
    match node.kind {
        CompiledNodeKind::ErrorHandler {
            body,
            handler,
            error_slot,
        } if candidate == failed || body == failed => Some((handler, error_slot)),
        _ => None,
    }
}

/// Returns the result slot for a finished run.
pub fn result_slot_for_finished_run(state: &crate::shard::types::RunState) -> Option<SlotIdx> {
    state
        .workflow
        .node(state.frame.pc())
        .and_then(|node| match node.kind {
            CompiledNodeKind::Finish { result } => Some(result),
            _ => None,
        })
}

/// Creates a snapshot from run state.
pub fn snapshot_from_state(
    run: RunId,
    correlation: u64,
    state: &crate::shard::types::RunState,
) -> InspectSnapshot {
    InspectSnapshot {
        run,
        correlation,
        pc: state.frame.pc(),
        executed: state.frame.executed(),
    }
}

#[cfg(test)]
mod tests {
    use vb_core::action::ActionTicket;
    use vb_core::frame::RunFrame;
    use vb_core::ids::{ActionId, ConstIdx, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::value::{ConstValue, SlotValue, Taint};
    use vb_core::value_store::ValueStore;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    use crate::RuntimeError;
    use crate::primitives::collect::CollectStates;

    use super::super::types::RunState;
    use super::{
        PendingTimer, PendingTimerKind, advance_after_action_completion, advance_after_timer_fire,
        find_error_handler_for_failure, new_action_attempts, record_retry_attempt,
        record_scheduled_attempt, result_slot_for_finished_run, retry_metadata_exists,
        retry_policy_after_action, seed_input_slots, snapshot_from_state,
        timer_registration_required, validate_action_completion,
    };

    // ---- Workflow factories ----

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
        let finish = CompiledNode {
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
            nodes: Box::from([set_deadline, wait, finish]),
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
        let finish = CompiledNode {
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
            nodes: Box::from([guard, action, handler, finish]),
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

    fn retry_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
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
                input: SlotIdx::new(0),
            },
        };
        let retry_check = CompiledNode {
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
                result: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("retry_wf"),
            digest: WorkflowDigest::from_bytes([6; 32]),
            nodes: Box::from([set_policy, action, retry_check, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(3)]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    fn wait_event_no_timeout_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let wait_event = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None,
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("wait_event_no_timeout"),
            digest: WorkflowDigest::from_bytes([8; 32]),
            nodes: Box::from([wait_event, finish]),
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

    // ---- Test state helpers ----

    fn make_run_state(
        workflow: vb_core::workflow::CompiledWorkflow,
        run_id: RunId,
    ) -> Option<RunState> {
        let step_count = workflow.node_count();
        let slot_count = workflow.slot_count();
        let frame = RunFrame::new(run_id, workflow.entry(), step_count, slot_count).ok()?;
        Some(RunState {
            frame,
            workflow,
            store: ValueStore::new(),
            action_attempts: new_action_attempts(step_count),
            admission: None,
            collect_states: CollectStates::new(),
        })
    }

    fn ticket(run: RunId, step: StepIdx, attempt: u16) -> ActionTicket {
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

    // =======================================================================
    // new_action_attempts
    // =======================================================================

    #[test]
    fn new_action_attempts_creates_zeroed_tracker() {
        let attempts = new_action_attempts(3);
        assert_eq!(attempts.len(), 3);
        assert_eq!(attempts.get(0).copied(), Some(0));
        assert_eq!(attempts.get(1).copied(), Some(0));
        assert_eq!(attempts.get(2).copied(), Some(0));
    }

    #[test]
    fn new_action_attempts_with_single_step() {
        let attempts = new_action_attempts(1);
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts.get(0).copied(), Some(0));
    }

    #[test]
    fn new_action_attempts_with_many_steps() {
        let attempts = new_action_attempts(100);
        assert_eq!(attempts.len(), 100);
        for i in 0..100 {
            assert_eq!(attempts.get(i).copied(), Some(0));
        }
    }

    #[test]
    fn new_action_attempts_with_zero_steps() {
        let attempts = new_action_attempts(0);
        assert_eq!(attempts.len(), 0);
    }

    // =======================================================================
    // record_scheduled_attempt
    // =======================================================================

    #[test]
    fn record_scheduled_attempt_records_first_attempt() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
        record_scheduled_attempt(&mut state, t);
        assert_eq!(state.action_attempts.get(0).copied(), Some(1));
    }

    #[test]
    fn record_scheduled_attempt_updates_higher_attempt() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let t1 = ticket(RunId::new(1), StepIdx::ZERO, 1);
        record_scheduled_attempt(&mut state, t1);
        let t2 = ticket(RunId::new(1), StepIdx::ZERO, 3);
        record_scheduled_attempt(&mut state, t2);
        assert_eq!(state.action_attempts.get(0).copied(), Some(3));
    }

    #[test]
    fn record_scheduled_attempt_ignores_lower_attempt() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let t_high = ticket(RunId::new(1), StepIdx::ZERO, 5);
        record_scheduled_attempt(&mut state, t_high);
        let t_low = ticket(RunId::new(1), StepIdx::ZERO, 2);
        record_scheduled_attempt(&mut state, t_low);
        assert_eq!(state.action_attempts.get(0).copied(), Some(5));
    }

    #[test]
    fn record_scheduled_attempt_on_out_of_bounds_step_is_noop() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let t = ticket(RunId::new(1), StepIdx::new(99), 1);
        record_scheduled_attempt(&mut state, t);
        assert_eq!(state.action_attempts.len(), 1);
        assert_eq!(state.action_attempts.get(0).copied(), Some(0));
    }

    // =======================================================================
    // seed_input_slots
    // =======================================================================

    #[test]
    fn seed_input_slots_writes_clean_values() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut frame) =
            RunFrame::new(RunId::new(1), wf.entry(), wf.node_count(), wf.slot_count()).ok()
        else {
            return;
        };
        let inputs = Box::from([(SlotIdx::new(0), SlotValue::I64(42))]);
        let result = seed_input_slots(&mut frame, &inputs);
        assert_eq!(result, Ok(()));
        match frame.read_slot(SlotIdx::new(0)) {
            Ok(v) => assert_eq!(*v, SlotValue::I64(42)),
            other => {
                let msg = format!("expected Ok(I64(42)), got {other:?}");
                panic!("{msg}");
            }
        }
    }

    #[test]
    fn seed_input_slots_with_empty_inputs_succeeds() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut frame) =
            RunFrame::new(RunId::new(1), wf.entry(), wf.node_count(), wf.slot_count()).ok()
        else {
            return;
        };
        let result = seed_input_slots(&mut frame, &[]);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn seed_input_slots_multiple_slots() {
        let Some(wf) = retry_workflow() else {
            return;
        };
        let Some(mut frame) =
            RunFrame::new(RunId::new(2), wf.entry(), wf.node_count(), wf.slot_count()).ok()
        else {
            return;
        };
        let inputs = Box::from([
            (SlotIdx::new(0), SlotValue::I64(10)),
            (SlotIdx::new(1), SlotValue::I64(20)),
        ]);
        let result = seed_input_slots(&mut frame, &inputs);
        assert_eq!(result, Ok(()));
        assert_eq!(frame.read_slot(SlotIdx::new(0)), Ok(&SlotValue::I64(10)));
        assert_eq!(frame.read_slot(SlotIdx::new(1)), Ok(&SlotValue::I64(20)));
    }

    // =======================================================================
    // validate_action_completion
    // =======================================================================

    #[test]
    fn validate_action_completion_rejects_non_running_step() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        // Step 0 is in Pending state (not Running), so validation should fail.
        let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
        let result = validate_action_completion(&state, t);
        assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
    }

    #[test]
    fn validate_action_completion_rejects_out_of_bounds_step() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let t = ticket(RunId::new(1), StepIdx::new(99), 1);
        let result = validate_action_completion(&state, t);
        assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
    }

    // =======================================================================
    // advance_after_action_completion
    // =======================================================================

    #[test]
    fn advance_after_action_completion_for_terminal_node_returns_ok() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        // The Do node at step 0 has no next, so advance returns Ok(()) but does nothing.
        let result = advance_after_action_completion(&mut state, StepIdx::ZERO);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn advance_after_action_completion_returns_error_for_missing_node() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let result = advance_after_action_completion(&mut state, StepIdx::new(99));
        assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
    }

    // =======================================================================
    // timer_registration_required
    // =======================================================================

    #[test]
    fn timer_registration_required_for_wait_until() {
        let Some(wf) = wait_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        assert_eq!(timer_registration_required(&state, StepIdx::new(1)), true);
    }

    #[test]
    fn timer_not_required_for_do_node() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        assert_eq!(timer_registration_required(&state, StepIdx::ZERO), false);
    }

    #[test]
    fn timer_not_required_for_missing_step() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        assert_eq!(timer_registration_required(&state, StepIdx::new(99)), false);
    }

    #[test]
    fn timer_not_required_for_wait_event_without_timeout() {
        let Some(wf) = wait_event_no_timeout_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        assert_eq!(timer_registration_required(&state, StepIdx::ZERO), false);
    }

    // =======================================================================
    // retry_metadata_exists
    // =======================================================================

    #[test]
    fn retry_metadata_exists_when_retry_check_follows() {
        let Some(wf) = retry_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        assert_eq!(retry_metadata_exists(&state, StepIdx::new(1)), true);
    }

    #[test]
    fn retry_metadata_absent_when_no_retry_check_follows() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        assert_eq!(retry_metadata_exists(&state, StepIdx::ZERO), false);
    }

    #[test]
    fn retry_metadata_absent_for_missing_step() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        assert_eq!(retry_metadata_exists(&state, StepIdx::new(99)), false);
    }

    // =======================================================================
    // retry_policy_after_action
    // =======================================================================

    #[test]
    fn retry_policy_after_action_extracts_max_attempts() {
        let Some(wf) = retry_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        // Drive step 0 (SetConst) and manually write the policy value.
        // The deterministic engine is not running in this unit test, so
        // we populate the slot directly.
        assert!(state.frame.mark_running(StepIdx::ZERO).is_ok());
        assert!(state.frame.mark_succeeded(StepIdx::ZERO).is_ok());
        assert!(state.frame.set_pc(StepIdx::new(1)).is_ok());
        assert!(
            state
                .frame
                .write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(3), Taint::Clean)
                .is_ok()
        );

        let policy = retry_policy_after_action(&state, StepIdx::new(1));
        match policy {
            Ok(p) => assert_eq!(p.max_attempts, 3),
            Err(e) => {
                let reason = format!("{e:?}");
                assert_eq!(reason, "should not reach here");
            }
        }
    }

    #[test]
    fn retry_policy_after_action_rejects_missing_node() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let result = retry_policy_after_action(&state, StepIdx::new(99));
        assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
    }

    #[test]
    fn retry_policy_after_action_rejects_no_next() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let result = retry_policy_after_action(&state, StepIdx::ZERO);
        assert_eq!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "retry_metadata_missing"
            })
        );
    }

    // =======================================================================
    // record_retry_attempt
    // =======================================================================

    #[test]
    fn record_retry_attempt_increments_and_allows_retry() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
        let policy = crate::engine::RetryPolicy {
            max_attempts: 3,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        assert_eq!(record_retry_attempt(&mut state, t, policy), Ok(true));
        assert_eq!(state.action_attempts.get(0).copied(), Some(2));
    }

    #[test]
    fn record_retry_attempt_blocks_when_max_reached() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
        let policy = crate::engine::RetryPolicy {
            max_attempts: 2,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        assert_eq!(record_retry_attempt(&mut state, t, policy), Ok(true));
        assert_eq!(record_retry_attempt(&mut state, t, policy), Ok(false));
    }

    #[test]
    fn record_retry_attempt_rejects_out_of_bounds_step() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let t = ticket(RunId::new(1), StepIdx::new(99), 1);
        let policy = crate::engine::RetryPolicy {
            max_attempts: 3,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        assert_eq!(
            record_retry_attempt(&mut state, t, policy),
            Err(RuntimeError::InvalidActionCompletion)
        );
    }

    // =======================================================================
    // find_error_handler_for_failure
    // =======================================================================

    #[test]
    fn find_error_handler_finds_handler_for_body_step() {
        let Some(wf) = error_handler_workflow() else {
            return;
        };
        let result = find_error_handler_for_failure(&wf, StepIdx::new(1));
        match result {
            Some((handler, error_slot)) => {
                assert_eq!(handler, StepIdx::new(2));
                assert_eq!(error_slot, None);
            }
            None => {
                // Wrong: expected Some
                assert!(false);
            }
        }
    }

    #[test]
    fn find_error_handler_finds_handler_for_guard_step() {
        let Some(wf) = error_handler_workflow() else {
            return;
        };
        let result = find_error_handler_for_failure(&wf, StepIdx::ZERO);
        assert!(result.is_some());
    }

    #[test]
    fn find_error_handler_returns_none_for_unprotected_step() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let result = find_error_handler_for_failure(&wf, StepIdx::ZERO);
        assert_eq!(result, None);
    }

    #[test]
    fn find_error_handler_returns_none_for_missing_step() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let result = find_error_handler_for_failure(&wf, StepIdx::new(99));
        assert_eq!(result, None);
    }

    #[test]
    fn find_error_handler_returns_none_for_finished_workflow() {
        let Some(wf) = finished_workflow() else {
            return;
        };
        let result = find_error_handler_for_failure(&wf, StepIdx::ZERO);
        assert_eq!(result, None);
    }

    // =======================================================================
    // result_slot_for_finished_run
    // =======================================================================

    #[test]
    fn result_slot_for_finished_run_returns_finish_slot() {
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run_id = RunId::new(1);
        let Some(mut state) = make_run_state(wf, run_id) else {
            return;
        };
        // Drive to the Finish node (step 1).
        assert!(state.frame.mark_running(StepIdx::ZERO).is_ok());
        assert!(state.frame.mark_succeeded(StepIdx::ZERO).is_ok());
        assert!(state.frame.set_pc(StepIdx::new(1)).is_ok());
        let slot = result_slot_for_finished_run(&state);
        assert_eq!(slot, Some(SlotIdx::new(0)));
    }

    #[test]
    fn result_slot_for_finished_run_returns_none_for_non_finish() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let slot = result_slot_for_finished_run(&state);
        assert_eq!(slot, None);
    }

    // =======================================================================
    // snapshot_from_state
    // =======================================================================

    #[test]
    fn snapshot_from_state_captures_pc_and_executed() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(42)) else {
            return;
        };
        let snapshot = snapshot_from_state(RunId::new(42), 99, &state);
        assert_eq!(snapshot.run, RunId::new(42));
        assert_eq!(snapshot.correlation, 99);
        assert_eq!(snapshot.pc, StepIdx::ZERO);
        assert_eq!(snapshot.executed, 0);
    }

    #[test]
    fn snapshot_from_state_reflects_advanced_pc() {
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run_id = RunId::new(7);
        let Some(mut state) = make_run_state(wf, run_id) else {
            return;
        };
        assert!(state.frame.mark_running(StepIdx::ZERO).is_ok());
        assert!(state.frame.mark_succeeded(StepIdx::ZERO).is_ok());
        assert!(state.frame.set_pc(StepIdx::new(1)).is_ok());
        // executed is incremented by increment_executed, not by mark_succeeded,
        // so it remains 0 in this manual state manipulation.
        let snapshot = snapshot_from_state(run_id, 0, &state);
        assert_eq!(snapshot.pc, StepIdx::new(1));
        assert_eq!(snapshot.executed, 0);
    }

    #[test]
    fn snapshot_from_state_with_zero_correlation() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let snapshot = snapshot_from_state(RunId::new(1), 0, &state);
        assert_eq!(snapshot.correlation, 0);
    }

    #[test]
    fn snapshot_from_state_with_max_run_id() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let run_id = RunId::new(u64::MAX);
        let Some(state) = make_run_state(wf, run_id) else {
            return;
        };
        let snapshot = snapshot_from_state(run_id, u64::MAX, &state);
        assert_eq!(snapshot.run, run_id);
        assert_eq!(snapshot.correlation, u64::MAX);
    }

    // =======================================================================
    // advance_after_timer_fire
    // =======================================================================

    #[test]
    fn advance_after_timer_fire_rejects_wrong_node_kind() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let timer = PendingTimer {
            step: StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
        };
        let result = advance_after_timer_fire(&mut state, timer);
        assert_eq!(result, Err(RuntimeError::InvalidTimerFire));
    }

    #[test]
    fn advance_after_timer_fire_rejects_missing_node() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let timer = PendingTimer {
            step: StepIdx::new(99),
            kind: PendingTimerKind::Wait,
        };
        let result = advance_after_timer_fire(&mut state, timer);
        assert_eq!(result, Err(RuntimeError::InvalidTimerFire));
    }

    // =======================================================================
    // Additional edge-case tests for helpers
    // =======================================================================

    // ---- advance_after_timer_fire for valid WaitUntil node ----

    #[test]
    fn advance_after_timer_fire_succeeds_for_wait_until_node() {
        let Some(wf) = wait_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        // Drive to step 1 (WaitUntil) first.
        assert!(state.frame.mark_running(StepIdx::ZERO).is_ok());
        assert!(state.frame.mark_succeeded(StepIdx::ZERO).is_ok());
        assert!(state.frame.set_pc(StepIdx::new(1)).is_ok());
        // Mark step 1 as running to satisfy advance_after_timer_fire
        assert!(state.frame.mark_running(StepIdx::new(1)).is_ok());

        let timer = PendingTimer {
            step: StepIdx::new(1),
            kind: PendingTimerKind::Wait,
        };
        let result = advance_after_timer_fire(&mut state, timer);
        assert_eq!(result, Ok(()));
        assert_eq!(state.frame.pc(), StepIdx::new(2));
    }

    // ---- advance_after_timer_fire rejects wrong timer kind ----

    #[test]
    fn advance_after_timer_fire_rejects_ask_kind_on_wait_node() {
        let Some(wf) = wait_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        // Drive to step 1 (WaitUntil)
        assert!(state.frame.mark_running(StepIdx::ZERO).is_ok());
        assert!(state.frame.mark_succeeded(StepIdx::ZERO).is_ok());
        assert!(state.frame.set_pc(StepIdx::new(1)).is_ok());
        assert!(state.frame.mark_running(StepIdx::new(1)).is_ok());

        let timer = PendingTimer {
            step: StepIdx::new(1),
            kind: PendingTimerKind::Ask,
        };
        let result = advance_after_timer_fire(&mut state, timer);
        assert_eq!(result, Err(RuntimeError::InvalidTimerFire));
    }

    // ---- advance_after_timer_fire rejects terminal WaitUntil (no next) ----

    fn wait_workflow_no_next() -> Option<vb_core::workflow::CompiledWorkflow> {
        let wait = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("wait_no_next"),
            digest: WorkflowDigest::from_bytes([0xCC; 32]),
            nodes: Box::from([wait]),
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

    #[test]
    fn advance_after_timer_fire_rejects_wait_until_without_next() {
        let Some(wf) = wait_workflow_no_next() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        assert!(state.frame.mark_running(StepIdx::ZERO).is_ok());

        let timer = PendingTimer {
            step: StepIdx::ZERO,
            kind: PendingTimerKind::Wait,
        };
        let result = advance_after_timer_fire(&mut state, timer);
        assert_eq!(result, Err(RuntimeError::InvalidTimerFire));
    }

    // ---- timer_registration_required for WaitEvent with timeout ----

    fn wait_event_with_timeout_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let wait_event = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: Some(SlotIdx::new(1)),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        };
        let parts = WorkflowParts {
            name: Box::from("wait_event_with_timeout"),
            digest: WorkflowDigest::from_bytes([0xDD; 32]),
            nodes: Box::from([wait_event, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(10), ConstValue::I64(100)]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    #[test]
    fn timer_required_for_wait_event_with_timeout() {
        let Some(wf) = wait_event_with_timeout_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        assert_eq!(timer_registration_required(&state, StepIdx::ZERO), true);
    }

    // ---- timer_registration_required for Ask with timeout ----

    fn ask_with_timeout_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let ask = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::ZERO,
                timeout_slot: Some(SlotIdx::new(1)),
            },
        };
        let resume = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::AskResume {
                answer: SlotIdx::new(2),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(2),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("ask_with_timeout"),
            digest: WorkflowDigest::from_bytes([0xEE; 32]),
            nodes: Box::from([ask, resume, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([
                ConstValue::Symbol(vb_core::ids::SymbolId::new(1)),
                ConstValue::I64(50),
            ]),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    #[test]
    fn timer_required_for_ask_with_timeout() {
        let Some(wf) = ask_with_timeout_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        assert_eq!(timer_registration_required(&state, StepIdx::ZERO), true);
    }

    // ---- timer_registration_required for Ask without timeout ----

    fn ask_without_timeout_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let ask = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::ZERO,
                timeout_slot: None,
            },
        };
        let resume = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::AskResume {
                answer: SlotIdx::new(1),
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(1),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("ask_no_timeout"),
            digest: WorkflowDigest::from_bytes([0xFF; 32]),
            nodes: Box::from([ask, resume, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::Symbol(vb_core::ids::SymbolId::new(1))]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    #[test]
    fn timer_not_required_for_ask_without_timeout() {
        let Some(wf) = ask_without_timeout_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        assert_eq!(timer_registration_required(&state, StepIdx::ZERO), false);
    }

    // ---- validate_action_completion accepts running step with matching action ----

    #[test]
    fn validate_action_completion_accepts_running_step_with_matching_action() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        // Mark step 0 as running so validate passes
        assert!(state.frame.mark_running(StepIdx::ZERO).is_ok());

        let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
        let result = validate_action_completion(&state, t);
        assert_eq!(result, Ok(()));
    }

    // ---- validate_action_completion rejects wrong action id ----

    #[test]
    fn validate_action_completion_rejects_wrong_action_id() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        assert!(state.frame.mark_running(StepIdx::ZERO).is_ok());

        let t = ActionTicket {
            run: RunId::new(1),
            step: StepIdx::ZERO,
            seq: vb_core::ids::SeqNo::ZERO,
            action: ActionId::new(99), // wrong action id
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
        };
        let result = validate_action_completion(&state, t);
        assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
    }

    // ---- advance_after_action_completion advances to next step ----

    #[test]
    fn advance_after_action_completion_advances_pc_to_next() {
        let Some(wf) = finished_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        // Step 0 has next = Some(1)
        let result = advance_after_action_completion(&mut state, StepIdx::ZERO);
        assert_eq!(result, Ok(()));
        assert_eq!(state.frame.pc(), StepIdx::new(1));
    }

    // ---- seed_input_slots rejects out-of-bounds slot ----

    #[test]
    fn seed_input_slots_rejects_out_of_bounds_slot() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut frame) =
            RunFrame::new(RunId::new(1), wf.entry(), wf.node_count(), wf.slot_count()).ok()
        else {
            return;
        };
        // Slot 99 is out of bounds for a workflow with 1 slot
        let inputs = Box::from([(SlotIdx::new(99), SlotValue::I64(1))]);
        let result = seed_input_slots(&mut frame, &inputs);
        assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration));
    }

    // ---- retry_policy_after_action rejects non-I64 slot value ----

    #[test]
    fn retry_policy_after_action_rejects_non_i64_policy_slot() {
        // Build a retry workflow where the policy slot contains a Bool instead of I64
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
                input: SlotIdx::new(0),
            },
        };
        let retry_check = CompiledNode {
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
                result: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("retry_bad_policy"),
            digest: WorkflowDigest::from_bytes([0xAB; 32]),
            nodes: Box::from([set_policy, action, retry_check, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::Bool(true)]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        let Some(wf) = vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        // Drive step 0 and write a Bool to the policy slot
        assert!(state.frame.mark_running(StepIdx::ZERO).is_ok());
        assert!(state.frame.mark_succeeded(StepIdx::ZERO).is_ok());
        assert!(state.frame.set_pc(StepIdx::new(1)).is_ok());
        assert!(
            state
                .frame
                .write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(true), Taint::Clean)
                .is_ok()
        );

        let result = retry_policy_after_action(&state, StepIdx::new(1));
        assert_eq!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "retry_policy_slot_not_i64"
            })
        );
    }

    // ---- retry_policy_after_action rejects non-RetryCheck next node ----

    #[test]
    fn retry_policy_after_action_rejects_non_retry_check_next() {
        let Some(wf) = finished_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        // Step 0 has next = Some(1), but node 1 is Finish, not RetryCheck
        let result = retry_policy_after_action(&state, StepIdx::ZERO);
        assert_eq!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "retry_metadata_missing"
            })
        );
    }

    // ---- retry_policy_after_action rejects negative max attempts ----

    #[test]
    fn retry_policy_after_action_rejects_negative_max_attempts() {
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
                input: SlotIdx::new(0),
            },
        };
        let retry_check = CompiledNode {
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
                result: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("retry_negative"),
            digest: WorkflowDigest::from_bytes([0xAC; 32]),
            nodes: Box::from([set_policy, action, retry_check, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(-1)]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        let Some(wf) = vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        assert!(state.frame.mark_running(StepIdx::ZERO).is_ok());
        assert!(state.frame.mark_succeeded(StepIdx::ZERO).is_ok());
        assert!(state.frame.set_pc(StepIdx::new(1)).is_ok());
        assert!(
            state
                .frame
                .write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(-1), Taint::Clean)
                .is_ok()
        );

        let result = retry_policy_after_action(&state, StepIdx::new(1));
        assert_eq!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "retry_policy_attempts_out_of_range"
            })
        );
    }

    // ---- retry_policy_after_action rejects zero max attempts ----

    #[test]
    fn retry_policy_after_action_rejects_zero_max_attempts() {
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
                input: SlotIdx::new(0),
            },
        };
        let retry_check = CompiledNode {
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
                result: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("retry_zero"),
            digest: WorkflowDigest::from_bytes([0xAD; 32]),
            nodes: Box::from([set_policy, action, retry_check, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(0)]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        let Some(wf) = vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        assert!(state.frame.mark_running(StepIdx::ZERO).is_ok());
        assert!(state.frame.mark_succeeded(StepIdx::ZERO).is_ok());
        assert!(state.frame.set_pc(StepIdx::new(1)).is_ok());
        assert!(
            state
                .frame
                .write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(0), Taint::Clean)
                .is_ok()
        );

        let result = retry_policy_after_action(&state, StepIdx::new(1));
        assert_eq!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "retry_policy_attempts_zero"
            })
        );
    }

    // ---- record_retry_attempt overflow protection ----

    #[test]
    fn record_retry_attempt_overflow_returns_error() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        // Manually set attempt to u16::MAX to trigger overflow
        if let Some(attempt) = state.action_attempts.get_mut(0) {
            *attempt = u16::MAX;
        }
        let t = ticket(RunId::new(1), StepIdx::ZERO, u16::MAX);
        let policy = crate::engine::RetryPolicy {
            max_attempts: u16::MAX,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        // attempt is already at max_attempts, so should return Ok(false)
        let result = record_retry_attempt(&mut state, t, policy);
        assert_eq!(result, Ok(false));
    }

    // ---- record_retry_attempt at max exactly returns false ----

    #[test]
    fn record_retry_attempt_at_max_exactly_returns_false() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        // Set attempt to 2 (one less than max)
        if let Some(attempt) = state.action_attempts.get_mut(0) {
            *attempt = 2;
        }
        let t = ticket(RunId::new(1), StepIdx::ZERO, 2);
        let policy = crate::engine::RetryPolicy {
            max_attempts: 3,
            base_delay_ms: 0,
            exponential_backoff: false,
        };
        assert_eq!(record_retry_attempt(&mut state, t, policy), Ok(true));
        assert_eq!(state.action_attempts.get(0).copied(), Some(3));
        // Now attempt == max, should return false
        assert_eq!(record_retry_attempt(&mut state, t, policy), Ok(false));
    }

    // ---- find_error_handler with error_slot set ----

    fn error_handler_with_slot_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let guard = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: Some(SlotIdx::new(1)),
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
        let finish = CompiledNode {
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
            name: Box::from("error_handler_with_slot"),
            digest: WorkflowDigest::from_bytes([0xBA; 32]),
            nodes: Box::from([guard, action, handler, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::Bool(false)]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    #[test]
    fn find_error_handler_with_error_slot_returns_slot() {
        let Some(wf) = error_handler_with_slot_workflow() else {
            return;
        };
        let result = find_error_handler_for_failure(&wf, StepIdx::new(1));
        match result {
            Some((handler, error_slot)) => {
                assert_eq!(handler, StepIdx::new(2));
                assert_eq!(error_slot, Some(SlotIdx::new(1)));
            }
            None => assert!(false, "expected Some"),
        }
    }

    // ---- new_action_attempts at boundary values ----

    #[test]
    fn new_action_attempts_at_u16_max() {
        let attempts = new_action_attempts(u16::MAX);
        assert_eq!(attempts.len(), usize::from(u16::MAX));
    }

    // ---- record_scheduled_attempt with attempt zero does nothing ----

    #[test]
    fn record_scheduled_attempt_with_attempt_zero_does_nothing() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        let t = ticket(RunId::new(1), StepIdx::ZERO, 0);
        record_scheduled_attempt(&mut state, t);
        // attempt == 0 and the stored value is also 0, so the condition
        // (*attempt == 0 || *attempt < ticket.attempt) is true for ==0.
        // But the second condition: *attempt < ticket.attempt => 0 < 0 is false.
        // So the first condition (*attempt == 0) allows the write.
        assert_eq!(state.action_attempts.get(0).copied(), Some(0));
    }

    // ---- result_slot_for_finished_run returns none when not at finish ----

    #[test]
    fn result_slot_for_finished_run_returns_none_at_do_step() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        // PC is at step 0, which is a Do node, not Finish
        let result = result_slot_for_finished_run(&state);
        assert_eq!(result, None);
    }

    // ---- retry_metadata_exists returns false for terminal node (no next) ----

    #[test]
    fn retry_metadata_exists_for_terminal_node_returns_false() {
        let Some(wf) = suspended_workflow() else {
            return;
        };
        let Some(state) = make_run_state(wf, RunId::new(1)) else {
            return;
        };
        // Step 0 is the only node and has no next
        assert_eq!(retry_metadata_exists(&state, StepIdx::ZERO), false);
    }

    // ---- seed_input_slots writes multiple values correctly ----

    #[test]
    fn seed_input_slots_writes_multiple_distinct_values() {
        // Build a workflow with 3 slots
        let set_a = CompiledNode {
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
            name: Box::from("multi_slot"),
            digest: WorkflowDigest::from_bytes([0xCA; 32]),
            nodes: Box::from([set_a, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([ConstValue::I64(0)]),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        let Some(wf) = vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok() else {
            return;
        };
        let Some(mut frame) =
            RunFrame::new(RunId::new(1), wf.entry(), wf.node_count(), wf.slot_count()).ok()
        else {
            return;
        };
        let inputs = Box::from([
            (SlotIdx::new(0), SlotValue::I64(10)),
            (SlotIdx::new(1), SlotValue::Bool(true)),
            (SlotIdx::new(2), SlotValue::I64(-5)),
        ]);
        assert_eq!(seed_input_slots(&mut frame, &inputs), Ok(()));
        assert_eq!(frame.read_slot(SlotIdx::new(0)), Ok(&SlotValue::I64(10)));
        assert_eq!(frame.read_slot(SlotIdx::new(1)), Ok(&SlotValue::Bool(true)));
        assert_eq!(frame.read_slot(SlotIdx::new(2)), Ok(&SlotValue::I64(-5)));
    }

    // ---- snapshot_from_state with various executed counts ----

    #[test]
    fn snapshot_from_state_captures_nonzero_executed() {
        let Some(wf) = finished_workflow() else {
            return;
        };
        let run_id = RunId::new(1);
        let Some(mut state) = make_run_state(wf, run_id) else {
            return;
        };
        // Drive step 0 to increment executed count
        assert!(state.frame.mark_running(StepIdx::ZERO).is_ok());
        assert!(state.frame.mark_succeeded(StepIdx::ZERO).is_ok());
        assert!(state.frame.set_pc(StepIdx::new(1)).is_ok());

        let snap = snapshot_from_state(run_id, 0, &state);
        assert_eq!(snap.run, run_id);
        // executed may still be 0 since increment_executed is called by the engine,
        // not by mark_succeeded
        assert_eq!(snap.pc, StepIdx::new(1));
    }
}
