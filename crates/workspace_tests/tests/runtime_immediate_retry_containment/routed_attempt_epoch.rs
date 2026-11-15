use super::{RUN, TestResult, fixture, rows, runtime};
use std::sync::Arc;
use vb_core::{ActionFailureCode, SlotValue, StepIdx, Taint};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_storage::{FjallJournal, JournalEvent};

#[test]
fn scalar_second_action_permanent_failure_recovers_actual_handler() -> TestResult {
    routed(false, 2)
}

#[test]
fn owned_second_action_permanent_failure_recovers_actual_handler() -> TestResult {
    routed(true, 2)
}

#[test]
fn scalar_third_action_exhaustion_recovers_actual_handler() -> TestResult {
    routed(false, 3)
}

#[test]
fn owned_third_action_exhaustion_recovers_actual_handler() -> TestResult {
    routed(true, 3)
}

#[test]
fn owned_exhausted_failure_without_outcome_refuses_without_mutation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path();
    let (journal, ticket) = super::super::historical::prefix(path, false, false, true, 3)?;
    assert_eq!((ticket.run_attempt, ticket.attempt), (1, 3));
    let workflow =
        vb_compile::compile_workflow(fixture::source(false, Some((3, "0ms"))).as_bytes())?;
    let prefix = journal.events_for_run_full(RUN)?;
    let artifact = prefix
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunAdmissionBudget {
                artifact_digest, ..
            }
            | JournalEvent::RunAdmission {
                artifact_digest, ..
            } => Some(*artifact_digest),
            _ => None,
        })
        .ok_or("actual admission absent")?;
    let receipt = fixture::receipt(&journal, &workflow, artifact)?;
    assert!(matches!(
        prefix.last(),
        Some(JournalEvent::ActionFailedEvent { attempt: 3, .. })
    ));
    assert!(
        !prefix
            .iter()
            .any(|event| matches!(event, JournalEvent::RunFailedEvent { .. }))
    );
    drop(journal);
    let before = rows(path)?;
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let mut cold = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    let counters = cold.counters_snapshot();
    let pending = cold.pending_boundary_snapshot(8);
    let usage = cold.usage_snapshot();
    let trace = cold.list_events(RUN);
    let error = cold
        .recover_product(RUN)
        .err()
        .ok_or("exhausted failure-only prefix became resumable")?;
    assert!(matches!(
        &error,
        vb_runtime::RuntimeError::Recovery { .. }
            | vb_runtime::RuntimeError::RecoveryCannotResume { .. }
    ));
    assert_eq!(cold.recover_and_resume(RUN), Err(error));
    cold.tick_all()?;
    assert_eq!(
        cold.inspect_run_state(RUN),
        Err(vb_runtime::RuntimeError::RunNotFound)
    );
    assert_eq!(cold.counters_snapshot(), counters);
    assert_eq!(cold.pending_boundary_snapshot(8), pending);
    assert_eq!(cold.usage_snapshot(), usage);
    assert_eq!(cold.list_events(RUN), trace);
    assert_eq!(journal.events_for_run_full(RUN)?, prefix);
    assert_eq!(fixture::receipt(&journal, &workflow, artifact)?, receipt);
    drop(cold);
    drop(journal);
    assert_eq!(rows(path)?, before);
    Ok(())
}

fn routed(owned: bool, attempt: u16) -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path();
    let (journal, mut live, first) = fixture::pending(path, true, Some((3, "0ms")), owned, false)?;
    let mut ticket = first;
    for _ in 1..attempt {
        live.fail_action(ticket, ActionFailureCode::Timeout.into())?;
        ticket = fixture::action(&mut live)?;
    }
    assert_eq!(
        (ticket.run_attempt, ticket.attempt, ticket.capacity),
        (1, attempt, 3)
    );
    let failure = if attempt == 3 {
        ActionFailureCode::Timeout
    } else {
        ActionFailureCode::InvalidInput
    };
    live.fail_action(ticket, failure.into())?;
    fixture::ask(&mut live)?;
    assert_eq!(live.lookup_pending_action_ticket(RUN, 0), None);
    let ask = live
        .lookup_pending_ask_ticket(RUN, StepIdx::new(1))
        .ok_or("actual handler Ask absent")?;
    let state = live.inspect_run_state(RUN)?.clone();
    assert_eq!(state.run_attempt.get(), 1);
    assert_eq!(state.action_attempts.first(), Some(&attempt));
    let prefix = journal.events_for_run_full(RUN)?;
    let last_failure = prefix
        .iter()
        .rposition(|event| matches!(event, JournalEvent::ActionFailedEvent { .. }))
        .ok_or("actual failure absent")?;
    assert!(
        matches!(prefix.get(last_failure), Some(JournalEvent::ActionFailedEvent { attempt: actual, step, action, .. }) if *actual == attempt && *step == ticket.step && *action == ticket.action)
    );
    assert!(
        matches!(prefix.get(last_failure.checked_add(1).ok_or("index overflow")?), Some(JournalEvent::StepStarted { step, attempt: 1, .. }) if *step == ask.ask_step())
    );
    let artifact = state
        .admission
        .as_ref()
        .ok_or("actual admission absent")?
        .artifact_digest();
    let receipt = fixture::receipt(&journal, &state.workflow, artifact)?;
    drop(live);
    drop(journal);
    let before = rows(path)?;
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let mut cold = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    cold.recover_and_resume(RUN)?;
    fixture::ask(&mut cold)?;
    assert_eq!(
        cold.lookup_pending_ask_ticket(RUN, StepIdx::new(1)),
        Some(ask)
    );
    assert_eq!(cold.lookup_pending_action_ticket(RUN, 0), None);
    let actual = cold.inspect_run_state(RUN)?;
    assert_eq!(actual.run_started_millis, state.run_started_millis);
    assert_eq!(actual.run_attempt.get(), 1);
    assert_eq!(
        actual.registered_execution,
        vb_runtime::shard::types::RegisteredExecutionMode::Live
    );
    let charged = prefix
        .iter()
        .try_fold(0_u64, |total, event| -> TestResult<u64> {
            total
                .checked_add(u64::try_from(postcard::to_allocvec(event)?.len())?)
                .ok_or_else(|| "journal charge overflow".into())
        })?;
    assert_eq!(actual.journal_bytes_written, charged);
    let mut expected = state;
    expected.journal_bytes_written = charged;
    assert_eq!(actual, &expected);
    assert_eq!(journal.events_for_run_full(RUN)?, prefix);
    assert_eq!(
        fixture::receipt(&journal, &actual.workflow, artifact)?,
        receipt
    );
    drop(cold);
    drop(journal);
    assert_eq!(rows(path)?, before);
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let mut cold = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    cold.recover_and_resume(RUN)?;
    fixture::ask(&mut cold)?;
    cold.answer_ask(vb_runtime::shard::AskAnswer::new(
        ask,
        ask.answer_slot(),
        SlotValue::Bool(true),
        Taint::Clean,
    ))?;
    for _ in 0..16 {
        cold.tick_all()?;
    }
    assert_eq!(cold.counters_snapshot().runs_completed, 1);
    let finished = journal.events_for_run_full(RUN)?;
    assert_eq!(
        finished
            .iter()
            .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        usize::from(attempt)
    );
    assert_eq!(
        finished
            .iter()
            .filter(|event| matches!(event, JournalEvent::ActionCompletedEnvelope { .. }))
            .count(),
        0
    );
    assert_eq!(
        finished
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunFinished { attempt: 1, .. }))
            .count(),
        1
    );
    assert_eq!(
        cold.recover_product(RUN)?.cannot_resume_reason(),
        Some("terminal_state")
    );
    Ok(())
}
