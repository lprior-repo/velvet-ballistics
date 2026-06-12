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
fn decode_rejects_header_length_mismatch() {
    // Given a valid record whose declared header length is 99 (not 60)
    // When decode_record is called
    // Then it returns HeaderLengthMismatch with found=99
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    let mut encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encoding should succeed");
    // Header length is at offset 8..12, little-endian. Set to 99.
    let len_bytes = 99u32.to_le_bytes();
    encoded[8] = len_bytes[0];
    encoded[9] = len_bytes[1];
    encoded[10] = len_bytes[2];
    encoded[11] = len_bytes[3];
    // Recompute CRC32C
    let header_prefix = &encoded[..56];
    let checksum = crc32c::crc32c(header_prefix);
    encoded[56] = (checksum & 0xFF) as u8;
    encoded[57] = ((checksum >> 8) & 0xFF) as u8;
    encoded[58] = ((checksum >> 16) & 0xFF) as u8;
    encoded[59] = ((checksum >> 24) & 0xFF) as u8;

    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    assert!(matches!(
        result,
        Err(JournalError::HeaderLengthMismatch { found: 99 })
    ));
}

#[test]
fn decode_rejects_truncated_payload() {
    // Given an encoded record with bytes truncated after the header
    // When decode_record is called
    // Then it returns UnexpectedEof
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encoding should succeed");
    // Keep only the 60-byte header, discarding all payload bytes
    let truncated = &encoded[..60];

    let result = decode_record::<JournalEvent>(truncated, MAGIC_JOURNAL_EVENT, 128);
    assert!(matches!(result, Err(JournalError::UnexpectedEof)));
}

// --- Section 1: Error Variant Exact-Assertion Tests ---

#[test]
fn decode_record_returns_bad_magic_when_magic_differs() {
    // Given an encoded record
    // When decoded with a different expected magic
    // Then it returns BadMagic with the encoded magic value
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; 32]),
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        128,
    )
    .expect("encoding should succeed");

    let result = decode_record::<JournalEvent>(&encoded, MAGIC_WORKFLOW_SOURCE, 128);
    let Err(JournalError::BadMagic { found }) = result else {
        panic!("expected BadMagic, got {:?}", result);
    };
    assert_eq!(found, MAGIC_JOURNAL_EVENT);
}

#[test]
fn decode_record_returns_unexpected_eof_when_bytes_too_short() {
    // Given a zero-length byte slice
    // When decode_record is called
    // Then it returns UnexpectedEof
    let empty: [u8; 0] = [];

    let result = decode_record::<JournalEvent>(&empty, MAGIC_JOURNAL_EVENT, 128);
    assert!(matches!(result, Err(JournalError::UnexpectedEof)));
}

#[test]
fn encode_record_returns_payload_too_large_when_payload_exceeds_max() {
    // Given a source record with source bytes larger than the max
    // When encode_record is called with a tiny max_payload_len
    // Then it returns PayloadTooLarge with correct len and max fields
    let source = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([1; 32]),
        source: vec![0xAB; 200],
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
fn encode_record_returns_record_kind_family_mismatch_for_wrong_kind() {
    // Given a blob kind paired with workflow source magic
    // When encode_record is called
    // Then it returns RecordKindFamilyMismatch with the exact magic and kind
    let source = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([1; 32]),
        source: vec![1],
    };
    let result = encode_record(MAGIC_WORKFLOW_SOURCE, RecordKind::Blob, 0, &source, 128);
    let Err(JournalError::RecordKindFamilyMismatch { magic, kind }) = result else {
        panic!("expected RecordKindFamilyMismatch, got {:?}", result);
    };
    assert_eq!(magic, MAGIC_WORKFLOW_SOURCE);
    assert_eq!(kind, RecordKind::Blob.id());
}

#[test]
fn decode_record_returns_header_checksum_mismatch_on_corrupt_crc() {
    // Given an encoded record with a flipped CRC byte
    // When decode_record is called
    // Then it returns HeaderChecksumMismatch
    let event = JournalEvent::RunFinished {
        run: RunId::new(5),
        seq: EventSeq::new(1),
        result: vb_core::SlotIdx::new(0),
        attempt: 1,
    };
    let mut encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunFinished,
        event.seq().get(),
        &event,
        128,
    )
    .expect("encoding should succeed");
    // Corrupt the CRC at byte 56
    if let Some(byte) = encoded.get_mut(56) {
        *byte = byte.wrapping_add(1);
    }

    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    assert!(matches!(result, Err(JournalError::HeaderChecksumMismatch)));
}

#[test]
fn decode_record_returns_payload_digest_mismatch_on_corrupt_payload() {
    // Given an encoded record with a flipped payload byte
    // When decode_record is called
    // Then it returns PayloadDigestMismatch
    let event = JournalEvent::StepStarted {
        run: RunId::new(2),
        seq: EventSeq::new(0),
        step: StepIdx::new(3),
        attempt: 1,
    };
    let mut encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::StepStarted,
        event.seq().get(),
        &event,
        128,
    )
    .expect("encoding should succeed");
    // Corrupt the first payload byte (immediately after the 60-byte header)
    if let Some(byte) = encoded.get_mut(60) {
        *byte = byte.wrapping_add(1);
    }

    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    assert!(matches!(result, Err(JournalError::PayloadDigestMismatch)));
}
