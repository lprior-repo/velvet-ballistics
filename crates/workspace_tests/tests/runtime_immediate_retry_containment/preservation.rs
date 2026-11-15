use super::{RUN, TestResult, fixture, runtime};
use std::sync::Arc;
use vb_core::ActionFailureCode;
use vb_runtime::journal::StorageRuntimeJournal;
use vb_storage::{FjallJournal, JournalEvent};

#[test]
fn warm_zero_delay_retry_keeps_issued_identity_with_and_without_handler() -> TestResult {
    for handler in [false, true] {
        let directory = tempfile::tempdir()?;
        let (_journal, mut live, first) =
            fixture::pending(directory.path(), handler, Some((3, "0ms")), false, false)?;
        live.fail_action(first, ActionFailureCode::Timeout.into())?;
        let next = fixture::action(&mut live)?;
        assert_eq!(next.attempt, 2);
        assert_eq!(next.run_attempt, first.run_attempt);
        assert_eq!(next.idempotency_key, first.idempotency_key);
        assert_eq!(next.capacity, first.capacity);
        assert_eq!(next.seq, first.seq);
        assert_ne!(next.fencing_token, first.fencing_token);
    }
    Ok(())
}

#[test]
fn nonretry_and_exhausted_handler_failure_cuts_retain_existing_recovery() -> TestResult {
    for retry in [None, Some((1, "0ms"))] {
        let directory = tempfile::tempdir()?;
        let (journal, mut live, ticket) =
            fixture::pending(directory.path(), true, retry, false, true)?;
        live.fail_action(ticket, ActionFailureCode::InvalidInput.into())?;
        assert_eq!(
            live.tick_all(),
            Err(vb_runtime::RuntimeError::JournalPoisoned)
        );
        drop(live);
        drop(journal);
        let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
        let mut recovered = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
        recovered.recover_and_resume(RUN)?;
        fixture::ask(&mut recovered)?;
        assert_eq!(
            journal
                .events_for_run_full(RUN)?
                .iter()
                .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
                .count(),
            1
        );
    }
    Ok(())
}

#[test]
fn committed_handler_entry_disambiguates_zero_delay_permanent_failure() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (journal, mut live, ticket) =
        fixture::pending(directory.path(), true, Some((3, "0ms")), false, false)?;
    live.fail_action(ticket, ActionFailureCode::InvalidInput.into())?;
    fixture::ask(&mut live)?;
    let ask = live.lookup_pending_ask_ticket(RUN, vb_core::StepIdx::new(1));
    drop(live);
    drop(journal);
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let mut recovered = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    recovered.recover_and_resume(RUN)?;
    fixture::ask(&mut recovered)?;
    assert_eq!(
        recovered.lookup_pending_ask_ticket(RUN, vb_core::StepIdx::new(1)),
        ask
    );
    assert_eq!(
        journal
            .events_for_run_full(RUN)?
            .iter()
            .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        1
    );
    Ok(())
}

#[test]
fn exhausted_action_attempt_two_keeps_existing_replay_refusal() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (journal, mut live, first) =
        fixture::pending_at_failure(directory.path(), true, Some((2, "0ms")), false, Some(2))?;
    live.fail_action(first, ActionFailureCode::Timeout.into())?;
    let second = fixture::action(&mut live)?;
    assert_eq!(
        (second.run_attempt, second.attempt, second.capacity),
        (1, 2, 2)
    );
    live.fail_action(second, ActionFailureCode::InvalidInput.into())?;
    assert_eq!(
        live.tick_all(),
        Err(vb_runtime::RuntimeError::JournalPoisoned)
    );
    let prefix = journal.events_for_run_full(RUN)?;
    assert!(matches!(
        prefix.last(),
        Some(JournalEvent::ActionFailedEvent { attempt: 2, .. })
    ));
    drop(live);
    drop(journal);
    let before = super::rows(directory.path())?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let recovered = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    assert_eq!(
        recovered.recover_product(RUN).err(),
        Some(vb_runtime::RuntimeError::Recovery {
            error: String::from(
                "non-idempotent action ActionId(0) at step StepIdx(0) cannot be re-executed during recovery"
            ),
        })
    );
    assert_eq!(journal.events_for_run_full(RUN)?, prefix);
    drop(recovered);
    drop(journal);
    assert_eq!(super::rows(directory.path())?, before);
    Ok(())
}

#[test]
fn zero_delay_nonretryable_terminal_outcomes_stay_terminal() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (journal, mut live, ticket) =
        fixture::pending(directory.path(), false, Some((3, "0ms")), false, false)?;
    live.fail_action(ticket, ActionFailureCode::InvalidInput.into())?;
    live.tick_all()?;
    let prefix = journal.events_for_run_full(RUN)?;
    assert!(
        prefix
            .iter()
            .any(|event| matches!(event, JournalEvent::RunFailedEvent { .. }))
    );
    drop(live);
    drop(journal);
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let recovered = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    assert_eq!(
        recovered.recover_product(RUN)?.cannot_resume_reason(),
        Some("terminal_state")
    );
    assert_eq!(journal.events_for_run_full(RUN)?, prefix);
    Ok(())
}
