use super::{RUN, TestResult, fixture, refusal, rows, runtime};
use std::sync::Arc;
use vb_runtime::{RuntimeError, journal::StorageRuntimeJournal};
use vb_storage::{FjallJournal, JournalEvent};

#[test]
fn remaining_second_action_attempt_with_handler_is_refused_cold() -> TestResult {
    remaining_second_attempt(true)
}

#[test]
fn remaining_second_action_attempt_without_handler_is_refused_cold() -> TestResult {
    remaining_second_attempt(false)
}

fn remaining_second_attempt(handler: bool) -> TestResult {
    let directory = tempfile::tempdir()?;
    let (journal, second) = super::historical::prefix(directory.path(), handler, false, false, 2)?;
    assert_eq!(
        (second.run_attempt, second.attempt, second.capacity),
        (1, 2, 3)
    );
    let prefix = journal.events_for_run_full(RUN)?;
    let first = prefix
        .iter()
        .find_map(|event| match event {
            JournalEvent::ActionScheduledTicket { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .ok_or("first issued ticket absent")?;
    assert_eq!(second.idempotency_key, first.idempotency_key);
    assert!(
        matches!(prefix.last(), Some(JournalEvent::ActionFailedEvent { step, action, attempt: 2, .. }) if *step == second.step && *action == second.action)
    );
    let workflow =
        vb_compile::compile_workflow(fixture::source(handler, Some((3, "0ms"))).as_bytes())?;
    let artifact = prefix
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunAdmissionBudget {
                artifact_digest, ..
            } => Some(*artifact_digest),
            _ => None,
        })
        .ok_or("issued admission absent")?;
    let receipt = fixture::receipt(&journal, &workflow, artifact)?;
    drop(journal);
    let before = rows(directory.path())?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let mut recovered = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    let counters = recovered.counters_snapshot();
    let pending = recovered.pending_boundary_snapshot(8);
    let quota = recovered.usage_snapshot();
    let trace = recovered.list_events(RUN);
    assert_eq!(recovered.recover_product(RUN).err(), Some(refusal()));
    assert_eq!(recovered.pending_boundary_snapshot(8), pending);
    assert_eq!(recovered.recover_and_resume(RUN), Err(refusal()));
    assert_eq!(recovered.pending_boundary_snapshot(8), pending);
    recovered.tick_all()?;
    assert_eq!(
        recovered.inspect_run_state(RUN),
        Err(RuntimeError::RunNotFound)
    );
    assert_eq!(
        recovered.lookup_pending_action_ticket(RUN, u64::from(second.step.get())),
        None
    );
    assert_eq!(recovered.counters_snapshot(), counters);
    assert_eq!(recovered.usage_snapshot(), quota);
    assert_eq!(recovered.pending_boundary_snapshot(8), pending);
    assert_eq!(recovered.list_events(RUN), trace);
    assert_eq!(journal.events_for_run_full(RUN)?, prefix);
    assert_eq!(fixture::receipt(&journal, &workflow, artifact)?, receipt);
    drop(recovered);
    drop(journal);
    assert_eq!(rows(directory.path())?, before);
    Ok(())
}
