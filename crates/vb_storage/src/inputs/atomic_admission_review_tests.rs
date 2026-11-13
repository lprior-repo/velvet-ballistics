use super::*;
use crate::{EventSeq, JournalEvent, JournalWriterQueue, StorageLimits};
use vb_core::{CapabilitySet, RuntimePolicy};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn record(
    run: u64,
    inputs: Box<[(SlotIdx, SlotValue)]>,
) -> Result<RunInputsRecord, Box<dyn std::error::Error>> {
    let workflow = super::tests::workflow()?;
    let artifact =
        crate::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    Ok(RunInputsRecord::new(
        RunId::new(run),
        artifact.digest,
        WorkflowDigest::from_bytes([2; 32]),
        &workflow,
        inputs,
    )?)
}

fn scalar_record(run: u64, value: i64) -> Result<RunInputsRecord, Box<dyn std::error::Error>> {
    record(run, Box::from([(SlotIdx::new(0), SlotValue::I64(value))]))
}

fn receipt(record: &RunInputsRecord, seq: u64) -> JournalEvent {
    JournalEvent::RunAdmission {
        run: record.run(),
        seq: EventSeq::new(seq),
        artifact_digest: record.artifact_digest(),
        input_digest: RunInputBinding::Digest(record.input_digest()),
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Strict,
        policy_pin: RunPolicyPin::Unverified,
    }
}

fn accepted(record: &RunInputsRecord, seq: u64, workflow: WorkflowDigest) -> JournalEvent {
    JournalEvent::RunAccepted {
        run: record.run(),
        seq: EventSeq::new(seq),
        workflow,
    }
}

#[test]
fn direct_admission_rejects_mismatched_and_ambiguous_authority_before_writes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let expected = scalar_record(701, 11)?;
    let mismatched = scalar_record(701, 22)?;
    let result = journal.append_runtime_admission_batch(
        &[receipt(&expected, 0)],
        &mismatched,
        crate::types::DurabilityProfile::Strict,
    );
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "{result:?}"
    );
    assert!(journal.events_for_run(expected.run())?.is_empty());
    assert_eq!(journal.run_inputs(expected.run())?, None);

    let duplicate_receipts = [receipt(&expected, 0), receipt(&expected, 1)];
    let result = journal.append_runtime_admission_batch(
        &duplicate_receipts,
        &expected,
        crate::types::DurabilityProfile::Strict,
    );
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "{result:?}"
    );
    assert!(journal.events_for_run(expected.run())?.is_empty());
    assert_eq!(journal.run_inputs(expected.run())?, None);

    let wrong_workflow = [
        accepted(&expected, 0, WorkflowDigest::from_bytes([9; 32])),
        receipt(&expected, 1),
    ];
    let result = journal.append_runtime_admission_batch(
        &wrong_workflow,
        &expected,
        crate::types::DurabilityProfile::Strict,
    );
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "{result:?}"
    );
    assert!(journal.events_for_run(expected.run())?.is_empty());
    assert_eq!(journal.run_inputs(expected.run())?, None);
    Ok(())
}

#[test]
fn direct_admission_persists_empty_and_nonempty_inputs_with_matching_workflow() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let empty = Vec::<(SlotIdx, SlotValue)>::new().into_boxed_slice();
    let nonempty = vec![(SlotIdx::new(0), SlotValue::I64(17))].into_boxed_slice();
    for (run, values) in [(702, empty), (703, nonempty)] {
        let record = record(run, values)?;
        let workflow = record.workflow_digest();
        let events = [accepted(&record, 0, workflow), receipt(&record, 1)];
        journal.append_runtime_admission_batch(
            &events,
            &record,
            crate::types::DurabilityProfile::Strict,
        )?;
        assert_eq!(journal.events_for_run(record.run())?, events);
        assert_eq!(journal.run_inputs(record.run())?, Some(record));
    }
    Ok(())
}

#[test]
fn direct_admission_does_not_replace_an_existing_run_input_row() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let first = scalar_record(706, 17)?;
    let conflict = scalar_record(706, 18)?;
    journal.append_runtime_admission_batch(
        &[receipt(&first, 0)],
        &first,
        crate::types::DurabilityProfile::Strict,
    )?;
    let result = journal.append_runtime_admission_batch(
        &[receipt(&conflict, 1)],
        &conflict,
        crate::types::DurabilityProfile::Strict,
    );
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "{result:?}"
    );
    assert_eq!(journal.events_for_run(first.run())?, [receipt(&first, 0)]);
    assert_eq!(journal.run_inputs(first.run())?, Some(first));
    Ok(())
}

#[test]
fn queued_inputs_reject_conflicts_atomically_and_allow_only_matching_duplicates() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT)?;
    let first = scalar_record(704, 11)?;
    let conflict = scalar_record(704, 22)?;
    queue.enqueue_journaled_admission_batch(vec![receipt(&first, 0)], first.clone())?;
    queue.enqueue_journaled_admission_batch(vec![receipt(&conflict, 1)], conflict.clone())?;
    let result = queue.flush_batch_durable(&journal);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "{result:?}"
    );
    assert_eq!(queue.pending_profile_counts()?.journaled, 2);
    assert!(journal.events_for_run(first.run())?.is_empty());
    assert_eq!(journal.run_inputs(first.run())?, None);

    let distinct = scalar_record(705, 33)?;
    let valid_queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT)?;
    valid_queue.enqueue_journaled_admission_batch(vec![receipt(&first, 0)], first.clone())?;
    valid_queue.enqueue_journaled_admission_batch(vec![receipt(&first, 1)], first.clone())?;
    valid_queue.enqueue_journaled_admission_batch(vec![receipt(&distinct, 0)], distinct.clone())?;
    let report = valid_queue.flush_batch_durable(&journal)?;
    assert_eq!((report.drained, report.written), (3, 3));
    assert_eq!(journal.run_inputs(first.run())?, Some(first.clone()));
    assert_eq!(journal.run_inputs(distinct.run())?, Some(distinct.clone()));
    let committed = journal.events_for_run(first.run())?;
    assert_eq!(committed, [receipt(&first, 0), receipt(&first, 1)]);

    valid_queue.enqueue_journaled_admission_batch(vec![receipt(&conflict, 2)], conflict)?;
    let result = valid_queue.flush_batch_durable(&journal);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "{result:?}"
    );
    assert_eq!(valid_queue.pending_profile_counts()?.journaled, 1);
    assert_eq!(journal.events_for_run(first.run())?, committed);
    assert_eq!(journal.run_inputs(first.run())?, Some(first));
    Ok(())
}

#[test]
fn queued_admission_bounds_and_staging_failures_preserve_rows_and_reservations() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let first = scalar_record(707, 11)?;
    let overflow = scalar_record(708, 22)?;
    let full = JournalWriterQueue::new(1, 1, StorageLimits::DEFAULT)?;
    full.enqueue_journaled_admission_batch(vec![receipt(&first, 0)], first.clone())?;
    let result =
        full.enqueue_journaled_admission_batch(vec![receipt(&overflow, 0)], overflow.clone());
    assert!(matches!(result, Err(JournalError::QueueFull)), "{result:?}");
    assert_eq!(full.pending_profile_counts()?.journaled, 1);
    assert!(journal.events_for_run(first.run())?.is_empty());
    assert_eq!(journal.run_inputs(first.run())?, None);
    assert!(journal.events_for_run(overflow.run())?.is_empty());
    assert_eq!(journal.run_inputs(overflow.run())?, None);

    let invalid_seq = scalar_record(709, 33)?;
    let staging = JournalWriterQueue::new(2, 2, StorageLimits::DEFAULT)?;
    staging.enqueue_journaled_admission_batch(
        vec![receipt(&invalid_seq, u64::MAX)],
        invalid_seq.clone(),
    )?;
    let result = staging.flush_batch_durable(&journal);
    assert!(
        matches!(result, Err(JournalError::SequenceOverflow)),
        "{result:?}"
    );
    assert_eq!(staging.pending_profile_counts()?.journaled, 1);
    assert!(journal.events_for_run(invalid_seq.run())?.is_empty());
    assert_eq!(journal.run_inputs(invalid_seq.run())?, None);

    let commit_failure = scalar_record(710, 44)?;
    let committed = JournalWriterQueue::new(2, 2, StorageLimits::DEFAULT)?;
    committed.enqueue_journaled_admission_batch(
        vec![receipt(&commit_failure, 0)],
        commit_failure.clone(),
    )?;
    journal.fail_next_batch_commit_for_test();
    let result = committed.flush_batch_durable(&journal);
    assert!(
        matches!(result, Err(JournalError::Fjall(fjall::Error::Io(_)))),
        "{result:?}"
    );
    assert_eq!(committed.pending_profile_counts()?.journaled, 1);
    assert!(journal.events_for_run(commit_failure.run())?.is_empty());
    assert_eq!(journal.run_inputs(commit_failure.run())?, None);
    let retry = committed.flush_batch_durable(&journal)?;
    assert_eq!((retry.drained, retry.written), (1, 1));
    assert_eq!(
        journal.events_for_run(commit_failure.run())?,
        [receipt(&commit_failure, 0)]
    );
    assert_eq!(
        journal.run_inputs(commit_failure.run())?,
        Some(commit_failure)
    );
    Ok(())
}
