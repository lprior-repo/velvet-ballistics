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


// =========================================================================
// Section: Batch Write-Through Integration Tests (60 new tests)
// =========================================================================

// --- JournalWriteBatch put_run_event round-trips (tests 1-12) ---

#[test]
fn batch_append_run_accepted_event_round_trips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(1001);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    let mut batch = journal.batch();
    batch
        .append_event(&event)
        .expect("batch.append_event must succeed");
    batch.commit().expect("batch.commit must succeed");
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 1, "one event must be stored");
    assert_eq!(events[0], event, "event must round-trip exactly");
}


#[test]
fn batch_append_step_started_event_round_trips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(1002);
    let event = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(1),
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
fn batch_append_step_succeeded_event_round_trips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(1003);
    let event = JournalEvent::StepSucceeded {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(2),
        output: SlotIdx::new(3),
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
fn batch_append_step_failed_event_round_trips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(1004);
    let event = JournalEvent::RunFailedEvent {
        run,
        seq: EventSeq::new(0),
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
fn batch_append_action_scheduled_event_round_trips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(1005);
    let event = JournalEvent::ActionScheduled {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(0),
        action: ActionId::new(7),
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
fn batch_append_action_completed_event_round_trips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(1006);
    let event = JournalEvent::ActionCompletedEvent {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(1),
        action: ActionId::new(8),
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
fn batch_append_action_failed_event_round_trips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(1007);
    let event = JournalEvent::ActionFailedEvent {
        run,
        seq: EventSeq::new(0),
        step: StepIdx::new(2),
        action: ActionId::new(9),
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
fn batch_append_run_finished_event_round_trips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(1008);
    let event = JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(0),
        result: SlotIdx::new(42),
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
fn batch_append_run_failed_event_round_trips() {
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
    let run = RunId::new(1009);
    let event = JournalEvent::RunFailedEvent {
        run,
        seq: EventSeq::new(0),
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
