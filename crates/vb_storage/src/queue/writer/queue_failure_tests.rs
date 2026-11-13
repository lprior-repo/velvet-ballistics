//! Queue reservation behavior across staging failures and batch boundaries.
use super::*;
use crate::EventSeq;
use vb_core::{RunId, StepIdx};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn checkpoint(seq: u64, payload_len: usize) -> JournalEvent {
    JournalEvent::RetryCheckpoint {
        run: RunId::new(900),
        seq: EventSeq::new(seq),
        attempt: 1,
        step: StepIdx::new(0),
        payload_digest: [0; 32],
        payload: vec![0; payload_len],
    }
}

fn record_bytes(event: &JournalEvent) -> Result<u64, Box<dyn std::error::Error>> {
    let payload = postcard::experimental::serialized_size(event)?;
    let header = u64::from(crate::constants::RECORD_HEADER_LEN);
    Ok(u64::try_from(payload)?
        .checked_add(header)
        .ok_or("record size overflow")?)
}

fn pending_bytes(queue: &JournalWriterQueue) -> Result<u64, Box<dyn std::error::Error>> {
    let state = queue
        .state
        .lock()
        .map_err(|_| std::io::Error::other("queue lock poisoned"))?;
    Ok(state.pending_bytes)
}

#[test]
fn partial_flush_releases_exact_atomic_batch_bytes() -> TestResult {
    let first = checkpoint(0, 16);
    let second = checkpoint(1, 24);
    let trailing = checkpoint(2, 8);
    let batch_bytes = record_bytes(&first)?
        .checked_add(record_bytes(&second)?)
        .ok_or("batch size overflow")?;
    let all_bytes = batch_bytes
        .checked_add(record_bytes(&trailing)?)
        .ok_or("queue size overflow")?;
    let queue = JournalWriterQueue::new(4, 1, StorageLimits::DEFAULT)?;
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;

    queue.enqueue_journaled_batch(vec![first.clone(), second.clone()])?;
    queue.enqueue_journaled(trailing.clone())?;
    assert_eq!(pending_bytes(&queue)?, all_bytes);

    let batch_report = queue.flush_batch_durable(&journal)?;
    assert_eq!((batch_report.written, batch_report.drained), (2, 2));
    assert_eq!(pending_bytes(&queue)?, record_bytes(&trailing)?);
    assert_eq!(queue.pending_profile_counts()?.journaled, 1);
    assert_eq!(
        journal.events_for_run(RunId::new(900))?,
        vec![first, second]
    );

    let trailing_report = queue.flush_batch_durable(&journal)?;
    assert_eq!((trailing_report.written, trailing_report.drained), (1, 1));
    assert_eq!(pending_bytes(&queue)?, 0);
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    assert_eq!(
        journal.events_for_run(RunId::new(900))?,
        vec![checkpoint(0, 16), checkpoint(1, 24), trailing]
    );
    Ok(())
}

#[test]
fn staging_failure_preserves_reservation_until_repaired_retry() -> TestResult {
    let queued = checkpoint(0, 16);
    let queue = JournalWriterQueue::new(2, 2, StorageLimits::DEFAULT)?;
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let key = crate::keys::run_event_key(queued.run_id(), queued.seq())?;
    let corrupt = crate::codec::encode_record(
        crate::constants::MAGIC_JOURNAL_EVENT,
        queued.record_kind(),
        queued.seq().get(),
        &(),
        crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    journal.events.insert(key.to_vec(), corrupt.clone())?;
    queue.enqueue_journaled(queued.clone())?;
    let expected_bytes = record_bytes(&queued)?;

    assert!(matches!(
        queue.flush_batch_durable(&journal),
        Err(JournalError::PostcardDecodeFailed(
            postcard::Error::DeserializeUnexpectedEnd
        ))
    ));
    assert_eq!(queue.pending_profile_counts()?.journaled, 1);
    assert_eq!(pending_bytes(&queue)?, expected_bytes);
    assert_eq!(
        journal
            .events
            .get(key.as_slice())?
            .ok_or("injected record missing")?
            .as_ref(),
        corrupt.as_slice()
    );

    journal.events.remove(key.as_slice())?;
    let report = queue.flush_batch_durable(&journal)?;
    assert_eq!((report.written, report.drained), (1, 1));
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    assert_eq!(pending_bytes(&queue)?, 0);
    assert_eq!(journal.events_for_run(RunId::new(900))?, vec![queued]);
    Ok(())
}

#[test]
fn shutdown_precedes_payload_validation_for_batch_enqueue() -> TestResult {
    let limits = StorageLimits {
        max_journal_event_payload_bytes: 1,
    };
    let queue = JournalWriterQueue::new(2, 1, limits)?;
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    assert_eq!(queue.shutdown_durable(&journal)?.drained, 0);

    let result = queue.enqueue_journaled_batch(vec![checkpoint(0, 64)]);
    assert!(matches!(result, Err(JournalError::QueueShutdown)));
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    assert_eq!(pending_bytes(&queue)?, 0);
    Ok(())
}
