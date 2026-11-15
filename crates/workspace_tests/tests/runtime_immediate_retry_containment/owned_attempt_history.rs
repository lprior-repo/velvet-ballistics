use super::{RUN, TestResult, fixture, rows, runtime};
use std::{path::Path, sync::Arc};
use vb_core::{
    ActionFailureCode, ActionOutputReady, ActionTicket, CapabilitySet, SlotValue, Taint,
};
use vb_runtime::{RuntimeError, journal::StorageRuntimeJournal, runtime::Runtime};
use vb_storage::{FjallJournal, JournalEvent};

const SOURCE: &str = "version: velvet-ballistics/v1\nname: owned_attempt_history\nwhen: {manual: {}}\nsteps:\n  - id: seed\n    set: {output: base, value: '42'}\n  - id: loop\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: work\n          try_again: {max_attempts: 3, delay: '0ms'}\n          do: {action: '0', input: '$step.seed'}\n  - id: done\n    finish: {result: '$step.loop'}\n";

fn pending(path: &Path) -> TestResult<(Arc<FjallJournal>, Runtime, ActionTicket)> {
    let workflow = vb_compile::compile_workflow(SOURCE.as_bytes())?;
    assert!(vb_storage::recovery::is_owned_for_each_contract(&workflow));
    let step = workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| {
            matches!(node.kind, vb_core::CompiledNodeKind::Do { .. }).then_some(node.id)
        })
        .ok_or("Do absent")?;
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let contract = fixture::contract()?;
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Strict,
        &[contract.clone()],
    )?;
    let mut live = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    live.submit_direct_with_inputs_grants_and_contracts(
        RUN,
        workflow,
        Box::new([]),
        CapabilitySet::empty(),
        Box::from([contract]),
    )?;
    let ticket = at_action(&mut live, step)?;
    assert_eq!((ticket.attempt, ticket.capacity), (1, 3));
    Ok((journal, live, ticket))
}

fn at_action(live: &mut Runtime, step: vb_core::StepIdx) -> TestResult<ActionTicket> {
    for _ in 0..32 {
        live.tick_all()?;
        if let Some(ticket) = live.lookup_pending_action_ticket(RUN, u64::from(step.get())) {
            return Ok(ticket);
        }
    }
    Err("issued action absent".into())
}

#[test]
fn first_attempt_two_owned_control_refuses_without_mutation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path();
    let (journal, live, first) = pending(path)?;
    let receipt = journal.run_inputs(RUN)?;
    let mut schedule = journal
        .events_for_run_full(RUN)?
        .last()
        .ok_or("schedule absent")?
        .clone();
    assert!(
        matches!(&schedule, JournalEvent::ActionScheduledTicket { ticket, .. } if *ticket == first)
    );
    if let JournalEvent::ActionScheduledTicket { ticket, .. } = &mut schedule {
        ticket.attempt = 2;
        assert!(vb_core::action::action_ticket_has_valid_key(*ticket));
    }
    drop(live);
    drop(journal);
    {
        let database = fjall::Database::builder(path).open()?;
        let events = database.keyspace("run_event", fjall::KeyspaceCreateOptions::default)?;
        let bytes = vb_storage::codec::encode_record(
            vb_storage::constants::MAGIC_JOURNAL_EVENT,
            schedule.record_kind(),
            schedule.seq().get(),
            &schedule,
            u32::MAX,
        )?;
        events.insert(vb_storage::keys::run_event_key(RUN, schedule.seq())?, bytes)?;
        database.persist(fjall::PersistMode::SyncAll)?;
    }
    let before = rows(path)?;
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let prefix = journal.events_for_run_full(RUN)?;
    assert!(
        !prefix
            .iter()
            .any(|event| matches!(event, JournalEvent::ActionFailedEvent { .. }))
    );
    let mut cold = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    let counters = cold.counters_snapshot();
    let pending = cold.pending_boundary_snapshot(8);
    let usage = cold.usage_snapshot();
    let trace = cold.list_events(RUN);
    let error = cold
        .recover_product(RUN)
        .err()
        .ok_or("accepted unissued first action attempt2")?;
    assert!(matches!(
        &error,
        RuntimeError::Recovery { .. } | RuntimeError::RecoveryCannotResume { .. }
    ));
    assert_eq!(cold.recover_and_resume(RUN), Err(error));
    cold.tick_all()?;
    assert_eq!(cold.inspect_run_state(RUN), Err(RuntimeError::RunNotFound));
    assert_eq!(cold.counters_snapshot(), counters);
    assert_eq!(cold.pending_boundary_snapshot(8), pending);
    assert_eq!(cold.usage_snapshot(), usage);
    assert_eq!(cold.list_events(RUN), trace);
    assert_eq!(journal.events_for_run_full(RUN)?, prefix);
    assert_eq!(journal.run_inputs(RUN)?, receipt);
    drop(cold);
    drop(journal);
    assert_eq!(rows(path)?, before);
    Ok(())
}

#[test]
fn later_owned_activation_keeps_attempt_two_after_real_retry() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path();
    let (journal, mut live, first) = pending(path)?;
    live.fail_action(first, ActionFailureCode::Timeout.into())?;
    let retried = at_action(&mut live, first.step)?;
    assert_eq!(retried.attempt, 2);
    let output_slot = live
        .inspect_run_state(RUN)?
        .workflow
        .node(first.step)
        .and_then(|node| node.output.slot())
        .ok_or("output absent")?;
    let value = SlotValue::I64(73);
    live.complete_action_with_output(
        retried,
        ActionOutputReady {
            output_slot,
            value,
            taint: Taint::Clean,
            encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
        },
    )?;
    let later = at_action(&mut live, first.step)?;
    assert_eq!(
        (later.run_attempt, later.attempt, later.capacity),
        (1, 2, 3)
    );
    assert!(later.seq > retried.seq);
    assert_ne!(later.idempotency_key, retried.idempotency_key);
    let state = live.inspect_run_state(RUN)?.clone();
    let prefix = journal.events_for_run_full(RUN)?;
    drop(live);
    drop(journal);
    let before = rows(path)?;
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let mut cold = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    cold.recover_and_resume(RUN)?;
    cold.tick_all()?;
    assert_eq!(
        cold.lookup_pending_action_ticket(RUN, u64::from(later.step.get())),
        Some(later)
    );
    let actual = cold.inspect_run_state(RUN)?;
    use vb_runtime::shard::types::RegisteredExecutionMode;
    assert_eq!(state.registered_execution, RegisteredExecutionMode::Live);
    assert_eq!(actual.run_started_millis, state.run_started_millis);
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
    let mut expected = state;
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
    let value = SlotValue::I64(74);
    cold.complete_action_with_output(
        later,
        ActionOutputReady {
            output_slot,
            value,
            taint: Taint::Clean,
            encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
        },
    )?;
    for _ in 0..32 {
        cold.tick_all()?;
    }
    assert_eq!(cold.counters_snapshot().runs_completed, 1);
    let completed = journal.events_for_run_full(RUN)?;
    assert_eq!(
        completed
            .iter()
            .filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
            .count(),
        3
    );
    assert_eq!(
        completed
            .iter()
            .filter(|event| matches!(event, JournalEvent::ActionCompletedEnvelope { .. }))
            .count(),
        2
    );
    assert_eq!(
        completed
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunFinished { .. }))
            .count(),
        1
    );
    Ok(())
}
