use super::{RUN, TestResult, cut::CutJournal, fixture, rows};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use vb_core::{ActionFailureCode, ActionTicket};
use vb_runtime::RuntimeError;
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

pub(super) fn prefix(
    path: &Path,
    handler: bool,
    resumed: bool,
    owned: bool,
    failure_to_cut: u16,
) -> TestResult<(Arc<FjallJournal>, ActionTicket)> {
    let issuer_path = path.join("issuer");
    let captured = Arc::new(Mutex::new(None));
    let (issuer, mut live, mut issued) =
        fixture::pending_with_adapter(&issuer_path, handler, Some((3, "0ms")), owned, |journal| {
            Arc::new(CutJournal::historical(
                journal,
                failure_to_cut,
                captured.clone(),
            ))
        })?;
    for _ in 1..failure_to_cut {
        live.fail_action(issued, ActionFailureCode::Timeout.into())?;
        issued = fixture::action(&mut live)?;
    }
    live.fail_action(issued, ActionFailureCode::Timeout.into())?;
    assert_eq!(live.tick_all(), Err(RuntimeError::JournalPoisoned));
    let proposed = captured
        .lock()
        .map_err(|_| "capture poisoned")?
        .clone()
        .ok_or("actual proposed failure absent")?;
    let failure = proposed
        .iter()
        .find(|event| {
            matches!(event,
                JournalEvent::ActionFailedEvent { attempt, .. } if *attempt == issued.attempt
            )
        })
        .cloned()
        .ok_or("actual proposed failure absent")?;
    let mut prefix = issuer.events_for_run_full(RUN)?;
    assert!(prefix.iter().all(|event| event.seq() < failure.seq()));
    prefix.push(failure.clone());
    if resumed {
        live.resume_run(RUN)?;
        match live.tick_all() {
            Ok(_) | Err(RuntimeError::JournalPoisoned) => {}
            Err(error) => return Err(error.into()),
        }
        let marker = issuer
            .events_for_run_full(RUN)?
            .into_iter()
            .find_map(|event| match event {
                JournalEvent::RunResumed { run, timestamp, .. } => Some((run, timestamp)),
                _ => None,
            })
            .ok_or("public resume marker absent")?;
        // Hostile metadata composition: preserve the public marker payload and place
        // it after a historical failure. This is not a current Strict crash prefix.
        prefix.push(JournalEvent::RunResumed {
            run: marker.0,
            seq: EventSeq::new(
                failure
                    .seq()
                    .get()
                    .checked_add(1)
                    .ok_or("sequence overflow")?,
            ),
            timestamp: marker.1,
        });
    }
    drop(live);
    drop(issuer);
    copy_retained_rows(&issuer_path, path)?;
    let journal = Arc::new(FjallJournal::open(path, None)?);
    // Materialize a negative legacy shape from actual proposed records. The adapter
    // rejected its whole batch; it never claimed that a partial Strict commit passed.
    journal.append_strict_batch(&prefix)?;
    Ok((journal, issued))
}

fn copy_retained_rows(source: &Path, target: &Path) -> TestResult {
    let source = rows(source)?;
    let database = fjall::Database::builder(target).open()?;
    let mut batch = database.batch();
    for (name, key, value) in source {
        if matches!(name.as_str(), "run_event" | "index_action") {
            continue;
        }
        let keyspace = database.keyspace(&name, fjall::KeyspaceCreateOptions::default)?;
        batch.insert(&keyspace, key, value);
    }
    batch
        .durability(Some(fjall::PersistMode::SyncAll))
        .commit()?;
    Ok(())
}
