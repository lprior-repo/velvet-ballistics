#![forbid(unsafe_code)]
//! SECTION 2.6: Record Envelope & Codec (BH-03, BH-07, BH-08, BH-09, BH-14, BH-15)

use crate::codec::{decode_record, encode_record};
use crate::constants::{
    CRC_OFFSET, MAGIC_BLOB, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    RECORD_HEADER_BYTES,
};
use crate::records::RecordKind;
use crate::{DIGEST_BYTES, EventSeq, JournalError, JournalEvent, SlotIdx};
use vb_core::RunId;

use crate::tests::fixtures::temp_journal;

/// TEST: decode_rejects_all_zero_bytes (BH-03)
///
/// Contract §6 BH-03: All-zero input returns error (not panic).
#[test]
fn decode_rejects_all_zero_bytes() -> Result<(), String> {
    let zeros = [0u8; RECORD_HEADER_BYTES + 64];
    let result = decode_record::<JournalEvent>(
        &zeros,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        result.is_err(),
        "all-zero bytes must be rejected, not panic"
    );
    Ok(())
}

/// TEST: decode_rejects_valid_header_with_corrupt_payload (BH-03)
///
/// Contract §6 BH-03: Tampered payload detected by BLAKE3.
#[test]
fn decode_rejects_valid_header_with_corrupt_payload() -> Result<(), String> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0x42; DIGEST_BYTES]),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map_err(|e| format!("encode failed: {e}"))?;

    let mut corrupt = bytes;
    for byte in corrupt.iter_mut().skip(RECORD_HEADER_BYTES) {
        *byte = byte.wrapping_add(1);
    }

    let result = decode_record::<JournalEvent>(
        &corrupt,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "corrupt payload must yield PayloadDigestMismatch"
    );
    Ok(())
}

/// TEST: event_seq_overflow_rejected (BH-04)
///
/// Contract §6 BH-04: EventSeq::MAX overflow rejected.
#[test]
fn event_seq_overflow_rejected() -> Result<(), String> {
    let seq = EventSeq::new(u64::MAX);
    let result = crate::codec::next_seq(seq);
    assert!(
        matches!(result, Err(JournalError::SequenceOverflow)),
        "u64::MAX + 1 must yield SequenceOverflow"
    );
    Ok(())
}

/// TEST: decode_rejects_header_only_when_payload_declared (BH-05)
///
/// Contract §6 BH-05: Header-only with declared payload returns UnexpectedEof.
#[test]
fn decode_rejects_header_only_when_payload_declared() -> Result<(), String> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let full = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map_err(|e| format!("encode failed: {e}"))?;

    let truncated = &full[..RECORD_HEADER_BYTES];
    let result = decode_record::<JournalEvent>(
        truncated,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "truncated record must yield UnexpectedEof"
    );
    Ok(())
}

/// TEST: decode_rejects_future_schema_version_in_full_record (BH-07)
///
/// Contract §6 BH-07: Future schema version → UnsupportedSchemaVersion.
#[test]
fn decode_rejects_future_schema_version_in_full_record() -> Result<(), String> {
    use crate::constants::CURRENT_SCHEMA_VERSION;

    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map_err(|e| format!("encode failed: {e}"))?;

    let future_version = CURRENT_SCHEMA_VERSION.saturating_add(1);
    bytes
        .get_mut(4..6)
        .ok_or_else(|| String::from("schema version field not found"))?
        .copy_from_slice(&future_version.to_le_bytes());

    // Recompute CRC
    let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
    bytes
        .get_mut(CRC_OFFSET..CRC_OFFSET + 4)
        .ok_or_else(|| String::from("CRC field not found"))?
        .copy_from_slice(&checksum.to_le_bytes());

    let result = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::UnsupportedSchemaVersion { .. })),
        "future schema version must yield UnsupportedSchemaVersion"
    );
    Ok(())
}

/// TEST: encode_rejects_kind_family_mismatch_workflow_in_journal (BH-08)
///
/// Contract §6 BH-08: Wrong magic for workflow record kind.
#[test]
fn encode_rejects_kind_family_mismatch_workflow_in_journal() -> Result<(), String> {
    use crate::WorkflowSourceRecord;
    use vb_core::WorkflowDigest;

    let record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        source: vec![1],
    };
    let result = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::WorkflowSource,
        0,
        &record,
        128,
    );
    assert!(
        matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
        "kind family mismatch must be rejected"
    );
    Ok(())
}

/// TEST: crc_single_bit_flip_detected (BH-09)
///
/// Contract §6 BH-09: Single-bit flip in header → HeaderChecksumMismatch.
#[test]
fn crc_single_bit_flip_detected() -> Result<(), String> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0xAA; DIGEST_BYTES]),
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map_err(|e| format!("encode failed: {e}"))?;

    if let Some(byte) = bytes.get_mut(CRC_OFFSET) {
        *byte ^= 0x01;
    }

    let result = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::HeaderChecksumMismatch)),
        "CRC bit flip must yield HeaderChecksumMismatch"
    );
    Ok(())
}

/// TEST: journal_event_respects_max_payload (BH-15)
///
/// Contract §6 BH-15: Payload exceeding max_payload_len → PayloadTooLarge.
#[test]
fn journal_event_respects_max_payload() -> Result<(), String> {
    let big_value = vec![0xFFu8; MAX_JOURNAL_EVENT_PAYLOAD_BYTES as usize + 1];
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        slot: SlotIdx::new(0),
        value: Some(big_value),
        extra: None,
        attempt: 1,
    };
    let result = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::SlotWritten,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::PayloadTooLarge { .. })),
        "oversized journal event must yield PayloadTooLarge"
    );
    Ok(())
}
