use super::*;
use crate::{EventSeq, JournalEvent, JournalWriterQueue, StorageLimits};
use vb_core::{CapabilitySet, RuntimePolicy};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn record(value: i64) -> Result<RunInputsRecord, Box<dyn std::error::Error>> {
    let workflow = super::tests::workflow()?;
    let artifact =
        crate::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    Ok(RunInputsRecord::new(
        RunId::new(701),
        artifact.digest,
        workflow.digest(),
        &workflow,
        Box::from([(SlotIdx::new(0), SlotValue::I64(value))]),
    )?)
}

fn receipt(record: &RunInputsRecord, seq: u64) -> JournalEvent {
    JournalEvent::RunAdmission {
        run: record.run(),
        seq: EventSeq::new(seq),
        artifact_digest: record.artifact_digest(),
        input_digest: RunInputBinding::Digest(record.input_digest()),
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Strict,
        policy_pin: crate::RunPolicyPin::Unverified,
    }
}

#[test]
fn review_queued_conflicting_inputs_reject_without_partial_publication() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT)?;
    let first = record(11)?;
    let second = record(22)?;
    queue.enqueue_journaled_admission_batch(vec![receipt(&first, 0)], first.clone())?;
    queue.enqueue_journaled_admission_batch(vec![receipt(&second, 1)], second)?;
    let result = queue.flush_batch_durable(&journal);
    let visible = journal.run_inputs(first.run())?;
    let events = journal.events_for_run(first.run())?;
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "{result:?}; visible inputs: {visible:?}; events: {events:?}"
    );
    assert_eq!(queue.pending_profile_counts()?.journaled, 2);
    assert!(journal.events_for_run(first.run())?.is_empty());
    assert!(journal.run_inputs(first.run())?.is_none());
    Ok(())
}

#[test]
fn review_atomic_admission_rejects_unbound_record_before_commit() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let first = record(11)?;
    let second = record(22)?;
    let result = journal.append_runtime_admission_batch(
        &[receipt(&first, 0)],
        &second,
        crate::DurabilityProfile::Strict,
    );
    let visible = journal.run_inputs(first.run())?;
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "{result:?}; visible inputs: {visible:?}"
    );
    assert!(journal.events_for_run(first.run())?.is_empty());
    assert!(journal.run_inputs(first.run())?.is_none());
    Ok(())
}

#[test]
fn review_matching_queue_rows_and_late_conflict_preserve_committed_identity() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT)?;
    let first = record(11)?;
    queue.enqueue_journaled_admission_batch(vec![receipt(&first, 0)], first.clone())?;
    queue.enqueue_journaled_admission_batch(vec![receipt(&first, 1)], first.clone())?;
    let report = queue.flush_batch_durable(&journal)?;
    assert_eq!((report.drained, report.written), (2, 2));
    assert_eq!(journal.run_inputs(first.run())?, Some(first.clone()));
    let before = journal.events_for_run(first.run())?;
    let second = record(22)?;
    queue.enqueue_journaled_admission_batch(vec![receipt(&second, 2)], second)?;
    assert!(matches!(
        queue.flush_batch_durable(&journal),
        Err(JournalError::PayloadDigestMismatch)
    ));
    assert_eq!(queue.pending_profile_counts()?.journaled, 1);
    assert_eq!(journal.events_for_run(first.run())?, before);
    assert_eq!(journal.run_inputs(first.run())?, Some(first));
    Ok(())
}
