use super::common::*;

/// Integration test: ValidatedJournalRecord::try_new succeeds when parity holds.
#[test]
fn validated_journal_record_succeeds_when_parity_holds() {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(42),
        step: StepIdx::new(7),
        output: SlotIdx::new(3),
    };
    let bytes = encode_journal_event_record(&event).expect("valid event should encode");

    let record = decode_validated_journal_record(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("canonical encoding must decode successfully");

    let parity = record.parity();
    assert!(
        parity.is_exact_match(),
        "parity must be exact match for canonical encoding"
    );
    assert_eq!(
        parity.envelope_kind(),
        RecordKind::StepSucceeded.id(),
        "envelope_kind must be 29 (StepSucceeded)"
    );
    assert_eq!(
        parity.payload_kind(),
        RecordKind::StepSucceeded.id(),
        "payload_kind must be 29 (StepSucceeded)"
    );
}

/// Integration test: ValidatedJournalRecord::try_new succeeds for SlotWrittenEvent.
#[test]
fn validated_journal_record_succeeds_for_slot_written_event() {
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(99),
        slot: SlotIdx::new(5),
        value: None,
        extra: None,
        attempt: 1,
    };
    let bytes = encode_journal_event_record(&event).expect("valid event should encode");

    let record = decode_validated_journal_record(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("canonical encoding must decode successfully");

    let parity = record.parity();
    assert!(
        parity.is_exact_match(),
        "parity must be exact match for canonical SlotWrittenEvent"
    );
    assert_eq!(
        parity.envelope_kind(),
        RecordKind::SlotWritten.id(),
        "envelope_kind must be 12 (SlotWritten)"
    );
    assert_eq!(
        parity.payload_kind(),
        RecordKind::SlotWritten.id(),
        "payload_kind must be 12 (SlotWritten)"
    );
}

/// Integration test: ValidatedJournalRecord::try_new fails when kind/payload mismatch.
#[test]
fn validated_journal_record_fails_when_kind_payload_mismatch() {
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

    let result = decode_validated_journal_record(
        &mismatched_bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::InvalidEvent)),
        "mismatched encoding must be rejected, got {:?}",
        result
    );
}

/// Integration test: ValidatedJournalRecord::try_new fails for structurally invalid event.
#[test]
fn validated_journal_record_fails_when_event_structurally_invalid() {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(0),
        seq: EventSeq::new(42),
        step: StepIdx::new(7),
        output: SlotIdx::new(3),
    };
    assert!(!event.is_valid(), "run_id=0 must be invalid");

    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::StepSucceeded,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("event should encode");

    let result = decode_validated_journal_record(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::InvalidEvent)),
        "invalid event must be rejected, got {:?}",
        result
    );
}

/// Integration test: validated and generic decode agree on canonical and mismatched records.
#[test]
fn validated_journal_record_vs_generic_decode_parity() {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(13),
        step: StepIdx::new(2),
        output: SlotIdx::new(3),
    };
    let bytes = encode_journal_event_record(&event).expect("valid event should encode");

    let validated_result = decode_validated_journal_record(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    let generic_result =
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);

    assert!(
        validated_result.is_ok(),
        "canonical must decode via validated path"
    );
    assert!(
        generic_result.is_ok(),
        "canonical must decode via generic path"
    );

    let mismatched_bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::SlotWritten,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("valid event should encode");

    let validated_mismatch = decode_validated_journal_record(
        &mismatched_bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    let generic_mismatch = decode_journal_event(
        &mismatched_bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );

    assert!(
        matches!(validated_mismatch, Err(JournalError::InvalidEvent)),
        "mismatch must be rejected via validated path"
    );
    assert!(
        matches!(generic_mismatch, Err(JournalError::InvalidEvent)),
        "mismatch must be rejected via generic path"
    );
}
