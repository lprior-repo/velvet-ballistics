use super::{RUN, TestResult, fixture, rows, runtime};
use std::sync::Arc;
use vb_core::{ActionFailureCode, ActionOutputReady, SlotValue, Taint};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_storage::{FjallJournal, JournalEvent};

#[path = "selected_damage.rs"]
mod damage;
#[path = "owned_attempt_history.rs"]
mod owned_attempt_history;
#[path = "routed_attempt_epoch.rs"]
mod routed_attempt_epoch;

#[test]
fn selected_scalar_attempt_two_recovers_exact_ticket() -> TestResult {
    selected(false, false, 2)
}

#[test]
fn selected_scalar_attempt_three_with_handler_recovers_exact_ticket() -> TestResult {
    selected(true, false, 3)
}

#[test]
fn selected_owned_attempt_two_with_handler_recovers_exact_ticket() -> TestResult {
    selected(true, true, 2)
}

#[test]
fn selected_owned_attempt_three_recovers_exact_ticket() -> TestResult {
    selected(false, true, 3)
}

fn selected(handler: bool, owned: bool, attempt: u16) -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path();
    let (journal, mut live, first) =
        fixture::pending(path, handler, Some((3, "0ms")), owned, false)?;
    let mut ticket = first;
    for _ in 1..attempt {
        live.fail_action(ticket, ActionFailureCode::Timeout.into())?;
        ticket = fixture::action(&mut live)?;
    }
    assert_eq!(
        (ticket.run_attempt, ticket.attempt, ticket.capacity),
        (1, attempt, 3)
    );
    assert_eq!(ticket.idempotency_key, first.idempotency_key);
    assert_eq!(ticket.seq, first.seq);
    assert!(ticket.fencing_token > first.fencing_token);
    let state = live.inspect_run_state(RUN)?.clone();
    let prefix = journal.events_for_run_full(RUN)?;
    assert!(
        matches!(prefix.last(), Some(JournalEvent::ActionScheduledTicket { ticket: actual, .. }) if *actual == ticket)
    );
    drop(live);
    drop(journal);
    let before = rows(path)?;
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let mut cold = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    cold.recover_and_resume(RUN)?;
    cold.tick_all()?;
    assert_eq!(cold.lookup_pending_action_ticket(RUN, 0), Some(ticket));
    let actual = cold.inspect_run_state(RUN)?;
    use vb_runtime::shard::types::RegisteredExecutionMode;
    assert_eq!(state.registered_execution, RegisteredExecutionMode::Live);
    assert_eq!(
        actual.registered_execution,
        RegisteredExecutionMode::ReconcileOnly
    );
    let charged = prefix
        .iter()
        .try_fold(0_u64, |total, event| -> TestResult<u64> {
            total
                .checked_add(u64::try_from(postcard::to_allocvec(event)?.len())?)
                .ok_or_else(|| "journal charge overflow".into())
        })?;
    assert_eq!(actual.journal_bytes_written, charged);
    assert_eq!(actual.run_started_millis, state.run_started_millis);
    let mut expected = state.clone();
    expected.registered_execution = RegisteredExecutionMode::ReconcileOnly;
    expected.journal_bytes_written = charged;
    assert_eq!(actual, &expected);
    assert_eq!(journal.events_for_run_full(RUN)?, prefix);
    drop(cold);
    drop(journal);
    assert_eq!(rows(path)?, before);
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let mut cold = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    cold.recover_and_resume(RUN)?;
    cold.tick_all()?;
    let value = SlotValue::I64(73);
    let output = ActionOutputReady {
        output_slot: state
            .workflow
            .node(ticket.step)
            .and_then(|node| node.output.slot())
            .ok_or("Do output absent")?,
        value,
        taint: Taint::Clean,
        encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
    };
    let unchanged = cold.inspect_run_state(RUN)?.clone();
    assert_eq!(
        cold.complete_action_with_output(first, output),
        Err(vb_runtime::RuntimeError::StaleAttempt {
            incoming: first.attempt,
            current: ticket.attempt
        })
    );
    assert_eq!(cold.inspect_run_state(RUN)?, &unchanged);
    assert_eq!(journal.events_for_run_full(RUN)?, prefix);
    cold.complete_action_with_output(ticket, output)?;
    fixture::ask(&mut cold)?;
    assert_eq!(cold.lookup_pending_action_ticket(RUN, 0), None);
    assert_eq!(journal.events_for_run_full(RUN)?.iter().filter(|event| matches!(event, JournalEvent::ActionCompletedEnvelope { ticket: actual, .. } if *actual == ticket)).count(), 1);
    let progressed = cold.inspect_run_state(RUN)?.clone();
    let progressed_prefix = journal.events_for_run_full(RUN)?;
    drop(cold);
    drop(journal);
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let mut cold = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    cold.recover_and_resume(RUN)?;
    fixture::ask(&mut cold)?;
    let actual = cold.inspect_run_state(RUN)?;
    assert_eq!(actual.registered_execution, RegisteredExecutionMode::Live);
    assert_eq!(
        progressed.registered_execution,
        RegisteredExecutionMode::ReconcileOnly
    );
    let progressed_charge =
        progressed_prefix
            .iter()
            .try_fold(0_u64, |total, event| -> TestResult<u64> {
                total
                    .checked_add(u64::try_from(postcard::to_allocvec(event)?.len())?)
                    .ok_or_else(|| "journal charge overflow".into())
            })?;
    assert_eq!(actual.journal_bytes_written, progressed_charge);
    assert_eq!(actual.run_started_millis, progressed.run_started_millis);
    let mut expected = progressed;
    expected.registered_execution = RegisteredExecutionMode::Live;
    expected.journal_bytes_written = progressed_charge;
    assert_eq!(actual, &expected);
    assert_eq!(journal.events_for_run_full(RUN)?, progressed_prefix);
    let ask = cold
        .lookup_pending_ask_ticket(RUN, vb_core::StepIdx::new(1))
        .ok_or("issued Ask missing")?;
    cold.answer_ask(vb_runtime::shard::AskAnswer::new(
        ask,
        ask.answer_slot(),
        SlotValue::Bool(true),
        Taint::Clean,
    ))?;
    for _ in 0..12 {
        cold.tick_all()?;
    }
    let finished = journal.events_for_run_full(RUN)?;
    assert_eq!(cold.counters_snapshot().runs_completed, 1);
    assert_eq!(
        finished
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunFinished { attempt: 1, .. }))
            .count(),
        1
    );
    assert_eq!(
        finished
            .iter()
            .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        usize::from(attempt)
    );
    assert_eq!(
        cold.recover_product(RUN)?.cannot_resume_reason(),
        Some("terminal_state")
    );
    Ok(())
}
