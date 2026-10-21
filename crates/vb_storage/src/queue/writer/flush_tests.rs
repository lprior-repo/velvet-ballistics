//! Durable writer service behavior at the production queue boundary.
use super::*;
use crate::EventSeq;
use vb_core::{RunId, WorkflowDigest};

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
