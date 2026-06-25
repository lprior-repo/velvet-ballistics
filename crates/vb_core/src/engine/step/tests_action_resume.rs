use super::test_support::{
    action_ticket, do_then_finish_workflow, do_with_error_handler_workflow, ensure_equal,
    test_frame,
};
use super::{resume_action_completion, resume_action_failure, step_once};
use crate::EngineSignal;
use crate::action::{ActionFailureCode, ActionJournalEvent, RetryPolicy};
use crate::errors::EngineError;
use crate::frame::StepState;
use crate::ids::{SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStore;

#[test]
fn completion_rejects_wrong_output_slot() -> Result<(), String> {
    let workflow = do_then_finish_workflow("wrong_output_slot", [0xB1; 32], 2)?;
    let mut run = suspended_run(&workflow)?;
    let ticket = action_ticket(1, 0, 1, 1);

    let result = resume_action_completion(
        &workflow,
        &mut run,
        ticket,
        SlotIdx::new(1),
        SlotValue::I64(7),
        Taint::Clean,
    );

    expect_resume_reason(result, "action_resume_output_mismatch")?;
    ensure_equal(run.pc(), StepIdx::new(0))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))
}

#[test]
fn failure_routes_to_error_handler_and_records_journal() -> Result<(), String> {
    let workflow = do_with_error_handler_workflow("failure_route", [0xB2; 32])?;
    let mut run = suspended_run(&workflow)?;
    let ticket = action_ticket(1, 0, 9, 1);

    let (signal, journal) = resume_action_failure(
        &workflow,
        &mut run,
        ticket,
        ActionFailureCode::PermissionDenied,
        RetryPolicy::NonRetryable,
    )
    .map_err(|e| e.to_string())?;

    ensure_equal(signal, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(2))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(1)).map_err(|e| e.to_string())?,
        SlotValue::I64(0),
    )?;
    expect_failure_journal(journal, ticket)
}

fn suspended_run(workflow: &crate::workflow::CompiledWorkflow) -> Result<crate::RunFrame, String> {
    let mut run = test_frame(workflow)?;
    let mut store = ValueStore::new();
    let signal = step_once(workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(signal, EngineSignal::AwaitingAction)?;
    Ok(run)
}

fn expect_resume_reason<T>(
    result: Result<T, EngineError>,
    expected: &'static str,
) -> Result<(), String> {
    match result {
        Err(EngineError::ActionResumeRejected { rejection }) => {
            ensure_equal(rejection.reason(), expected)
        }
        Err(error) => Err(format!("expected resume rejection, got {error:?}")),
        Ok(_) => Err(String::from("expected resume error, got success")),
    }
}

fn expect_failure_journal(
    journal: ActionJournalEvent,
    expected_ticket: crate::action::ActionTicket,
) -> Result<(), String> {
    match journal {
        ActionJournalEvent::Failed {
            ticket,
            attempt,
            code,
            retry_policy,
        } => {
            ensure_equal(ticket, expected_ticket)?;
            ensure_equal(attempt, 1)?;
            ensure_equal(code, ActionFailureCode::PermissionDenied)?;
            ensure_equal(retry_policy, RetryPolicy::NonRetryable)
        }
        other => Err(format!("unexpected journal event: {other:?}")),
    }
}
