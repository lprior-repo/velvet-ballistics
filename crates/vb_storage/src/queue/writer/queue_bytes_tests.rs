//! Queue payload and pending-byte reservation behaviors.
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
        payload: vec![0u8; payload_len],
    }
}

fn encoded_record_bytes(event: &JournalEvent) -> Result<u64, Box<dyn std::error::Error>> {
    let payload = postcard::experimental::serialized_size(event)?;
    let record = u64::try_from(payload)?
        .checked_add(u64::from(crate::constants::RECORD_HEADER_LEN))
        .ok_or_else(|| std::io::Error::other("record size overflow"))?;
    Ok(record)
}

fn pending_bytes(queue: &JournalWriterQueue) -> Result<u64, Box<dyn std::error::Error>> {
    let state = queue
        .state
        .lock()
        .map_err(|_| std::io::Error::other("queue lock poisoned"))?;
    Ok(state.pending_bytes)
}

#[test]
fn configured_payload_limit_rejects_before_queue_mutation() -> TestResult {
    let accepted = checkpoint(0, 64);
    let measured = postcard::experimental::serialized_size(&accepted)?;
    let configured = u32::try_from(measured.checked_sub(1).ok_or("empty payload size")?)?;
    let limits = StorageLimits {
        max_journal_event_payload_bytes: configured,
    };
    let queue = JournalWriterQueue::new(4, 2, limits)?;

    let result = queue.enqueue_journaled(accepted);

    assert!(matches!(
        result,
        Err(JournalError::PayloadTooLarge { len, max })
            if usize::try_from(len).ok() == Some(measured) && max == configured
    ));
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    assert_eq!(pending_bytes(&queue)?, 0);

    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let healthy = checkpoint(0, 0);
    queue.enqueue_journaled(healthy.clone())?;
    let first_report = queue.flush_batch_durable(&journal)?;
    assert_eq!((first_report.written, first_report.drained), (1, 1));
    assert_eq!(pending_bytes(&queue)?, 0);

    let after_flush = checkpoint(1, 0);
    queue.enqueue_journaled(after_flush.clone())?;
    let second_report = queue.flush_batch_durable(&journal)?;
    assert_eq!((second_report.written, second_report.drained), (1, 1));
    assert_eq!(pending_bytes(&queue)?, 0);
    assert_eq!(
        journal.events_for_run(RunId::new(900))?,
        vec![healthy, after_flush]
    );
    Ok(())
}

#[test]
fn configured_payload_limit_accepts_exact_serialized_boundary() -> TestResult {
    let accepted = checkpoint(0, 64);
    let configured = u32::try_from(postcard::experimental::serialized_size(&accepted)?)?;
    let limits = StorageLimits {
        max_journal_event_payload_bytes: configured,
    };
    let queue = JournalWriterQueue::new(4, 2, limits)?;
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;

    queue.enqueue_journaled(accepted.clone())?;
    let report = queue.flush_batch_durable(&journal)?;

    assert_eq!((report.written, report.drained), (1, 1));
    assert_eq!(journal.events_for_run(RunId::new(900))?, vec![accepted]);
    Ok(())
}

#[test]
fn zero_payload_limit_rejects_nonempty_event() -> TestResult {
    let rejected = checkpoint(0, 0);
    let measured = postcard::experimental::serialized_size(&rejected)?;
    let limits = StorageLimits {
        max_journal_event_payload_bytes: 0,
    };
    let queue = JournalWriterQueue::new(1, 1, limits)?;

    let result = queue.enqueue_journaled(rejected);

    assert!(matches!(
        result,
        Err(JournalError::PayloadTooLarge { len, max })
            if usize::try_from(len).ok() == Some(measured) && max == 0
    ));
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    Ok(())
}

#[test]
fn configured_payload_limit_above_global_cap_keeps_global_ceiling() -> TestResult {
    let global_max = crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
    let payload_len = usize::try_from(global_max)?
        .checked_add(1)
        .ok_or("size overflow")?;
    let rejected = checkpoint(0, payload_len);
    let measured = postcard::experimental::serialized_size(&rejected)?;
    let limits = StorageLimits {
        max_journal_event_payload_bytes: u32::MAX,
    };
    let queue = JournalWriterQueue::new(1, 1, limits)?;

    let result = queue.enqueue_journaled(rejected);

    assert!(matches!(
        result,
        Err(JournalError::PayloadTooLarge { len, max })
            if usize::try_from(len).ok() == Some(measured) && max == global_max
    ));
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    Ok(())
}

#[test]
fn late_oversized_batch_member_precedes_capacity_error_without_mutation() -> TestResult {
    let queued = checkpoint(0, 64);
    let first = checkpoint(2, 64);
    let oversized = checkpoint(3, 65);
    let configured = u32::try_from(postcard::experimental::serialized_size(&first)?)?;
    let limits = StorageLimits {
        max_journal_event_payload_bytes: configured,
    };
    let queue = JournalWriterQueue::new(1, 1, limits)?;
    queue.enqueue_journaled(queued.clone())?;
    let reserved_bytes = encoded_record_bytes(&queued)?;
    assert_eq!(pending_bytes(&queue)?, reserved_bytes);
    let measured = postcard::experimental::serialized_size(&oversized)?;

    let result = queue.enqueue_journaled_batch(vec![first, oversized]);

    assert!(matches!(
        result,
        Err(JournalError::PayloadTooLarge { len, max })
            if usize::try_from(len).ok() == Some(measured) && max == configured
    ));
    assert_eq!(queue.pending_profile_counts()?.journaled, 1);
    assert_eq!(pending_bytes(&queue)?, reserved_bytes);
    assert_eq!(
        queue.pending_high_water_for_run(RunId::new(900))?,
        Some(EventSeq::new(0))
    );

    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let report = queue.flush_batch_durable(&journal)?;
    assert_eq!((report.written, report.drained), (1, 1));
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    assert_eq!(pending_bytes(&queue)?, 0);
    assert_eq!(journal.events_for_run(RunId::new(900))?, vec![queued]);
    Ok(())
}

#[test]
fn valid_batch_over_capacity_returns_queue_full_without_reservation() -> TestResult {
    let queue = JournalWriterQueue::new(2, 1, StorageLimits::DEFAULT)?;

    let result = queue.enqueue_journaled_batch(vec![
        checkpoint(0, 64),
        checkpoint(1, 64),
        checkpoint(2, 64),
    ]);

    assert!(matches!(result, Err(JournalError::QueueFull)));
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    assert_eq!(pending_bytes(&queue)?, 0);
    Ok(())
}

#[test]
fn queue_byte_reservation_is_retained_on_failure_and_released_after_commit() -> TestResult {
    let first = checkpoint(0, 600_000);
    let second = checkpoint(1, 600_000);
    let queue = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT)?;
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    let expected_bytes = encoded_record_bytes(&first)?
        .checked_add(encoded_record_bytes(&second)?)
        .ok_or_else(|| std::io::Error::other("queue size overflow"))?;
    queue.enqueue_journaled(first.clone())?;
    queue.enqueue_journaled(second.clone())?;
    assert_eq!(queue.pending_profile_counts()?.journaled, 2);
    assert_eq!(pending_bytes(&queue)?, expected_bytes);
    journal.fail_next_batch_commit_for_test();
    assert!(matches!(
        queue.flush_batch_durable(&journal),
        Err(JournalError::Fjall(_))
    ));
    assert_eq!(queue.pending_profile_counts()?.journaled, 2);
    assert_eq!(pending_bytes(&queue)?, expected_bytes);
    assert_eq!(queue.flush_batch_durable(&journal)?.drained, 2);

    assert_eq!(pending_bytes(&queue)?, 0);
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    assert_eq!(
        journal.events_for_run(RunId::new(900))?,
        vec![first, second]
    );
    Ok(())
}

#[test]
fn atomic_batch_larger_than_flush_count_is_committed_whole() -> TestResult {
    let queue = JournalWriterQueue::new(4, 1, StorageLimits::DEFAULT)?;
    let first = checkpoint(0, 64);
    let second = checkpoint(1, 64);
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;

    queue.enqueue_journaled_batch(vec![first.clone(), second.clone()])?;
    let expected_bytes = encoded_record_bytes(&first)?
        .checked_add(encoded_record_bytes(&second)?)
        .ok_or_else(|| std::io::Error::other("batch size overflow"))?;
    assert_eq!(queue.pending_profile_counts()?.journaled, 2);
    assert_eq!(pending_bytes(&queue)?, expected_bytes);
    let report = queue.flush_batch_durable(&journal)?;

    assert_eq!((report.written, report.drained), (2, 2));
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    assert_eq!(pending_bytes(&queue)?, 0);
    assert_eq!(
        journal.events_for_run(RunId::new(900))?,
        vec![first, second]
    );
    Ok(())
}
