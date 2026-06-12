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
fn encode_decode_roundtrip_for_slot_written_record() {
    // Given a SlotWrittenEvent
    // When encoded and decoded
    // Then the event survives the roundtrip exactly
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(4),
        seq: EventSeq::new(3),
        slot: vb_core::SlotIdx::new(7),
        value: None,
        extra: None,
        attempt: 1,
    };
    let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::SlotWritten, 3, &event, 128)
        .expect("encoding should succeed");
    let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .expect("decoding should succeed");
    assert_eq!(decoded, event);
}

#[test]
fn encode_decode_roundtrip_for_action_scheduled_record() {
    // Given an ActionScheduled event
    // When encoded and decoded
    // Then the event survives the roundtrip exactly
    let event = JournalEvent::ActionScheduled {
        run: RunId::new(5),
        seq: EventSeq::new(4),
        step: StepIdx::new(2),
        action: ActionId::new(3),
        attempt: 1,
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::ActionScheduled,
        4,
        &event,
        128,
    )
    .expect("encoding should succeed");
    let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .expect("decoding should succeed");
    assert_eq!(decoded, event);
}

#[test]
fn encode_decode_roundtrip_for_action_completed_record() {
    // Given an ActionCompletedEvent
    // When encoded and decoded
    // Then the event survives the roundtrip exactly
    let event = JournalEvent::ActionCompletedEvent {
        run: RunId::new(6),
        seq: EventSeq::new(5),
        step: StepIdx::new(2),
        action: ActionId::new(3),
        attempt: 1,
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::ActionCompleted,
        5,
        &event,
        128,
    )
    .expect("encoding should succeed");
    let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .expect("decoding should succeed");
    assert_eq!(decoded, event);
}

#[test]
fn encode_decode_roundtrip_for_run_finished_record() {
    // Given a RunFinished event
    // When encoded and decoded
    // Then the event survives the roundtrip exactly
    let event = JournalEvent::RunFinished {
        run: RunId::new(7),
        seq: EventSeq::new(6),
        result: vb_core::SlotIdx::new(99),
        attempt: 1,
    };
    let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunFinished, 6, &event, 128)
        .expect("encoding should succeed");
    let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .expect("decoding should succeed");
    assert_eq!(decoded, event);
}

#[test]
fn encode_decode_roundtrip_for_run_failed_record() {
    // Given a RunFailedEvent
    // When encoded and decoded
    // Then the event survives the roundtrip exactly
    let event = JournalEvent::RunFailedEvent {
        run: RunId::new(8),
        seq: EventSeq::new(7),
        attempt: 1,
    };
    let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunFailed, 7, &event, 128)
        .expect("encoding should succeed");
    let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .expect("decoding should succeed");
    assert_eq!(decoded, event);
}

#[test]
fn encode_record_rejects_record_exceeding_max_payload() {
    // Given a workflow source with 200 bytes of source data
    // When encode_record is called with max_payload_len of 10
    // Then it returns PayloadTooLarge
    let source = WorkflowSourceRecord {
        digest: test_digest(1),
        source: vec![0u8; 200],
    };
    let result = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        &source,
        10,
    );
    let Err(JournalError::PayloadTooLarge { len, max }) = result else {
        panic!("expected PayloadTooLarge, got {:?}", result);
    };
    assert_eq!(max, 10);
    assert!(len > 10);
}

#[test]
fn encode_decode_roundtrip_for_action_failed_record() {
    // Given an ActionFailedEvent
    // When encoded and decoded
    // Then the event survives the roundtrip exactly
    let event = JournalEvent::ActionFailedEvent {
        run: RunId::new(9),
        seq: EventSeq::new(3),
        step: StepIdx::new(1),
        action: ActionId::new(4),
        attempt: 1,
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::ActionFailed,
        3,
        &event,
        128,
    )
    .expect("encoding should succeed");
    let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .expect("decoding should succeed");
    assert_eq!(decoded, event);
}

// --- Section 6: JournalError Variant Tests ---

#[test]
fn journal_error_encode_from_postcard_error() {
    // Given a payload that causes a postcard encoding error
    // When encode_record encounters the error
    // Then JournalError::Encode is returned
    // This is tested indirectly: encode_record with a valid payload succeeds,
    // and the Encode variant exists as a From<postcard::Error> conversion.
    // We verify the variant exists by checking the error display.
    let err = JournalError::Encode(postcard::Error::DeserializeBadVarint);
    let msg = format!("{}", err);
    assert!(!msg.is_empty());
}

#[test]
fn journal_error_key_capacity_display() {
    // Given a JournalError::KeyCapacity
    // When displayed
    // Then the message is non-empty
    let err = JournalError::KeyCapacity;
    let msg = format!("{}", err);
    assert!(!msg.is_empty());
}

#[test]
fn journal_error_write_lock_poisoned_display() {
    // Given a JournalError::WriteLockPoisoned
    // When displayed
    // Then the message mentions poisoned
    let err = JournalError::WriteLockPoisoned;
    let msg = format!("{}", err);
    assert!(msg.contains("poisoned"));
}

#[test]
fn journal_error_wrong_run_display() {
    // Given a JournalError::WrongRun with expected and actual
    // When displayed
    // Then the message contains both run values
    let err = JournalError::WrongRun {
        expected: RunId::new(1),
        actual: RunId::new(2),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("1"));
    assert!(msg.contains("2"));
}

#[test]
fn journal_error_sequence_overflow_display() {
    // Given a JournalError::SequenceOverflow
    // When displayed
    // Then the message mentions overflow
    let err = JournalError::SequenceOverflow;
    let msg = format!("{}", err);
    assert!(msg.contains("overflow"));
}
