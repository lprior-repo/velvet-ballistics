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
mod tests {
    use super::*;
    use crate::{
        BlobRecord, CompiledIrRecord, JournalEvent, RecordKind, WorkflowSourceRecord,
        constants::*,
        events::JournalEvent as _,
        types::EventSeq,
    };
    use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};

    // =========================================================================
    // Round-trip encode/decode
    // =========================================================================

    #[test]
    fn encode_decode_roundtrip_journal_event_run_accepted() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(42),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xAA; DIGEST_BYTES]),
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        let Ok(bytes) = encoded else {
            panic!("encode_record should succeed for RunAccepted");
        };
        let decoded = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        let Ok((envelope, decoded_event)) = decoded else {
            panic!("decode_record should succeed for valid encoded RunAccepted");
        };
        assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(envelope.record_kind, RecordKind::RunAccepted.id());
        assert_eq!(envelope.sequence, 0);
        assert_eq!(decoded_event, event);
    }

    #[test]
    fn encode_decode_roundtrip_journal_event_step_started() {
        let event = JournalEvent::StepStarted {
            run: RunId::new(100),
            seq: EventSeq::new(1),
            step: StepIdx::new(5),
        };
        let Ok(bytes) = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::StepStarted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) else {
            panic!("encode should succeed");
        };
        let Ok((_, decoded)) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES) else {
            panic!("decode should succeed");
        };
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_journal_event_run_finished() {
        let event = JournalEvent::RunFinished {
            run: RunId::new(7),
            seq: EventSeq::new(99),
            result: SlotIdx::new(3),
        };
        let Ok(bytes) = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunFinished,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) else {
            panic!("encode should succeed");
        };
        let Ok((_, decoded)) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES) else {
            panic!("decode should succeed");
        };
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_journal_event_slot_written_with_value() {
        let slot_bytes = postcard::to_allocvec(&vb_core::SlotValue::Bool(true))
            .unwrap_or_default();
        let event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(10),
            seq: EventSeq::new(3),
            slot: SlotIdx::new(0),
            value: Some(slot_bytes),
        };
        let Ok(bytes) = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::SlotWritten,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) else {
            panic!("encode should succeed");
        };
        let Ok((_, decoded)) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES) else {
            panic!("decode should succeed");
        };
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_journal_event_run_cancelled() {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(55),
            seq: EventSeq::new(2),
        };
        let Ok(bytes) = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) else {
            panic!("encode should succeed");
        };
        let Ok((_, decoded)) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES) else {
            panic!("decode should succeed");
        };
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_journal_event_action_failed() {
        let event = JournalEvent::ActionFailedEvent {
            run: RunId::new(200),
            seq: EventSeq::new(15),
            step: StepIdx::new(2),
            action: vb_core::ActionId::new(7),
        };
        let Ok(bytes) = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionFailed,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) else {
            panic!("encode should succeed");
        };
        let Ok((_, decoded)) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES) else {
            panic!("decode should succeed");
        };
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_workflow_source_record() {
        let source = b"workflow: test".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord {
            digest,
            source,
        };
        let Ok(bytes) = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &record,
            MAX_WORKFLOW_SOURCE_BYTES,
        ) else {
            panic!("encode should succeed");
        };
        let Ok((envelope, decoded)) = decode_record::<WorkflowSourceRecord>(&bytes, MAGIC_WORKFLOW_SOURCE, MAX_WORKFLOW_SOURCE_BYTES) else {
            panic!("decode should succeed");
        };
        assert_eq!(envelope.magic, MAGIC_WORKFLOW_SOURCE);
        assert_eq!(envelope.record_kind, RecordKind::WorkflowSource.id());
        assert_eq!(decoded, record);
    }

    #[test]
    fn encode_decode_roundtrip_compiled_ir_record() {
        let ir = b"compiled-ir-bytes".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
        let record = CompiledIrRecord { digest, ir };
        let Ok(bytes) = encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            0,
            &record,
            MAX_COMPILED_IR_BYTES,
        ) else {
            panic!("encode should succeed");
        };
        let Ok((_, decoded)) = decode_record::<CompiledIrRecord>(&bytes, MAGIC_COMPILED_ARTIFACT, MAX_COMPILED_IR_BYTES) else {
            panic!("decode should succeed");
        };
        assert_eq!(decoded, record);
    }

    #[test]
    fn encode_decode_roundtrip_blob_record() {
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
        let record = BlobRecord {
            digest,
            bytes: payload,
        };
        let Ok(bytes) = encode_record(
            MAGIC_BLOB,
            RecordKind::Blob,
            0,
            &record,
            MAX_BLOB_BYTES,
        ) else {
            panic!("encode should succeed");
        };
        let Ok((_, decoded)) = decode_record::<BlobRecord>(&bytes, MAGIC_BLOB, MAX_BLOB_BYTES) else {
            panic!("decode should succeed");
        };
        assert_eq!(decoded, record);
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
    fn decode_rejects_wrong_magic() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };
        let Ok(bytes) = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) else {
            panic!("encode should succeed");
        };
        let result = decode_record::<JournalEvent>(&bytes, MAGIC_BLOB, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        let Err(JournalError::BadMagic { found }) = result else {
            panic!("wrong magic must yield BadMagic, got {:?}", result);
        };
        assert_eq!(found, MAGIC_JOURNAL_EVENT);
    }

    #[test]
    fn decode_rejects_corrupted_header_crc() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; DIGEST_BYTES]),
        };
        let Ok(mut bytes) = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) else {
            panic!("encode should succeed");
        };
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
    }

    #[test]
    fn decode_rejects_corrupted_payload() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([2; DIGEST_BYTES]),
        };
        let Ok(mut bytes) = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) else {
            panic!("encode should succeed");
        };
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
    }

    #[test]
    fn decode_rejects_truncated_payload_bytes() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([3; DIGEST_BYTES]),
        };
        let Ok(full) = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) else {
            panic!("encode should succeed");
        };
        // Keep only header + half the payload
        let truncated_len = RECORD_HEADER_BYTES + 1;
        let truncated = &full[..truncated_len];
        let result = decode_record::<JournalEvent>(truncated, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "truncated payload must yield UnexpectedEof, got {:?}",
            result
        );
    }

    // =========================================================================
    // Payload size limits
    // =========================================================================

    #[test]
    fn encode_rejects_payload_exceeding_max() {
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
        let Err(JournalError::PayloadTooLarge { len, max }) = result else {
            panic!("oversized payload must yield PayloadTooLarge, got {:?}", result);
        };
        assert_eq!(max, 10);
        assert!(len > 10, "reported length should exceed max");
    }

    #[test]
    fn encode_accepts_payload_at_exact_max_boundary() {
        // Build a tiny serializable payload that fits exactly in a small max
        let event = JournalEvent::RunCancelled {
            run: RunId::new(0),
            seq: EventSeq::new(0),
        };
        // First encode to discover the actual payload size
        let Ok(probe) = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) else {
            panic!("probe encode should succeed");
        };
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
    }

    // =========================================================================
    // Header-only encode/decode round-trip
    // =========================================================================

    #[test]
    fn header_encode_decode_roundtrip() {
        let payload = b"test payload data";
        let Ok(header) = encode_record_header(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            42,
            payload,
            1024,
        ) else {
            panic!("encode_record_header should succeed");
        };
        assert_eq!(header.len(), RECORD_HEADER_BYTES);
        let Ok(decoded) = decode_record_header(&header, MAGIC_JOURNAL_EVENT, 1024) else {
            panic!("decode_record_header should succeed");
        };
        assert_eq!(decoded.magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(decoded.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(decoded.record_kind, RecordKind::RunAccepted.id());
        assert_eq!(decoded.sequence, 42);
        assert_eq!(decoded.header_len, RECORD_HEADER_LEN);
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
    fn encode_rejects_kind_family_mismatch() {
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
        let Err(JournalError::RecordKindFamilyMismatch { magic, kind }) = result else {
            panic!("kind family mismatch should be rejected, got {:?}", result);
        };
        assert_eq!(magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(kind, RecordKind::WorkflowSource.id());
    }

    // =========================================================================
    // Schema version validation (via header decode)
    // =========================================================================

    #[test]
    fn decode_rejects_future_schema_version() {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
        };
        let Ok(mut bytes) = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) else {
            panic!("encode should succeed");
        };
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
    }

    // =========================================================================
    // Event replay validation helpers
    // =========================================================================

    #[test]
    fn next_seq_increments_correctly() {
        let seq = EventSeq::new(5);
        let Ok(next) = next_seq(seq) else {
            panic!("next_seq should succeed");
        };
        assert_eq!(next.get(), 6);
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
    fn encoded_output_length_equals_header_plus_payload() {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
        };
        let Ok(bytes) = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) else {
            panic!("encode should succeed");
        };
        // The decoded header should report a payload_len that makes total = header + payload
        let Ok((envelope, _)) = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES) else {
            panic!("decode should succeed");
        };
        let _ = envelope; // used above in decode
        // Verify by decoding just the header
        let Ok(header) = decode_record_header(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES) else {
            panic!("header decode should succeed");
        };
        let expected_total = RECORD_HEADER_BYTES.saturating_add(usize::try_from(header.payload_len).unwrap_or(0));
        assert_eq!(bytes.len(), expected_total);
    }
}
