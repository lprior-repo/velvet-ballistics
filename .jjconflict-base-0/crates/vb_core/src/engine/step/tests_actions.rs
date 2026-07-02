use super::test_support::{
    action_contract, action_ticket, do_then_finish_workflow, ensure_equal, failure_payload,
    ready_output, single_do_workflow, test_frame,
};
use super::{journal_action_suspended, resume_action_completion, resume_action_failure, step_once};
use crate::EngineSignal;
use crate::action::{ActionFailureCode, ActionJournalEvent, RetryPolicy};
use crate::errors::EngineError;
use crate::frame::StepState;
use crate::ids::{ActionId, RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStore;

#[test]
fn resume_action_completion_writes_output_and_advances_pc() -> Result<(), String> {
    let workflow = do_then_finish_workflow("resume_ok", [0xA1; 32], 1)?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    let suspend = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    ensure_equal(suspend, EngineSignal::AwaitingAction)?;

    let ticket = action_ticket(1, 0, 1, 1);
    let contract = action_contract(1, 8);
    let encoded_payload = b"out";
    let output = ready_output(
        SlotIdx::new(0),
        SlotValue::I64(99),
        Taint::Clean,
        encoded_payload,
    )?;
    let executed_before = run.executed();
    let (signal, journal) = resume_action_completion(
        &workflow,
        &mut run,
        ticket,
        output,
        encoded_payload,
        &contract,
    )
    .map_err(|e| e.to_string())?;

    ensure_equal(signal, EngineSignal::Continue)?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(0)).map_err(|e| e.to_string())?,
        SlotValue::I64(99),
    )?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    ensure_equal(run.executed(), executed_before.saturating_add(1))?;
    expect_completed_journal(journal, ticket, output)
}

#[test]
fn resume_action_failure_marks_step_failed() -> Result<(), String> {
    let workflow = single_do_workflow("resume_fail", [0xA2; 32], 1)?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    let _suspend = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;
    let ticket = action_ticket(1, 0, 1, 1);
    let contract = action_contract(1, 8);
    let encoded_payload = b"err";
    let failure = failure_payload(
        ActionFailureCode::Timeout,
        RetryPolicy::NonRetryable,
        Taint::Clean,
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

    ensure_equal(signal, EngineSignal::ActionFailureUnhandled)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Failed))?;
    expect_failed_journal(journal, ticket, SlotIdx::new(0), failure)
}

#[test]
fn journal_action_suspended_captures_all_fields() -> Result<(), String> {
    let ticket = action_ticket(1, 0, 5, 5);
    let event = journal_action_suspended(
        ticket,
        ActionId::new(5),
        SlotIdx::new(0),
        SlotIdx::new(1),
        StepIdx::new(0),
    )
    .map_err(|e| e.to_string())?;

    match event {
        ActionJournalEvent::Suspended {
            ticket: t,
            attempt,
            action,
            input_slot,
            output_slot,
            step,
        } => {
            ensure_equal(t, ticket)?;
            ensure_equal(t.run, RunId::new(1))?;
            ensure_equal(attempt, 1)?;
            ensure_equal(action, ActionId::new(5))?;
            ensure_equal(input_slot, SlotIdx::new(0))?;
            ensure_equal(output_slot, SlotIdx::new(1))?;
            ensure_equal(step, StepIdx::new(0))
        }
        other => Err(format!("unexpected event: {other:?}")),
    }
}

#[test]
fn journal_action_suspended_rejects_action_mismatch() -> Result<(), String> {
    let ticket = action_ticket(1, 0, 5, 5);
    let result = journal_action_suspended(
        ticket,
        ActionId::new(6),
        SlotIdx::new(0),
        SlotIdx::new(1),
        StepIdx::new(0),
    );

    expect_journal_rejection(result, "action_resume_action_mismatch")
}

#[test]
fn journal_action_suspended_rejects_step_mismatch() -> Result<(), String> {
    let ticket = action_ticket(1, 0, 5, 5);
    let result = journal_action_suspended(
        ticket,
        ActionId::new(5),
        SlotIdx::new(0),
        SlotIdx::new(1),
        StepIdx::new(1),
    );

    expect_journal_rejection(result, "action_resume_step_not_current_pc")
}

#[test]
fn journal_action_suspended_rejects_malformed_ticket_shape() -> Result<(), String> {
    let mut ticket = action_ticket(1, 0, 5, 5);
    ticket.capacity = 0;
    let result = journal_action_suspended(
        ticket,
        ActionId::new(5),
        SlotIdx::new(0),
        SlotIdx::new(1),
        StepIdx::new(0),
    );

    expect_journal_rejection(result, "action_resume_capacity_zero")
}

fn expect_completed_journal(
    journal: ActionJournalEvent,
    expected_ticket: crate::action::ActionTicket,
    expected_output: crate::action::ActionOutputReady,
) -> Result<(), String> {
    match journal {
        ActionJournalEvent::Completed {
            ticket,
            attempt,
            output,
        } => {
            ensure_equal(ticket, expected_ticket)?;
            ensure_equal(attempt, expected_ticket.attempt)?;
            ensure_equal(output, expected_output)
        }
        other => Err(format!("unexpected journal event: {other:?}")),
    }
}

fn expect_journal_rejection(
    result: Result<ActionJournalEvent, EngineError>,
    expected: &'static str,
) -> Result<(), String> {
    match result {
        Err(EngineError::ActionResumeRejected { report }) => {
            ensure_equal(report.rejection.reason(), expected)
        }
        Err(error) => Err(format!("expected resume rejection, got {error:?}")),
        Ok(event) => Err(format!("expected resume rejection, got {event:?}")),
    }
}

fn expect_failed_journal(
    journal: ActionJournalEvent,
    expected_ticket: crate::action::ActionTicket,
    expected_output_slot: SlotIdx,
    expected_failure: crate::action::ActionFailure,
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
            ensure_equal(output_slot, expected_output_slot)?;
            ensure_equal(failure, expected_failure)
        }
        other => Err(format!("unexpected journal event: {other:?}")),
    }
}
