use super::common::*;

/// Integration test: StepSucceeded encodes with RecordKind::StepSucceeded (id=29).
#[test]
fn step_succeeded_encodes_with_record_kind_step_succeeded_id_29() {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        output: SlotIdx::new(0),
    };
    assert_eq!(
        event.record_kind(),
        RecordKind::StepSucceeded,
        "StepSucceeded must use RecordKind::StepSucceeded"
    );
    assert_eq!(
        event.record_kind().id(),
        29,
        "RecordKind::StepSucceeded.id() must be 29"
    );
}

/// Integration test: SlotWrittenEvent encodes with RecordKind::SlotWritten (id=12).
#[test]
fn slot_written_event_encodes_with_record_kind_slot_written_id_12() {
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        slot: SlotIdx::new(0),
        value: None,
        extra: None,
        attempt: 1,
    };
    assert_eq!(
        event.record_kind(),
        RecordKind::SlotWritten,
        "SlotWrittenEvent must use RecordKind::SlotWritten"
    );
    assert_eq!(
        event.record_kind().id(),
        12,
        "RecordKind::SlotWritten.id() must be 12"
    );
}

/// Integration test: StepSucceeded and SlotWrittenEvent kinds are never equal.
#[test]
fn step_succeeded_and_slot_written_record_kinds_are_never_equal() {
    let step_event = JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        output: SlotIdx::new(0),
    };
    let slot_event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        slot: SlotIdx::new(0),
        value: None,
        extra: None,
        attempt: 1,
    };
    assert_ne!(
        step_event.record_kind(),
        slot_event.record_kind(),
        "StepSucceeded and SlotWrittenEvent must use different record kinds"
    );
}
