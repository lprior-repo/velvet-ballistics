use super::{TestResult, open, raw_rows, submit_pending, tick_pending};
use std::{path::Path, sync::Arc};
use vb_core::{RunId, SlotIdx, SlotValue, Taint};
use vb_runtime::journal::{RuntimeJournal, RuntimeJournalEvent, StorageRuntimeJournal};
use vb_storage::{DurableActionOutcome, FjallJournal, JournalEvent};

fn runtime_event(event: &JournalEvent) -> TestResult<RuntimeJournalEvent> {
    match event {
        JournalEvent::StepStarted {
            run, step, attempt, ..
        } => Ok(RuntimeJournalEvent::StepStarted {
            run: *run,
            step: *step,
            attempt: *attempt,
        }),
        JournalEvent::ActionScheduledTicket {
            ticket,
            input,
            output,
            action_abi_digest,
            ..
        } => Ok(RuntimeJournalEvent::ActionScheduledTicket {
            ticket: *ticket,
            input: *input,
            output: *output,
            action_abi_digest: *action_abi_digest,
        }),
        JournalEvent::ActionCompletedEnvelope {
            ticket,
            output,
            outcome: DurableActionOutcome::Ready,
            value,
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
            ..
        } => Ok(RuntimeJournalEvent::ActionCompletedEnvelope {
            ticket: *ticket,
            output: *output,
            value: value.clone(),
            encoded_len: *encoded_len,
            taint: *taint,
            value_digest: *value_digest,
            action_abi_digest: *action_abi_digest,
        }),
        _ => Err("unexpected issued action event".into()),
    }
}

fn issued_action_events(
    path: &Path,
    run: RunId,
) -> TestResult<(Arc<FjallJournal>, [JournalEvent; 3])> {
    let (journal, mut runtime) = open(path)?;
    submit_pending(&runtime, &journal, run)?;
    let ticket = tick_pending(&mut runtime, run)?;
    let output = vb_ipc::IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::I64(42),
        taint: Taint::Clean,
    };
    let encoded_len = u32::try_from(postcard::to_allocvec(&output.value)?.len())?;
    runtime.complete_action_with_output(ticket, output.into_action_output(encoded_len))?;
    for _ in 0..8 {
        runtime.tick_all()?;
    }
    assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    let source = journal.events_for_run_full(run)?;
    let started = source
        .iter()
        .find(|event| {
            matches!(event,
                JournalEvent::StepStarted { step, .. } if *step == ticket.step
            )
        })
        .ok_or("missing issued action start")?
        .clone();
    let scheduled = source
        .iter()
        .find(|event| {
            matches!(event,
                JournalEvent::ActionScheduledTicket { ticket: actual, .. } if *actual == ticket
            )
        })
        .ok_or("missing issued action schedule")?
        .clone();
    let completed = source
        .iter()
        .find(|event| {
            matches!(event,
                JournalEvent::ActionCompletedEnvelope { ticket: actual, .. } if *actual == ticket
            )
        })
        .ok_or("missing issued action completion")?
        .clone();
    assert_eq!(
        started.seq().get().checked_add(1),
        Some(scheduled.seq().get())
    );
    assert_eq!(
        scheduled.seq().get().checked_add(1),
        Some(completed.seq().get())
    );
    Ok((journal, [started, scheduled, completed]))
}

fn action_index(event: &JournalEvent) -> TestResult<Vec<u8>> {
    let JournalEvent::ActionScheduledTicket { ticket, .. } = event else {
        return Err("missing issued scheduled ticket".into());
    };
    Ok(vb_storage::keys::index_action_key(ticket.action, ticket.run, ticket.step)?.to_vec())
}

#[test]
fn storage_runtime_batch_commits_exact_issued_events_and_action_index() -> TestResult {
    // Given adjacent records from a successfully completed Strict Runtime action.
    let dir = tempfile::tempdir()?;
    let run = RunId::new(12345);
    let (issuer, [started, scheduled, _]) = issued_action_events(&dir.path().join("issuer"), run)?;
    let index = action_index(&scheduled)?;
    let started_bytes = issuer
        .get_event_bytes(run, started.seq())?
        .ok_or("missing issued start bytes")?;
    let scheduled_bytes = issuer
        .get_event_bytes(run, scheduled.seq())?
        .ok_or("missing issued schedule bytes")?;
    let path = dir.path().join("batch");
    let journal = Arc::new(FjallJournal::open(&path, None)?);
    let adapter = StorageRuntimeJournal::strict(Arc::clone(&journal));
    // When both issued records are appended at their original sequence positions.
    adapter.append_sequenced_batch(
        &[runtime_event(&started)?, runtime_event(&scheduled)?],
        started.seq(),
    )?;
    drop(adapter);
    drop(journal);
    // Then cold storage retains the exact source records and automatic action index.
    let cold = FjallJournal::open(&path, None)?;
    assert_eq!(
        cold.get_event_bytes(run, started.seq())?,
        Some(started_bytes)
    );
    assert_eq!(
        cold.get_event_bytes(run, scheduled.seq())?,
        Some(scheduled_bytes)
    );
    assert!(cold.has_action_index_entry(&index)?);
    Ok(())
}

#[test]
fn storage_runtime_batch_late_duplicate_preserves_all_keyspaces() -> TestResult {
    // Given an actual completed record already persisted at its issued sequence.
    let dir = tempfile::tempdir()?;
    let run = RunId::new(12345);
    let (issuer, [_, scheduled, completed]) =
        issued_action_events(&dir.path().join("issuer"), run)?;
    let index = action_index(&scheduled)?;
    let completed_bytes = issuer
        .get_event_bytes(run, completed.seq())?
        .ok_or("missing issued completion bytes")?;
    let path = dir.path().join("batch");
    let journal = Arc::new(FjallJournal::open(&path, None)?);
    let adapter = StorageRuntimeJournal::strict(Arc::clone(&journal));
    adapter.append_sequenced(runtime_event(&completed)?, completed.seq())?;
    assert!(!journal.has_action_index_entry(&index)?);
    drop(adapter);
    drop(journal);
    let before = raw_rows(&path)?;
    let journal = Arc::new(FjallJournal::open(&path, None)?);
    let adapter = StorageRuntimeJournal::strict(Arc::clone(&journal));
    // When the scheduled prefix stages before a second-position duplicate completion.
    let result = adapter.append_sequenced_batch(
        &[runtime_event(&scheduled)?, runtime_event(&completed)?],
        scheduled.seq(),
    );
    assert!(
        matches!(&result,
            Err(vb_runtime::RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::DuplicateEvent { run: actual, seq }
                    if *actual == run && *seq == completed.seq())
        ),
        "{result:?}"
    );
    drop(adapter);
    drop(journal);
    // Then every declared keyspace is unchanged cold, including prefix and index absence.
    assert_eq!(raw_rows(&path)?, before);
    let cold = FjallJournal::open(&path, None)?;
    assert_eq!(cold.get_event_bytes(run, scheduled.seq())?, None);
    assert_eq!(
        cold.get_event_bytes(run, completed.seq())?,
        Some(completed_bytes)
    );
    assert!(!cold.has_action_index_entry(&index)?);
    Ok(())
}
