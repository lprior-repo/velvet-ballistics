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
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
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
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::ActionScheduled,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
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
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::ActionCompleted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_wait_scheduled() -> Result<(), JournalError> {
    let event = JournalEvent::WaitScheduledEvent {
        run: RunId::new(40),
        seq: EventSeq::new(5),
        step: StepIdx::new(2),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::WaitScheduled,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_ask_scheduled() -> Result<(), JournalError> {
    let event = JournalEvent::AskScheduledEvent {
        run: RunId::new(50),
        seq: EventSeq::new(6),
        step: StepIdx::new(3),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskScheduled,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_ask_answered() -> Result<(), JournalError> {
    let event = JournalEvent::AskAnsweredEvent {
        run: RunId::new(60),
        seq: EventSeq::new(7),
        step: StepIdx::new(4),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskAnswered,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_retry_scheduled() -> Result<(), JournalError> {
    let event = JournalEvent::RetryScheduledEvent {
        run: RunId::new(70),
        seq: EventSeq::new(8),
        step: StepIdx::new(5),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RetryScheduled,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn kind_31_wait_resolved_record_roundtrip() -> Result<(), JournalError> {
    // PO: RE-009 — WaitResolvedEvent (kind 31) is the resumption of a suspended
    // run, distinct from RetryScheduledEvent (kind 19). This test pins the
    // wire-format roundtrip so future codec changes cannot silently break it.
    let event = JournalEvent::WaitResolvedEvent {
        run: RunId::new(71),
        seq: EventSeq::new(9),
        step: StepIdx::new(6),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::WaitResolved,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (envelope, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(
        envelope.record_kind,
        RecordKind::WaitResolved.id(),
        "envelope.record_kind must be WaitResolved (id 31), got {}",
        envelope.record_kind
    );
    assert_eq!(
        decoded, event,
        "WaitResolvedEvent must roundtrip to itself through encode/decode"
    );
    Ok(())
}

#[test]
fn journal_event_payload_starts_with_record_kind_tag() -> Result<(), JournalError> {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(72),
        seq: EventSeq::new(10),
        step: StepIdx::new(7),
        output: SlotIdx::new(3),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::StepSucceeded,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let tag = bytes
        .get(RECORD_HEADER_BYTES)
        .copied()
        .ok_or(JournalError::UnexpectedEof)?;
    assert_eq!(
        tag,
        u8::try_from(RecordKind::StepSucceeded.id()).unwrap_or(u8::MAX),
        "first postcard payload byte must be the stable RecordKind tag, not enum-order or seq length"
    );
    Ok(())
}

#[test]
fn encode_decode_roundtrip_run_failed() -> Result<(), JournalError> {
    let event = JournalEvent::RunFailedEvent {
        run: RunId::new(80),
        seq: EventSeq::new(9),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunFailed,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
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
        extra: None,
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::SlotWritten,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
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

#[test]
fn encode_decode_roundtrip_empty_blob_payload() -> Result<(), JournalError> {
    let empty_bytes: Vec<u8> = vec![];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&empty_bytes).into();
    let record = crate::records::BlobRecord {
        digest,
        bytes: empty_bytes,
    };
    let bytes = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, MAX_BLOB_BYTES)?;
    let (_, decoded) =
        decode_record::<crate::records::BlobRecord>(&bytes, MAGIC_BLOB, MAX_BLOB_BYTES)?;
    assert_eq!(
        decoded.bytes.len(),
        0,
        "empty payload should roundtrip as empty"
    );
    assert_eq!(decoded, record);
    Ok(())
}

#[test]
fn encode_decode_with_max_sequence() -> Result<(), JournalError> {
    // u64::MAX is the overflow sentinel that `is_valid()` rejects, so use the
    // largest representable value that still passes the invariant gate.
    let event = JournalEvent::RunCancelled {
        run: RunId::new(u64::MAX),
        seq: EventSeq::new(u64::MAX - 1),
        attempt: 1,
        reason: None,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
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
fn decode_header_rejects_unknown_record_kind() -> Result<(), JournalError> {
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
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::UnknownRecordKind { .. })),
        "unknown kind must yield UnknownRecordKind, got {:?}",
        result
    );
    Ok(())
}
