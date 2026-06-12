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
fn append_strict_writes_step_started_event_with_correct_step() {
    // Given an open journal
    // When a StepStarted event with step 5 is appended and retrieved
    // Then the event carries step 5
    let (_guard, journal) = open_journal();
    let run = RunId::new(10);
    let step = StepIdx::new(5);
    let event = JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(0),
        step,
        attempt: 1,
    };
    journal
        .append_strict(&event)
        .expect("journal.append_strict must succeed");

    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 1);
    let JournalEvent::StepStarted {
        step: found_step, ..
    } = events[0]
    else {
        panic!("expected StepStarted event");
    };
    assert_eq!(found_step, step);
}

#[test]
fn append_strict_writes_step_ended_event_with_correct_step() {
    // Given an open journal
    // When a StepSucceeded event with step 3 is appended and retrieved
    // Then the event carries step 3 and output slot 7
    let (_guard, journal) = open_journal();
    let run = RunId::new(11);
    let step = StepIdx::new(3);
    let output = vb_core::SlotIdx::new(7);
    let event = JournalEvent::StepSucceeded {
        run,
        seq: EventSeq::new(0),
        step,
        output,
    };
    journal
        .append_strict(&event)
        .expect("journal.append_strict must succeed");

    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 1);
    let JournalEvent::StepSucceeded {
        step: found_step,
        output: found_output,
        ..
    } = events[0]
    else {
        panic!("expected StepSucceeded event");
    };
    assert_eq!(found_step, step);
    assert_eq!(found_output, output);
}

#[test]
fn append_strict_writes_slot_written_event_with_correct_slot() {
    // Given an open journal
    // When a SlotWrittenEvent with slot 9 is appended and retrieved
    // Then the event carries slot 9
    let (_guard, journal) = open_journal();
    let run = RunId::new(12);
    let slot = vb_core::SlotIdx::new(9);
    let event = JournalEvent::SlotWrittenEvent {
        run,
        seq: EventSeq::new(0),
        slot,
        value: None,
        extra: None,
        attempt: 1,
    };
    journal
        .append_strict(&event)
        .expect("journal.append_strict must succeed");

    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 1);
    let JournalEvent::SlotWrittenEvent {
        slot: found_slot, ..
    } = events[0]
    else {
        panic!("expected SlotWrittenEvent");
    };
    assert_eq!(found_slot, slot);
}

#[test]
fn append_strict_writes_action_scheduled_event_with_correct_step() {
    // Given an open journal
    // When an ActionScheduled event with step 4 is appended and retrieved
    // Then the event carries step 4 and action 2
    let (_guard, journal) = open_journal();
    let run = RunId::new(13);
    let step = StepIdx::new(4);
    let action = ActionId::new(2);
    let event = JournalEvent::ActionScheduled {
        run,
        seq: EventSeq::new(0),
        step,
        action,
        attempt: 1,
    };
    journal
        .append_strict(&event)
        .expect("journal.append_strict must succeed");

    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 1);
    let JournalEvent::ActionScheduled {
        step: found_step,
        action: found_action,
        ..
    } = events[0]
    else {
        panic!("expected ActionScheduled event");
    };
    assert_eq!(found_step, step);
    assert_eq!(found_action, action);
}

#[test]
fn append_strict_writes_action_completed_event_with_correct_step() {
    // Given an open journal
    // When an ActionCompletedEvent with step 6 is appended and retrieved
    // Then the event carries step 6 and action 3
    let (_guard, journal) = open_journal();
    let run = RunId::new(14);
    let step = StepIdx::new(6);
    let action = ActionId::new(3);
    let event = JournalEvent::ActionCompletedEvent {
        run,
        seq: EventSeq::new(0),
        step,
        action,
        attempt: 1,
    };
    journal
        .append_strict(&event)
        .expect("journal.append_strict must succeed");

    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 1);
    let JournalEvent::ActionCompletedEvent {
        step: found_step,
        action: found_action,
        ..
    } = events[0]
    else {
        panic!("expected ActionCompletedEvent");
    };
    assert_eq!(found_step, step);
    assert_eq!(found_action, action);
}

#[test]
fn append_strict_writes_run_finished_event_with_correct_result() {
    // Given an open journal
    // When a RunFinished event with result slot 15 is appended and retrieved
    // Then the event carries result 15
    let (_guard, journal) = open_journal();
    let run = RunId::new(15);
    let result = vb_core::SlotIdx::new(15);
    let event = JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(0),
        result,
        attempt: 1,
    };
    journal
        .append_strict(&event)
        .expect("journal.append_strict must succeed");

    let events = journal
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 1);
    let JournalEvent::RunFinished {
        result: found_result,
        ..
    } = events[0]
    else {
        panic!("expected RunFinished event");
    };
    assert_eq!(found_result, result);
}
