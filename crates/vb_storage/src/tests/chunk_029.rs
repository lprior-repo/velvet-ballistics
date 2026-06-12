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
fn batch_append_run_cancelled_event_round_trips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(1010);
    let event = JournalEvent::RunCancelled {
        run,
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let mut batch = journal.batch();
    batch
        .append_event(&event)
        .expect("batch.append_event must succeed");
    batch.commit().expect("batch.commit must succeed");
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], event);
}

#[test]
fn batch_append_slot_written_event_round_trips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(1011);
    let event = JournalEvent::SlotWrittenEvent {
        run,
        seq: EventSeq::new(0),
        slot: SlotIdx::new(5),
        value: None,
        extra: None,
        attempt: 1,
    };
    let mut batch = journal.batch();
    batch
        .append_event(&event)
        .expect("batch.append_event must succeed");
    batch.commit().expect("batch.commit must succeed");
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], event);
}

#[test]
fn batch_append_suspended_event_round_trips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(1012);
    let event = JournalEvent::WaitScheduledEvent {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(3),
        attempt: 1,
        deadline_ms: 30000,
    };
    let mut batch = journal.batch();
    batch
        .append_event(&event)
        .expect("batch.append_event must succeed");
    batch.commit().expect("batch.commit must succeed");
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], event);
}

// --- Multi-run isolation (tests 13-16) ---

#[test]
fn events_for_run_isolates_run_a_from_run_b() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run_a = RunId::new(2001);
    let run_b = RunId::new(2002);
    let event_a = JournalEvent::RunAccepted {
        run: run_a,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0xAA; 32]),
    };
    let event_b = JournalEvent::RunAccepted {
        run: run_b,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0xBB; 32]),
    };
    let event_a2 = JournalEvent::RunFinished {
        run: run_a,
        seq: EventSeq::new(1),
        result: SlotIdx::new(0),
        attempt: 1,
    };
    let mut batch = journal.batch();
    batch
        .append_event(&event_a)
        .expect("batch.append_event must succeed");
    batch
        .append_event(&event_b)
        .expect("batch.append_event must succeed");
    batch
        .append_event(&event_a2)
        .expect("batch.append_event must succeed");
    batch.commit().expect("batch.commit must succeed");
    let events_a = journal
        .events_for_run(run_a)
        .expect("events_for_run A must succeed");
    assert_eq!(events_a.len(), 2, "run A must have exactly 2 events");
    assert_eq!(events_a[0], event_a);
    assert_eq!(events_a[1], event_a2);
    let events_b = journal
        .events_for_run(run_b)
        .expect("events_for_run B must succeed");
    assert_eq!(events_b.len(), 1, "run B must have exactly 1 event");
    assert_eq!(events_b[0], event_b);
}

#[test]
fn run_header_isolation_between_runs() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run_1 = RunId::new(3001);
    let run_2 = RunId::new(3002);
    let header_1 = RunHeaderRecord {
        run: run_1,
        workflow_id: WorkflowId::new(10),
        compiled_digest: WorkflowDigest::from_bytes([1; 32]),
        status: 1,
        accepted_at_ms: 100,
    };
    let header_2 = RunHeaderRecord {
        run: run_2,
        workflow_id: WorkflowId::new(20),
        compiled_digest: WorkflowDigest::from_bytes([2; 32]),
        status: 2,
        accepted_at_ms: 200,
    };
    let mut batch = journal.batch();
    batch
        .put_run_header(&header_1)
        .expect("batch.put_run_header must succeed");
    batch
        .put_run_header(&header_2)
        .expect("batch.put_run_header must succeed");
    batch.commit().expect("batch.commit must succeed");
    let found_1 = journal
        .run_header(run_1)
        .expect("run_header run_1 must succeed");
    assert_eq!(found_1, Some(header_1), "run 1 header must match exactly");
    let found_2 = journal
        .run_header(run_2)
        .expect("run_header run_2 must succeed");
    assert_eq!(found_2, Some(header_2), "run 2 header must match exactly");
}

#[test]
fn snapshot_isolation_between_runs() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run_a = RunId::new(3003);
    let run_b = RunId::new(3004);
    let snap_a = RunSnapshot {
        run: run_a,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0xA; 32]),
        slots: vec![1, 2, 3],
        taint: Vec::new(),
    };
    let snap_b = RunSnapshot {
        run: run_b,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0xB; 32]),
        slots: vec![4, 5, 6],
        taint: Vec::new(),
    };
    let mut batch = journal.batch();
    batch
        .put_snapshot(&snap_a)
        .expect("batch.put_snapshot must succeed");
    batch
        .put_snapshot(&snap_b)
        .expect("batch.put_snapshot must succeed");
    batch.commit().expect("batch.commit must succeed");
    let found_a = journal
        .snapshot(run_a, EventSeq::new(0))
        .expect("snapshot A must succeed");
    assert_eq!(found_a, Some(snap_a), "snapshot for run A must match");
    let found_b = journal
        .snapshot(run_b, EventSeq::new(0))
        .expect("snapshot B must succeed");
    assert_eq!(found_b, Some(snap_b), "snapshot for run B must match");
}
