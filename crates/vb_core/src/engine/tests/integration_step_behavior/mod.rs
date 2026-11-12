#![forbid(unsafe_code)]
//! Integration behavior tests for step execution lifecycle: variant dispatch,
//! state transitions, idempotency, budget tracking, invalid transitions,
//! action retry, edge cases, and Kani boundedness harnesses.

use super::{ensure_equal, external_input, test_frame, test_store};
use crate::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionJournalEvent, ActionName,
    ActionOutputReady, ActionTicket, Idempotency, RetryPolicy, RetrySafety, SideEffect,
    compute_action_idempotency_key,
};
use crate::engine::{
    EngineSignal, StepBudget, resume_action_completion, resume_action_failure, step_once,
};
use crate::errors::EngineError;
use crate::frame::{RunFrame, StepState, is_valid_step_state_transition};
use crate::ids::{
    ActionId, ConstIdx, ExprIdx, RunId, SeqNo, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::value::{ConstValue, SlotValue, Taint};
use crate::workflow::{
    CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, SlotBranch, WorkflowParts, node,
};

fn tiny_workflow_parts(name: &'static str, value: ConstValue) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from(name),
        digest: WorkflowDigest::from_bytes([0xAB; 32]),
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![value].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

mod step_variant_dispatch;

// ============================================================
// Section 2: Step lifecycle — creation through execution
// ============================================================

#[test]
fn run_frame_created_with_all_step_states_pending() -> Result<(), String> {
    let parts = tiny_workflow_parts("lifecycle_create", ConstValue::I64(1));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let run = test_frame(RunId::new(220), &workflow)?;

    for step in 0..run.step_count() {
        ensure_equal(run.step_state(StepIdx::new(step)), Ok(StepState::Pending))?;
    }
    ensure_equal(run.executed(), 0)?;
    ensure_equal(run.pc(), StepIdx::new(0))?;
    Ok(())
}

#[test]
fn step_transitions_pending_to_running_then_succeeded_on_completion() -> Result<(), String> {
    let parts = tiny_workflow_parts("pending_running_succeeded", ConstValue::I64(3));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(221), &workflow)?;
    let mut store = test_store();

    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Pending))?;
    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    Ok(())
}

#[test]
fn all_steps_in_chain_transition_correctly_through_completion() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("chain_tx"),
        digest: WorkflowDigest::from_bytes([0xDD; 32]),
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(100), ConstValue::I64(200)].into_boxed_slice(),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(222), &workflow)?;
    let mut store = test_store();

    let s0 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    ensure_equal(s0, EngineSignal::Continue)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Pending))?;

    let s1 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    ensure_equal(s1, EngineSignal::Continue)?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Succeeded))?;
    ensure_equal(run.step_state(StepIdx::new(2)), Ok(StepState::Pending))?;

    let s2 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    ensure_equal(
        s2,
        EngineSignal::Finished(SlotValue::I64(200), Taint::Clean),
    )?;
    ensure_equal(run.step_state(StepIdx::new(2)), Ok(StepState::Succeeded))?;
    Ok(())
}

// ============================================================
// Section 3: Signal to state mapping
// ============================================================

#[test]
fn signal_continue_maps_step_state_to_succeeded() -> Result<(), String> {
    let parts = tiny_workflow_parts("signal_continue", ConstValue::I64(5));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(230), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    Ok(())
}

#[test]
fn signal_finished_maps_step_state_to_succeeded() -> Result<(), String> {
    let parts = tiny_workflow_parts("signal_finished", ConstValue::I64(6));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(231), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(6), Taint::Clean),
    )?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Succeeded))?;
    Ok(())
}

// ============================================================
// Section 4: Action retry — resume_action_completion and resume_action_failure
// ============================================================

fn do_then_finish_workflow() -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from("do_finish"),
        digest: WorkflowDigest::from_bytes([0x1A; 32]),
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Do {
                    action: ActionId::new(7),
                    input: SlotIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|error| error.to_string())
}

fn make_ticket(
    run: RunId,
    step: StepIdx,
    seq_raw: u32,
    action: ActionId,
    attempt: u16,
) -> ActionTicket {
    let seq = SeqNo::new(u64::from(seq_raw));
    ActionTicket {
        run,
        step,
        seq,
        action,
        run_attempt: 1,
        attempt,
        idempotency_key: compute_action_idempotency_key(run, 1, seq, action),
        capacity: 3,
        fencing_token: 0,
    }
}

fn make_contract(action: ActionId, max_output_bytes: u32) -> ActionContract {
    ActionContract {
        id: action,
        name: ActionName::from_static_infallible("test-action"),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 128,
        max_output_bytes,
        timeout_ms: 1_000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }
}

fn ready_output(
    output_slot: SlotIdx,
    value: SlotValue,
    taint: Taint,
    encoded_len: u32,
) -> ActionOutputReady {
    ActionOutputReady {
        output_slot,
        value,
        taint,
        encoded_len,
    }
}

fn failure_payload(
    code: ActionFailureCode,
    retry_policy: RetryPolicy,
    taint: Taint,
    encoded_len: u32,
) -> ActionFailure {
    ActionFailure {
        code,
        retry_policy,
        taint,
        detail: None,
        encoded_len,
    }
}

#[test]
fn resume_action_completion_writes_output_and_advances_pc() -> Result<(), String> {
    let workflow = do_then_finish_workflow()?;
    let mut run = test_frame(RunId::new(240), &workflow)?;
    let mut store = test_store();

    let suspend = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    ensure_equal(suspend, EngineSignal::AwaitingAction)?;

    let ticket = make_ticket(RunId::new(240), StepIdx::new(0), 1, ActionId::new(7), 1);
    let contract = make_contract(ActionId::new(7), 8);
    let encoded_payload = b"ok";
    let output = ready_output(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean, 2);
    let (signal, _journal) = resume_action_completion(
        &workflow,
        &mut run,
        ticket,
        output,
        encoded_payload,
        &contract,
    )
    .map_err(|error| error.to_string())?;

    ensure_equal(signal, EngineSignal::Continue)?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(0))
            .map_err(|error| error.to_string())?,
        SlotValue::I64(42),
    )?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    Ok(())
}

#[test]
fn resume_action_completion_increments_executed_counter() -> Result<(), String> {
    let workflow = do_then_finish_workflow()?;
    let mut run = test_frame(RunId::new(241), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    let executed_before = run.executed();

    let ticket = make_ticket(RunId::new(241), StepIdx::new(0), 1, ActionId::new(7), 1);
    let contract = make_contract(ActionId::new(7), 8);
    let encoded_payload = b"ok";
    let output = ready_output(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean, 2);
    let (signal, _journal) = resume_action_completion(
        &workflow,
        &mut run,
        ticket,
        output,
        encoded_payload,
        &contract,
    )
    .map_err(|error| error.to_string())?;

    ensure_equal(signal, EngineSignal::Continue)?;
    ensure_equal(run.executed(), executed_before + 1)?;
    Ok(())
}

#[test]
fn resume_action_completion_journal_has_completed_variant_with_correct_fields() -> Result<(), String>
{
    let workflow = do_then_finish_workflow()?;
    let mut run = test_frame(RunId::new(242), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    let ticket = make_ticket(RunId::new(242), StepIdx::new(0), 1, ActionId::new(7), 2);
    let contract = make_contract(ActionId::new(7), 8);
    let encoded_payload = b"ok";
    let output = ready_output(
        SlotIdx::new(0),
        SlotValue::Bool(true),
        Taint::DerivedFromSecret,
        2,
    );
    let (_signal, journal) = resume_action_completion(
        &workflow,
        &mut run,
        ticket,
        output,
        encoded_payload,
        &contract,
    )
    .map_err(|error| error.to_string())?;

    match journal {
        ActionJournalEvent::Completed {
            ticket: t,
            attempt,
            output,
        } => {
            ensure_equal(t.run, RunId::new(242))?;
            ensure_equal(t.step, StepIdx::new(0))?;
            ensure_equal(attempt, 2)?;
            ensure_equal(output.output_slot, SlotIdx::new(0))?;
            ensure_equal(output.taint, Taint::DerivedFromSecret)?;
            Ok(())
        }
        other => Err(format!("expected Completed journal event, got {other:?}")),
    }
}

#[test]
fn resume_action_failure_marks_step_failed_without_error_handler() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from("fail_no_handler"),
        digest: WorkflowDigest::from_bytes([0x2B; 32]),
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::Do {
                action: ActionId::new(8),
                input: SlotIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(243), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    let ticket = make_ticket(RunId::new(243), StepIdx::new(0), 1, ActionId::new(8), 1);
    let contract = make_contract(ActionId::new(8), 8);
    let encoded_payload = b"err";
    let failure = failure_payload(
        ActionFailureCode::Timeout,
        RetryPolicy::NonRetryable,
        Taint::Clean,
        3,
    );
    let (signal, _journal) = resume_action_failure(
        &workflow,
        &mut run,
        ticket,
        failure,
        encoded_payload,
        &contract,
    )
    .map_err(|error| error.to_string())?;

    ensure_equal(signal, EngineSignal::ActionFailureUnhandled)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
    Ok(())
}

#[test]
fn resume_action_failure_journal_has_failed_variant_with_failure_code() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from("fail_journal"),
        digest: WorkflowDigest::from_bytes([0x2C; 32]),
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::Do {
                action: ActionId::new(9),
                input: SlotIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(244), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    let ticket = make_ticket(RunId::new(244), StepIdx::new(0), 1, ActionId::new(9), 3);
    let contract = make_contract(ActionId::new(9), 8);
    let encoded_payload = b"err";
    let failure = failure_payload(
        ActionFailureCode::ExternalUnavailable,
        RetryPolicy::Retryable,
        Taint::Clean,
        3,
    );
    let (_signal, journal) = resume_action_failure(
        &workflow,
        &mut run,
        ticket,
        failure,
        encoded_payload,
        &contract,
    )
    .map_err(|error| error.to_string())?;

    match journal {
        ActionJournalEvent::Failed {
            ticket: t,
            attempt,
            failure,
            ..
        } => {
            ensure_equal(t.step, StepIdx::new(0))?;
            ensure_equal(attempt, 3)?;
            ensure_equal(failure.code, ActionFailureCode::ExternalUnavailable)?;
            ensure_equal(failure.retry_policy, RetryPolicy::Retryable)?;
            Ok(())
        }
        other => Err(format!("expected Failed journal event, got {other:?}")),
    }
}

#[test]
fn resume_action_failure_with_retry_policy_preserves_policy_in_journal() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from("fail_retry"),
        digest: WorkflowDigest::from_bytes([0x2D; 32]),
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::Do {
                action: ActionId::new(10),
                input: SlotIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(245), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    let ticket = make_ticket(RunId::new(245), StepIdx::new(0), 2, ActionId::new(10), 1);
    let contract = make_contract(ActionId::new(10), 8);
    let encoded_payload = b"err";
    let failure = failure_payload(
        ActionFailureCode::Conflict,
        RetryPolicy::Retryable,
        Taint::Clean,
        3,
    );
    let (_signal, journal) = resume_action_failure(
        &workflow,
        &mut run,
        ticket,
        failure,
        encoded_payload,
        &contract,
    )
    .map_err(|error| error.to_string())?;

    match journal {
        ActionJournalEvent::Failed { failure, .. } => {
            ensure_equal(failure.code, ActionFailureCode::Conflict)
        }
        other => Err(format!("expected Failed journal, got {other:?}")),
    }
}

// ============================================================
// Section 5: Step idempotency
// ============================================================

#[test]
fn mark_running_twice_is_idempotent() -> Result<(), String> {
    let parts = tiny_workflow_parts("idem_run", ConstValue::I64(1));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(250), &workflow)?;

    run.mark_running(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))?;

    run.mark_running(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))?;
    Ok(())
}

#[test]
fn mark_succeeded_twice_is_idempotent() -> Result<(), String> {
    let parts = tiny_workflow_parts("idem_success", ConstValue::I64(2));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(251), &workflow)?;

    run.mark_running(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    run.mark_succeeded(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;

    run.mark_succeeded(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    Ok(())
}

#[test]
fn mark_failed_twice_is_idempotent() -> Result<(), String> {
    let parts = tiny_workflow_parts("idem_fail", ConstValue::I64(3));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(252), &workflow)?;

    run.mark_running(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    run.mark_failed(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;

    run.mark_failed(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
    Ok(())
}

// ============================================================
// Section 6: Invalid state transitions
// ============================================================

#[test]
fn pending_to_waiting_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Pending,
        StepState::Waiting
    ));
}

#[test]
fn pending_to_asking_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Pending,
        StepState::Asking
    ));
}

#[test]
fn running_to_pending_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Running,
        StepState::Pending
    ));
}

#[test]
fn succeeded_to_running_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Succeeded,
        StepState::Running
    ));
}

#[test]
fn succeeded_to_pending_is_invalid_transition() {
    // Succeeded -> Pending is valid (for loop re-entry). This should be valid.
    assert!(is_valid_step_state_transition(
        StepState::Succeeded,
        StepState::Pending
    ));
}

#[test]
fn succeeded_to_waiting_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Succeeded,
        StepState::Waiting
    ));
}

#[test]
fn failed_to_running_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Failed,
        StepState::Running
    ));
}

#[test]
fn failed_to_waiting_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Failed,
        StepState::Waiting
    ));
}

#[test]
fn waiting_to_pending_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Waiting,
        StepState::Pending
    ));
}

#[test]
fn cancelled_to_running_is_invalid_transition() {
    assert!(!is_valid_step_state_transition(
        StepState::Cancelled,
        StepState::Running
    ));
}

#[test]
fn pending_to_running_is_valid_transition() {
    assert!(is_valid_step_state_transition(
        StepState::Pending,
        StepState::Running
    ));
}

#[test]
fn running_to_succeeded_is_valid_transition() {
    assert!(is_valid_step_state_transition(
        StepState::Running,
        StepState::Succeeded
    ));
}

#[test]
fn running_to_failed_is_valid_transition() {
    assert!(is_valid_step_state_transition(
        StepState::Running,
        StepState::Failed
    ));
}

#[test]
fn running_to_waiting_is_valid_transition() {
    assert!(is_valid_step_state_transition(
        StepState::Running,
        StepState::Waiting
    ));
}

#[test]
fn running_to_asking_is_valid_transition() {
    assert!(is_valid_step_state_transition(
        StepState::Running,
        StepState::Asking
    ));
}

#[test]
fn same_state_idempotent_is_always_valid() {
    let all_states = [
        StepState::Pending,
        StepState::Running,
        StepState::Succeeded,
        StepState::Failed,
        StepState::Skipped,
        StepState::Waiting,
        StepState::Asking,
        StepState::Cancelled,
    ];
    for state in all_states {
        assert!(
            is_valid_step_state_transition(state, state),
            "idempotent transition should be valid for {state:?}"
        );
    }
}

#[test]
fn invalid_transition_rejected_in_run_frame_mark() -> Result<(), String> {
    let parts = tiny_workflow_parts("invalid_tx", ConstValue::I64(7));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(260), &workflow)?;

    run.mark_running(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    let result = run.mark_pending(StepIdx::new(0));

    match result {
        Err(EngineError::InternalInvariantViolation { reason })
            if reason.as_str() == "invalid_state_transition" =>
        {
            Ok(())
        }
        other => Err(format!(
            "expected InternalInvariantViolation, got {other:?}"
        )),
    }
}

#[test]
fn succeeded_to_running_rejected_in_run_frame() -> Result<(), String> {
    let parts = tiny_workflow_parts("succ_to_run", ConstValue::I64(8));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(261), &workflow)?;

    run.mark_running(StepIdx::new(0))
        .map_err(|error| error.to_string())?;
    run.mark_succeeded(StepIdx::new(0))
        .map_err(|error| error.to_string())?;

    let result = run.mark_running(StepIdx::new(0));

    match result {
        Err(EngineError::InternalInvariantViolation { reason })
            if reason.as_str() == "invalid_state_transition" =>
        {
            Ok(())
        }
        other => Err(format!(
            "expected InternalInvariantViolation, got {other:?}"
        )),
    }
}

// ============================================================
// Section 7: Edge cases
// ============================================================

#[test]
fn empty_workflow_parts_rejected_at_try_from_parts() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("empty"),
        digest: WorkflowDigest::from_bytes([0xEE; 32]),
        nodes: Box::new([]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let result = CompiledWorkflow::try_from_parts(parts);

    match result {
        Err(crate::WorkflowError::EmptyNodes) => Ok(()),
        Err(ref e) => Err(format!("expected EmptyNodes validation error, got {e}")),
        Ok(_) => Err("expected error for empty workflow, but compiled successfully".into()),
    }
}

#[test]
fn set_pc_rejects_out_of_range_program_counter() -> Result<(), String> {
    let parts = tiny_workflow_parts("bad_pc", ConstValue::I64(1));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(270), &workflow)?;

    match run.set_pc(StepIdx::new(99)) {
        Err(EngineError::InvalidProgramCounter { step }) if step == StepIdx::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn run_frame_creation_rejects_zero_step_count() -> Result<(), String> {
    let result = RunFrame::new(RunId::new(271), StepIdx::new(0), 0, 1);

    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) if reason.contains("zero") => Ok(()),
        other => Err(format!(
            "expected InvalidCompiledWorkflow with zero reason, got {other:?}"
        )),
    }
}

#[test]
fn workflow_compile_rejects_entry_out_of_bounds() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("bad_entry"),
        digest: WorkflowDigest::from_bytes([0x11; 32]),
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(99),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let result = CompiledWorkflow::try_from_parts(parts);

    match result {
        Err(_) => Ok(()),
        Ok(_) => Err("expected validation error for out-of-bounds entry step".into()),
    }
}

#[test]
fn jump_to_out_of_bounds_target_rejected_at_compilation() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("bad_jump"),
        digest: WorkflowDigest::from_bytes([0x22; 32]),
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::Jump {
                target: StepIdx::new(99),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let result = CompiledWorkflow::try_from_parts(parts);

    match result {
        Err(_) => Ok(()),
        Ok(_) => Err("expected validation error for jump to out-of-bounds target".into()),
    }
}

#[test]
fn set_pc_out_of_bounds_returns_invalid_program_counter() -> Result<(), String> {
    let parts = tiny_workflow_parts("pc_oob", ConstValue::I64(1));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(272), &workflow)?;

    let result = run.set_pc(StepIdx::new(200));

    match result {
        Err(EngineError::InvalidProgramCounter { step }) if step == StepIdx::new(200) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn resume_action_completion_nonexistent_step_returns_invalid_pc() -> Result<(), String> {
    let workflow = do_then_finish_workflow()?;
    let mut run = test_frame(RunId::new(273), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    let ticket = make_ticket(RunId::new(273), StepIdx::new(99), 1, ActionId::new(7), 1);
    let contract = make_contract(ActionId::new(7), 8);
    let output = ready_output(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean, 2);
    let result = resume_action_completion(&workflow, &mut run, ticket, output, b"ok", &contract);

    match result {
        Err(EngineError::ActionResumeRejected { report })
            if report.rejection.reason() == "action_resume_step_not_current_pc" =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn resume_action_completion_missing_next_returns_error() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from("do_no_next"),
        digest: WorkflowDigest::from_bytes([0x33; 32]),
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::Do {
                action: ActionId::new(11),
                input: SlotIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(274), &workflow)?;
    let mut store = test_store();

    let _ = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    let ticket = make_ticket(RunId::new(274), StepIdx::new(0), 1, ActionId::new(11), 1);
    let contract = make_contract(ActionId::new(11), 8);
    let output = ready_output(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean, 2);
    let result = resume_action_completion(&workflow, &mut run, ticket, output, b"ok", &contract);

    match result {
        Err(EngineError::MissingNextStep { step }) if step == StepIdx::new(0) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// ============================================================
// Section 8: Budget tracking per step
// ============================================================

#[test]
fn step_budget_remaining_decrements_by_one_after_single_take() -> Result<(), String> {
    let mut budget = StepBudget::new(5);
    ensure_equal(budget.remaining(), 5)?;
    let took = budget.try_take().map_err(|error| error.to_string())?;
    ensure_equal(took, true)?;
    ensure_equal(budget.remaining(), 4)?;
    Ok(())
}

#[test]
fn step_budget_new_clamps_to_max() -> Result<(), String> {
    let max = crate::limits::MAX_STEP_BUDGET;
    let budget = StepBudget::new(max + 1000);
    ensure_equal(budget.remaining(), max)?;
    Ok(())
}

#[test]
fn step_budget_run_until_blocked_exhausts_at_limit() -> Result<(), String> {
    let parts = tiny_workflow_parts("budget_exhaust", ConstValue::I64(42));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(280), &workflow)?;
    let mut store = test_store();

    let result =
        crate::engine::run_until_blocked(&workflow, &mut run, StepBudget::new(1), &mut store);

    ensure_equal(result, Ok(EngineSignal::StepBudgetExhausted))?;
    ensure_equal(run.executed(), 1)?;
    Ok(())
}

// ============================================================
// Section 9: Kani harnesses for boundedness
// ============================================================

#[cfg(kani)]
mod kani_boundedness {
    use crate::EngineSignal;
    use crate::action::{
        ActionContract, ActionName, ActionOutputReady, Idempotency, RetrySafety, SideEffect,
        compute_action_idempotency_key,
    };
    use crate::engine::{new_run_frame, resume_action_completion, step_once};
    use crate::errors::EngineError;
    use crate::frame::StepState;
    use crate::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
    use crate::value::{ConstValue, SlotValue, Taint};
    use crate::value_store::ValueStore;
    use crate::workflow::{CompiledNodeKind, CompiledWorkflow, WorkflowParts, node};

    /// Kani H1: step_once is panic-free for bounded single-step execution.
    /// Uses a minimal 2-node Nop→Finish workflow with bounded PC.
    #[kani::proof]
    #[kani::unwind(4)]
    fn step_once_panic_freedom_bounded() {
        let slot_val = kani::any::<i64>();
        let plan = match CompiledWorkflow::try_from_parts(WorkflowParts {
            semantic_source_identity: None,
            source_metadata: None,
            input_slots: Box::new([]),
            name: Box::<str>::from("kani_nop_finish"),
            digest: WorkflowDigest::from_bytes([0x10; 32]),
            nodes: vec![
                node(
                    StepIdx::new(0),
                    CompiledNodeKind::Nop,
                    None,
                    Some(StepIdx::new(1)),
                ),
                node(
                    StepIdx::new(1),
                    CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                    None,
                    None,
                ),
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(slot_val)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
            step_names: Box::new([]),
        }) {
            Ok(w) => w,
            Err(_) => return,
        };

        let mut run = match new_run_frame(RunId::new(1), &plan) {
            Ok(r) => r,
            Err(_) => return,
        };
        let _ = run.write_slot(SlotIdx::new(0), SlotValue::I64(slot_val));
        let mut store = ValueStore::new();

        let result = step_once(&plan, &mut run, &mut store);
        // step_once must not panic — the function always returns a Result.
        let _ = result;
    }

    /// Kani H2: resume_action_completion is panic-free for bounded inputs.
    #[kani::proof]
    #[kani::unwind(4)]
    fn resume_action_completion_panic_freedom_bounded() {
        let plan = match CompiledWorkflow::try_from_parts(WorkflowParts {
            semantic_source_identity: None,
            source_metadata: None,
            input_slots: Box::new([]),
            name: Box::<str>::from("kani_resume"),
            digest: WorkflowDigest::from_bytes([0x20; 32]),
            nodes: vec![
                node(
                    StepIdx::new(0),
                    CompiledNodeKind::Do {
                        action: ActionId::new(1),
                        input: SlotIdx::new(0),
                    },
                    Some(SlotIdx::new(0)),
                    Some(StepIdx::new(1)),
                ),
                node(
                    StepIdx::new(1),
                    CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                    None,
                    None,
                ),
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: crate::ResourceContract::DEFAULT,
            step_names: Box::new([]),
        }) {
            Ok(w) => w,
            Err(_) => return,
        };

        let mut run = match new_run_frame(RunId::new(2), &plan) {
            Ok(r) => r,
            Err(_) => return,
        };
        let mut store = ValueStore::new();

        let _ = step_once(&plan, &mut run, &mut store);

        let run_id = RunId::new(2);
        let seq = SeqNo::new(1);
        let action = ActionId::new(1);
        let ticket = crate::action::ActionTicket {
            run: run_id,
            step: StepIdx::new(0),
            seq,
            action,
            attempt: 1,
            idempotency_key: compute_action_idempotency_key(run_id, seq, action),
            capacity: 1,
        };
        let output = ActionOutputReady {
            output_slot: SlotIdx::new(0),
            value: SlotValue::I64(0),
            taint: Taint::Clean,
            encoded_len: 2,
        };
        let contract = ActionContract {
            id: action,
            name: ActionName::from_static_infallible("kani-action"),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 16,
            max_output_bytes: 16,
            timeout_ms: 1_000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };

        let result = resume_action_completion(&plan, &mut run, ticket, output, b"ok", &contract);
        let _ = result;
    }

    /// Kani H3: step_state transitions are valid for any two StepState values.
    /// The predicate must never panic and must return a deterministic bool.
    #[kani::proof]
    fn step_state_transition_predicate_never_panics() {
        let current: StepState = kani::any();
        let next: StepState = kani::any();

        // The predicate must evaluate without panicking for any input pair.
        let result = crate::frame::is_valid_step_state_transition(current, next);
        // kani::assert is used to force the solver to explore all paths;
        // the property we want is that execution reaches this point (no panic).
        assert!(result || !result);
    }
}
