use super::test_support::{action_ticket, do_then_finish_workflow, ensure_equal, test_frame};
use super::{resume_action_completion, resume_action_failure, step_once};
use crate::EngineSignal;
use crate::action::{ActionFailureCode, ActionTicket, RetryPolicy};
use crate::errors::EngineError;
use crate::ids::SlotIdx;
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStore;

#[test]
fn completion_and_failure_reject_attempt_zero() -> Result<(), String> {
    let mut ticket = action_ticket(1, 0, 1, 1);
    ticket.attempt = 0;
    expect_completion_and_failure_rejection(ticket, "action_resume_attempt_zero")
}

#[test]
fn completion_and_failure_reject_capacity_zero() -> Result<(), String> {
    let mut ticket = action_ticket(1, 0, 1, 1);
    ticket.capacity = 0;
    expect_completion_and_failure_rejection(ticket, "action_resume_capacity_zero")
}

#[test]
fn completion_and_failure_reject_attempt_over_capacity() -> Result<(), String> {
    let mut ticket = action_ticket(1, 0, 1, 1);
    ticket.attempt = 2;
    ticket.capacity = 1;
    expect_completion_and_failure_rejection(ticket, "action_resume_attempt_exceeds_capacity")
}

#[test]
fn completion_and_failure_reject_invalid_idempotency_key() -> Result<(), String> {
    let mut ticket = action_ticket(1, 0, 1, 1);
    ticket.idempotency_key = ticket.idempotency_key.wrapping_add(1);
    expect_completion_and_failure_rejection(ticket, "action_resume_idempotency_key_mismatch")
}

fn expect_completion_and_failure_rejection(
    ticket: ActionTicket,
    reason: &'static str,
) -> Result<(), String> {
    let workflow = do_then_finish_workflow("malformed_action_ticket", [0xD1; 32], 1)?;
    let mut completion_run = suspended_run(&workflow)?;
    let completion = resume_action_completion(
        &workflow,
        &mut completion_run,
        ticket,
        SlotIdx::new(0),
        SlotValue::I64(1),
        Taint::Clean,
    );
    expect_resume_reason(completion, reason)?;

    let mut failure_run = suspended_run(&workflow)?;
    let failure = resume_action_failure(
        &workflow,
        &mut failure_run,
        ticket,
        ActionFailureCode::Timeout,
        RetryPolicy::NonRetryable,
    );
    expect_resume_reason(failure, reason)
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
