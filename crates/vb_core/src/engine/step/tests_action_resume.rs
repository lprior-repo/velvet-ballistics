use super::test_support::{
    action_contract, action_ticket, do_then_finish_workflow, do_with_error_handler_workflow,
    ensure_equal, failure_payload, ready_output, test_frame,
};
use super::{resume_action_completion, resume_action_failure, step_once};
use crate::EngineSignal;
use crate::action::{ActionFailure, ActionFailureCode, ActionJournalEvent, RetryPolicy};
use crate::errors::EngineError;
use crate::frame::StepState;
use crate::ids::{ActionId, BlobId, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

#[test]
fn completion_rejects_wrong_output_slot() -> Result<(), String> {
    let workflow = do_then_finish_workflow("wrong_output_slot", [0xB1; 32], 2)?;
    let mut run = suspended_run(&workflow)?;
    let ticket = action_ticket(1, 0, 1, 1);
    let contract = action_contract(1, 8);
    let encoded_payload = b"ok";
    let output = ready_output(
        SlotIdx::new(1),
        SlotValue::I64(7),
        Taint::Clean,
        encoded_payload,
    )?;

    let result = resume_action_completion(
        &workflow,
        &mut run,
        ticket,
        output,
        encoded_payload,
        &contract,
    );

    expect_resume_reason(result, "action_resume_output_mismatch")?;
    ensure_equal(run.pc(), StepIdx::new(0))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))
}

#[test]
fn completion_accepts_absolute_output_slot_when_contract_declares_one_output() -> Result<(), String>
{
    let workflow = do_then_finish_output_slot_one()?;
    let mut run = suspended_run(&workflow)?;
    let ticket = action_ticket(1, 0, 1, 1);
    let contract = action_contract(1, 8);
    let output = ready_output(SlotIdx::new(1), SlotValue::I64(70), Taint::Clean, b"ok")?;

    let (signal, _journal) =
        resume_action_completion(&workflow, &mut run, ticket, output, b"ok", &contract)
            .map_err(|e| e.to_string())?;

    ensure_equal(signal, EngineSignal::Continue)?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(1)).map_err(|e| e.to_string())?,
        SlotValue::I64(70),
    )
}

#[test]
fn failure_routes_to_error_handler_and_records_journal() -> Result<(), String> {
    let workflow = do_with_error_handler_workflow("failure_route", [0xB2; 32])?;
    let mut run = suspended_run(&workflow)?;
    let ticket = action_ticket(1, 0, 9, 1);
    let contract = action_contract(9, 8);
    let encoded_payload = b"err";
    let failure = failure_payload(
        ActionFailureCode::PermissionDenied,
        RetryPolicy::NonRetryable,
        Taint::DerivedFromSecret,
        encoded_payload,
    )?;

    let (signal, journal) = resume_action_failure(
        &workflow,
        &mut run,
        ticket,
        failure.clone(),
        encoded_payload,
        &contract,
    )
    .map_err(|e| e.to_string())?;

    ensure_equal(signal, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(2))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(1)).map_err(|e| e.to_string())?,
        SlotValue::I64(0),
    )?;
    expect_failure_journal(journal, ticket, failure)
}

#[test]
fn completion_rejects_forged_encoded_len_without_mutation() -> Result<(), String> {
    let workflow = do_then_finish_workflow("forged_completion_len", [0xB3; 32], 1)?;
    let mut run = suspended_run(&workflow)?;
    let snapshot = frame_snapshot(&run);
    let ticket = action_ticket(1, 0, 1, 1);
    let contract = action_contract(1, 8);
    let output = crate::action::ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::I64(7),
        taint: Taint::Clean,
        encoded_len: 99,
    };

    let result = resume_action_completion(&workflow, &mut run, ticket, output, b"ok", &contract);

    expect_resume_reason(result, "action_resume_encoded_payload_len_mismatch")?;
    ensure_frame_unchanged(&run, snapshot)
}

#[test]
fn failure_rejects_forged_encoded_len_without_mutation() -> Result<(), String> {
    let workflow = do_then_finish_workflow("forged_failure_len", [0xB4; 32], 1)?;
    let mut run = suspended_run(&workflow)?;
    let snapshot = frame_snapshot(&run);
    let ticket = action_ticket(1, 0, 1, 1);
    let contract = action_contract(1, 8);
    let failure = ActionFailure {
        code: ActionFailureCode::Timeout,
        retry_policy: RetryPolicy::Retryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 99,
    };

    let result = resume_action_failure(&workflow, &mut run, ticket, failure, b"err", &contract);

    expect_resume_reason(result, "action_resume_encoded_payload_len_mismatch")?;
    ensure_frame_unchanged(&run, snapshot)
}

#[test]
fn completion_rejects_contract_mismatch_without_mutation() -> Result<(), String> {
    let workflow = do_then_finish_workflow("completion_contract_mismatch", [0xB5; 32], 1)?;
    let mut run = suspended_run(&workflow)?;
    let snapshot = frame_snapshot(&run);
    let ticket = action_ticket(1, 0, 1, 1);
    let contract = action_contract(2, 8);
    let output = ready_output(SlotIdx::new(0), SlotValue::I64(7), Taint::Clean, b"ok")?;

    let result = resume_action_completion(&workflow, &mut run, ticket, output, b"ok", &contract);

    expect_resume_reason(result, "action_resume_contract_mismatch")?;
    ensure_frame_unchanged(&run, snapshot)
}

#[test]
fn failure_rejects_contract_mismatch_without_mutation() -> Result<(), String> {
    let workflow = do_then_finish_workflow("failure_contract_mismatch", [0xB6; 32], 1)?;
    let mut run = suspended_run(&workflow)?;
    let snapshot = frame_snapshot(&run);
    let ticket = action_ticket(1, 0, 1, 1);
    let contract = action_contract(2, 8);
    let failure = failure_payload(
        ActionFailureCode::Timeout,
        RetryPolicy::Retryable,
        Taint::Clean,
        b"err",
    )?;

    let result = resume_action_failure(&workflow, &mut run, ticket, failure, b"err", &contract);

    expect_resume_reason(result, "action_resume_contract_mismatch")?;
    ensure_frame_unchanged(&run, snapshot)
}

#[test]
fn completion_rejects_payload_over_contract_limit_without_mutation() -> Result<(), String> {
    let workflow = do_then_finish_workflow("completion_payload_too_large", [0xB7; 32], 1)?;
    let mut run = suspended_run(&workflow)?;
    let snapshot = frame_snapshot(&run);
    let ticket = action_ticket(1, 0, 1, 1);
    let contract = action_contract(1, 1);
    let output = ready_output(SlotIdx::new(0), SlotValue::I64(7), Taint::Clean, b"ok")?;

    let result = resume_action_completion(&workflow, &mut run, ticket, output, b"ok", &contract);

    expect_resume_reason(result, "action_resume_encoded_payload_too_large")?;
    ensure_frame_unchanged(&run, snapshot)
}

#[test]
fn failure_rejects_payload_over_contract_limit_without_mutation() -> Result<(), String> {
    let workflow = do_then_finish_workflow("failure_payload_too_large", [0xB8; 32], 1)?;
    let mut run = suspended_run(&workflow)?;
    let snapshot = frame_snapshot(&run);
    let ticket = action_ticket(1, 0, 1, 1);
    let contract = action_contract(1, 1);
    let failure = failure_payload(
        ActionFailureCode::Timeout,
        RetryPolicy::Retryable,
        Taint::Clean,
        b"err",
    )?;

    let result = resume_action_failure(&workflow, &mut run, ticket, failure, b"err", &contract);

    expect_resume_reason(result, "action_resume_encoded_payload_too_large")?;
    ensure_frame_unchanged(&run, snapshot)
}

#[test]
fn completion_rejects_contract_output_undeclared_without_mutation() -> Result<(), String> {
    let workflow = do_then_finish_workflow("completion_contract_output_oob", [0xB9; 32], 1)?;
    let mut run = suspended_run(&workflow)?;
    let snapshot = frame_snapshot(&run);
    let ticket = action_ticket(1, 0, 1, 1);
    let mut contract = action_contract(1, 8);
    contract.output_slot_count = 0;
    let output = ready_output(SlotIdx::new(0), SlotValue::I64(7), Taint::Clean, b"ok")?;

    let result = resume_action_completion(&workflow, &mut run, ticket, output, b"ok", &contract);

    expect_resume_reason(result, "action_resume_contract_output_undeclared")?;
    ensure_frame_unchanged(&run, snapshot)
}

#[test]
fn failure_rejects_contract_output_undeclared_without_mutation() -> Result<(), String> {
    let workflow = do_then_finish_workflow("failure_contract_output_oob", [0xBB; 32], 1)?;
    let mut run = suspended_run(&workflow)?;
    let snapshot = frame_snapshot(&run);
    let ticket = action_ticket(1, 0, 1, 1);
    let mut contract = action_contract(1, 8);
    contract.output_slot_count = 0;
    let failure = failure_payload(
        ActionFailureCode::Timeout,
        RetryPolicy::Retryable,
        Taint::Clean,
        b"err",
    )?;

    let result = resume_action_failure(&workflow, &mut run, ticket, failure, b"err", &contract);

    expect_resume_reason(result, "action_resume_contract_output_undeclared")?;
    ensure_frame_unchanged(&run, snapshot)
}

#[test]
fn failure_journal_preserves_failure_detail() -> Result<(), String> {
    let workflow = do_then_finish_workflow("failure_detail", [0xBA; 32], 1)?;
    let mut run = suspended_run(&workflow)?;
    let ticket = action_ticket(1, 0, 1, 1);
    let contract = action_contract(1, 8);
    let failure = ActionFailure {
        code: ActionFailureCode::PermissionDenied,
        retry_policy: RetryPolicy::NonRetryable,
        taint: Taint::Secret,
        detail: Some(BlobId::new(7)),
        encoded_len: 3,
    };

    let (_signal, journal) = resume_action_failure(
        &workflow,
        &mut run,
        ticket,
        failure.clone(),
        b"err",
        &contract,
    )
    .map_err(|e| e.to_string())?;

    expect_failure_journal(journal, ticket, failure)
}

fn suspended_run(workflow: &crate::workflow::CompiledWorkflow) -> Result<crate::RunFrame, String> {
    let mut run = test_frame(workflow)?;
    let mut store = ValueStore::new();
    let signal = step_once(workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(signal, EngineSignal::AwaitingAction)?;
    Ok(run)
}

fn do_then_finish_output_slot_one() -> Result<CompiledWorkflow, String> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("do_output_slot_one"),
        digest: WorkflowDigest::from_bytes([0xBC; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
    .map_err(|e| e.to_string())
}

fn expect_resume_reason<T>(
    result: Result<T, EngineError>,
    expected: &'static str,
) -> Result<(), String> {
    match result {
        Err(EngineError::ActionResumeRejected { report }) => {
            ensure_equal(report.rejection.reason(), expected)
        }
        Err(error) => Err(format!("expected resume rejection, got {error:?}")),
        Ok(_) => Err(String::from("expected resume error, got success")),
    }
}

fn expect_failure_journal(
    journal: ActionJournalEvent,
    expected_ticket: crate::action::ActionTicket,
    expected_failure: ActionFailure,
) -> Result<(), String> {
    match journal {
        ActionJournalEvent::Failed {
            ticket,
            attempt,
            output_slot,
            failure,
        } => {
            ensure_equal(ticket, expected_ticket)?;
            ensure_equal(attempt, expected_ticket.attempt)?;
            ensure_equal(output_slot, SlotIdx::new(0))?;
            ensure_equal(failure, expected_failure)
        }
        other => Err(format!("unexpected journal event: {other:?}")),
    }
}

fn frame_snapshot(
    run: &crate::RunFrame,
) -> (
    StepIdx,
    u64,
    Vec<crate::frame::StepState>,
    Vec<Option<SlotValue>>,
    Vec<Taint>,
) {
    (
        run.pc(),
        run.executed(),
        run.states_snapshot(),
        run.slots_snapshot(),
        run.taint_snapshot(),
    )
}

fn ensure_frame_unchanged(
    run: &crate::RunFrame,
    snapshot: (
        StepIdx,
        u64,
        Vec<crate::frame::StepState>,
        Vec<Option<SlotValue>>,
        Vec<Taint>,
    ),
) -> Result<(), String> {
    ensure_equal(run.pc(), snapshot.0)?;
    ensure_equal(run.executed(), snapshot.1)?;
    ensure_equal(run.states_snapshot(), snapshot.2)?;
    ensure_equal(run.slots_snapshot(), snapshot.3)?;
    ensure_equal(run.taint_snapshot(), snapshot.4)
}
