#![allow(
    unused_imports,
    dead_code,
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
use super::prelude::*;

#[test]
fn journal_writer_queue_drain_all_flushes_until_empty() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
    let queue = JournalWriterQueue::new(4, 1, StorageLimits::DEFAULT).expect("q");
    let run = RunId::new(2);
    let workflow = test_digest(2);

    assert!(
        queue
            .enqueue_journaled(JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow,
            })
            .is_ok()
    );
    assert!(
        queue
            .enqueue_journaled(JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(1),
                attempt: 1,
                reason: None,
            })
            .is_ok()
    );

    assert!(matches!(
        queue.drain_all(&journal),
        Ok(report) if report.drained == 2 && report.written == 2
    ));
    assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() == 2));
}

#[test]
fn journal_writer_queue_retains_events_when_append_fails() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
    let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT).expect("q");
    let run = RunId::new(3);
    let duplicate = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: test_digest(3),
    };
    let conflicting_duplicate = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: test_digest(33),
    };
    let next = JournalEvent::RunCancelled {
        run,
        seq: EventSeq::new(1),
        attempt: 1,
        reason: None,
    };

    assert!(matches!(journal.append_journaled(&duplicate), Ok(())));
    assert!(matches!(
        queue.enqueue_journaled(conflicting_duplicate),
        Ok(())
    ));
    assert!(matches!(queue.enqueue_journaled(next), Ok(())));

    assert!(matches!(
        queue.flush_batch(&journal),
        Err(JournalError::DuplicateEvent { run: found, seq })
            if found == run && seq == EventSeq::new(0)
    ));
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 2 && counts.strict == 0
    ));
}

#[test]
fn journal_writer_queue_flush_persists_journaled_events_before_drain() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let path = temp_dir.path().to_path_buf();
    let journal = FjallJournal::open(&path, None).expect("opens");
    let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT).expect("q");
    let run = RunId::new(4);
    let accepted = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: test_digest(4),
    };
    let cancelled = JournalEvent::RunCancelled {
        run,
        seq: EventSeq::new(1),
        attempt: 1,
        reason: None,
    };

    queue
        .enqueue_journaled(accepted)
        .expect("queue.enqueue_journaled must succeed");
    queue
        .enqueue_journaled(cancelled)
        .expect("queue.enqueue_journaled must succeed");
    assert!(matches!(
        queue.flush_batch(&journal),
        Ok(report) if report.drained == 2 && report.written == 2
    ));
    drop(journal);

    let reopened = FjallJournal::open(&path, None).expect("reopen");
    assert!(matches!(reopened.events_for_run(run), Ok(events) if events.len() == 2));
}

#[test]
fn journal_writer_queue_shutdown_rejects_new_writes_after_durable_drain() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
    let queue = JournalWriterQueue::new(4, 1, StorageLimits::DEFAULT).expect("q");
    let run = RunId::new(5);
    let accepted = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: test_digest(5),
    };
    let cancelled = JournalEvent::RunCancelled {
        run,
        seq: EventSeq::new(1),
        attempt: 1,
        reason: None,
    };

    queue
        .enqueue_journaled(accepted.clone())
        .expect("queue.enqueue_journaled must succeed");
    queue
        .enqueue_strict(cancelled)
        .expect("queue.enqueue_strict must succeed");
    assert!(matches!(
        queue.shutdown(&journal),
        Ok(report) if report.drained == 2 && report.written == 2
    ));
    assert!(matches!(
        queue.enqueue_journaled(accepted),
        Err(JournalError::QueueShutdown)
    ));
    assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() == 2));
}

#[test]
fn journal_writer_queue_crash_window_retry_drains_already_written_same_event() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
    let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT).expect("q");
    let run = RunId::new(6);
    let accepted = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: test_digest(6),
    };
    let cancelled = JournalEvent::RunCancelled {
        run,
        seq: EventSeq::new(1),
        attempt: 1,
        reason: None,
    };

    journal
        .append_journaled(&accepted)
        .expect("journal.append_journaled must succeed");
    queue
        .enqueue_journaled(accepted)
        .expect("queue.enqueue_journaled must succeed");
    queue
        .enqueue_journaled(cancelled)
        .expect("queue.enqueue_journaled must succeed");

    // This models the crash window where a prior attempt reached Fjall before
    // the queue could durably drain. Retrying accepts the identical event only.
    assert!(matches!(
        queue.flush_batch(&journal),
        Ok(report) if report.drained == 2 && report.written == 2
    ));
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 0 && counts.strict == 0
    ));
    assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() == 2));
}
