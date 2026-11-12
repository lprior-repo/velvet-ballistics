#[test]
fn encode_decode_roundtrip_workflow_source_record() -> Result<(), JournalError> {
    let source = b"workflow: test".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord { digest, source };
    let bytes = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        &record,
        MAX_WORKFLOW_SOURCE_BYTES,
    )?;
    let (envelope, decoded) = decode_record::<WorkflowSourceRecord>(
        &bytes,
        MAGIC_WORKFLOW_SOURCE,
        MAX_WORKFLOW_SOURCE_BYTES,
    )?;
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
    let (_, decoded) =
        decode_record::<CompiledIrRecord>(&bytes, MAGIC_COMPILED_ARTIFACT, MAX_COMPILED_IR_BYTES)?;
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
    let bytes = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, MAX_BLOB_BYTES)?;
    let (_, decoded) = decode_record::<BlobRecord>(&bytes, MAGIC_BLOB, MAX_BLOB_BYTES)?;
    assert_eq!(decoded, record);
    Ok(())
}

#[test]
fn decode_rejects_empty_input() {
    let result =
        decode_record::<JournalEvent>(&[], MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "empty input must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_rejects_input_shorter_than_header() {
    let short = [0u8; RECORD_HEADER_BYTES - 1];
    let result =
        decode_record::<JournalEvent>(&short, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
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
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
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
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
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
    let result = decode_record::<JournalEvent>(
        truncated,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "truncated payload must yield UnexpectedEof, got {:?}",
        result
    );
    Ok(())
}

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
        attempt: 1,
        reason: None,
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
    assert!(
        result.is_ok(),
        "payload at exact max boundary should be accepted"
    );
    Ok(())
}

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

#[test]
fn decode_rejects_future_schema_version() -> Result<(), JournalError> {
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
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::UnsupportedSchemaVersion { .. })),
        "future schema must yield UnsupportedSchemaVersion, got {:?}",
        result
    );
    Ok(())
}

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
    assert!(
        result.is_ok(),
        "matching run and seq should pass validation"
    );
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

#[test]
fn encoded_output_length_equals_header_plus_payload() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    // The decoded header should report a payload_len that makes total = header + payload
    let (envelope, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(envelope.record_kind, RecordKind::RunCancelled.id());
    assert_eq!(envelope.sequence, 0);
    assert_eq!(decoded, event);
    // Verify by decoding just the header
    let header =
        decode_record_header(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    let expected_total =
        RECORD_HEADER_BYTES.saturating_add(usize::try_from(header.payload_len).unwrap_or(0));
    assert_eq!(bytes.len(), expected_total);
    Ok(())
}
