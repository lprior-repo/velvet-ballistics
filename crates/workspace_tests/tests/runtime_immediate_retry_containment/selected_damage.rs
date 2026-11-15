use super::{RUN, TestResult, fixture, rows, runtime};
use std::{path::Path, sync::Arc};
use vb_core::{ActionFailureCode, ActionTicket, SlotIdx, WorkflowDigest};
use vb_runtime::{RuntimeError, journal::StorageRuntimeJournal};
use vb_storage::{FjallJournal, JournalEvent};

#[derive(Clone, Copy, Debug)]
enum Damage {
    BareStart,
    Attempt,
    RunAttempt,
    ForeignRun,
    Capacity,
    LogicalSeq,
    Key,
    SameFence,
    ZeroFence,
    LowerFence,
    Input,
    Output,
    Abi,
    PredecessorCapacity,
    FailureAttempt,
    WholeRunEpoch,
}

#[test]
fn malformed_selected_retry_evidence_refuses_without_mutation() -> TestResult {
    for damage in [
        Damage::BareStart,
        Damage::Attempt,
        Damage::RunAttempt,
        Damage::ForeignRun,
        Damage::Capacity,
        Damage::LogicalSeq,
        Damage::Key,
        Damage::SameFence,
        Damage::ZeroFence,
        Damage::LowerFence,
        Damage::Input,
        Damage::Output,
        Damage::Abi,
        Damage::PredecessorCapacity,
        Damage::FailureAttempt,
        Damage::WholeRunEpoch,
    ] {
        damaged(damage)?;
    }
    Ok(())
}

fn damaged(damage: Damage) -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path();
    let (journal, mut live, first) = fixture::pending(path, true, Some((3, "0ms")), false, false)?;
    live.fail_action(first, ActionFailureCode::Timeout.into())?;
    let next = fixture::action(&mut live)?;
    assert_eq!((next.attempt, next.run_attempt), (2, 1));
    let state = live.inspect_run_state(RUN)?.clone();
    let artifact = state
        .admission
        .as_ref()
        .ok_or("admission absent")?
        .artifact_digest();
    let receipt = fixture::receipt(&journal, &state.workflow, artifact)?;
    let prefix = journal.events_for_run_full(RUN)?;
    drop(live);
    drop(journal);
    rewrite(path, &prefix, first, damage)?;
    let before = rows(path)?;
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let loaded = journal.events_for_run_full(RUN)?;
    let mut cold = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    let pending = cold.pending_boundary_snapshot(8);
    let counters = cold.counters_snapshot();
    let quota = cold.usage_snapshot();
    let trace = cold.list_events(RUN);
    let error = cold
        .recover_product(RUN)
        .err()
        .ok_or_else(|| format!("accepted {damage:?}"))?;
    if matches!(damage, Damage::WholeRunEpoch) {
        assert!(
            loaded
                .iter()
                .filter_map(|event| match event {
                    JournalEvent::ActionScheduledTicket { ticket, .. } => Some(*ticket),
                    _ => None,
                })
                .all(vb_core::action::action_ticket_has_valid_key)
        );
        assert_eq!(
            error,
            RuntimeError::RecoveryCannotResume {
                reason: "run_attempt_authority_mismatch".into()
            }
        );
    }
    assert!(
        matches!(
            &error,
            RuntimeError::Recovery { .. } | RuntimeError::RecoveryCannotResume { .. }
        ),
        "{damage:?}: {error:?}"
    );
    assert_eq!(cold.recover_and_resume(RUN), Err(error), "{damage:?}");
    cold.tick_all()?;
    assert_eq!(cold.inspect_run_state(RUN), Err(RuntimeError::RunNotFound));
    assert_eq!(cold.pending_boundary_snapshot(8), pending);
    assert_eq!(cold.counters_snapshot(), counters);
    assert_eq!(cold.usage_snapshot(), quota);
    assert_eq!(cold.list_events(RUN), trace);
    assert_eq!(journal.events_for_run_full(RUN)?, loaded);
    assert_eq!(
        fixture::receipt(&journal, &state.workflow, artifact)?,
        receipt
    );
    drop(cold);
    drop(journal);
    assert_eq!(rows(path)?, before, "{damage:?}");
    Ok(())
}

fn rewrite(
    path: &Path,
    prefix: &[JournalEvent],
    first: ActionTicket,
    damage: Damage,
) -> TestResult {
    let database = fjall::Database::builder(path).open()?;
    let events = database.keyspace("run_event", fjall::KeyspaceCreateOptions::default)?;
    let selected = prefix.last().ok_or("selected record absent")?;
    if matches!(damage, Damage::WholeRunEpoch) {
        for event in prefix {
            let mut changed = event.clone();
            match &mut changed {
                JournalEvent::StepStarted { attempt, .. } => *attempt = 2,
                JournalEvent::ActionScheduledTicket { ticket, .. } => {
                    ticket.run_attempt = 2;
                    ticket.idempotency_key = vb_core::action::compute_action_idempotency_key(
                        ticket.run,
                        ticket.run_attempt,
                        ticket.seq,
                        ticket.action,
                    );
                }
                _ => continue,
            }
            let bytes = vb_storage::codec::encode_record(
                vb_storage::constants::MAGIC_JOURNAL_EVENT,
                changed.record_kind(),
                changed.seq().get(),
                &changed,
                u32::MAX,
            )?;
            events.insert(vb_storage::keys::run_event_key(RUN, changed.seq())?, bytes)?;
        }
    } else if matches!(damage, Damage::BareStart) {
        events.remove(vb_storage::keys::run_event_key(RUN, selected.seq())?)?;
    } else {
        let target = match damage {
            Damage::PredecessorCapacity => prefix.iter().find(|event| matches!(event, JournalEvent::ActionScheduledTicket { ticket, .. } if *ticket == first)),
            Damage::FailureAttempt => prefix.iter().find(|event| matches!(event, JournalEvent::ActionFailedEvent { .. })),
            _ => Some(selected),
        }.ok_or("actual target absent")?;
        let mut changed = target.clone();
        change(&mut changed, first, damage)?;
        let bytes = vb_storage::codec::encode_record(
            vb_storage::constants::MAGIC_JOURNAL_EVENT,
            changed.record_kind(),
            changed.seq().get(),
            &changed,
            u32::MAX,
        )?;
        events.insert(vb_storage::keys::run_event_key(RUN, changed.seq())?, bytes)?;
    }
    database.persist(fjall::PersistMode::SyncAll)?;
    Ok(())
}

fn change(event: &mut JournalEvent, first: ActionTicket, damage: Damage) -> TestResult {
    if let JournalEvent::ActionFailedEvent { attempt, .. } = event {
        *attempt = 2;
        return Ok(());
    }
    let JournalEvent::ActionScheduledTicket {
        ticket,
        input,
        output,
        action_abi_digest,
        ..
    } = event
    else {
        return Err("schedule absent".into());
    };
    match damage {
        Damage::Attempt => ticket.attempt = 3,
        Damage::RunAttempt => ticket.run_attempt = 2,
        Damage::ForeignRun => ticket.run = vb_core::RunId::new(92422),
        Damage::Capacity | Damage::PredecessorCapacity => ticket.capacity = 2,
        Damage::LogicalSeq => {
            ticket.seq = vb_core::SeqNo::new(ticket.seq.get().checked_add(1).ok_or("seq overflow")?)
        }
        Damage::Key => ticket.idempotency_key ^= 1,
        Damage::SameFence => ticket.fencing_token = first.fencing_token,
        Damage::ZeroFence => ticket.fencing_token = 0,
        Damage::LowerFence => {
            ticket.fencing_token = first.fencing_token.checked_sub(1).ok_or("no lower fence")?
        }
        Damage::Input => *input = SlotIdx::new(2),
        Damage::Output => *output = SlotIdx::new(2),
        Damage::Abi => *action_abi_digest = WorkflowDigest::from_bytes([5; 32]),
        _ => return Err("wrong damage target".into()),
    }
    Ok(())
}
