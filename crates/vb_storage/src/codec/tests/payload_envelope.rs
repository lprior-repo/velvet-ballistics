#[test]
fn encode_rejects_workflow_source_payload_exceeding_max() -> Result<(), JournalError> {
    let large_source = vec![0u8; (MAX_WORKFLOW_SOURCE_BYTES as usize).saturating_add(1)];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&large_source).into());
    let record = WorkflowSourceRecord {
        digest,
        source: large_source,
    };
    let result = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        &record,
        MAX_WORKFLOW_SOURCE_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::PayloadTooLarge { .. })),
        "oversized workflow source must yield PayloadTooLarge, got {result:?}"
    );
    Ok(())
}

#[test]
fn encode_rejects_compiled_ir_payload_exceeding_max() -> Result<(), JournalError> {
    let large_ir = vec![0u8; (MAX_COMPILED_IR_BYTES as usize).saturating_add(1)];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&large_ir).into());
    let record = CompiledIrRecord {
        digest,
        ir: large_ir,
    };
    let result = encode_record(
        MAGIC_COMPILED_ARTIFACT,
        RecordKind::CompiledIr,
        0,
        &record,
        MAX_COMPILED_IR_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::PayloadTooLarge { .. })),
        "oversized compiled IR must yield PayloadTooLarge, got {result:?}"
    );
    Ok(())
}

#[test]
fn encode_with_empty_source_and_generous_max_succeeds() -> Result<(), JournalError> {
    let empty_source: Vec<u8> = vec![];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&empty_source).into());
    let record = WorkflowSourceRecord {
        digest,
        source: empty_source,
    };
    let result = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        &record,
        128,
    );
    assert!(
        result.is_ok(),
        "empty source with generous max should succeed, got {result:?}"
    );
    Ok(())
}

#[test]
fn encode_decode_roundtrip_run_snapshot_record() -> Result<(), JournalError> {
    let snapshot = crate::recovery::RunSnapshot {
        run: RunId::new(55),
        seq: EventSeq::new(42),
        workflow: WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]),
        slots: vec![0x01_u8, 0x02, 0x03],
        taint: vec![0xFF_u8],
    };
    let bytes = encode_record(
        MAGIC_SNAPSHOT,
        RecordKind::Snapshot,
        snapshot.seq.get(),
        &snapshot,
        MAX_SNAPSHOT_BYTES,
    )?;
    let (envelope, decoded) =
        decode_record::<crate::recovery::RunSnapshot>(&bytes, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES)?;
    assert_eq!(envelope.magic, MAGIC_SNAPSHOT);
    assert_eq!(envelope.record_kind, RecordKind::Snapshot.id());
    assert_eq!(envelope.sequence, 42);
    assert_eq!(decoded, snapshot);
    Ok(())
}

#[test]
fn encode_decode_snapshot_large_slots_empty_taint() -> Result<(), JournalError> {
    let slots = vec![0xAB_u8; 8192];
    let snapshot = crate::recovery::RunSnapshot {
        run: RunId::new(100),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        slots: slots.clone(),
        taint: vec![],
    };
    let bytes = encode_record(
        MAGIC_SNAPSHOT,
        RecordKind::Snapshot,
        0,
        &snapshot,
        MAX_SNAPSHOT_BYTES,
    )?;
    let (_, decoded) =
        decode_record::<crate::recovery::RunSnapshot>(&bytes, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES)?;
    assert_eq!(decoded.slots, slots);
    assert!(decoded.taint.is_empty());
    Ok(())
}

#[test]
fn decode_record_header_succeeds_without_payload() -> Result<(), JournalError> {
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
    )?;
    let header = decode_record_header(
        &full[..RECORD_HEADER_BYTES],
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(header.magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(header.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(header.record_kind, RecordKind::RunCancelled.id());
    assert_eq!(header.header_len, RECORD_HEADER_LEN);
    assert!(header.payload_len > 0);
    Ok(())
}

#[test]
fn encoded_size_matches_header_declared_payload_len() -> Result<(), JournalError> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(99),
        seq: EventSeq::new(5),
        workflow: WorkflowDigest::from_bytes([0xCC; DIGEST_BYTES]),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        5,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let header =
        decode_record_header(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    let payload_len = usize::try_from(header.payload_len).unwrap_or(0);
    assert_eq!(
        bytes.len(),
        RECORD_HEADER_BYTES.saturating_add(payload_len),
        "total bytes must equal header plus declared payload length"
    );
    Ok(())
}

#[test]
fn encode_produces_valid_envelope_for_each_record_type() -> Result<(), JournalError> {
    // Workflow source
    let source = b"test".to_vec();
    let ws_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let ws_record = WorkflowSourceRecord {
        digest: ws_digest,
        source,
    };
    let ws_bytes = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        &ws_record,
        MAX_WORKFLOW_SOURCE_BYTES,
    )?;
    let (ws_env, _) = decode_record::<WorkflowSourceRecord>(
        &ws_bytes,
        MAGIC_WORKFLOW_SOURCE,
        MAX_WORKFLOW_SOURCE_BYTES,
    )?;
    assert_eq!(ws_env.magic, MAGIC_WORKFLOW_SOURCE);

    // Compiled IR
    let ir = b"ir".to_vec();
    let ir_digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
    let ir_record = CompiledIrRecord {
        digest: ir_digest,
        ir,
    };
    let ir_bytes = encode_record(
        MAGIC_COMPILED_ARTIFACT,
        RecordKind::CompiledIr,
        0,
        &ir_record,
        MAX_COMPILED_IR_BYTES,
    )?;
    let (ir_env, _) = decode_record::<CompiledIrRecord>(
        &ir_bytes,
        MAGIC_COMPILED_ARTIFACT,
        MAX_COMPILED_IR_BYTES,
    )?;
    assert_eq!(ir_env.magic, MAGIC_COMPILED_ARTIFACT);

    // Blob
    let blob_data = vec![0xDE_u8, 0xAD];
    let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_data).into();
    let blob_record = BlobRecord {
        digest: blob_digest,
        bytes: blob_data,
    };
    let blob_bytes = encode_record(
        MAGIC_BLOB,
        RecordKind::Blob,
        0,
        &blob_record,
        MAX_BLOB_BYTES,
    )?;
    let (blob_env, _) = decode_record::<BlobRecord>(&blob_bytes, MAGIC_BLOB, MAX_BLOB_BYTES)?;
    assert_eq!(blob_env.magic, MAGIC_BLOB);

    Ok(())
}

#[test]
fn magic_journal_event_accepts_all_journal_event_kinds() -> Result<(), JournalError> {
    let journal_kinds = [
        RecordKind::RunAccepted,
        RecordKind::StepStarted,
        RecordKind::SlotWritten,
        RecordKind::ActionScheduled,
        RecordKind::ActionScheduledTicket,
        RecordKind::ActionCompleted,
        RecordKind::ActionCompletedEnvelope,
        RecordKind::ActionFailed,
        RecordKind::WaitScheduled,
        RecordKind::AskScheduled,
        RecordKind::AskAnswered,
        RecordKind::RetryScheduled,
        RecordKind::StepFailed,
        RecordKind::RunCancelled,
        RecordKind::RunKilled,
        RecordKind::AskTimedOut,
        RecordKind::WaitResolved,
        RecordKind::ActionAbandoned,
        RecordKind::RunFinished,
        RecordKind::RunFailed,
        RecordKind::RunAdmission,
        RecordKind::RunResumed,
        RecordKind::RunRetried,
        RecordKind::RunAnswered,
    ];
    let payload: Vec<u8> = vec![0u8; 4];
    for kind in &journal_kinds {
        let result = encode_record(
            MAGIC_JOURNAL_EVENT,
            *kind,
            0,
            &payload,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            result.is_ok(),
            "MAGIC_JOURNAL_EVENT should accept kind {:?} (id {}), got {:?}",
            kind,
            kind.id(),
            result
        );
    }
    Ok(())
}

#[test]
fn corrupted_payload_byte_is_detected() -> Result<(), JournalError> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(42),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let payload_start = RECORD_HEADER_BYTES;
    if bytes.len() > payload_start {
        if let Some(byte) = bytes.get_mut(payload_start) {
            *byte = byte.wrapping_add(1);
        }
        let result = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "corrupted first payload byte must yield PayloadDigestMismatch, got {:?}",
            result
        );
    }
    Ok(())
}

#[test]
fn forged_payload_trailing_postcard_bytes_are_rejected_exactly() -> Result<(), JournalError> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(43),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; DIGEST_BYTES]),
    };
    let mut payload = postcard::to_allocvec(&event).map_err(JournalError::Encode)?;
    payload.push(0x00);
    let payload_len =
        super::payload::payload_len_u32(payload.len(), MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    let bytes = super::payload::encode_record_payload(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &payload,
        payload_len,
    )?;
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(
            result,
            Err(JournalError::PostcardDecodeFailed(
                postcard::Error::DeserializeBadEncoding
            ))
        ),
        "payload with digest-valid trailing postcard bytes must yield exact PostcardDecodeFailed(DeserializeBadEncoding), got {:?}",
        result
    );
    Ok(())
}

#[test]
fn fully_corrupted_crc_is_detected() -> Result<(), JournalError> {
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
    for i in CRC_OFFSET..CRC_OFFSET.saturating_add(4) {
        if let Some(byte) = bytes.get_mut(i) {
            *byte = byte.wrapping_add(1);
        }
    }
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::HeaderChecksumMismatch)),
        "fully corrupted CRC must yield HeaderChecksumMismatch, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn encode_accepts_index_update_kind_with_index_record_magic() {
    let payload: Vec<u8> = vec![0u8; 4];
    let result = encode_record(
        MAGIC_INDEX_RECORD,
        RecordKind::IndexUpdate,
        0,
        &payload,
        1024,
    );
    assert!(
        result.is_ok(),
        "IndexUpdate (kind 50) should be accepted by MAGIC_INDEX_RECORD, got {:?}",
        result
    );
}

#[test]
fn decode_record_header_rejects_kind_family_mismatch() -> Result<(), JournalError> {
    let payload = b"test data";
    let mut header = encode_record_header(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        payload,
        1024,
    )?;

    // Overwrite kind field at offset 6 with Blob kind (40)
    let blob_kind = RecordKind::Blob.id();
    let kind_bytes = blob_kind.to_le_bytes();
    if let Some(slice) = header.get_mut(6..8) {
        slice.copy_from_slice(&kind_bytes);
    }
    // Recompute CRC
    let checksum = crc32c::crc32c(&header[..CRC_OFFSET]);
    let crc_bytes = checksum.to_le_bytes();
    if let Some(slice) = header.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
        slice.copy_from_slice(&crc_bytes);
    }

    let result = decode_record_header(&header, MAGIC_JOURNAL_EVENT, 1024);
    assert!(
        matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
        "kind family mismatch in header must be rejected, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn encode_decode_header_with_max_sequence_roundtrip() -> Result<(), JournalError> {
    let payload = b"data";
    let header = encode_record_header(
        MAGIC_SNAPSHOT,
        RecordKind::Snapshot,
        u64::MAX,
        payload,
        MAX_SNAPSHOT_BYTES,
    )?;
    let decoded = decode_record_header(&header, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES)?;
    assert_eq!(decoded.sequence, u64::MAX);
    Ok(())
}

#[test]
fn empty_blob_record_round_trip_through_codec() -> Result<(), JournalError> {
    let empty: Vec<u8> = vec![];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&empty).into();
    let record = BlobRecord {
        digest,
        bytes: empty,
    };
    let bytes = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, MAX_BLOB_BYTES)?;
    let (_, decoded) = decode_record::<BlobRecord>(&bytes, MAGIC_BLOB, MAX_BLOB_BYTES)?;
    assert_eq!(decoded.bytes.len(), 0);
    assert_eq!(decoded.digest, digest);
    Ok(())
}

#[test]
fn validate_replayed_event_with_zero_run_and_seq() {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(0),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let result = validate_replayed_event(RunId::new(0), EventSeq::new(0), &event);
    assert!(result.is_ok(), "zero run and seq should pass validation");
}

#[test]
fn validate_replayed_event_with_max_run_and_seq() {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(u64::MAX),
        seq: EventSeq::new(u64::MAX),
        attempt: 1,
        reason: None,
    };
    let result = validate_replayed_event(RunId::new(u64::MAX), EventSeq::new(u64::MAX), &event);
    assert!(result.is_ok(), "max run and seq should pass validation");
}

#[test]
fn next_seq_from_zero_yields_one() -> Result<(), JournalError> {
    let result = next_seq(EventSeq::new(0))?;
    assert_eq!(result.get(), 1);
    Ok(())
}

#[test]
fn header_with_zero_length_payload_has_valid_blake3_digest() -> Result<(), JournalError> {
    let empty_payload: &[u8] = &[];
    let header = encode_record_header(MAGIC_BLOB, RecordKind::Blob, 0, empty_payload, 1024)?;
    let decoded = decode_record_header(&header, MAGIC_BLOB, 1024)?;
    let expected_array: [u8; DIGEST_BYTES] = blake3::hash(empty_payload).into();
    assert_eq!(decoded.payload_digest, expected_array);
    Ok(())
}

#[test]
fn verify_digest_match_empty_payload() {
    let empty: &[u8] = &[];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(empty).into();
    let result = verify_digest_match(empty, digest);
    assert!(
        result.is_ok(),
        "empty payload with correct digest should pass"
    );
}

#[test]
fn decode_with_valid_header_but_garbage_payload_fails() -> Result<(), JournalError> {
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

    // Replace all payload bytes with garbage
    let payload_start = RECORD_HEADER_BYTES;
    for i in payload_start..bytes.len() {
        if let Some(byte) = bytes.get_mut(i) {
            *byte = 0xFF;
        }
    }

    // Fix the digest in the header to match the new payload
    let payload = &bytes[payload_start..];
    let new_digest = blake3::hash(payload);
    let digest_bytes = new_digest.as_bytes();
    for (i, &b) in digest_bytes.iter().enumerate() {
        if let Some(byte) = bytes.get_mut(24usize.saturating_add(i)) {
            *byte = b;
        }
    }

    // Fix the CRC
    let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
    let crc_bytes = checksum.to_le_bytes();
    for (i, &b) in crc_bytes.iter().enumerate() {
        if let Some(byte) = bytes.get_mut(CRC_OFFSET.saturating_add(i)) {
            *byte = b;
        }
    }

    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::PostcardDecodeFailed(_))),
        "garbage payload with valid header should yield PostcardDecodeFailed, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn header_encode_decode_consistency_for_all_magics() -> Result<(), JournalError> {
    let payload = b"consistency test payload data";

    let test_cases: Vec<(u32, RecordKind, u32)> = vec![
        (
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            MAX_WORKFLOW_SOURCE_BYTES,
        ),
        (
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            MAX_COMPILED_IR_BYTES,
        ),
        (
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ),
        (MAGIC_SNAPSHOT, RecordKind::Snapshot, MAX_SNAPSHOT_BYTES),
        (MAGIC_BLOB, RecordKind::Blob, MAX_BLOB_BYTES),
        (
            MAGIC_INDEX_RECORD,
            RecordKind::RunHeader,
            MAX_RUN_HEADER_BYTES,
        ),
    ];

    for (magic, kind, max_len) in &test_cases {
        let header = encode_record_header(*magic, *kind, 42, payload, *max_len)?;
        let decoded = decode_record_header(&header, *magic, *max_len)?;
        assert_eq!(decoded.magic, *magic, "magic mismatch for kind {:?}", kind);
        assert_eq!(
            decoded.record_kind,
            kind.id(),
            "kind mismatch for {:?}",
            kind
        );
        assert_eq!(decoded.sequence, 42, "sequence mismatch for {:?}", kind);
        assert_eq!(
            decoded.schema_version, CURRENT_SCHEMA_VERSION,
            "schema version mismatch for {:?}",
            kind
        );
        assert_eq!(
            decoded.header_len, RECORD_HEADER_LEN,
            "header_len mismatch for {:?}",
            kind
        );
    }
    Ok(())
}
