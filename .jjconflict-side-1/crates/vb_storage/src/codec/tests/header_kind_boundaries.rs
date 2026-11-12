#[test]
fn decode_header_rejects_header_length_mismatch() -> Result<(), JournalError> {
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
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
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
    // Decode with a smaller max_payload_len to trigger rejection
    let result = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, 1);
    assert!(
        matches!(result, Err(JournalError::PayloadTooLarge { .. })),
        "payload exceeding max must yield PayloadTooLarge, got {:?}",
        result
    );
    Ok(())
}

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
                assert_ne!(
                    a, b,
                    "magic at index {i} must differ from magic at index {j}"
                );
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

#[test]
fn record_kind_ids_are_distinct() {
    let kinds = [
        RecordKind::WorkflowSource,
        RecordKind::CompiledIr,
        RecordKind::RunHeader,
        RecordKind::RunAccepted,
        RecordKind::StepStarted,
        RecordKind::StepSucceeded,
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
        RecordKind::RunFinished,
        RecordKind::RunFailed,
        RecordKind::RunAdmission,
        RecordKind::RunResumed,
        RecordKind::RunRetried,
        RecordKind::RunAnswered,
        RecordKind::RunKilled,
        RecordKind::AskTimedOut,
        RecordKind::Snapshot,
        RecordKind::WaitResolved,
        RecordKind::ActionAbandoned,
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
    assert_eq!(RecordKind::RunAdmission.id(), 24);
    assert_eq!(RecordKind::RunResumed.id(), 25);
    assert_eq!(RecordKind::RunRetried.id(), 26);
    assert_eq!(RecordKind::RunAnswered.id(), 27);
    assert_eq!(RecordKind::RunKilled.id(), 28);
    assert_eq!(RecordKind::AskTimedOut.id(), 29);
    assert_eq!(RecordKind::Snapshot.id(), 30);
    assert_eq!(RecordKind::WaitResolved.id(), 31);
    assert_eq!(RecordKind::ActionAbandoned.id(), 32);
    assert_eq!(RecordKind::StepSucceeded.id(), 33);
    assert_eq!(RecordKind::ActionScheduledTicket.id(), 34);
    assert_eq!(RecordKind::ActionCompletedEnvelope.id(), 35);
    assert_eq!(RecordKind::RetryCheckpoint.id(), 37);
    assert_eq!(RecordKind::Blob.id(), 40);
    assert_eq!(RecordKind::IndexUpdate.id(), 50);
}

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
    let result = encode_record(MAGIC_BLOB, RecordKind::Snapshot, 0, &record, 128);
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
    let result = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::Blob, 0, &record, 128);
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
    let result = encode_record(MAGIC_SNAPSHOT, RecordKind::RunHeader, 0, &record, 128);
    assert!(
        matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
        "RunHeader kind with MAGIC_SNAPSHOT must fail, got {result:?}"
    );
}

#[test]
fn encode_rejects_payload_one_byte_over_max() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(0),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
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

#[test]
fn decode_rejects_trailing_bytes_beyond_declared_payload() -> Result<(), JournalError> {
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
    // Append garbage after the valid record
    bytes.push(0xFF);
    bytes.push(0xFE);
    bytes.push(0xFD);
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(
            result,
            Err(JournalError::PostcardDecodeFailed(
                postcard::Error::DeserializeBadEncoding
            ))
        ),
        "bytes beyond RECORD_HEADER_BYTES + payload_len must be rejected, got {result:?}"
    );
    Ok(())
}

#[test]
fn decode_rejects_header_only_input_with_nonzero_payload_len() -> Result<(), JournalError> {
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
    // Keep only the header portion
    let header_only = &full[..RECORD_HEADER_BYTES];
    let result = decode_record::<JournalEvent>(
        header_only,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    // The header declares a nonzero payload_len but no payload bytes follow
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "header-only with nonzero payload_len must yield UnexpectedEof, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn decode_rejects_mismatched_digest_in_header() -> Result<(), JournalError> {
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
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "corrupted digest must yield PayloadDigestMismatch, got {:?}",
        result
    );
    Ok(())
}

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
    assert!(
        result.is_ok(),
        "RunHeader (kind 3) should be accepted by MAGIC_INDEX_RECORD"
    );
    Ok(())
}

#[test]
fn step_succeeded_event_maps_to_unique_kind() {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        step: StepIdx::new(0),
        output: SlotIdx::new(0),
    };
    assert_eq!(
        event.record_kind(),
        RecordKind::StepSucceeded,
        "StepSucceeded event should map to its unique record kind"
    );
}

#[test]
fn encode_record_header_with_empty_payload() -> Result<(), JournalError> {
    let payload: &[u8] = &[];
    let header = encode_record_header(MAGIC_BLOB, RecordKind::Blob, 0, payload, 1024)?;
    let decoded = decode_record_header(&header, MAGIC_BLOB, 1024)?;
    assert_eq!(
        decoded.payload_len, 0,
        "empty payload should report zero length"
    );
    assert_eq!(decoded.magic, MAGIC_BLOB);
    Ok(())
}

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
    assert_eq!(
        decoded.sequence, sequence,
        "sequence must survive round-trip"
    );
    assert_eq!(decoded.record_kind, RecordKind::WorkflowSource.id());
    assert_eq!(decoded.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(decoded.header_len, RECORD_HEADER_LEN);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_minimum_valid_run_accepted() -> Result<(), JournalError> {
    // Smallest RunAccepted that still satisfies `is_valid()`: run must be non-zero,
    // seq must not be the u64::MAX overflow sentinel.
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
    let (envelope, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(envelope.record_kind, RecordKind::RunAccepted.id());
    assert_eq!(envelope.sequence, 0);
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_max_field_values() -> Result<(), JournalError> {
    // Use max run ID and the largest sequence that still satisfies
    // `is_valid()` (u64::MAX is the overflow sentinel and is rejected).
    let event = JournalEvent::RunAccepted {
        run: RunId::new(u64::MAX),
        seq: EventSeq::new(u64::MAX - 1),
        workflow: WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        u64::MAX - 1,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (envelope, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(envelope.sequence, u64::MAX - 1);
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_slot_written_with_large_value() -> Result<(), JournalError> {
    // Build a SlotWrittenEvent with a large value payload near the max
    let large_value = vec![0xAB_u8; 1024];
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        slot: SlotIdx::new(u16::MAX.into()),
        value: Some(large_value.clone()),
        extra: None,
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::SlotWritten,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    // Verify the large value survived
    if let JournalEvent::SlotWrittenEvent { value: Some(v), .. } = decoded {
        assert_eq!(v.len(), large_value.len());
    } else {
        panic!("expected SlotWrittenEvent with value");
    }
    Ok(())
}

#[test]
fn decode_rejects_1_byte_input() {
    let result = decode_record::<JournalEvent>(
        &[0x56],
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "1-byte input must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_rejects_4_byte_magic_only() {
    // Just the 4-byte magic, far short of the 60-byte header
    let magic_bytes = MAGIC_JOURNAL_EVENT.to_le_bytes();
    let result = decode_record::<JournalEvent>(
        &magic_bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "4-byte (magic-only) input must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_rejects_59_byte_header_one_short() {
    let partial = [0u8; RECORD_HEADER_BYTES.saturating_sub(1)];
    let result = decode_record::<JournalEvent>(
        &partial,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "59-byte input (one byte short of header) must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn encode_decode_header_with_zero_length_payload_roundtrip() -> Result<(), JournalError> {
    let payload: &[u8] = &[];
    let header = encode_record_header(MAGIC_BLOB, RecordKind::Blob, 0, payload, 1024)?;
    let decoded = decode_record_header(&header, MAGIC_BLOB, 1024)?;
    assert_eq!(decoded.payload_len, 0);
    assert_eq!(decoded.magic, MAGIC_BLOB);
    assert_eq!(decoded.header_len, RECORD_HEADER_LEN);
    Ok(())
}

#[test]
fn multiple_sequential_encode_decode_cycles() -> Result<(), JournalError> {
    // `is_valid()` requires a non-zero run id, so offset the loop by 1.
    let events: Vec<JournalEvent> = (0..10)
        .map(|i| JournalEvent::RunAccepted {
            run: RunId::new((i + 1) as u64),
            seq: EventSeq::new(i as u64),
            workflow: WorkflowDigest::from_bytes([i as u8; DIGEST_BYTES]),
        })
        .collect();

    for event in &events {
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        assert_eq!(decoded, *event);
    }
    Ok(())
}

#[test]
fn sequential_cycles_with_varying_kinds() -> Result<(), JournalError> {
    let run = RunId::new(42);
    let digest = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);

    let events = [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(2),
            attempt: 1,
            reason: None,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(3),
            result: SlotIdx::new(0),
            attempt: 1,
        },
    ];

    let mut accumulated = Vec::new();
    for event in &events {
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        accumulated.push(bytes);
    }

    // Now decode all accumulated bytes back
    for (i, bytes) in accumulated.iter().enumerate() {
        let (_, decoded) = decode_record::<JournalEvent>(
            bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        assert_eq!(decoded, events[i], "mismatch at cycle {i}");
    }
    Ok(())
}

#[test]
fn all_journal_event_kinds_encode_and_decode_correctly() -> Result<(), JournalError> {
    let events_and_kinds = all_journal_event_cases()?;
    for (i, (event, expected_kind)) in events_and_kinds.iter().enumerate() {
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            *expected_kind,
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (envelope, decoded) = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        assert_eq!(
            envelope.record_kind,
            expected_kind.id(),
            "kind mismatch at index {i}: expected {}, got {}",
            expected_kind.id(),
            envelope.record_kind,
        );
        assert_eq!(decoded, *event, "event mismatch at index {i}");
    }
    Ok(())
}

#[test]
fn kind_id_matches_wire_value_for_every_variant() {
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
    assert_eq!(RecordKind::RunAdmission.id(), 24);
    assert_eq!(RecordKind::RunResumed.id(), 25);
    assert_eq!(RecordKind::RunRetried.id(), 26);
    assert_eq!(RecordKind::RunAnswered.id(), 27);
    assert_eq!(RecordKind::RunKilled.id(), 28);
    assert_eq!(RecordKind::AskTimedOut.id(), 29);
    assert_eq!(RecordKind::Snapshot.id(), 30);
    assert_eq!(RecordKind::WaitResolved.id(), 31);
    assert_eq!(RecordKind::ActionAbandoned.id(), 32);
    assert_eq!(RecordKind::StepSucceeded.id(), 33);
    assert_eq!(RecordKind::ActionScheduledTicket.id(), 34);
    assert_eq!(RecordKind::ActionCompletedEnvelope.id(), 35);
    assert_eq!(RecordKind::RetryCheckpoint.id(), 37);
    assert_eq!(RecordKind::Blob.id(), 40);
    assert_eq!(RecordKind::IndexUpdate.id(), 50);
}

#[test]
fn decode_rejects_old_schema_version_with_migration_required() -> Result<(), JournalError> {
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
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::MigrationRequired { from, to }) if from == 0 && to == CURRENT_SCHEMA_VERSION),
        "old schema must yield MigrationRequired, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn every_event_variant_roundtrips_via_record_kind_method() -> Result<(), JournalError> {
    let cases = all_journal_event_cases()?;
    for (i, (event, expected_kind)) in cases.iter().enumerate() {
        let kind = event.record_kind();
        assert_eq!(kind, *expected_kind, "record_kind mismatch at index {i}");
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            kind,
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        assert_eq!(
            bytes.get(RECORD_HEADER_BYTES).copied(),
            Some(u8::try_from(kind.id()).unwrap_or(u8::MAX)),
            "first postcard payload tag must be stable RecordKind id at index {i}"
        );
        let (envelope, decoded) = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        assert_eq!(
            envelope.magic, MAGIC_JOURNAL_EVENT,
            "magic mismatch at index {i}"
        );
        assert_eq!(
            envelope.record_kind,
            kind.id(),
            "kind mismatch at index {i}"
        );
        assert_eq!(
            envelope.sequence,
            event.seq().get(),
            "sequence mismatch at index {i}"
        );
        assert_eq!(decoded, *event, "payload mismatch at index {i}");
    }
    Ok(())
}
