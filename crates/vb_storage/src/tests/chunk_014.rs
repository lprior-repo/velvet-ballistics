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
fn append_strict_writes_run_failed_event() {
    // Given an open journal
    // When a RunFailedEvent is appended and retrieved
    // Then the event carries the correct run
    let (_guard, journal) = open_journal();
    let run = RunId::new(16);
    let event = JournalEvent::RunFailedEvent {
        run,
        seq: EventSeq::new(0),
        attempt: 1,
    };
    journal
        .append_strict(&event)
        .expect("journal.append_strict must succeed");

    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].run_id(), run);
}

#[test]
fn append_strict_assigns_monotonically_increasing_sequences() {
    // Given an open journal
    // When three events are appended with seq 0, 1, 2
    // Then events_for_run returns them in contiguous order
    let (_guard, journal) = open_journal();
    let run = RunId::new(17);
    let e0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: test_digest(1),
    };
    let e1 = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    let e2 = JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(2),
        result: vb_core::SlotIdx::new(0),
        attempt: 1,
    };
    journal
        .append_strict(&e0)
        .expect("journal.append_strict must succeed");
    journal
        .append_strict(&e1)
        .expect("journal.append_strict must succeed");
    journal
        .append_strict(&e2)
        .expect("journal.append_strict must succeed");

    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].seq(), EventSeq::new(0));
    assert_eq!(events[1].seq(), EventSeq::new(1));
    assert_eq!(events[2].seq(), EventSeq::new(2));
}

#[test]
fn append_strict_rejects_duplicate_sequence() {
    // Given an open journal with an event at seq 0 for run 50
    // When the same event is appended again
    // Then DuplicateEvent is returned with exact run and seq
    let (_guard, journal) = open_journal();
    let run = RunId::new(50);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: test_digest(1),
    };
    journal
        .append_strict(&event)
        .expect("journal.append_strict must succeed");

    let result = journal.append_strict(&event);
    let Err(JournalError::DuplicateEvent {
        run: dup_run,
        seq: dup_seq,
    }) = result
    else {
        panic!("expected DuplicateEvent, got {:?}", result);
    };
    assert_eq!(dup_run, run);
    assert_eq!(dup_seq, EventSeq::new(0));
}

#[test]
fn events_for_run_returns_events_in_sequence_order() {
    // Given a journal with 5 events for a run
    // When events_for_run is called
    // Then events are returned in ascending sequence order
    let (_guard, journal) = open_journal();
    let run = RunId::new(18);
    let e0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: test_digest(1),
    };
    let e1 = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    let e2 = JournalEvent::SlotWrittenEvent {
        run,
        seq: EventSeq::new(2),
        slot: vb_core::SlotIdx::new(0),
        value: None,
        extra: None,
        attempt: 1,
    };
    let e3 = JournalEvent::StepSucceeded {
        run,
        seq: EventSeq::new(3),
        step: StepIdx::new(0),
        output: vb_core::SlotIdx::new(1),
    };
    let e4 = JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(4),
        result: vb_core::SlotIdx::new(1),
        attempt: 1,
    };
    journal
        .append_journaled(&e0)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&e1)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&e2)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&e3)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&e4)
        .expect("journal.append_journaled must succeed");

    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 5);
    assert_eq!(events[0], e0);
    assert_eq!(events[1], e1);
    assert_eq!(events[2], e2);
    assert_eq!(events[3], e3);
    assert_eq!(events[4], e4);
}

#[test]
fn events_for_run_returns_empty_for_run_with_no_events() {
    // Given an open journal with events for run 1
    // When events_for_run is called for run 2
    // Then it returns an empty vec
    let (_guard, journal) = open_journal();
    let run_a = RunId::new(1);
    let event = JournalEvent::RunAccepted {
        run: run_a,
        seq: EventSeq::new(0),
        workflow: test_digest(1),
    };
    journal
        .append_journaled(&event)
        .expect("journal.append_journaled must succeed");

    let events = journal
        .events_for_run(RunId::new(2))
        .expect("events_for_run should succeed");
    assert!(events.is_empty());
}
