//! Durable writer service behavior at the production queue boundary.
use super::*;
use crate::EventSeq;
use vb_core::{RunId, StepIdx, WorkflowDigest};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn event(seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run: RunId::new(900),
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([9; 32]),
    }
}

#[test]
fn durable_service_respects_batch_limit_and_fifo() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)?;
    for seq in 0..3 {
        queue.enqueue_journaled(event(seq))?;
    }
    let first = queue.flush_batch_durable(&journal)?;
    assert_eq!((first.written, first.drained), (2, 2));
    assert_eq!(
        journal.events_for_run(RunId::new(900))?,
        vec![event(0), event(1)]
    );
    assert_eq!(queue.pending_profile_counts()?.journaled, 1);
    let last = queue.flush_batch_durable(&journal)?;
    assert_eq!((last.written, last.drained), (1, 1));
    assert_eq!(
        journal.events_for_run(RunId::new(900))?,
        vec![event(0), event(1), event(2)]
    );
    Ok(())
}

#[test]
fn commit_failure_preserves_entire_batch_and_retry_commits_once() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)?;
    queue.enqueue_journaled(event(0))?;
    queue.enqueue_journaled(event(1))?;
    journal.fail_next_batch_commit_for_test();
    let result = queue.flush_batch_durable(&journal);
    assert!(
        matches!(result, Err(JournalError::Fjall(fjall::Error::Io(error))) if error.kind() == std::io::ErrorKind::Other)
    );
    assert_eq!(queue.pending_profile_counts()?.journaled, 2);
    assert!(journal.events_for_run(RunId::new(900))?.is_empty());
    let retry = queue.flush_batch_durable(&journal)?;
    assert_eq!((retry.drained, retry.written), (2, 2));
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    assert_eq!(
        journal.events_for_run(RunId::new(900))?,
        vec![event(0), event(1)]
    );
    Ok(())
}

#[test]
fn idempotent_visible_event_requires_barrier_before_durable_drain() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)?;
    journal.append_journaled(&event(0))?;
    queue.enqueue_journaled(event(0))?;
    journal.fail_next_persist_for_test();
    assert!(matches!(
        queue.flush_batch_durable(&journal),
        Err(JournalError::StrictDurabilityFailed)
    ));
    assert_eq!(queue.pending_profile_counts()?.journaled, 1);
    assert_eq!(journal.events_for_run(RunId::new(900))?, vec![event(0)]);
    let retry = queue.flush_batch_durable(&journal)?;
    assert_eq!((retry.drained, retry.written), (1, 1));
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    assert_eq!(journal.events_for_run(RunId::new(900))?, vec![event(0)]);
    Ok(())
}

#[test]
fn shutdown_commit_failure_retains_batch_closes_admission_and_retries() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(3, 2, StorageLimits::DEFAULT)?;
    for seq in 0..3 {
        queue.enqueue_journaled(event(seq))?;
    }
    journal.fail_next_batch_commit_for_test();
    assert!(matches!(
        queue.shutdown_durable(&journal),
        Err(JournalError::Fjall(_))
    ));
    assert_eq!(queue.pending_profile_counts()?.journaled, 3);
    assert!(journal.events_for_run(RunId::new(900))?.is_empty());
    assert!(matches!(
        queue.enqueue_journaled(event(3)),
        Err(JournalError::QueueShutdown)
    ));
    let retry = queue.shutdown_durable(&journal)?;
    assert_eq!((retry.drained, retry.written), (3, 3));
    assert_eq!(
        journal.events_for_run(RunId::new(900))?,
        vec![event(0), event(1), event(2)]
    );
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    assert_eq!(queue.shutdown_durable(&journal)?.drained, 0);
    Ok(())
}

#[test]
fn shutdown_idempotent_barrier_failure_retains_ownership_for_retry() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(2, 1, StorageLimits::DEFAULT)?;
    journal.append_journaled(&event(0))?;
    queue.enqueue_journaled(event(0))?;
    queue.enqueue_journaled(event(1))?;
    journal.fail_next_persist_for_test();
    assert!(matches!(
        queue.shutdown_durable(&journal),
        Err(JournalError::StrictDurabilityFailed)
    ));
    assert_eq!(queue.pending_profile_counts()?.journaled, 2);
    assert_eq!(journal.events_for_run(RunId::new(900))?, vec![event(0)]);
    let retry = queue.shutdown_durable(&journal)?;
    assert_eq!((retry.drained, retry.written), (2, 2));
    assert_eq!(
        journal.events_for_run(RunId::new(900))?,
        vec![event(0), event(1)]
    );
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    Ok(())
}

fn checkpoint(seq: u64, payload_len: usize) -> JournalEvent {
    JournalEvent::RetryCheckpoint {
        run: RunId::new(900),
        seq: EventSeq::new(seq),
        attempt: 1,
        step: StepIdx::new(0),
        payload_digest: [0; 32],
        payload: vec![0u8; payload_len],
    }
}

fn max_payload_bytes() -> Result<usize, Box<dyn std::error::Error>> {
    Ok(usize::try_from(
        crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?)
}

#[test]
fn oversized_payload_rejected_at_enqueue_leaves_queue_draining() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)?;
    let max = max_payload_bytes()?;
    let oversized = checkpoint(0, max + 1);
    let size = u32::try_from(postcard::experimental::serialized_size(&oversized)?)?;
    assert!(matches!(
        queue.enqueue_journaled(oversized),
        Err(JournalError::PayloadTooLarge { len, max: limit })
            if len == size && limit == crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES
    ));
    // The rejected event never entered the queue, so healthy entries drain.
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    queue.enqueue_journaled(event(1))?;
    let report = queue.flush_batch_durable(&journal)?;
    assert_eq!((report.written, report.drained), (1, 1));
    Ok(())
}

#[test]
fn payload_gate_matches_flush_boundary_exactly() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)?;
    let max = max_payload_bytes()?;
    // Fixed framing around the raw payload bytes: the empty-payload size
    // minus its 1-byte length prefix, plus the 3-byte varint a ~1MiB length
    // occupies. Both size asserts below pin the exact boundary, so any width
    // change fails loudly instead of silently weakening the gate.
    let empty_size = postcard::experimental::serialized_size(&checkpoint(0, 0))?;
    let exact_fit = max - empty_size - 2;
    let fitting = checkpoint(1, exact_fit);
    assert_eq!(postcard::experimental::serialized_size(&fitting)?, max);
    queue.enqueue_journaled(fitting)?;
    let report = queue.flush_batch_durable(&journal)?;
    assert_eq!((report.written, report.drained), (1, 1));

    let one_over = checkpoint(2, exact_fit + 1);
    assert_eq!(postcard::experimental::serialized_size(&one_over)?, max + 1);
    assert!(matches!(
        queue.enqueue_journaled(one_over),
        Err(JournalError::PayloadTooLarge { .. })
    ));
    Ok(())
}

#[test]
fn oversized_batch_member_rejects_whole_batch_without_partial_push() -> TestResult {
    let queue = JournalWriterQueue::new(8, 4, StorageLimits::DEFAULT)?;
    let max = max_payload_bytes()?;
    let result = queue.enqueue_journaled_batch(vec![event(0), checkpoint(1, max + 1), event(2)]);
    assert!(matches!(result, Err(JournalError::PayloadTooLarge { .. })));
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    Ok(())
}

#[test]
fn payload_check_runs_before_capacity_check() -> TestResult {
    let queue = JournalWriterQueue::new(2, 2, StorageLimits::DEFAULT)?;
    queue.enqueue_journaled(event(0))?;
    queue.enqueue_journaled(event(1))?;
    assert!(matches!(
        queue.enqueue_journaled(event(2)),
        Err(JournalError::QueueFull)
    ));
    let max = max_payload_bytes()?;
    // Payload size is intrinsic to the event, queue occupancy is transient:
    // the permanent rejection wins even when the queue is also full.
    assert!(matches!(
        queue.enqueue_journaled(checkpoint(3, max + 1)),
        Err(JournalError::PayloadTooLarge { .. })
    ));
    Ok(())
}

#[test]
fn shutdown_check_runs_before_payload_check() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let queue = JournalWriterQueue::new(8, 2, StorageLimits::DEFAULT)?;
    queue.shutdown(&journal)?;
    let max = max_payload_bytes()?;
    assert!(matches!(
        queue.enqueue_journaled(checkpoint(0, max + 1)),
        Err(JournalError::QueueShutdown)
    ));
    Ok(())
}
