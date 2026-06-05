use super::common::*;

/// Integration test: parse_event rejects StepSucceeded payload under SlotWritten envelope.
#[test]
fn parse_event_rejects_step_succeeded_under_slot_written_envelope() {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(42),
        step: StepIdx::new(7),
        output: SlotIdx::new(3),
    };

    let mismatched_bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::SlotWritten,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("valid event should encode");

    let result = parse_event(&mismatched_bytes);
    assert!(
        matches!(result, Err(JournalError::InvalidEvent)),
        "parse_event must reject StepSucceeded under SlotWritten envelope, got {:?}",
        result
    );
}

/// Integration test: parse_event rejects SlotWrittenEvent payload under StepSucceeded envelope.
#[test]
fn parse_event_rejects_slot_written_under_step_succeeded_envelope() {
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(99),
        slot: SlotIdx::new(5),
        value: None,
        extra: None,
        attempt: 1,
    };

    let mismatched_bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::StepSucceeded,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("valid event should encode");

    let result = parse_event(&mismatched_bytes);
    assert!(
        matches!(result, Err(JournalError::InvalidEvent)),
        "parse_event must reject SlotWrittenEvent under StepSucceeded envelope, got {:?}",
        result
    );
}

/// Integration test: parse_event accepts canonical StepSucceeded encoding.
#[test]
fn parse_event_accepts_canonical_step_succeeded() {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(13),
        step: StepIdx::new(2),
        output: SlotIdx::new(3),
    };

    let canonical_bytes = encode_journal_event_record(&event).expect("valid event should encode");
    let parsed = parse_event(&canonical_bytes).expect("canonical encoding must parse successfully");
    assert!(
        matches!(parsed, JournalEvent::StepSucceeded { run, seq, step, output }
            if run == RunId::new(1)
            && seq == EventSeq::new(13)
            && step == StepIdx::new(2)
            && output == SlotIdx::new(3)
        ),
        "parsed event must be StepSucceeded with correct fields, got {:?}",
        parsed
    );
}

/// Integration test: parse_event accepts canonical SlotWrittenEvent encoding.
#[test]
fn parse_event_accepts_canonical_slot_written_event() {
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(42),
        seq: EventSeq::new(7),
        slot: SlotIdx::new(11),
        value: None,
        extra: None,
        attempt: 3,
    };

    let canonical_bytes = encode_journal_event_record(&event).expect("valid event should encode");
    let parsed = parse_event(&canonical_bytes).expect("canonical encoding must parse successfully");
    assert!(
        matches!(parsed, JournalEvent::SlotWrittenEvent { slot, attempt, .. }
            if slot == SlotIdx::new(11) && attempt == 3
        ),
        "parsed event must be SlotWrittenEvent with correct fields, got {:?}",
        parsed
    );
}
