//! Record encoding and decoding functions.
//!
//! Provides the binary wire format for all storage records.
//! Each record is prefixed with a 60-byte header containing magic,
//! schema version, kind, lengths, sequence, digest, and CRC.

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{
    binary::{read_u16, read_u32, read_u64, write_digest, write_u16, write_u32, write_u64},
    constants::{
        CRC_OFFSET, CURRENT_SCHEMA_VERSION, DIGEST_BYTES, MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT,
        MAGIC_INDEX_RECORD, MAGIC_JOURNAL_EVENT, MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE,
        RECORD_HEADER_BYTES, RECORD_HEADER_LEN,
    },
    error::JournalError,
    records::RecordKind,
    types::{RecordEnvelope, RecordHeader},
};

/// Encodes a postcard payload behind the 60-byte storage envelope.
pub fn encode_record<T: Serialize>(
    magic: u32,
    kind: RecordKind,
    sequence: u64,
    payload: &T,
    max_payload_len: u32,
) -> Result<Vec<u8>, JournalError> {
    validate_kind_family(magic, kind.id())?;
    let payload_bytes = postcard::to_allocvec(payload)?;
    let payload_len = payload_len_u32(payload_bytes.len(), max_payload_len)?;
    encode_record_payload(magic, kind, sequence, &payload_bytes, payload_len)
}

/// Decodes and postcard-deserializes an enveloped record.
pub fn decode_record<T: DeserializeOwned>(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, T), JournalError> {
    let (envelope, payload) = decode_record_payload(bytes, expected_magic, max_payload_len)?;
    let value = postcard::from_bytes(payload).map_err(|_| JournalError::PostcardDecodeFailed)?;
    Ok((envelope, value))
}

/// Encodes only the 60-byte storage record header for an existing payload.
pub fn encode_record_header(
    magic: u32,
    kind: RecordKind,
    sequence: u64,
    payload: &[u8],
    max_payload_len: u32,
) -> Result<[u8; RECORD_HEADER_BYTES], JournalError> {
    validate_kind_family(magic, kind.id())?;
    let payload_len = payload_len_u32(payload.len(), max_payload_len)?;
    build_record_header(magic, kind, sequence, payload, payload_len)
}

/// Decodes and validates only the 60-byte storage record header.
pub fn decode_record_header(
    header: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<RecordHeader, JournalError> {
    let header = header
        .get(..RECORD_HEADER_BYTES)
        .ok_or(JournalError::UnexpectedEof)?;
    let decoded = decode_record_header_unchecked_len(header)?;
    if decoded.magic != expected_magic {
        return Err(JournalError::BadMagic {
            found: decoded.magic,
        });
    }
    validate_schema_version(decoded.schema_version)?;
    validate_known_kind(decoded.record_kind)?;
    validate_kind_family(decoded.magic, decoded.record_kind)?;
    if decoded.header_len != RECORD_HEADER_LEN {
        return Err(JournalError::HeaderLengthMismatch {
            found: decoded.header_len,
        });
    }
    if decoded.payload_len > max_payload_len {
        return Err(JournalError::PayloadTooLarge {
            len: decoded.payload_len,
            max: max_payload_len,
        });
    }
    if crc32c::crc32c(header_prefix_for_crc(header)?) != decoded.header_checksum {
        return Err(JournalError::HeaderChecksumMismatch);
    }
    Ok(decoded)
}

/// Verifies a payload against an expected BLAKE3 digest.
pub fn verify_digest_match(
    payload: &[u8],
    expected_digest: [u8; DIGEST_BYTES],
) -> Result<(), JournalError> {
    if blake3::hash(payload).as_bytes() == &expected_digest {
        Ok(())
    } else {
        Err(JournalError::PayloadDigestMismatch)
    }
}

fn payload_len_u32(len: usize, max: u32) -> Result<u32, JournalError> {
    let payload_len = u32::try_from(len).map_err(|_| JournalError::PayloadTooLarge {
        len: 4_294_967_295,
        max,
    })?;
    if payload_len > max {
        return Err(JournalError::PayloadTooLarge {
            len: payload_len,
            max,
        });
    }
    Ok(payload_len)
}

fn encode_record_payload(
    magic: u32,
    kind: RecordKind,
    sequence: u64,
    payload: &[u8],
    payload_len: u32,
) -> Result<Vec<u8>, JournalError> {
    let capacity =
        RECORD_HEADER_BYTES
            .checked_add(payload.len())
            .ok_or(JournalError::PayloadTooLarge {
                len: payload_len,
                max: 4_294_967_295,
            })?;
    let header = build_record_header(magic, kind, sequence, payload, payload_len)?;

    let mut encoded = Vec::with_capacity(capacity);
    encoded.extend_from_slice(&header);
    encoded.extend_from_slice(payload);
    Ok(encoded)
}

fn decode_record_payload(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, &[u8]), JournalError> {
    let header = decode_record_header(bytes, expected_magic, max_payload_len)?;
    let payload_start =
        usize::try_from(header.header_len).map_err(|_| JournalError::UnexpectedEof)?;
    let payload_len_usize =
        usize::try_from(header.payload_len).map_err(|_| JournalError::UnexpectedEof)?;
    let payload_end = payload_start
        .checked_add(payload_len_usize)
        .ok_or(JournalError::UnexpectedEof)?;
    let payload = bytes
        .get(payload_start..payload_end)
        .ok_or(JournalError::UnexpectedEof)?;
    verify_digest_match(payload, header.payload_digest)?;
    Ok((
        RecordEnvelope {
            magic: header.magic,
            schema_version: header.schema_version,
            record_kind: header.record_kind,
            sequence: header.sequence,
        },
        payload,
    ))
}

fn build_record_header(
    magic: u32,
    kind: RecordKind,
    sequence: u64,
    payload: &[u8],
    payload_len: u32,
) -> Result<[u8; RECORD_HEADER_BYTES], JournalError> {
    let mut header = [0_u8; RECORD_HEADER_BYTES];
    write_u32(&mut header, 0, magic)?;
    write_u16(&mut header, 4, CURRENT_SCHEMA_VERSION)?;
    write_u16(&mut header, 6, kind.id())?;
    write_u32(&mut header, 8, RECORD_HEADER_LEN)?;
    write_u32(&mut header, 12, payload_len)?;
    write_u64(&mut header, 16, sequence)?;
    write_digest(&mut header, blake3::hash(payload).as_bytes())?;
    let checksum = crc32c::crc32c(header_prefix_for_crc(&header)?);
    write_u32(&mut header, CRC_OFFSET, checksum)?;
    Ok(header)
}

fn decode_record_header_unchecked_len(header: &[u8]) -> Result<RecordHeader, JournalError> {
    Ok(RecordHeader {
        magic: read_u32(header, 0)?,
        schema_version: read_u16(header, 4)?,
        record_kind: read_u16(header, 6)?,
        header_len: read_u32(header, 8)?,
        payload_len: read_u32(header, 12)?,
        sequence: read_u64(header, 16)?,
        payload_digest: digest_from_header(header)?,
        header_checksum: read_u32(header, CRC_OFFSET)?,
    })
}

fn validate_schema_version(version: u16) -> Result<(), JournalError> {
    if version == CURRENT_SCHEMA_VERSION {
        Ok(())
    } else if version < CURRENT_SCHEMA_VERSION {
        Err(JournalError::MigrationRequired {
            from: version,
            to: CURRENT_SCHEMA_VERSION,
        })
    } else {
        Err(JournalError::UnsupportedSchemaVersion { version })
    }
}

fn validate_known_kind(kind: u16) -> Result<(), JournalError> {
    if matches!(kind, 1 | 2 | 3 | 10..=23 | 30 | 40 | 50) {
        Ok(())
    } else {
        Err(JournalError::UnknownRecordKind { kind })
    }
}

fn validate_kind_family(magic: u32, kind: u16) -> Result<(), JournalError> {
    let valid = match magic {
        MAGIC_WORKFLOW_SOURCE => kind == RecordKind::WorkflowSource.id(),
        MAGIC_COMPILED_ARTIFACT => kind == RecordKind::CompiledIr.id(),
        MAGIC_JOURNAL_EVENT => matches!(kind, 10..=23),
        MAGIC_SNAPSHOT => kind == RecordKind::Snapshot.id(),
        MAGIC_BLOB => kind == RecordKind::Blob.id(),
        MAGIC_INDEX_RECORD => matches!(kind, 3 | 50),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(JournalError::RecordKindFamilyMismatch { magic, kind })
    }
}

fn header_prefix_for_crc(header: &[u8]) -> Result<&[u8], JournalError> {
    header.get(..CRC_OFFSET).ok_or(JournalError::UnexpectedEof)
}

fn digest_from_header(header: &[u8]) -> Result<[u8; DIGEST_BYTES], JournalError> {
    let digest = header
        .get(24..CRC_OFFSET)
        .ok_or(JournalError::UnexpectedEof)?;
    <[u8; DIGEST_BYTES]>::try_from(digest).map_err(|_| JournalError::UnexpectedEof)
}

pub(crate) fn next_seq(
    seq: crate::types::EventSeq,
) -> Result<crate::types::EventSeq, JournalError> {
    seq.get()
        .checked_add(1)
        .map(crate::types::EventSeq::new)
        .ok_or(JournalError::SequenceOverflow)
}

pub(crate) fn validate_replayed_event(
    run: vb_core::RunId,
    expected: crate::types::EventSeq,
    event: &crate::events::JournalEvent,
) -> Result<(), JournalError> {
    if event.run_id() != run {
        return Err(JournalError::WrongRun {
            expected: run,
            actual: event.run_id(),
        });
    }
    if event.seq() != expected {
        return Err(JournalError::SequenceGap {
            expected,
            actual: event.seq(),
        });
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
)]
mod tests {
    use super::*;
    use crate::{
        BlobRecord, CompiledIrRecord, JournalEvent, RecordKind, WorkflowSourceRecord,
        constants::*,
        types::EventSeq,
    };
    use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};

    // =========================================================================
    // Round-trip encode/decode
    // =========================================================================

    #[test]
    fn encode_decode_roundtrip_journal_event_run_accepted() -> Result<(), JournalError> {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(42),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xAA; DIGEST_BYTES]),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (envelope, decoded_event) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(envelope.record_kind, RecordKind::RunAccepted.id());
        assert_eq!(envelope.sequence, 0);
        assert_eq!(decoded_event, event);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_journal_event_step_started() -> Result<(), JournalError> {
        let event = JournalEvent::StepStarted {
            run: RunId::new(100),
            seq: EventSeq::new(1),
            step: StepIdx::new(5),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::StepStarted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_journal_event_run_finished() -> Result<(), JournalError> {
        let event = JournalEvent::RunFinished {
            run: RunId::new(7),
            seq: EventSeq::new(99),
            result: SlotIdx::new(3),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunFinished,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_journal_event_slot_written_with_value() -> Result<(), JournalError> {
        let slot_bytes = postcard::to_allocvec(&vb_core::SlotValue::Bool(true))?;
        let event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(10),
            seq: EventSeq::new(3),
            slot: SlotIdx::new(0),
            value: Some(slot_bytes),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::SlotWritten,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_journal_event_run_cancelled() -> Result<(), JournalError> {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(55),
            seq: EventSeq::new(2),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_journal_event_action_failed() -> Result<(), JournalError> {
        let event = JournalEvent::ActionFailedEvent {
            run: RunId::new(200),
            seq: EventSeq::new(15),
            step: StepIdx::new(2),
            action: vb_core::ActionId::new(7),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionFailed,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_workflow_source_record() -> Result<(), JournalError> {
        let source = b"workflow: test".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord {
            digest,
            source,
        };
        let bytes = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &record,
            MAX_WORKFLOW_SOURCE_BYTES,
        )?;
        let (envelope, decoded) = decode_record::<WorkflowSourceRecord>(&bytes, MAGIC_WORKFLOW_SOURCE, MAX_WORKFLOW_SOURCE_BYTES)?;
        assert_eq!(envelope.magic, MAGIC_WORKFLOW_SOURCE);
        assert_eq!(envelope.record_kind, RecordKind::WorkflowSource.id());
        assert_eq!(decoded, record);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_compiled_ir_record() -> Result<(), JournalError> {
        let ir = b"compiled-ir-bytes".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
        let record = CompiledIrRecord { digest, ir };
        let bytes = encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            0,
            &record,
            MAX_COMPILED_IR_BYTES,
        )?;
        let (_, decoded) = decode_record::<CompiledIrRecord>(&bytes, MAGIC_COMPILED_ARTIFACT, MAX_COMPILED_IR_BYTES)?;
        assert_eq!(decoded, record);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_blob_record() -> Result<(), JournalError> {
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
        let record = BlobRecord {
            digest,
            bytes: payload,
        };
        let bytes = encode_record(
            MAGIC_BLOB,
            RecordKind::Blob,
            0,
            &record,
            MAX_BLOB_BYTES,
        )?;
        let (_, decoded) = decode_record::<BlobRecord>(&bytes, MAGIC_BLOB, MAX_BLOB_BYTES)?;
        assert_eq!(decoded, record);
        Ok(())
    }

    // =========================================================================
    // Corrupt input rejection
    // =========================================================================

    #[test]
    fn decode_rejects_empty_input() {
        let result = decode_record::<JournalEvent>(&[], MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "empty input must yield UnexpectedEof, got {:?}",
            result
        );
    }

    #[test]
    fn decode_rejects_input_shorter_than_header() {
        let short = [0u8; RECORD_HEADER_BYTES - 1];
        let result = decode_record::<JournalEvent>(&short, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "input shorter than 60-byte header must yield UnexpectedEof, got {:?}",
            result
        );
    }

    #[test]
    fn decode_rejects_wrong_magic() -> Result<(), JournalError> {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let result = decode_record::<JournalEvent>(&bytes, MAGIC_BLOB, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::BadMagic { .. })),
            "wrong magic must yield BadMagic, got {result:?}"
        );
        if let Err(JournalError::BadMagic { found }) = result {
            assert_eq!(found, MAGIC_JOURNAL_EVENT);
        }
        Ok(())
    }

    #[test]
    fn decode_rejects_corrupted_header_crc() -> Result<(), JournalError> {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; DIGEST_BYTES]),
        };
        let mut bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        // Flip a byte in the CRC field at offset CRC_OFFSET
        if let Some(byte) = bytes.get_mut(CRC_OFFSET) {
            *byte = byte.wrapping_add(1);
        }
        let result = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::HeaderChecksumMismatch)),
            "corrupt CRC must yield HeaderChecksumMismatch, got {:?}",
            result
        );
        Ok(())
    }

    #[test]
    fn decode_rejects_corrupted_payload() -> Result<(), JournalError> {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([2; DIGEST_BYTES]),
        };
        let mut bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        // Flip a byte in the payload (after header)
        if let Some(byte) = bytes.get_mut(RECORD_HEADER_BYTES) {
            *byte = byte.wrapping_add(1);
        }
        let result = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "corrupt payload must yield PayloadDigestMismatch, got {:?}",
            result
        );
        Ok(())
    }

    #[test]
    fn decode_rejects_truncated_payload_bytes() -> Result<(), JournalError> {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([3; DIGEST_BYTES]),
        };
        let full = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        // Keep only header + half the payload
        let truncated_len = RECORD_HEADER_BYTES + 1;
        let truncated = &full[..truncated_len];
        let result = decode_record::<JournalEvent>(truncated, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "truncated payload must yield UnexpectedEof, got {:?}",
            result
        );
        Ok(())
    }

    // =========================================================================
    // Payload size limits
    // =========================================================================

    #[test]
    fn encode_rejects_payload_exceeding_max() -> Result<(), JournalError> {
        let large_source = vec![0xFF; 200];
        let record = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
            source: large_source,
        };
        let result = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &record,
            10,
        );
        assert!(
            matches!(result, Err(JournalError::PayloadTooLarge { .. })),
            "oversized payload must yield PayloadTooLarge, got {result:?}"
        );
        if let Err(JournalError::PayloadTooLarge { len, max }) = result {
            assert_eq!(max, 10);
            assert!(len > 10, "reported length should exceed max");
        }
        Ok(())
    }

    #[test]
    fn encode_accepts_payload_at_exact_max_boundary() -> Result<(), JournalError> {
        // Build a tiny serializable payload that fits exactly in a small max
        let event = JournalEvent::RunCancelled {
            run: RunId::new(0),
            seq: EventSeq::new(0),
        };
        // First encode to discover the actual payload size
        let probe = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let payload_len = probe.len().saturating_sub(RECORD_HEADER_BYTES);
        let max_len = u32::try_from(payload_len).unwrap_or(u32::MAX);
        // Now encode again with exact max
        let result = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            max_len,
        );
        assert!(result.is_ok(), "payload at exact max boundary should be accepted");
        Ok(())
    }

    // =========================================================================
    // Header-only encode/decode round-trip
    // =========================================================================

    #[test]
    fn header_encode_decode_roundtrip() -> Result<(), JournalError> {
        let payload = b"test payload data";
        let header = encode_record_header(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            42,
            payload,
            1024,
        )?;
        assert_eq!(header.len(), RECORD_HEADER_BYTES);
        let decoded = decode_record_header(&header, MAGIC_JOURNAL_EVENT, 1024)?;
        assert_eq!(decoded.magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(decoded.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(decoded.record_kind, RecordKind::RunAccepted.id());
        assert_eq!(decoded.sequence, 42);
        assert_eq!(decoded.header_len, RECORD_HEADER_LEN);
        Ok(())
    }

    // =========================================================================
    // Digest verification
    // =========================================================================

    #[test]
    fn verify_digest_match_accepts_correct_digest() {
        let payload = b"hello world";
        let digest: [u8; DIGEST_BYTES] = blake3::hash(payload).into();
        let result = verify_digest_match(payload, digest);
        assert!(result.is_ok(), "correct digest should pass verification");
    }

    #[test]
    fn verify_digest_match_rejects_wrong_digest() {
        let payload = b"hello world";
        let wrong_digest: [u8; DIGEST_BYTES] = blake3::hash(b"something else").into();
        let result = verify_digest_match(payload, wrong_digest);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "wrong digest must yield PayloadDigestMismatch, got {:?}",
            result
        );
    }

    // =========================================================================
    // Kind-family validation
    // =========================================================================

    #[test]
    fn encode_rejects_kind_family_mismatch() -> Result<(), JournalError> {
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
            "kind family mismatch should be rejected, got {result:?}"
        );
        if let Err(JournalError::RecordKindFamilyMismatch { magic, kind }) = result {
            assert_eq!(magic, MAGIC_JOURNAL_EVENT);
            assert_eq!(kind, RecordKind::WorkflowSource.id());
        }
        Ok(())
    }

    // =========================================================================
    // Schema version validation (via header decode)
    // =========================================================================

    #[test]
    fn decode_rejects_future_schema_version() -> Result<(), JournalError> {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
        };
        let mut bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        // Write a future schema version at offset 4 (u16 LE)
        let future_version = (CURRENT_SCHEMA_VERSION).saturating_add(1);
        let version_bytes = future_version.to_le_bytes();
        if let Some(slice) = bytes.get_mut(4..6) {
            slice.copy_from_slice(&version_bytes);
        }
        // Recompute CRC after modifying header
        let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
        let crc_bytes = checksum.to_le_bytes();
        if let Some(slice) = bytes.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
            slice.copy_from_slice(&crc_bytes);
        }
        let result = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::UnsupportedSchemaVersion { .. })),
            "future schema must yield UnsupportedSchemaVersion, got {:?}",
            result
        );
        Ok(())
    }

    // =========================================================================
    // Event replay validation helpers
    // =========================================================================

    #[test]
    fn next_seq_increments_correctly() -> Result<(), JournalError> {
        let seq = EventSeq::new(5);
        let next = next_seq(seq)?;
        assert_eq!(next.get(), 6);
        Ok(())
    }

    #[test]
    fn next_seq_rejects_overflow() {
        let seq = EventSeq::new(u64::MAX);
        let result = next_seq(seq);
        assert!(
            matches!(result, Err(JournalError::SequenceOverflow)),
            "overflow must yield SequenceOverflow, got {:?}",
            result
        );
    }

    #[test]
    fn validate_replayed_event_accepts_matching_run_and_seq() {
        let run = RunId::new(42);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };
        let result = validate_replayed_event(run, EventSeq::new(0), &event);
        assert!(result.is_ok(), "matching run and seq should pass validation");
    }

    #[test]
    fn validate_replayed_event_rejects_wrong_run() {
        let run = RunId::new(42);
        let other_run = RunId::new(99);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };
        let result = validate_replayed_event(other_run, EventSeq::new(0), &event);
        assert!(
            matches!(result, Err(JournalError::WrongRun { .. })),
            "wrong run must yield WrongRun, got {:?}",
            result
        );
    }

    #[test]
    fn validate_replayed_event_rejects_sequence_gap() {
        let run = RunId::new(42);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(5),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };
        let result = validate_replayed_event(run, EventSeq::new(3), &event);
        assert!(
            matches!(result, Err(JournalError::SequenceGap { .. })),
            "sequence gap must yield SequenceGap, got {:?}",
            result
        );
    }

    // =========================================================================
    // Encoded output structure invariants
    // =========================================================================

    #[test]
    fn encoded_output_length_equals_header_plus_payload() -> Result<(), JournalError> {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        // The decoded header should report a payload_len that makes total = header + payload
        let (envelope, _) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        let _ = envelope; // used above in decode
        // Verify by decoding just the header
        let header = decode_record_header(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        let expected_total = RECORD_HEADER_BYTES.saturating_add(usize::try_from(header.payload_len).unwrap_or(0));
        assert_eq!(bytes.len(), expected_total);
        Ok(())
    }

    // =========================================================================
    // Additional event variant roundtrips
    // =========================================================================

    #[test]
    fn encode_decode_roundtrip_step_succeeded() -> Result<(), JournalError> {
        let event = JournalEvent::StepSucceeded {
            run: RunId::new(10),
            seq: EventSeq::new(2),
            step: StepIdx::new(1),
            output: SlotIdx::new(0),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_action_scheduled() -> Result<(), JournalError> {
        let event = JournalEvent::ActionScheduled {
            run: RunId::new(20),
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            action: vb_core::ActionId::new(5),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionScheduled,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_action_completed() -> Result<(), JournalError> {
        let event = JournalEvent::ActionCompletedEvent {
            run: RunId::new(30),
            seq: EventSeq::new(4),
            step: StepIdx::new(1),
            action: vb_core::ActionId::new(5),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionCompleted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_wait_scheduled() -> Result<(), JournalError> {
        let event = JournalEvent::WaitScheduledEvent {
            run: RunId::new(40),
            seq: EventSeq::new(5),
            step: StepIdx::new(2),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::WaitScheduled,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_ask_scheduled() -> Result<(), JournalError> {
        let event = JournalEvent::AskScheduledEvent {
            run: RunId::new(50),
            seq: EventSeq::new(6),
            step: StepIdx::new(3),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::AskScheduled,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_ask_answered() -> Result<(), JournalError> {
        let event = JournalEvent::AskAnsweredEvent {
            run: RunId::new(60),
            seq: EventSeq::new(7),
            step: StepIdx::new(4),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::AskAnswered,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_retry_scheduled() -> Result<(), JournalError> {
        let event = JournalEvent::RetryScheduledEvent {
            run: RunId::new(70),
            seq: EventSeq::new(8),
            step: StepIdx::new(5),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RetryScheduled,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_run_failed() -> Result<(), JournalError> {
        let event = JournalEvent::RunFailedEvent {
            run: RunId::new(80),
            seq: EventSeq::new(9),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunFailed,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_slot_written_with_none_value() -> Result<(), JournalError> {
        let event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(90),
            seq: EventSeq::new(10),
            slot: SlotIdx::new(2),
            value: None,
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::SlotWritten,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event);
        Ok(())
    }

    #[test]
    fn encode_decode_roundtrip_run_header_record() -> Result<(), JournalError> {
        let record = crate::records::RunHeaderRecord {
            run: RunId::new(42),
            workflow_id: vb_core::WorkflowId::new(7),
            compiled_digest: WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]),
            status: 2,
            accepted_at_ms: 1_700_000_000,
        };
        let bytes = encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::RunHeader,
            record.run.get(),
            &record,
            MAX_RUN_HEADER_BYTES,
        )?;
        let (envelope, decoded) = decode_record::<crate::records::RunHeaderRecord>(
            &bytes,
            MAGIC_INDEX_RECORD,
            MAX_RUN_HEADER_BYTES,
        )?;
        assert_eq!(envelope.magic, MAGIC_INDEX_RECORD);
        assert_eq!(envelope.record_kind, RecordKind::RunHeader.id());
        assert_eq!(decoded, record);
        Ok(())
    }

    // =========================================================================
    // Edge case: empty payload roundtrip
    // =========================================================================

    #[test]
    fn encode_decode_roundtrip_empty_blob_payload() -> Result<(), JournalError> {
        let empty_bytes: Vec<u8> = vec![];
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&empty_bytes).into();
        let record = crate::records::BlobRecord {
            digest,
            bytes: empty_bytes,
        };
        let bytes = encode_record(
            MAGIC_BLOB,
            RecordKind::Blob,
            0,
            &record,
            MAX_BLOB_BYTES,
        )?;
        let (_, decoded) = decode_record::<crate::records::BlobRecord>(&bytes, MAGIC_BLOB, MAX_BLOB_BYTES)?;
        assert_eq!(decoded.bytes.len(), 0, "empty payload should roundtrip as empty");
        assert_eq!(decoded, record);
        Ok(())
    }

    // =========================================================================
    // Edge case: large sequence numbers
    // =========================================================================

    #[test]
    fn encode_decode_with_max_sequence() -> Result<(), JournalError> {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(u64::MAX),
            seq: EventSeq::new(u64::MAX),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            u64::MAX,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (envelope, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(envelope.sequence, u64::MAX);
        assert_eq!(decoded, event);
        Ok(())
    }

    // =========================================================================
    // Header decode edge cases
    // =========================================================================

    #[test]
    fn decode_header_rejects_unknown_record_kind() -> Result<(), JournalError> {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
        };
        let mut bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        // Overwrite the kind field at offset 6 with an invalid value
        let invalid_kind: u16 = 999;
        let kind_bytes = invalid_kind.to_le_bytes();
        if let Some(slice) = bytes.get_mut(6..8) {
            slice.copy_from_slice(&kind_bytes);
        }
        // Recompute CRC after modifying header
        let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
        let crc_bytes = checksum.to_le_bytes();
        if let Some(slice) = bytes.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
            slice.copy_from_slice(&crc_bytes);
        }
        let result = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::UnknownRecordKind { .. })),
            "unknown kind must yield UnknownRecordKind, got {:?}",
            result
        );
        Ok(())
    }

    #[test]
    fn decode_header_rejects_header_length_mismatch() -> Result<(), JournalError> {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
        };
        let mut bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        // Overwrite the header_len field at offset 8 with a wrong value
        let wrong_len: u32 = 99;
        let len_bytes = wrong_len.to_le_bytes();
        if let Some(slice) = bytes.get_mut(8..12) {
            slice.copy_from_slice(&len_bytes);
        }
        // Recompute CRC after modifying header
        let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
        let crc_bytes = checksum.to_le_bytes();
        if let Some(slice) = bytes.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
            slice.copy_from_slice(&crc_bytes);
        }
        let result = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::HeaderLengthMismatch { .. })),
            "wrong header len must yield HeaderLengthMismatch, got {:?}",
            result
        );
        Ok(())
    }

    #[test]
    fn decode_header_rejects_payload_exceeding_max() -> Result<(), JournalError> {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        // Decode with a smaller max_payload_len to trigger rejection
        let result = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, 1);
        assert!(
            matches!(result, Err(JournalError::PayloadTooLarge { .. })),
            "payload exceeding max must yield PayloadTooLarge, got {:?}",
            result
        );
        Ok(())
    }

    // =========================================================================
    // All magic bytes are distinct and well-formed
    // =========================================================================

    #[test]
    fn all_magic_constants_are_distinct() {
        let magics = [
            MAGIC_WORKFLOW_SOURCE,
            MAGIC_COMPILED_ARTIFACT,
            MAGIC_JOURNAL_EVENT,
            MAGIC_SNAPSHOT,
            MAGIC_BLOB,
            MAGIC_INDEX_RECORD,
        ];
        for (i, &a) in magics.iter().enumerate() {
            for (j, &b) in magics.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "magic at index {i} must differ from magic at index {j}");
                }
            }
        }
    }

    #[test]
    fn magic_bytes_match_ascii_sentinels() {
        // VBSR = 0x56425352
        assert_eq!(MAGIC_WORKFLOW_SOURCE, 0x5642_5352);
        // VBIR = 0x56424952
        assert_eq!(MAGIC_COMPILED_ARTIFACT, 0x5642_4952);
        // VBJE = 0x56424A45
        assert_eq!(MAGIC_JOURNAL_EVENT, 0x5642_4A45);
        // VBSN = 0x5642534E
        assert_eq!(MAGIC_SNAPSHOT, 0x5642_534E);
        // VBBL = 0x5642424C
        assert_eq!(MAGIC_BLOB, 0x5642_424C);
        // VBIX = 0x56424958
        assert_eq!(MAGIC_INDEX_RECORD, 0x5642_4958);
    }

    // =========================================================================
    // RecordKind::id() round-trip for all variants
    // =========================================================================

    #[test]
    fn record_kind_ids_are_distinct() {
        let kinds = [
            RecordKind::WorkflowSource,
            RecordKind::CompiledIr,
            RecordKind::RunHeader,
            RecordKind::RunAccepted,
            RecordKind::StepStarted,
            RecordKind::SlotWritten,
            RecordKind::ActionScheduled,
            RecordKind::ActionCompleted,
            RecordKind::ActionFailed,
            RecordKind::WaitScheduled,
            RecordKind::AskScheduled,
            RecordKind::AskAnswered,
            RecordKind::RetryScheduled,
            RecordKind::StepFailed,
            RecordKind::RunCancelled,
            RecordKind::RunFinished,
            RecordKind::RunFailed,
            RecordKind::Snapshot,
            RecordKind::Blob,
            RecordKind::IndexUpdate,
        ];
        let mut seen = std::collections::HashSet::new();
        for kind in &kinds {
            let id = kind.id();
            assert!(
                seen.insert(id),
                "RecordKind::{kind:?} produced duplicate id {id}"
            );
        }
        assert_eq!(seen.len(), kinds.len(), "all kind ids must be unique");
    }

    #[test]
    fn record_kind_ids_match_discriminant_values() {
        assert_eq!(RecordKind::WorkflowSource.id(), 1);
        assert_eq!(RecordKind::CompiledIr.id(), 2);
        assert_eq!(RecordKind::RunHeader.id(), 3);
        assert_eq!(RecordKind::RunAccepted.id(), 10);
        assert_eq!(RecordKind::StepStarted.id(), 11);
        assert_eq!(RecordKind::SlotWritten.id(), 12);
        assert_eq!(RecordKind::ActionScheduled.id(), 13);
        assert_eq!(RecordKind::ActionCompleted.id(), 14);
        assert_eq!(RecordKind::ActionFailed.id(), 15);
        assert_eq!(RecordKind::WaitScheduled.id(), 16);
        assert_eq!(RecordKind::AskScheduled.id(), 17);
        assert_eq!(RecordKind::AskAnswered.id(), 18);
        assert_eq!(RecordKind::RetryScheduled.id(), 19);
        assert_eq!(RecordKind::StepFailed.id(), 20);
        assert_eq!(RecordKind::RunCancelled.id(), 21);
        assert_eq!(RecordKind::RunFinished.id(), 22);
        assert_eq!(RecordKind::RunFailed.id(), 23);
        assert_eq!(RecordKind::Snapshot.id(), 30);
        assert_eq!(RecordKind::Blob.id(), 40);
        assert_eq!(RecordKind::IndexUpdate.id(), 50);
    }

    // =========================================================================
    // Kind-family validation: each magic rejects wrong kind
    // =========================================================================

    #[test]
    fn encode_rejects_compiled_ir_kind_with_workflow_source_magic() {
        let record = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
            source: vec![1],
        };
        let result = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::CompiledIr,
            0,
            &record,
            128,
        );
        assert!(
            matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
            "CompiledIr kind with MAGIC_WORKFLOW_SOURCE must fail, got {result:?}"
        );
    }

    #[test]
    fn encode_rejects_workflow_source_kind_with_compiled_ir_magic() {
        let record = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
            source: vec![1],
        };
        let result = encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::WorkflowSource,
            0,
            &record,
            128,
        );
        assert!(
            matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
            "WorkflowSource kind with MAGIC_COMPILED_ARTIFACT must fail, got {result:?}"
        );
    }

    #[test]
    fn encode_rejects_snapshot_kind_with_blob_magic() {
        let record = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
            source: vec![1],
        };
        let result = encode_record(
            MAGIC_BLOB,
            RecordKind::Snapshot,
            0,
            &record,
            128,
        );
        assert!(
            matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
            "Snapshot kind with MAGIC_BLOB must fail, got {result:?}"
        );
    }

    #[test]
    fn encode_rejects_blob_kind_with_journal_event_magic() {
        let record = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
            source: vec![1],
        };
        let result = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::Blob,
            0,
            &record,
            128,
        );
        assert!(
            matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
            "Blob kind with MAGIC_JOURNAL_EVENT must fail, got {result:?}"
        );
    }

    #[test]
    fn encode_rejects_run_header_kind_with_snapshot_magic() {
        let record = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
            source: vec![1],
        };
        let result = encode_record(
            MAGIC_SNAPSHOT,
            RecordKind::RunHeader,
            0,
            &record,
            128,
        );
        assert!(
            matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
            "RunHeader kind with MAGIC_SNAPSHOT must fail, got {result:?}"
        );
    }

    // =========================================================================
    // Payload boundary: one byte over max is rejected
    // =========================================================================

    #[test]
    fn encode_rejects_payload_one_byte_over_max() -> Result<(), JournalError> {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(0),
            seq: EventSeq::new(0),
        };
        // Discover actual payload size
        let probe = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let payload_len = probe.len().saturating_sub(RECORD_HEADER_BYTES);
        // max_len = actual size - 1 so the payload is exactly one byte too large
        let max_len = u32::try_from(payload_len.saturating_sub(1)).unwrap_or(u32::MAX);
        let result = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            max_len,
        );
        assert!(
            matches!(result, Err(JournalError::PayloadTooLarge { len, max }) if len == u32::try_from(payload_len).unwrap_or(u32::MAX) && max == max_len),
            "payload one byte over max must be rejected with exact sizes, got {result:?}"
        );
        Ok(())
    }

    // =========================================================================
    // Malformed input: extra trailing bytes after payload are ignored
    // =========================================================================

    #[test]
    fn decode_ignores_trailing_bytes_beyond_payload() -> Result<(), JournalError> {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
        };
        let mut bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        // Append garbage after the valid record
        bytes.push(0xFF);
        bytes.push(0xFE);
        bytes.push(0xFD);
        let (_, decoded) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event, "trailing bytes should be ignored on decode");
        Ok(())
    }

    // =========================================================================
    // Malformed input: header-only input with no payload bytes
    // =========================================================================

    #[test]
    fn decode_rejects_header_only_input_with_nonzero_payload_len() -> Result<(), JournalError> {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
        };
        let full = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        // Keep only the header portion
        let header_only = &full[..RECORD_HEADER_BYTES];
        let result = decode_record::<JournalEvent>(header_only, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        // The header declares a nonzero payload_len but no payload bytes follow
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "header-only with nonzero payload_len must yield UnexpectedEof, got {:?}",
            result
        );
        Ok(())
    }

    // =========================================================================
    // Malformed input: zero-byte payload declared but header digest mismatch
    // =========================================================================

    #[test]
    fn decode_rejects_mismatched_digest_in_header() -> Result<(), JournalError> {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
        };
        let mut bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        // Corrupt the digest bytes (offsets 24..56 are the 32-byte digest)
        // Modify one byte in the digest region
        let digest_offset = 24;
        if let Some(byte) = bytes.get_mut(digest_offset) {
            *byte = byte.wrapping_add(1);
        }
        // Recompute CRC so the header is internally consistent but digest is wrong
        let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
        let crc_bytes = checksum.to_le_bytes();
        if let Some(slice) = bytes.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
            slice.copy_from_slice(&crc_bytes);
        }
        let result = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "corrupted digest must yield PayloadDigestMismatch, got {:?}",
            result
        );
        Ok(())
    }

    // =========================================================================
    // Kind family: MAGIC_INDEX_RECORD accepts RunHeader (3) and IndexUpdate (50)
    // =========================================================================

    #[test]
    fn encode_accepts_run_header_kind_with_index_record_magic() -> Result<(), JournalError> {
        let record = crate::records::RunHeaderRecord {
            run: RunId::new(1),
            workflow_id: vb_core::WorkflowId::new(1),
            compiled_digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
            status: 0,
            accepted_at_ms: 100,
        };
        let result = encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::RunHeader,
            1,
            &record,
            MAX_RUN_HEADER_BYTES,
        );
        assert!(result.is_ok(), "RunHeader (kind 3) should be accepted by MAGIC_INDEX_RECORD");
        Ok(())
    }

    // =========================================================================
    // StepSucceeded event maps to SlotWritten record kind
    // =========================================================================

    #[test]
    fn step_succeeded_event_maps_to_slot_written_kind() {
        let event = JournalEvent::StepSucceeded {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        };
        assert_eq!(
            event.record_kind(),
            RecordKind::SlotWritten,
            "StepSucceeded event should map to SlotWritten record kind"
        );
    }

    // =========================================================================
    // Encode header with zero-length payload
    // =========================================================================

    #[test]
    fn encode_record_header_with_empty_payload() -> Result<(), JournalError> {
        let payload: &[u8] = &[];
        let header = encode_record_header(
            MAGIC_BLOB,
            RecordKind::Blob,
            0,
            payload,
            1024,
        )?;
        let decoded = decode_record_header(&header, MAGIC_BLOB, 1024)?;
        assert_eq!(decoded.payload_len, 0, "empty payload should report zero length");
        assert_eq!(decoded.magic, MAGIC_BLOB);
        Ok(())
    }

    // =========================================================================
    // Encode header round-trip preserves all fields
    // =========================================================================

    #[test]
    fn header_roundtrip_preserves_sequence_and_kind() -> Result<(), JournalError> {
        let payload = b"test";
        let sequence: u64 = 0xDEAD_BEEF_CAFE_BABE;
        let header = encode_record_header(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            sequence,
            payload,
            1024,
        )?;
        let decoded = decode_record_header(&header, MAGIC_WORKFLOW_SOURCE, 1024)?;
        assert_eq!(decoded.sequence, sequence, "sequence must survive round-trip");
        assert_eq!(decoded.record_kind, RecordKind::WorkflowSource.id());
        assert_eq!(decoded.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(decoded.header_len, RECORD_HEADER_LEN);
        Ok(())
    }

    // =========================================================================
    // Schema version: old version triggers migration required
    // =========================================================================

    #[test]
    fn decode_rejects_old_schema_version_with_migration_required() -> Result<(), JournalError> {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
        };
        let mut bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        // Write schema version 0 at offset 4 (u16 LE)
        let old_version: u16 = 0;
        let version_bytes = old_version.to_le_bytes();
        if let Some(slice) = bytes.get_mut(4..6) {
            slice.copy_from_slice(&version_bytes);
        }
        // Recompute CRC after modifying header
        let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
        let crc_bytes = checksum.to_le_bytes();
        if let Some(slice) = bytes.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
            slice.copy_from_slice(&crc_bytes);
        }
        let result = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::MigrationRequired { from, to }) if from == 0 && to == CURRENT_SCHEMA_VERSION),
            "old schema must yield MigrationRequired, got {:?}",
            result
        );
        Ok(())
    }
}
