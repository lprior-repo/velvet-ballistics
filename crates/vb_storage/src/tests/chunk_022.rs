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
fn encode_decode_roundtrip_for_ask_scheduled_record() {
    // Given an AskScheduledEvent
    // When encoded and decoded
    // Then the event survives the roundtrip exactly
    let event = JournalEvent::AskScheduledEvent {
        run: RunId::new(11),
        seq: EventSeq::new(3),
        step: StepIdx::new(4),
        attempt: 1,
        deadline_ms: 30000,
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskScheduled,
        3,
        &event,
        128,
    )
    .expect("encoding should succeed");
    let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .expect("decoding should succeed");
    assert_eq!(decoded, event);
}

#[test]
fn encode_decode_roundtrip_for_ask_answered_record() {
    // Given an AskAnsweredEvent
    // When encoded and decoded
    // Then the event survives the roundtrip exactly
    let event = JournalEvent::AskAnsweredEvent {
        run: RunId::new(12),
        seq: EventSeq::new(4),
        step: StepIdx::new(5),
        attempt: 1,
    };
    let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::AskAnswered, 4, &event, 128)
        .expect("encoding should succeed");
    let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .expect("decoding should succeed");
    assert_eq!(decoded, event);
}

#[test]
fn encode_decode_roundtrip_for_retry_scheduled_record() {
    // Given a RetryScheduledEvent
    // When encoded and decoded
    // Then the event survives the roundtrip exactly
    let event = JournalEvent::RetryScheduledEvent {
        run: RunId::new(13),
        seq: EventSeq::new(5),
        step: StepIdx::new(6),
        attempt: 1,
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RetryScheduled,
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
fn encode_decode_roundtrip_for_run_cancelled_record() {
    // Given a RunCancelled event
    // When encoded and decoded
    // Then the event survives the roundtrip exactly
    let event = JournalEvent::RunCancelled {
        run: RunId::new(14),
        seq: EventSeq::new(6),
        attempt: 1,
        reason: None,
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        6,
        &event,
        128,
    )
    .expect("encoding should succeed");
    let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .expect("decoding should succeed");
    assert_eq!(decoded, event);
}

#[test]
fn adversarial_decode_wrong_magic_for_family_returns_bad_magic() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: test_digest(1),
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("ok");
    let result = decode_record::<JournalEvent>(&encoded, MAGIC_SNAPSHOT, 128);
    let Err(JournalError::BadMagic { found }) = result else {
        panic!("expected BadMagic, got {:?}", result)
    };
    assert_eq!(found, MAGIC_JOURNAL_EVENT);
}

#[test]
fn adversarial_decode_vbir_magic_on_journal_returns_bad_magic() {
    let record = CompiledIrRecord {
        digest: test_digest(1),
        ir: vec![1, 2, 3],
        ..Default::default()
    };
    let encoded = encode_record(
        MAGIC_COMPILED_ARTIFACT,
        RecordKind::CompiledIr,
        0,
        &record,
        MAX_COMPILED_IR_BYTES,
    )
    .expect("ok");
    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    let Err(JournalError::BadMagic { found }) = result else {
        panic!("expected BadMagic, got {:?}", result)
    };
    assert_eq!(found, MAGIC_COMPILED_ARTIFACT);
}

#[test]
fn adversarial_decode_unsupported_schema_version_returns_exact_version() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(2),
        seq: EventSeq::new(0),
        workflow: test_digest(2),
    };
    let encoded = encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &5u16.to_le_bytes());
    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    let Err(JournalError::UnsupportedSchemaVersion { version }) = result else {
        panic!("expected UnsupportedSchemaVersion, got {:?}", result)
    };
    assert_eq!(version, 5);
}

#[test]
fn adversarial_decode_unknown_record_kind_returns_exact_kind() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(3),
        seq: EventSeq::new(0),
        workflow: test_digest(3),
    };
    let encoded = encode_and_patch_field(&event, RecordKind::RunAccepted, 6, &99u16.to_le_bytes());
    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    let Err(JournalError::UnknownRecordKind { kind }) = result else {
        panic!("expected UnknownRecordKind, got {:?}", result)
    };
    assert_eq!(kind, 99);
}

#[test]
fn adversarial_decode_kind_family_mismatch_snapshot_kind_in_journal() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(4),
        seq: EventSeq::new(0),
        workflow: test_digest(4),
    };
    let encoded = encode_and_patch_field(&event, RecordKind::RunAccepted, 6, &30u16.to_le_bytes());
    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    let Err(JournalError::RecordKindFamilyMismatch { magic, kind }) = result else {
        panic!("expected mismatch, got {:?}", result)
    };
    assert_eq!(magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(kind, 30);
}

#[test]
fn adversarial_decode_kind_family_mismatch_blob_in_snapshot() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(5),
        seq: EventSeq::new(0),
        workflow: test_digest(5),
    };
    let result = encode_record(
        MAGIC_SNAPSHOT,
        RecordKind::Blob,
        event.seq().get(),
        &event,
        MAX_SNAPSHOT_BYTES,
    );
    let Err(JournalError::RecordKindFamilyMismatch { magic, kind }) = result else {
        panic!("expected mismatch, got {:?}", result)
    };
    assert_eq!(magic, MAGIC_SNAPSHOT);
    assert_eq!(kind, RecordKind::Blob.id());
}
