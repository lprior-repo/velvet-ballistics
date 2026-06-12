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
fn batch_writes_for_multiple_runs_commit_atomically() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run_1 = RunId::new(4001);
    let run_2 = RunId::new(4002);
    let run_3 = RunId::new(4003);
    let mut batch = journal.batch();
    batch
        .append_event(&JournalEvent::RunAccepted {
            run: run_1,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        })
        .expect("batch.append_event must succeed");
    batch
        .append_event(&JournalEvent::RunAccepted {
            run: run_2,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([2; 32]),
        })
        .expect("batch.append_event must succeed");
    batch
        .append_event(&JournalEvent::RunAccepted {
            run: run_3,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([3; 32]),
        })
        .expect("batch.append_event must succeed");
    batch.commit().expect("batch.commit must succeed");
    assert_eq!(
        journal
            .events_for_run(run_1)
            .expect("run_1 must succeed")
            .len(),
        1,
        "run 1 must have 1 event"
    );
    assert_eq!(
        journal
            .events_for_run(run_2)
            .expect("run_2 must succeed")
            .len(),
        1,
        "run 2 must have 1 event"
    );
    assert_eq!(
        journal
            .events_for_run(run_3)
            .expect("run_3 must succeed")
            .len(),
        1,
        "run 3 must have 1 event"
    );
}

// --- Writer Queue edge cases (tests 17-22) ---

#[test]
fn queue_journaled_enqueue_and_drain_preserves_order() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
    let run = RunId::new(5001);
    let event_0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    let event_1 = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    let event_2 = JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(2),
        result: SlotIdx::new(0),
        attempt: 1,
    };
    queue
        .enqueue_journaled(event_0.clone())
        .expect("enqueue 0 must succeed");
    queue
        .enqueue_journaled(event_1.clone())
        .expect("enqueue 1 must succeed");
    queue
        .enqueue_journaled(event_2.clone())
        .expect("enqueue 2 must succeed");
    let report = queue.drain_all(&journal).expect("drain_all must succeed");
    assert_eq!(report.drained, 3);
    assert_eq!(report.written, 3);
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events[0], event_0, "first event must be seq 0");
    assert_eq!(events[1], event_1, "second event must be seq 1");
    assert_eq!(events[2], event_2, "third event must be seq 2");
}

#[test]
fn queue_strict_enqueue_and_drain_preserves_order() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
    let run = RunId::new(5002);
    let event_0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([2; 32]),
    };
    let event_1 = JournalEvent::RunCancelled {
        run,
        seq: EventSeq::new(1),
        attempt: 1,
        reason: None,
    };
    queue
        .enqueue_strict(event_0.clone())
        .expect("enqueue 0 must succeed");
    queue
        .enqueue_strict(event_1.clone())
        .expect("enqueue 1 must succeed");
    let report = queue.drain_all(&journal).expect("drain_all must succeed");
    assert_eq!(report.drained, 2);
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events[0], event_0);
    assert_eq!(events[1], event_1);
}

#[test]
fn queue_mixed_journaled_and_strict_drain_returns_both() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
    let run = RunId::new(5003);
    let journaled_event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([3; 32]),
    };
    let strict_event = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::ZERO,
        attempt: 1,
    };
    queue
        .enqueue_journaled(journaled_event.clone())
        .expect("enqueue journaled must succeed");
    queue
        .enqueue_strict(strict_event.clone())
        .expect("enqueue strict must succeed");
    let report = queue.drain_all(&journal).expect("drain_all must succeed");
    assert_eq!(report.drained, 2, "both events must be drained");
    assert_eq!(report.written, 2, "both events must be written");
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], journaled_event);
    assert_eq!(events[1], strict_event);
}

#[test]
fn queue_flush_persists_before_drain() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
    let run = RunId::new(5004);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([4; 32]),
    };
    queue
        .enqueue_journaled(event.clone())
        .expect("enqueue must succeed");
    let report = queue
        .flush_batch(&journal)
        .expect("flush_batch must succeed");
    assert_eq!(report.written, 1, "one event must be written");
    let events_before = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events_before.len(), 1, "event must be on disk before drain");
    assert_eq!(events_before[0], event);
}

#[test]
fn queue_empty_drain_returns_zero_events() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
    let report = queue.drain_all(&journal).expect("drain_all must succeed");
    assert_eq!(report.drained, 0, "empty queue must drain zero events");
    assert_eq!(report.written, 0, "empty queue must write zero events");
}
