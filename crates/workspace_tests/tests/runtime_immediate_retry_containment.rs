#![forbid(unsafe_code)]

use std::{num::NonZeroUsize, path::Path, sync::Arc};
use vb_core::{RunId, RuntimePolicy};
use vb_runtime::{
    RuntimeError, journal::SharedRuntimeJournal, runtime::Runtime, shard::ShardConfig,
};
use vb_storage::{FjallJournal, JournalEvent};

#[path = "runtime_immediate_retry_containment/attempt_identity.rs"]
mod attempt_identity;
#[path = "runtime_immediate_retry_containment/cut.rs"]
mod cut;
#[path = "runtime_immediate_retry_containment/fixture.rs"]
mod fixture;
#[path = "runtime_immediate_retry_containment/preservation.rs"]
mod preservation;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const RUN: RunId = RunId::new(92421);

fn runtime(journal: SharedRuntimeJournal) -> TestResult<Runtime> {
    Ok(Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            step_budget_per_tick: 16,
            ..ShardConfig::default()
        },
        journal,
    )?)
}

fn rows(path: &Path) -> TestResult<Vec<(String, Vec<u8>, Vec<u8>)>> {
    let database = fjall::Database::builder(path).open()?;
    let mut rows = Vec::new();
    for name in FjallJournal::declared_keyspaces() {
        let keyspace = database.keyspace(name, fjall::KeyspaceCreateOptions::default)?;
        for entry in keyspace.iter() {
            let (key, value) = entry.into_inner()?;
            rows.push((name.to_owned(), key.to_vec(), value.to_vec()));
        }
    }
    Ok(rows)
}

fn refusal() -> RuntimeError {
    RuntimeError::RecoveryCannotResume {
        reason: String::from("immediate_retry_decision_missing"),
    }
}

#[test]
fn strict_scalar_retry_failure_without_handler_is_refused_cold() -> TestResult {
    ambiguous(false, false, false)
}

#[test]
fn strict_scalar_retry_failure_with_handler_is_refused_cold() -> TestResult {
    ambiguous(true, false, false)
}

#[test]
fn strict_scalar_resumed_retry_without_handler_is_refused_cold() -> TestResult {
    ambiguous(false, true, false)
}

#[test]
fn strict_scalar_resumed_retry_with_handler_is_refused_cold() -> TestResult {
    ambiguous(true, true, false)
}

#[test]
fn strict_owned_retry_failure_without_handler_is_refused_cold() -> TestResult {
    ambiguous(false, false, true)
}

#[test]
fn strict_owned_retry_failure_with_handler_is_refused_cold() -> TestResult {
    ambiguous(true, false, true)
}

#[test]
fn strict_owned_resumed_retry_without_handler_is_refused_cold() -> TestResult {
    ambiguous(false, true, true)
}

#[test]
fn strict_owned_resumed_retry_with_handler_is_refused_cold() -> TestResult {
    ambiguous(true, true, true)
}

fn ambiguous(handler: bool, resumed: bool, owned: bool) -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("journal");
    let (journal, live, issued) = fixture::crashed(&path, handler, resumed, owned)?;
    let prefix = journal.events_for_run_full(RUN)?;
    assert_eq!(
        prefix
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunResumed { .. }))
            .count(),
        usize::from(resumed)
    );
    assert_eq!(issued.attempt, 1);
    assert_eq!(issued.capacity, 3);
    assert!(
        matches!(prefix.iter().rev().find(|event| !matches!(event, JournalEvent::RunResumed { .. })), Some(JournalEvent::ActionFailedEvent { step, action, attempt: 1, .. }) if *step == issued.step && *action == issued.action)
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
        .ok_or("actual admission absent")?;
    let receipt = fixture::receipt(&journal, &workflow, artifact)?;
    assert_eq!(receipt.0, owned);
    drop(live);
    drop(journal);
    let before = rows(&path)?;
    let journal = Arc::new(FjallJournal::open(&path, None)?);
    let mut recovered = runtime(vb_runtime::journal::StorageRuntimeJournal::shared_strict(
        journal.clone(),
    ))?;
    let counters = recovered.counters_snapshot();
    let trace = recovered.list_events(RUN);
    let pending = recovered.pending_boundary_snapshot(8);
    let quota = recovered.usage_snapshot();
    assert_eq!(recovered.recover_product(RUN).err(), Some(refusal()));
    assert_eq!(recovered.pending_boundary_snapshot(8), pending);
    assert_eq!(recovered.recover_and_resume(RUN), Err(refusal()));
    assert_eq!(recovered.pending_boundary_snapshot(8), pending);
    recovered.tick_all()?;
    assert!(matches!(
        recovered.inspect_run_state(RUN),
        Err(RuntimeError::RunNotFound)
    ));
    assert_eq!(
        recovered.lookup_pending_action_ticket(RUN, u64::from(issued.step.get())),
        None
    );
    assert_eq!(
        recovered.lookup_pending_ask_ticket(RUN, vb_core::StepIdx::new(1)),
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
    assert_eq!(rows(&path)?, before);
    Ok(())
}

#[test]
fn resolved_scalar_failure_with_wrong_compiled_capacity_is_refused_without_mutation() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let (journal, live, issued) = fixture::crashed(directory.path(), true, false, false)?;
    let mut schedule = journal
        .events_for_run_full(RUN)?
        .into_iter()
        .find(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
        .ok_or("issued schedule absent")?;
    if let JournalEvent::ActionScheduledTicket { ticket, .. } = &mut schedule {
        assert_eq!(ticket.capacity, 3);
        ticket.capacity = 2;
    }
    let workflow =
        vb_compile::compile_workflow(fixture::source(true, Some((3, "0ms"))).as_bytes())?;
    let artifact = live
        .inspect_run_state(RUN)?
        .admission
        .as_ref()
        .ok_or("actual admission absent")?
        .artifact_digest();
    let receipt = fixture::receipt(&journal, &workflow, artifact)?;
    drop(live);
    drop(journal);
    {
        let database = fjall::Database::builder(directory.path()).open()?;
        let keyspace = database.keyspace("run_event", fjall::KeyspaceCreateOptions::default)?;
        let encoded = vb_storage::codec::encode_record(
            vb_storage::constants::MAGIC_JOURNAL_EVENT,
            schedule.record_kind(),
            schedule.seq().get(),
            &schedule,
            u32::MAX,
        )?;
        keyspace.insert(
            vb_storage::keys::run_event_key(RUN, schedule.seq())?,
            encoded,
        )?;
        database.persist(fjall::PersistMode::SyncAll)?;
    }
    let before = rows(directory.path())?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let prefix = journal.events_for_run_full(RUN)?;
    assert!(prefix.iter().any(|event| matches!(event, JournalEvent::ActionScheduledTicket { ticket, .. } if ticket.capacity == 2)));
    let mut recovered = runtime(vb_runtime::journal::StorageRuntimeJournal::shared_strict(
        journal.clone(),
    ))?;
    let pending = recovered.pending_boundary_snapshot(8);
    let counters = recovered.counters_snapshot();
    let quota = recovered.usage_snapshot();
    let expected = RuntimeError::RecoveryCannotResume {
        reason: String::from("pending_action_capacity_mismatch"),
    };
    assert_eq!(recovered.recover_product(RUN).err(), Some(expected.clone()));
    assert_eq!(recovered.recover_and_resume(RUN), Err(expected));
    assert_eq!(recovered.pending_boundary_snapshot(8), pending);
    recovered.tick_all()?;
    assert_eq!(recovered.pending_boundary_snapshot(8), pending);
    assert_eq!(recovered.counters_snapshot(), counters);
    assert_eq!(recovered.usage_snapshot(), quota);
    assert_eq!(
        recovered.inspect_run_state(RUN),
        Err(RuntimeError::RunNotFound)
    );
    assert_eq!(
        recovered.lookup_pending_action_ticket(RUN, u64::from(issued.step.get())),
        None
    );
    assert_eq!(journal.events_for_run_full(RUN)?, prefix);
    assert_eq!(fixture::receipt(&journal, &workflow, artifact)?, receipt);
    drop(recovered);
    drop(journal);
    assert_eq!(rows(directory.path())?, before);
    Ok(())
}
