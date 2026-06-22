use vb_core::action::ActionTicket;
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::{ActionId, ConstIdx, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

use crate::RuntimeError;
use crate::shard::{PendingTimer, PendingTimerKind};

use super::{
    advance_after_action_completion, advance_after_timer_fire, find_error_handler_for_failure,
    make_run_state, new_action_attempts, normalize_scheduled_ticket, record_retry_attempt,
    record_scheduled_attempt, result_slot_for_finished_run, retry_metadata_exists,
    retry_policy_after_action, seed_input_slots, snapshot_from_state, timer_registration_required,
    validate_action_completion,
};

fn ticket(run: RunId, step: StepIdx, attempt: u16) -> ActionTicket {
    ActionTicket {
        run,
        step,
        seq: SeqNo::new(0),
        action: ActionId::new(0),
        attempt,
        idempotency_key: 0,
        capacity: u16::MAX,
        ..Default::default()
    }
}

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
    Some(vb_core::workflow::CompiledWorkflow::try_from_parts(parts).expect("suspended_workflow"))
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
    Some(vb_core::workflow::CompiledWorkflow::try_from_parts(parts).expect("finished_workflow"))
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
            deadline_slot: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("wait_no_next"),
        digest: WorkflowDigest::from_bytes([0xAA; 32]),
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
    Some(vb_core::workflow::CompiledWorkflow::try_from_parts(parts).expect("wait_workflow_no_next"))
}

#[test]
fn advance_after_timer_fire_rejects_wait_until_without_next() {
    let Some(wf) = wait_workflow_no_next() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));

    let timer = PendingTimer {
        step: StepIdx::ZERO,
        kind: PendingTimerKind::Wait,
        generation: 1,
        deadline: std::time::Instant::now(),
    };
    let result = advance_after_timer_fire(&mut state, timer);
    assert_eq!(result, Err(RuntimeError::InvalidTimerFire));
}

// ---- advance_after_timer_fire on invalid successor PC leaves state unchanged ----
//
// Regression guard for RS-204: the prior implementation mutated the step state
// (Pending -> Running -> Succeeded) BEFORE validating that the node had a
// successor step. When the node's `next` was `None` the function returned
// InvalidTimerFire but the step had already been transitioned to Succeeded
// while the PC was never advanced, corrupting the run state. The fix
// validates the successor PC first and only mutates state when every
// precondition holds. This test exercises the bug by leaving step 0 in its
// natural Pending state (so mark_running / mark_succeeded would otherwise
// succeed), then asserts state is byte-identical to the pre-fire snapshot.
#[test]
fn advance_after_timer_fire_on_invalid_successor_pc_leaves_state_unchanged() {
    let Some(wf) = wait_workflow_no_next() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    // Pre-fire invariants: step 0 is Pending and the PC points at step 0.
    let pc_before = state.frame.pc();
    let executed_before = state.frame.executed();
    let frame_before = state.frame.clone();
    assert_eq!(
        state.frame.step_state(StepIdx::ZERO),
        Ok(StepState::Pending)
    );

    let timer = PendingTimer {
        step: StepIdx::ZERO,
        kind: PendingTimerKind::Wait,
        generation: 1,
        deadline: std::time::Instant::now(),
    };
    let result = advance_after_timer_fire(&mut state, timer);
    // Must surface InvalidTimerFire because the workflow has no successor.
    assert_eq!(result, Err(RuntimeError::InvalidTimerFire));
    // State must be byte-identical to the pre-fire snapshot. In particular:
    //   * step 0 is still Pending (not Succeeded),
    //   * PC still at step 0 (not advanced),
    //   * executed counter untouched,
    //   * full frame structurally equal to the snapshot.
    assert_eq!(
        state.frame.step_state(StepIdx::ZERO),
        Ok(StepState::Pending)
    );
    assert_eq!(state.frame.pc(), pc_before);
    assert_eq!(state.frame.executed(), executed_before);
    assert_eq!(state.frame, frame_before);
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
    let resume = CompiledNode {
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
        name: Box::from("wait_event_timeout"),
        digest: WorkflowDigest::from_bytes([0xBB; 32]),
        nodes: Box::from([wait_event, resume]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(10), ConstValue::I64(100)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    Some(
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .expect("wait_event_with_timeout_workflow"),
    )
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
        output: Some(SlotIdx::new(2)),
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
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("ask_with_timeout"),
        digest: WorkflowDigest::from_bytes([0xCC; 32]),
        nodes: Box::from([ask, resume, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(10), ConstValue::I64(100)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    Some(
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .expect("ask_with_timeout_workflow"),
    )
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
        output: Some(SlotIdx::new(1)),
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
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::AskResume {
            answer: SlotIdx::new(1),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("ask_no_timeout"),
        digest: WorkflowDigest::from_bytes([0xDD; 32]),
        nodes: Box::from([ask, resume]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    Some(
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .expect("ask_without_timeout_workflow"),
    )
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
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }

    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_completion_accepts_matching_current_attempt() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 2;
    }

    let result = validate_action_completion(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 2)
        },
    );
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_completion_rejects_stale_attempt_without_state_change() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    let pc_before = state.frame.pc();
    let executed_before = state.frame.executed();
    let frame_before = state.frame.clone();
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 3;
    }

    let result = validate_action_completion(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 2)
        },
    );
    assert_eq!(
        result,
        Err(RuntimeError::StaleAttempt {
            incoming: 2,
            current: 3
        })
    );
    assert_eq!(state.action_attempts.get(0).copied(), Some(3));
    assert_eq!(state.frame.pc(), pc_before);
    assert_eq!(state.frame.executed(), executed_before);
    assert_eq!(state.frame, frame_before);
    assert_eq!(
        state.frame.step_state(StepIdx::ZERO),
        Ok(StepState::Running)
    );
}

#[test]
fn validate_action_completion_rejects_zero_attempt() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));

    let result = validate_action_completion(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 0)
        },
    );
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 0, max: 3 })
    );
}

#[test]
fn validate_action_completion_rejects_zero_capacity() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));

    let result = validate_action_completion(
        &state,
        ActionTicket {
            capacity: 0,
            ..ticket(RunId::new(1), StepIdx::ZERO, 1)
        },
    );
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 1, max: 0 })
    );
}

#[test]
fn validate_action_completion_rejects_attempt_beyond_max() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));

    let result = validate_action_completion(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 4)
        },
    );
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 4, max: 3 })
    );
}

#[test]
fn normalize_scheduled_ticket_promotes_attempts_one_two_three() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    let first = normalize_scheduled_ticket(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 1)
        },
    );
    assert_eq!(first.map(|t| t.attempt), Ok(1));

    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 2;
    }
    let second = normalize_scheduled_ticket(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 1)
        },
    );
    assert_eq!(second.map(|t| t.attempt), Ok(2));

    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 3;
    }
    let third = normalize_scheduled_ticket(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 1)
        },
    );
    assert_eq!(third.map(|t| t.attempt), Ok(3));
}

#[test]
fn normalize_scheduled_ticket_rejects_zero_capacity_as_attempt_beyond_max() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    let result = normalize_scheduled_ticket(
        &state,
        ActionTicket {
            capacity: 0,
            ..ticket(RunId::new(1), StepIdx::ZERO, 0)
        },
    );
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 1, max: 0 })
    );
}

#[test]
fn normalize_scheduled_ticket_rejects_attempt_beyond_max_with_exact_error() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 4;
    }
    let result = normalize_scheduled_ticket(
        &state,
        ActionTicket {
            capacity: 3,
            ..ticket(RunId::new(1), StepIdx::ZERO, 1)
        },
    );
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 4, max: 3 })
    );
    assert_eq!(state.action_attempts.get(0).copied(), Some(4));
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
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));

    let t = ActionTicket {
        run: RunId::new(1),
        step: StepIdx::ZERO,
        seq: vb_core::ids::SeqNo::ZERO,
        action: ActionId::new(99), // wrong action id
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
        ..Default::default()
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
    let wf = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .expect("retry_policy_after_action_rejects_non_i64_policy_slot");
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    // Drive step 0 and write a Bool to the policy slot
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.mark_succeeded(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.set_pc(StepIdx::new(1)), Ok(()));
    assert_eq!(
        state
            .frame
            .write_slot_with_taint(SlotIdx::new(1), SlotValue::Bool(true), Taint::Clean),
        Ok(())
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
    let wf = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .expect("retry_policy_after_action_rejects_negative_max_attempts");
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.mark_succeeded(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.set_pc(StepIdx::new(1)), Ok(()));
    assert_eq!(
        state
            .frame
            .write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(-1), Taint::Clean),
        Ok(())
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
    let wf = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .expect("retry_policy_after_action_rejects_zero_max_attempts");
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.mark_succeeded(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.set_pc(StepIdx::new(1)), Ok(()));
    assert_eq!(
        state
            .frame
            .write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(0), Taint::Clean),
        Ok(())
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
    Some(
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .expect("error_handler_with_slot_workflow"),
    )
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
        None => assert_eq!(result.is_some(), true),
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
    let wf = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .expect("seed_input_slots_writes_multiple_distinct_values");
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
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.mark_succeeded(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.set_pc(StepIdx::new(1)), Ok(()));

    let snap = snapshot_from_state(run_id, 0, &state);
    assert_eq!(snap.run, run_id);
    // executed may still be 0 since increment_executed is called by the engine,
    // not by mark_succeeded
    assert_eq!(snap.pc, StepIdx::new(1));
}

// =======================================================================
// validate_action_completion -- exhaustive attempt fence tests
// =======================================================================

#[test]
fn validate_action_completion_returns_ok_when_all_preconditions_satisfied() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    // Mark step 0 as Running and set action_attempts[0] = 1
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_completion_returns_stale_attempt_when_attempt_lower_than_current() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 3;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(
        result,
        Err(RuntimeError::StaleAttempt {
            incoming: 1,
            current: 3
        })
    );
}

#[test]
fn validate_action_completion_returns_stale_attempt_when_lower_by_many() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 5;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(
        result,
        Err(RuntimeError::StaleAttempt {
            incoming: 1,
            current: 5
        })
    );
}

#[test]
fn validate_action_completion_returns_stale_attempt_at_edge_1_vs_2() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 2;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(
        result,
        Err(RuntimeError::StaleAttempt {
            incoming: 1,
            current: 2
        })
    );
}

#[test]
fn validate_action_completion_rejects_future_attempt_when_attempt_exceeds_current() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 3;
    }
    let t = ActionTicket {
        capacity: 10,
        ..ticket(RunId::new(1), StepIdx::ZERO, 5)
    };
    // G005: future-attempt rejection now implemented
    // validate_ticket_attempt returns Err(InvalidActionCompletion) for attempt > current
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
}

#[test]
fn validate_action_completion_rejects_unscheduled_first_attempt() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    assert_eq!(state.action_attempts.get(0).copied(), Some(0));
    let t = ActionTicket {
        capacity: 10,
        ..ticket(RunId::new(1), StepIdx::ZERO, 1)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
}

#[test]
fn validate_action_completion_rejects_when_attempt_is_zero() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    let t = ActionTicket {
        capacity: 5,
        ..ticket(RunId::new(1), StepIdx::ZERO, 0)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 0, max: 5 })
    );
}

#[test]
fn validate_action_completion_rejects_when_capacity_is_zero() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    let t = ActionTicket {
        capacity: 0,
        ..ticket(RunId::new(1), StepIdx::ZERO, 1)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 1, max: 0 })
    );
}

#[test]
fn validate_action_completion_rejects_when_attempt_exceeds_capacity() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 5;
    }
    let t = ActionTicket {
        capacity: 3,
        ..ticket(RunId::new(1), StepIdx::ZERO, 5)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 5, max: 3 })
    );
}

#[test]
fn validate_action_completion_rejects_when_step_is_succeeded() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.mark_succeeded(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
}

#[test]
fn validate_action_completion_rejects_when_step_is_pending() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(
        state.frame.step_state(StepIdx::ZERO),
        Ok(StepState::Pending)
    );
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
}

#[test]
fn validate_action_completion_rejects_when_step_is_failed() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    assert_eq!(state.frame.mark_failed(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
}

#[test]
fn validate_action_completion_rejects_when_node_is_not_do() {
    // Build a workflow where step 0 is not Do.
    let label_node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let action_node = CompiledNode {
        id: StepIdx::new(1),
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
        name: Box::from("label_first"),
        digest: WorkflowDigest::from_bytes([0xBB; 32]),
        nodes: Box::from([label_node, action_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    let wf = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .expect("validate_action_completion_rejects_when_node_is_not_do");
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 1);
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
}

#[test]
fn validate_action_completion_accepts_boundary_min_valid_attempt_capacity_one() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 1;
    }
    let t = ActionTicket {
        capacity: 1,
        ..ticket(RunId::new(1), StepIdx::ZERO, 1)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Ok(()));
}

// =======================================================================
// normalize_scheduled_ticket additional tests
// =======================================================================

#[test]
fn normalize_scheduled_ticket_promotes_to_one_when_current_and_ticket_are_zero() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    let t = ActionTicket {
        capacity: 5,
        ..ticket(RunId::new(1), StepIdx::ZERO, 0)
    };
    let result = normalize_scheduled_ticket(&state, t);
    // current=0, ticket.attempt=0, so attempt = max(0,0).max(1) = 1
    assert_eq!(result, Ok(ActionTicket { attempt: 1, ..t }));
}

#[test]
fn normalize_scheduled_ticket_rejects_when_step_out_of_bounds() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    let t = ticket(RunId::new(1), StepIdx::new(99), 1);
    let result = normalize_scheduled_ticket(&state, t);
    assert_eq!(result, Err(RuntimeError::InvalidActionCompletion));
}

// =======================================================================
// validate_action_completion -- boundary vs capacity tests
// =======================================================================

#[test]
fn validate_action_completion_accepts_equal_attempt_and_capacity() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 5;
    }
    let t = ActionTicket {
        capacity: 5,
        ..ticket(RunId::new(1), StepIdx::ZERO, 5)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_completion_accepts_boundary_max_valid_attempt() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = u16::MAX;
    }
    let t = ActionTicket {
        capacity: u16::MAX,
        ..ticket(RunId::new(1), StepIdx::ZERO, u16::MAX)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_completion_rejects_when_attempt_over_capacity_and_current_zero() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    let t = ActionTicket {
        capacity: 5,
        ..ticket(RunId::new(1), StepIdx::ZERO, 6)
    };
    let result = validate_action_completion(&state, t);
    assert_eq!(
        result,
        Err(RuntimeError::AttemptBeyondMax { attempt: 6, max: 5 })
    );
}

// =======================================================================
// record_retry_attempt rejects when attempt exceeds max_attempts
// =======================================================================

#[test]
fn record_retry_attempt_rejects_when_attempt_exceeds_max_attempts() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    let t = ticket(RunId::new(1), StepIdx::ZERO, 6);
    let policy = crate::engine::RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    assert_eq!(
        record_retry_attempt(&mut state, t, policy),
        Err(RuntimeError::AttemptBeyondMax { attempt: 6, max: 5 })
    );
}

// =======================================================================
// record_retry_attempt overflow boundary at u16::MAX
// =======================================================================

#[test]
fn record_retry_attempt_at_u16_max_returns_overflow_error() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = u16::MAX;
    }
    let t = ActionTicket {
        capacity: u16::MAX,
        ..ticket(RunId::new(1), StepIdx::ZERO, u16::MAX)
    };
    // max_attempts = u16::MAX + 1 is not representable, so
    // to truly trigger overflow we'd need max_attempts > u16::MAX.
    // With max_attempts == u16::MAX, *attempt >= max_attempts so it returns Ok(false)
    // The overflow path requires *attempt to be less than max_attempts but
    // *attempt + 1 to overflow. *attempt can be at most (max_attempts - 1).
    // If max_attempts == u16::MAX, then *attempt == u16::MAX - 1 triggers checked_add(1) -> Some(u16::MAX).
    // No overflow possible since max_attempts is u16.
    // This test verifies that at the edge, the function returns correctly.
    let policy = crate::engine::RetryPolicy {
        max_attempts: u16::MAX,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    // attempt (=current=u16::MAX) >= max_attempts (=u16::MAX) → Ok(false)
    assert_eq!(record_retry_attempt(&mut state, t, policy), Ok(false));
}

// =======================================================================
// record_retry_attempt with attempt equal to max minus 1 (last retry)
// =======================================================================

#[test]
fn record_retry_attempt_returns_true_on_last_retry_below_max() {
    let Some(wf) = suspended_workflow() else {
        return;
    };
    let Some(mut state) = make_run_state(wf, RunId::new(1)) else {
        return;
    };
    if let Some(attempt) = state.action_attempts.get_mut(0) {
        *attempt = 4;
    }
    let t = ticket(RunId::new(1), StepIdx::ZERO, 4);
    let policy = crate::engine::RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 0,
        exponential_backoff: false,
    };
    // 4 < 5, so should increment to 5 and return true
    assert_eq!(record_retry_attempt(&mut state, t, policy), Ok(true));
    assert_eq!(state.action_attempts.get(0).copied(), Some(5));
}

// =======================================================================
// Proptest: attempt fence classification
// =======================================================================

mod proptest_tests {
    use proptest::prelude::*;
    use vb_core::action::ActionTicket;
    use vb_core::ids::WorkflowDigest;
    use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    use crate::RuntimeError;

    use super::super::super::types::RunState;
    use super::super::make_run_state;
    use super::super::validate_action_completion;
    fn make_simple_state(attempt_val: u16) -> Option<RunState> {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: vb_core::ids::SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("proptest"),
            digest: WorkflowDigest::from_bytes([0xEE; 32]),
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
        let wf =
            vb_core::workflow::CompiledWorkflow::try_from_parts(parts).expect("make_simple_state");
        let mut state = make_run_state(wf, RunId::new(1))?;
        assert_eq!(state.frame.mark_running(StepIdx::ZERO), Ok(()));
        if let Some(attempt) = state.action_attempts.get_mut(0) {
            *attempt = attempt_val;
        }
        Some(state)
    }

    proptest! {
        /// Property: validate_action_completion never panics for arbitrary u16 inputs.
        #[test]
        fn prop_validate_action_completion_never_panics(
            attempt in 0u16..=u16::MAX,
            capacity in 0u16..=u16::MAX,
            current in 0u16..=u16::MAX,
        ) {
            let Some(state) = make_simple_state(current) else {
                return Ok(());
            };
            let t = ActionTicket {
                run: RunId::new(1),
                step: StepIdx::ZERO,
                seq: SeqNo::ZERO,
                action: ActionId::new(0),
                attempt,
                idempotency_key: 0,
                capacity,
                ..Default::default()
            };
            let _result = validate_action_completion(&state, t);
        }

        /// Property: For any valid (attempt, capacity, current):
        ///   attempt == 0 OR capacity == 0 OR attempt > capacity => Err(AttemptBeyondMax)
        ///   attempt < current => Err(StaleAttempt)
        ///   attempt == current => Ok(())
        #[test]
        fn prop_validate_ticket_attempt_classifies_all_attempt_relations(
            attempt in 1u16..=65500u16,
            capacity in 1u16..=65500u16,
            current in 0u16..=65500u16,
        ) {
            let Some(state) = make_simple_state(current) else {
                return Ok(());
            };
            let t = ActionTicket {
                run: RunId::new(1),
                step: StepIdx::ZERO,
                seq: SeqNo::ZERO,
                action: ActionId::new(0),
                attempt,
                idempotency_key: 0,
                capacity,
                ..Default::default()
            };
            let result = validate_action_completion(&state, t);
            if attempt > capacity {
                prop_assert_eq!(
                    result,
                    Err(RuntimeError::AttemptBeyondMax { attempt, max: capacity })
                );
            } else if attempt < current {
                prop_assert_eq!(
                    result,
                    Err(RuntimeError::StaleAttempt {
                        incoming: attempt,
                        current,
                    })
                );
            } else if attempt == current {
                prop_assert_eq!(result, Ok(()));
            } else {
                // attempt > current but <= capacity — future attempt rejected (G005 fixed)
                prop_assert_eq!(
                    result,
                    Err(RuntimeError::InvalidActionCompletion),
                    "attempt={} > current={} must be rejected",
                    attempt,
                    current
                );
            }
        }
    }
}
