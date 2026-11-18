use super::{RUN, TestResult, fixture, rows, runtime};
use std::{path::Path, sync::Arc};
use vb_runtime::{RuntimeError, journal::StorageRuntimeJournal};
use vb_storage::{EventSeq, FjallJournal, JournalError, JournalEvent};

#[derive(Clone, Copy)]
enum Damage {
    Missing,
    Payload,
    OutOfOrder,
}

#[test]
fn missing_issued_completion_refuses_resume_without_mutation() -> TestResult {
    refuses(Damage::Missing)
}

#[test]
fn tampered_issued_completion_refuses_resume_without_mutation() -> TestResult {
    refuses(Damage::Payload)
}

#[test]
fn out_of_order_issued_completion_and_resume_refuse_without_mutation() -> TestResult {
    refuses(Damage::OutOfOrder)
}

fn corrupt(
    path: &Path,
    damage: Damage,
    completion: EventSeq,
    resumed: EventSeq,
) -> TestResult<JournalError> {
    let database = fjall::Database::builder(path).open()?;
    let events = database.keyspace("run_event", fjall::KeyspaceCreateOptions::default)?;
    let completed_key = vb_storage::keys::run_event_key(RUN, completion)?;
    let resumed_key = vb_storage::keys::run_event_key(RUN, resumed)?;
    let expected = match damage {
        Damage::Missing => {
            events.remove(completed_key)?;
            JournalError::SequenceGap {
                expected: completion,
                actual: resumed,
            }
        }
        Damage::Payload => {
            let mut bytes = events
                .get(completed_key)?
                .ok_or("issued completion bytes absent")?
                .to_vec();
            let last = bytes.last_mut().ok_or("issued completion bytes empty")?;
            *last ^= 1;
            events.insert(completed_key, bytes)?;
            JournalError::PayloadDigestMismatch
        }
        Damage::OutOfOrder => {
            let completed_bytes = events
                .get(completed_key)?
                .ok_or("issued completion bytes absent")?;
            let resumed_bytes = events
                .get(resumed_key)?
                .ok_or("issued resume bytes absent")?;
            events.insert(completed_key, resumed_bytes)?;
            events.insert(resumed_key, completed_bytes)?;
            JournalError::ReplayKeyMismatch {
                run: RUN,
                key_seq: completion.get(),
                payload_seq: resumed.get(),
            }
        }
    };
    database.persist(fjall::PersistMode::SyncAll)?;
    Ok(expected)
}

fn exact_storage_refusal(journal: &FjallJournal, expected: &JournalError) -> TestResult {
    let result = journal.events_for_run_full(RUN);
    let error = result.err().ok_or("damaged prefix unexpectedly replayed")?;
    match (&error, expected) {
        (JournalError::PayloadDigestMismatch, JournalError::PayloadDigestMismatch) => {}
        (
            JournalError::SequenceGap {
                expected: actual_expected,
                actual: actual_seq,
            },
            JournalError::SequenceGap { expected, actual },
        ) => assert_eq!((actual_expected, actual_seq), (expected, actual)),
        (
            JournalError::ReplayKeyMismatch {
                run,
                key_seq,
                payload_seq,
            },
            JournalError::ReplayKeyMismatch {
                run: expected_run,
                key_seq: expected_key,
                payload_seq: expected_payload,
            },
        ) => {
            assert_eq!(
                (run, key_seq, payload_seq),
                (expected_run, expected_key, expected_payload)
            );
        }
        _ => {
            return Err(
                format!("unexpected storage refusal: {error:?}; expected {expected:?}").into(),
            );
        }
    }
    Ok(())
}

fn refuses(damage: Damage) -> TestResult {
    let directory = tempfile::tempdir()?;
    let (journal, live, _) = fixture::crashed(directory.path(), true)?;
    let prefix = journal.events_for_run_full(RUN)?;
    let completion = prefix
        .iter()
        .find(|event| matches!(event, JournalEvent::ActionCompletedEnvelope { .. }))
        .ok_or("issued completion absent")?
        .seq();
    let resumed = prefix
        .iter()
        .find(|event| matches!(event, JournalEvent::RunResumed { .. }))
        .ok_or("issued resume absent")?
        .seq();
    assert_eq!(completion.get().checked_add(1), Some(resumed.get()));
    let receipt = journal.run_inputs(RUN)?;
    drop(live);
    drop(journal);
    let expected = corrupt(directory.path(), damage, completion, resumed)?;
    let before = rows(directory.path())?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    exact_storage_refusal(&journal, &expected)?;
    let recovered = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    let counters = recovered.counters_snapshot();
    let pending = recovered.pending_boundary_snapshot(8);
    let state = recovered.inspect_run_state(RUN).cloned();
    assert_eq!(state, Err(RuntimeError::RunNotFound));
    let expected_runtime = RuntimeError::Recovery {
        error: expected.to_string(),
    };
    let product_error = recovered
        .recover_product(RUN)
        .err()
        .ok_or("damaged prefix produced recovery product")?;
    assert_eq!(product_error, expected_runtime);
    assert_eq!(recovered.recover_and_resume(RUN), Err(expected_runtime));
    assert_eq!(recovered.inspect_run_state(RUN).cloned(), state);
    assert_eq!(recovered.counters_snapshot(), counters);
    assert_eq!(recovered.pending_boundary_snapshot(8), pending);
    assert_eq!(journal.run_inputs(RUN)?, receipt);
    exact_storage_refusal(&journal, &expected)?;
    drop(recovered);
    drop(journal);
    assert_eq!(rows(directory.path())?, before);
    Ok(())
}
