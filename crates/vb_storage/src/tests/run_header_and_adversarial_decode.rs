// --- Section 7: RunHeaderRecord Integration Tests ---

#[test]
fn run_header_overwrite_replaces_existing_header() {
    // Given a journal with a stored run header
    // When a new header with the same run id is stored
    // Then the new header replaces the old one
    let (_guard, journal) = open_journal();
    let original = RunHeaderRecord {
        run: RunId::new(1),
        workflow_id: WorkflowId::new(10),
        compiled_digest: test_digest(1),
        status: 0,
        accepted_at_ms: 100,
    };
    let updated = RunHeaderRecord {
        run: RunId::new(1),
        workflow_id: WorkflowId::new(20),
        compiled_digest: test_digest(2),
        status: 1,
        accepted_at_ms: 200,
    };
    journal
        .put_run_header(&original)
        .expect("journal.put_run_header must succeed");
    journal
        .put_run_header(&updated)
        .expect("journal.put_run_header must succeed");

    let retrieved = journal
        .run_header(RunId::new(1))
        .expect("lookup should succeed");
    assert_eq!(retrieved, Some(updated));
}

#[test]
fn multiple_runs_have_independent_events() {
    // Given a journal with 2 events for run 1 and 3 events for run 2
    // When events_for_run is called for each
    // Then each run returns only its own events
    let (_guard, journal) = open_journal();
    let run1 = RunId::new(1);
    let run2 = RunId::new(2);

    let r1_e0 = JournalEvent::RunAccepted {
        run: run1,
        seq: EventSeq::new(0),
        workflow: test_digest(1),
    };
    let r1_e1 = JournalEvent::StepStarted {
        run: run1,
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        attempt: 1,
    };
    let r2_e0 = JournalEvent::RunAccepted {
        run: run2,
        seq: EventSeq::new(0),
        workflow: test_digest(2),
    };
    let r2_e1 = JournalEvent::StepStarted {
        run: run2,
        seq: EventSeq::new(1),
        step: StepIdx::new(1),
        attempt: 1,
    };
    let r2_e2 = JournalEvent::RunFinished {
        run: run2,
        seq: EventSeq::new(2),
        result: vb_core::SlotIdx::new(0),
        attempt: 1,
    };

    journal
        .append_journaled(&r1_e0)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&r1_e1)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&r2_e0)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&r2_e1)
        .expect("journal.append_journaled must succeed");
    journal
        .append_journaled(&r2_e2)
        .expect("journal.append_journaled must succeed");

    let events1 = journal
        .events_for_run(run1)
        .expect("events_for_run run1 should succeed");
    assert_eq!(events1.len(), 2);
    let events2 = journal
        .events_for_run(run2)
        .expect("events_for_run run2 should succeed");
    assert_eq!(events2.len(), 3);
}

#[test]
fn event_seq_ordering_is_correct() {
    // Given two EventSeq values
    // When compared
    // Then ordering follows the inner u64
    assert!(EventSeq::new(0) < EventSeq::new(1));
    assert!(EventSeq::new(100) < EventSeq::new(200));
    assert_eq!(EventSeq::new(5), EventSeq::new(5));
}

#[test]
fn record_kind_all_variants_have_distinct_ids() {
    // Given all RecordKind variants
    // When their ids are collected
    // Then no two variants share an id
    let ids = [
        RecordKind::WorkflowSource.id(),
        RecordKind::CompiledIr.id(),
        RecordKind::RunHeader.id(),
        RecordKind::RunAccepted.id(),
        RecordKind::StepStarted.id(),
        RecordKind::SlotWritten.id(),
        RecordKind::ActionScheduled.id(),
        RecordKind::ActionCompleted.id(),
        RecordKind::ActionFailed.id(),
        RecordKind::WaitScheduled.id(),
        RecordKind::AskScheduled.id(),
        RecordKind::AskAnswered.id(),
        RecordKind::RetryScheduled.id(),
        RecordKind::StepFailed.id(),
        RecordKind::RunCancelled.id(),
        RecordKind::RunFinished.id(),
        RecordKind::RunFailed.id(),
        RecordKind::RunAdmission.id(),
        RecordKind::RunResumed.id(),
        RecordKind::RunRetried.id(),
        RecordKind::RunAnswered.id(),
        RecordKind::RunKilled.id(),
        RecordKind::AskTimedOut.id(),
        RecordKind::Snapshot.id(),
        RecordKind::WaitResolved.id(),
        RecordKind::ActionAbandoned.id(),
        RecordKind::StepSucceeded.id(),
        RecordKind::ActionScheduledTicket.id(),
        RecordKind::ActionCompletedEnvelope.id(),
        RecordKind::Blob.id(),
        RecordKind::IndexUpdate.id(),
    ];
    let mut sorted = ids.to_vec();
    sorted.sort();
    let mut deduped = sorted.clone();
    deduped.dedup();
    assert_eq!(
        ids.len(),
        deduped.len(),
        "all RecordKind ids must be distinct"
    );
}

#[test]
fn constants_have_expected_values() {
    // Given the module constants
    // When inspected
    // Then they match the contract values
    assert_eq!(RECORD_HEADER_LEN, 60);
    assert_eq!(CURRENT_SCHEMA_VERSION, 1);
    assert_eq!(MAGIC_COMPILED_ARTIFACT, 0x5642_4952);
    assert_eq!(MAGIC_JOURNAL_EVENT, 0x5642_4A45);
    assert_eq!(MAGIC_SNAPSHOT, 0x5642_534E);
    assert_eq!(MAGIC_BLOB, 0x5642_424C);
    assert_eq!(MAGIC_IPC_FRAME, 0x5642_4C54);
    assert_eq!(MAGIC_WORKFLOW_SOURCE, 0x5642_5352);
    assert_eq!(MAGIC_INDEX_RECORD, 0x5642_4958);
}

#[test]
fn prefix_constants_have_expected_values() {
    // Given the prefix constants
    // When inspected
    // Then they match the contract values
    assert_eq!(PREFIX_WORKFLOW_SOURCE, 0x01);
    assert_eq!(PREFIX_COMPILED_IR, 0x02);
    assert_eq!(PREFIX_RUN_HEADER, 0x10);
    assert_eq!(PREFIX_RUN_EVENT, 0x11);
    assert_eq!(PREFIX_RUN_SNAPSHOT, 0x12);
    assert_eq!(PREFIX_BLOB, 0x20);
    assert_eq!(PREFIX_INDEX_STATUS, 0x30);
    assert_eq!(PREFIX_INDEX_WORKFLOW, 0x31);
    assert_eq!(PREFIX_INDEX_ACTION, 0x32);
}

#[test]
fn max_payload_constants_are_sensible() {
    // Given the max payload constants
    // When inspected
    // Then they are non-zero and in reasonable ranges
    assert!(MAX_JOURNAL_EVENT_PAYLOAD_BYTES > 0);
    assert!(MAX_WORKFLOW_SOURCE_BYTES > 0);
    assert!(MAX_COMPILED_IR_BYTES > 0);
    assert!(MAX_RUN_HEADER_BYTES > 0);
    assert!(MAX_SNAPSHOT_BYTES > 0);
    assert!(MAX_BLOB_BYTES > 0);
}

#[test]
fn validate_replayed_event_accepts_matching_run_and_seq() {
    // Given an event with run 42, seq 5
    // When validate_replayed_event is called with matching expected run and seq
    // Then it returns Ok (tested indirectly via events_for_run)
    let (_guard, journal) = open_journal();
    let run = RunId::new(42);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: test_digest(1),
    };
    journal
        .append_journaled(&event)
        .expect("journal.append_journaled must succeed");
    let events = journal
        .events_for_run(run)
        .expect("should succeed with contiguous events");
    assert_eq!(events.len(), 1);
}

#[test]
fn journal_reopen_preserves_multiple_event_types() {
    // Given a journal with multiple event types for a run
    // When the journal is closed and reopened
    // Then all events are preserved
    let temp_dir = tempfile::tempdir().expect("setup: tempdir");
    let run = RunId::new(999);

    {
        let journal = FjallJournal::open(temp_dir.path(), None).expect("open should succeed");
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: test_digest(1),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(2),
                slot: vb_core::SlotIdx::new(0),
                value: None,
                extra: None,
                attempt: 1,
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(0),
                action: ActionId::new(1),
                attempt: 1,
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(4),
                step: StepIdx::new(0),
                action: ActionId::new(1),
                attempt: 1,
            },
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(5),
                step: StepIdx::new(0),
                output: vb_core::SlotIdx::new(1),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(6),
                result: vb_core::SlotIdx::new(1),
                attempt: 1,
            },
        ];

        for event in &events {
            journal
                .append_strict(event)
                .expect("journal.append_strict must succeed");
        }
    }

    let journal2 = FjallJournal::open(temp_dir.path(), None).expect("reopen should succeed");
    let events = journal2
        .events_for_run(run)
        .expect("events_for_run should succeed");
    assert_eq!(events.len(), 7);
    assert_eq!(events[0].seq(), EventSeq::new(0));
    assert_eq!(events[6].seq(), EventSeq::new(6));
}

#[test]
fn run_header_stores_all_fields_correctly() {
    // Given a RunHeaderRecord with specific field values
    // When stored and retrieved
    // Then all fields match exactly
    let (_guard, journal) = open_journal();
    let record = RunHeaderRecord {
        run: RunId::new(42),
        workflow_id: WorkflowId::new(7),
        compiled_digest: test_digest(99),
        status: 3,
        accepted_at_ms: 1700000000,
    };
    journal
        .put_run_header(&record)
        .expect("journal.put_run_header must succeed");
    let retrieved = journal
        .run_header(RunId::new(42))
        .expect("lookup should succeed");
    let Some(found) = retrieved else {
        panic!("expected Some(record)");
    };
    assert_eq!(found.run, record.run);
    assert_eq!(found.workflow_id, record.workflow_id);
    assert_eq!(found.compiled_digest, record.compiled_digest);
    assert_eq!(found.status, record.status);
    assert_eq!(found.accepted_at_ms, record.accepted_at_ms);
}

#[test]
fn journal_stores_and_retrieves_blob_with_zero_bytes() {
    // Given a blob with zero bytes
    // When stored and retrieved
    // Then the record survives with empty bytes
    let (_guard, journal) = open_journal();
    let blob_bytes: Vec<u8> = vec![];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
    let record = BlobRecord {
        digest,
        bytes: blob_bytes,
    };
    journal
        .put_blob(&record)
        .expect("journal.put_blob must succeed");
    let retrieved = journal.blob(digest).expect("lookup should succeed");
    assert_eq!(retrieved, Some(record));
}

#[test]
fn workflow_source_stores_and_retrieves_empty_source() {
    // Given a workflow source with zero source bytes
    // When stored and retrieved
    // Then the record survives with empty source
    let (_guard, journal) = open_journal();
    let source: Vec<u8> = vec![];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord { digest, source };
    journal
        .put_workflow_source(&record)
        .expect("journal.put_workflow_source must succeed");
    let retrieved = journal
        .workflow_source(digest)
        .expect("lookup should succeed");
    assert_eq!(retrieved, Some(record));
}

#[test]
fn encode_decode_roundtrip_for_wait_scheduled_record() {
    // Given a WaitScheduledEvent
    // When encoded and decoded
    // Then the event survives the roundtrip exactly
    let event = JournalEvent::WaitScheduledEvent {
        run: RunId::new(10),
        seq: EventSeq::new(2),
        step: StepIdx::new(3),
        attempt: 1,
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::WaitScheduled,
        2,
        &event,
        128,
    )
    .expect("encoding should succeed");
    let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .expect("decoding should succeed");
    assert_eq!(decoded, event);
}

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
fn encode_decode_roundtrip_for_ask_timed_out_record_uses_distinct_kind() -> Result<(), String> {
    let event = JournalEvent::AskTimedOutEvent {
        run: RunId::new(29),
        seq: EventSeq::new(29),
        step: StepIdx::new(5),
        attempt: 1,
    };

    assert_eq!(event.record_kind(), RecordKind::AskTimedOut);
    assert_ne!(event.record_kind(), RecordKind::AskAnswered);
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskTimedOut,
        event.seq().get(),
        &event,
        128,
    )
    .map_err(|error| format!("encoding should succeed: {error:?}"))?;
    let (envelope, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .map_err(|error| format!("decoding should succeed: {error:?}"))?;

    assert_eq!(envelope.record_kind, RecordKind::AskTimedOut.id());
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn master_wire_contract_lists_ask_timed_out_kind_29() {
    let master = include_str!("../../../../velvet-ballistics-MASTER.md");

    assert!(master.contains("| 29 | `AskTimedOut` |"));
    let words = master.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(words.contains("`CURRENT_SCHEMA_VERSION` remains `1`"));
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

// =========================================================================
// Section: Adversarial Record Header Decode Tests
// =========================================================================

fn encode_and_patch_field(
    event: &JournalEvent,
    kind: RecordKind,
    offset: usize,
    new_bytes: &[u8],
) -> Vec<u8> {
    let mut encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        kind,
        event.seq().get(),
        event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("encoding should succeed");
    let end = offset.saturating_add(new_bytes.len());
    assert!(end <= 56, "patch must be within CRC-protected region");
    encoded
        .get_mut(offset..end)
        .expect("patch range valid")
        .copy_from_slice(new_bytes);
    let header_prefix = &encoded[..56];
    let checksum = crc32c::crc32c(header_prefix);
    encoded[56] = (checksum & 0xFF) as u8;
    encoded[57] = ((checksum >> 8) & 0xFF) as u8;
    encoded[58] = ((checksum >> 16) & 0xFF) as u8;
    encoded[59] = ((checksum >> 24) & 0xFF) as u8;
    encoded
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

#[test]
fn adversarial_decode_header_len_not_60_returns_mismatch() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(6),
        seq: EventSeq::new(0),
        workflow: test_digest(6),
    };
    let encoded = encode_and_patch_field(&event, RecordKind::RunAccepted, 8, &48u32.to_le_bytes());
    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
    let Err(JournalError::HeaderLengthMismatch { found }) = result else {
        panic!("expected mismatch, got {:?}", result)
    };
    assert_eq!(found, 48);
}

#[test]
fn adversarial_decode_payload_len_above_limit_returns_too_large() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(7),
        seq: EventSeq::new(0),
        workflow: test_digest(7),
    };
    let encoded =
        encode_and_patch_field(&event, RecordKind::RunAccepted, 12, &9999u32.to_le_bytes());
    let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 100);
    let Err(JournalError::PayloadTooLarge { len, max }) = result else {
        panic!("expected PayloadTooLarge, got {:?}", result)
    };
    assert_eq!(len, 9999);
    assert_eq!(max, 100);
}

#[test]
fn adversarial_decode_corrupt_header_crc_returns_checksum_mismatch() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(8),
        seq: EventSeq::new(0),
        workflow: test_digest(8),
    };
    let mut encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("ok");
    if let Some(b) = encoded.get_mut(57) {
        *b ^= 0x80;
    }
    assert!(matches!(
        decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128),
        Err(JournalError::HeaderChecksumMismatch)
    ));
}

#[test]
fn adversarial_decode_corrupt_payload_digest_returns_digest_mismatch() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(9),
        seq: EventSeq::new(0),
        workflow: test_digest(9),
    };
    let mut encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("ok");
    if let Some(b) = encoded.get_mut(61) {
        *b ^= 0xFF;
    }
    assert!(matches!(
        decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128),
        Err(JournalError::PayloadDigestMismatch)
    ));
}

#[test]
fn adversarial_decode_truncated_before_full_header_returns_unexpected_eof() {
    let truncated = [0u8; 45];
    assert!(matches!(
        decode_record::<JournalEvent>(&truncated, MAGIC_JOURNAL_EVENT, 128),
        Err(JournalError::UnexpectedEof)
    ));
}

#[test]
fn adversarial_decode_truncated_before_full_payload_returns_unexpected_eof() {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(10),
        seq: EventSeq::new(0),
        workflow: test_digest(10),
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("ok");
    let truncated = encoded.get(..62).expect("slice");
    assert!(matches!(
        decode_record::<JournalEvent>(truncated, MAGIC_JOURNAL_EVENT, 128),
        Err(JournalError::UnexpectedEof)
    ));
}
