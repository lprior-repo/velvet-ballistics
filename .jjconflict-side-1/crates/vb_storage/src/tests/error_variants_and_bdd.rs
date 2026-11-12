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

    #[test]
    fn validate_replayed_event_returns_wrong_run_when_run_id_mismatch() {
        // Given events stored for run 10 and a replay request for run 20
        // When events_for_run is called for run 20 on a journal that only has run 10 events
        // Then no events are returned (no prefix match), producing an empty result
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run_a = RunId::new(10);
        let event = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        journal
            .append_journaled(&event)
            .expect("journal.append_journaled must succeed");

        let run_b = RunId::new(20);
        let result = journal.events_for_run(run_b);
        let events = result.expect("events_for_run should succeed for missing run");
        assert!(events.is_empty(), "no events should exist for run_b");
    }

    #[test]
    fn validate_replayed_event_returns_sequence_gap_when_seq_out_of_order() {
        // Given a journal with seq 0 then seq 2 for the same run
        // When events_for_run replays
        // Then it returns SequenceGap with expected=1, actual=2
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(100);
        let event0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        journal
            .append_journaled(&event0)
            .expect("journal.append_journaled must succeed");

        // Manually insert an event at seq 2 (skipping seq 1)
        let event2 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        };
        journal
            .append_journaled(&event2)
            .expect("journal.append_journaled must succeed");

        let result = journal.events_for_run(run);
        let Err(JournalError::SequenceGap { expected, actual }) = result else {
            panic!("expected SequenceGap, got {:?}", result);
        };
        assert_eq!(expected, EventSeq::new(1));
        assert_eq!(actual, EventSeq::new(2));
    }

    #[test]
    fn next_seq_returns_sequence_overflow_at_max() {
        // Given EventSeq at u64::MAX
        // When the next sequence is computed
        // Then it returns SequenceOverflow
        let seq = EventSeq::new(u64::MAX);
        let result = seq.get().checked_add(1).map(EventSeq::new);
        assert!(result.is_none());
    }

    #[test]
    fn duplicate_event_returns_exact_run_and_seq() {
        // Given a journal with a RunAccepted event for run 42, seq 7
        // When the same event is appended again
        // Then DuplicateEvent is returned with run=42, seq=7
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let event = JournalEvent::RunAccepted {
            run: RunId::new(42),
            seq: EventSeq::new(7),
            workflow: WorkflowDigest::from_bytes([3; 32]),
        };
        journal
            .append_journaled(&event)
            .expect("journal.append_journaled must succeed");

        let result = journal.append_journaled(&event);
        let Err(JournalError::DuplicateEvent { run, seq }) = result else {
            panic!("expected DuplicateEvent, got {:?}", result);
        };
        assert_eq!(run, RunId::new(42));
        assert_eq!(seq, EventSeq::new(7));
    }

    #[test]
    fn decode_record_returns_migration_required_for_old_schema() {
        // Given an encoded record with schema version set to 0
        // When decode_record is called
        // Then it returns MigrationRequired with from=0, to=1
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
        // Patch schema version at offset 4..6 to 0
        encoded[4] = 0;
        encoded[5] = 0;
        // Recompute CRC
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::MigrationRequired { from, to }) = result else {
            panic!("expected MigrationRequired, got {:?}", result);
        };
        assert_eq!(from, 0);
        assert_eq!(to, 1);
    }

    #[test]
    fn decode_record_returns_unsupported_schema_version_for_future() {
        // Given an encoded record with schema version 99
        // When decode_record is called
        // Then it returns UnsupportedSchemaVersion with version=99
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
        encoded[4] = 99;
        encoded[5] = 0;
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::UnsupportedSchemaVersion { version }) = result else {
            panic!("expected UnsupportedSchemaVersion, got {:?}", result);
        };
        assert_eq!(version, 99);
    }

    #[test]
    fn decode_record_returns_unknown_record_kind_for_invalid_kind() {
        // Given an encoded record with kind patched to 200
        // When decode_record is called
        // Then it returns UnknownRecordKind with kind=200
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
        // Patch kind at offset 6..8 to 200
        let kind_bytes = 200u16.to_le_bytes();
        encoded[6] = kind_bytes[0];
        encoded[7] = kind_bytes[1];
        // Recompute CRC
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::UnknownRecordKind { kind }) = result else {
            panic!("expected UnknownRecordKind, got {:?}", result);
        };
        assert_eq!(kind, 200);
    }

    #[test]
    fn decode_record_returns_header_length_mismatch_for_wrong_len() {
        // Given an encoded record with header_len patched to 99
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
        let len_bytes = 99u32.to_le_bytes();
        encoded[8] = len_bytes[0];
        encoded[9] = len_bytes[1];
        encoded[10] = len_bytes[2];
        encoded[11] = len_bytes[3];
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        let Err(JournalError::HeaderLengthMismatch { found }) = result else {
            panic!("expected HeaderLengthMismatch, got {:?}", result);
        };
        assert_eq!(found, 99);
    }

    // --- Section 2: Key Function Behavior Tests ---

    #[test]
    fn run_event_key_produces_expected_key_bytes() {
        // Given run_id=1, seq=0
        // When run_event_key is called
        // Then the key is [0x11][1_be][0_be]
        let key = run_event_key(RunId::new(1), EventSeq::new(0));
        let key = key.expect("run_event_key should succeed");
        assert_eq!(key[0], 0x11);
        assert_eq!(key[1..9], 1u64.to_be_bytes());
        assert_eq!(key[9..17], 0u64.to_be_bytes());
    }

    #[test]
    fn run_header_key_produces_expected_key_bytes() {
        // Given run_id=0xAABBCCDD_EEFF0011
        // When run_header_key is called
        // Then the key is [0x10][run_id_be]
        let run = RunId::new(0xAABB_CCDD_EEFF_0011);
        let key = run_header_key(run);
        let key = key.expect("run_header_key should succeed");
        assert_eq!(key[0], 0x10);
        assert_eq!(key[1..9], run.get().to_be_bytes());
    }

    #[test]
    fn run_snapshot_key_produces_expected_key_bytes() {
        // Given run_id=5, seq=99
        // When run_snapshot_key is called
        // Then the key is [0x12][5_be][99_be]
        let key = run_snapshot_key(RunId::new(5), EventSeq::new(99));
        let key = key.expect("run_snapshot_key should succeed");
        assert_eq!(key[0], 0x12);
        assert_eq!(key[1..9], 5u64.to_be_bytes());
        assert_eq!(key[9..17], 99u64.to_be_bytes());
    }

    #[test]
    fn workflow_source_key_produces_expected_key_bytes() {
        // Given a 32-byte digest of all 7s
        // When workflow_source_key is called
        // Then the key is [0x01][digest]
        let digest = [7u8; 32];
        let key = workflow_source_key(digest);
        let key = key.expect("workflow_source_key should succeed");
        assert_eq!(key[0], 0x01);
        assert_eq!(key[1..33], digest);
    }

    #[test]
    fn compiled_ir_key_produces_expected_key_bytes() {
        // Given a 32-byte digest of all 2s
        // When compiled_ir_key is called
        // Then the key is [0x02][digest]
        let digest = [2u8; 32];
        let key = compiled_ir_key(digest);
        let key = key.expect("compiled_ir_key should succeed");
        assert_eq!(key[0], 0x02);
        assert_eq!(key[1..33], digest);
    }

    #[test]
    fn index_action_key_produces_expected_key_bytes() {
        // Given action=100, run=200, step=300
        // When index_action_key is called
        // Then the key is [0x32][action_u16_be][run_u64_be][step_u16_be]
        let key = index_action_key(ActionId::new(100), RunId::new(200), StepIdx::new(300));
        let key = key.expect("index_action_key should succeed");
        assert_eq!(key[0], 0x32);
        assert_eq!(key[1..3], 100u16.to_be_bytes());
        assert_eq!(key[3..11], 200u64.to_be_bytes());
        assert_eq!(key[11..13], 300u16.to_be_bytes());
    }

    #[test]
    fn index_status_key_produces_expected_key_bytes() {
        // Given state=5, timestamp=1000, run=50
        // When index_status_key is called
        // Then the key is [0x30][state_u8][timestamp_u64_be][run_u64_be]
        let key = index_status_key(IndexStatusState::Other(5), 1000, RunId::new(50));
        let key = key.expect("index_status_key should succeed");
        assert_eq!(key[0], 0x30);
        assert_eq!(key[1], 5);
        assert_eq!(key[2..10], 1000u64.to_be_bytes());
        assert_eq!(key[10..18], 50u64.to_be_bytes());
    }

    #[test]
    fn index_workflow_key_produces_expected_key_bytes() {
        // Given workflow_id=42, run=99
        // When index_workflow_key is called
        // Then the key is [0x31][workflow_u32_be][run_u64_be]
        let key = index_workflow_key(WorkflowId::new(42), RunId::new(99));
        let key = key.expect("index_workflow_key should succeed");
        assert_eq!(key[0], 0x31);
        assert_eq!(key[1..5], 42u32.to_be_bytes());
        assert_eq!(key[5..13], 99u64.to_be_bytes());
    }

    #[test]
    fn blob_key_produces_expected_key_bytes() {
        // Given a 32-byte digest of all 0xAB
        // When blob_key is called
        // Then the key is [0x20][digest]
        let digest = [0xAB; 32];
        let key = blob_key(digest);
        let key = key.expect("blob_key should succeed");
        assert_eq!(key[0], 0x20);
        assert_eq!(key[1..33], digest);
    }

    // --- Section 3: BDD Integration-Style Tests ---

    #[test]
    fn journal_opens_and_closes_without_error() {
        // Given a temporary directory
        // When FjallJournal::open is called
        // Then the journal opens successfully
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None);
        assert!(journal.is_ok(), "journal should open with default config");
    }

    #[test]
    fn public_open_wrappers_create_declared_keyspaces() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");

        let journal = open_store(temp_dir.path());
        assert!(journal.is_ok(), "open_store should succeed");
        drop(journal);

        let reopened = init_keyspaces(temp_dir.path());
        assert!(reopened.is_ok(), "init_keyspaces should succeed");
        assert_eq!(FjallJournal::declared_keyspaces().len(), 12);
    }

    #[test]
    fn public_wrappers_delegate_to_journal_storage_paths() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = open_store(temp_dir.path()).expect("setup: journal open");
        let run = RunId::new(70);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(1),
            workflow: WorkflowDigest::from_bytes([7; 32]),
        };
        let blob_bytes = vec![1, 2, 3];
        let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
        let blob = BlobRecord {
            digest: blob_digest,
            bytes: blob_bytes,
        };
        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([7; 32]),
            slots: vec![4, 5, 6],
            taint: Vec::new(),
        };

        append_journal_event(&journal, &event).expect("append_journal_event must succeed");
        journal
            .put_blob(&blob)
            .expect("journal.put_blob must succeed");
        write_snapshot(&journal, &snapshot).expect("write_snapshot must succeed");

        // Snapshot at seq 0 covers events 0..0; event at seq 1 is after snapshot
        let events = read_run_events(&journal, run);
        let events = events.expect("read_run_events should succeed");
        assert_eq!(events, vec![event.clone()]);
        let loaded_blob = read_blob(&journal, blob.digest);
        let loaded_blob = loaded_blob.expect("read_blob should succeed");
        assert_eq!(loaded_blob, Some(blob));
        let loaded_snapshot = journal.snapshot(run, EventSeq::new(0));
        let loaded_snapshot = loaded_snapshot.expect("snapshot lookup should succeed");
        assert_eq!(loaded_snapshot, Some(snapshot));
    }

    #[test]
    fn replay_journal_wrapper_uses_recovery_replay() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = open_store(temp_dir.path()).expect("setup: journal open");
        let run = RunId::new(71);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([8; 32]),
        };
        append_journal_event(&journal, &event).expect("append_journal_event must succeed");

        let mut tracker = ActionReplayTracker::new();
        let replayed = replay_journal(&journal, run, &mut tracker, &[], &[]);

        let replayed = replayed.expect("replay_journal should succeed");
        assert_eq!(replayed, vec![event]);
    }

    #[test]
    fn append_strict_persists_submitted_event() {
        // Given an open journal
        // When append_strict is called with a RunAccepted event
        // Then the event can be retrieved via events_for_run
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(55);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let result = journal.append_strict(&event);
        result.expect("action must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn append_strict_rejects_out_of_order_sequence() {
        // Given an open journal with a seq-0 event
        // When append_strict is called with seq 2 (skipping seq 1)
        // Then events_for_run returns SequenceGap
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(60);
        let event0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        journal
            .append_strict(&event0)
            .expect("journal.append_strict must succeed");

        let event2 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        };
        journal
            .append_strict(&event2)
            .expect("journal.append_strict must succeed");

        let result = journal.events_for_run(run);
        let Err(JournalError::SequenceGap { expected, actual }) = result else {
            panic!("expected SequenceGap, got {:?}", result);
        };
        assert_eq!(expected, EventSeq::new(1));
        assert_eq!(actual, EventSeq::new(2));
    }

    #[test]
    fn persist_strict_flushes_and_reopens_cleanly() {
        // Given an open journal with a persisted event
        // When the journal is closed and reopened
        // Then the same event is visible
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");

        let run = RunId::new(77);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([5; 32]),
        };
        {
            let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
            journal
                .append_strict(&event)
                .expect("journal.append_strict must succeed");
        }

        let journal2 = FjallJournal::open(temp_dir.path(), None);
        let journal2 = journal2.expect("journal should reopen cleanly");
        let events = journal2
            .events_for_run(run)
            .expect("events_for_run should succeed after reopen");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn put_workflow_source_stores_and_retrieves() {
        // Given an open journal and a workflow source record
        // When put_workflow_source is called
        // Then the record can be retrieved by digest
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let source = vec![b'h', b'e', b'l', b'l', b'o'];
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord { digest, source };
        journal
            .put_workflow_source(&record)
            .expect("journal.put_workflow_source must succeed");

        let retrieved = journal
            .workflow_source(digest)
            .expect("workflow_source lookup should succeed");
        assert_eq!(retrieved, Some(record));
    }

    #[test]
    fn put_workflow_source_returns_none_for_missing_digest() {
        // Given an open journal with no stored workflow source
        // When workflow_source is called with an arbitrary digest
        // Then it returns None
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let missing = WorkflowDigest::from_bytes([99; 32]);
        let result = journal
            .workflow_source(missing)
            .expect("lookup should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn put_run_header_stores_and_retrieves() {
        // Given an open journal and a run header record
        // When put_run_header is called
        // Then the record can be retrieved by run id
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let record = RunHeaderRecord {
            run: RunId::new(123),
            workflow_id: WorkflowId::new(456),
            compiled_digest: WorkflowDigest::from_bytes([8; 32]),
            status: 1,
            accepted_at_ms: 1700000000,
        };
        journal
            .put_run_header(&record)
            .expect("journal.put_run_header must succeed");

        let retrieved = journal
            .run_header(RunId::new(123))
            .expect("run_header lookup should succeed");
        assert_eq!(retrieved, Some(record));
    }

    // CC-002: run_headers must fail closed on a malformed key (length mismatch).
    #[test]
    fn cc002_run_headers_fails_closed_on_malformed_key() {
        // Given a journal with one valid run header
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let record = RunHeaderRecord {
            run: RunId::new(0xABCD),
            workflow_id: WorkflowId::new(1),
            compiled_digest: WorkflowDigest::from_bytes([0; 32]),
            status: 1,
            accepted_at_ms: 0,
        };
        journal
            .put_run_header(&record)
            .expect("setup: put_run_header");

        // When a malformed key (3 bytes — well-formed keys are 9 bytes) is
        // planted directly into the run_header partition
        let bad_key: Vec<u8> = vec![PREFIX_RUN_HEADER, 0xAB, 0xCD];
        journal
            .run_header
            .insert(bad_key, b"any-value".to_vec())
            .expect("malformed key insert at partition level");

        // Then run_headers fails closed with MalformedKeyspaceRow
        let err = journal
            .run_headers()
            .expect_err("run_headers must fail closed on malformed key");
        match err {
            JournalError::MalformedKeyspaceRow {
                prefix,
                expected_len,
                actual_len,
            } => {
                assert_eq!(prefix, PREFIX_RUN_HEADER);
                assert_eq!(expected_len, 9);
                assert_eq!(actual_len, 3);
            }
            other => panic!("expected MalformedKeyspaceRow, got {other:?}"),
        }
    }

    #[test]
    fn put_compiled_ir_stores_and_retrieves() {
        // Given an open journal and a compiled IR record
        // When put_compiled_ir is called
        // Then the record can be retrieved by digest
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let record = compiled_ir_record(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        let digest = record.digest;
        journal
            .put_compiled_ir(&record)
            .expect("journal.put_compiled_ir must succeed");

        let retrieved = journal
            .compiled_ir(digest)
            .expect("compiled_ir lookup should succeed");
        assert_eq!(retrieved, Some(record));
    }

    #[test]
    fn put_blob_stores_and_retrieves() {
        // Given an open journal and a blob record
        // When put_blob is called
        // Then the record can be retrieved by digest
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let blob_bytes = vec![1, 2, 3, 4, 5];
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
        let record = BlobRecord {
            digest,
            bytes: blob_bytes,
        };
        journal
            .put_blob(&record)
            .expect("journal.put_blob must succeed");

        let retrieved = journal.blob(digest).expect("blob lookup should succeed");
        assert_eq!(retrieved, Some(record));
    }

    #[test]
    fn put_snapshot_stores_and_retrieves() {
        // Given an open journal and a run snapshot
        // When put_snapshot is called
        // Then the snapshot can be retrieved by run and seq
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let snapshot = RunSnapshot {
            run: RunId::new(88),
            seq: EventSeq::new(10),
            workflow: WorkflowDigest::from_bytes([7; 32]),
            slots: vec![1, 2, 3],
            taint: Vec::new(),
        };
        journal
            .put_snapshot(&snapshot)
            .expect("journal.put_snapshot must succeed");

        let retrieved = journal
            .snapshot(RunId::new(88), EventSeq::new(10))
            .expect("snapshot lookup should succeed");
        assert_eq!(retrieved, Some(snapshot));
    }

    #[test]
    fn put_action_index_stores_and_retrieves() {
        // Given an open journal
        // When put_action_index is called
        // Then no error is returned and the index entry exists
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let result = journal.put_action_index(ActionId::new(1), RunId::new(2), StepIdx::new(3));
        result.expect("action must succeed");
    }

    #[test]
    fn put_status_index_stores_and_retrieves() {
        // Given an open journal
        // When put_status_index is called
        // Then no error is returned
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let result =
            journal.put_status_index(IndexStatusState::Submitted, 1700000000, RunId::new(99));
        result.expect("action must succeed");
    }

    #[test]
    fn put_workflow_index_stores_and_retrieves() {
        // Given an open journal
        // When put_workflow_index is called
        // Then no error is returned
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let result = journal.put_workflow_index(WorkflowId::new(7), RunId::new(8));
        result.expect("action must succeed");
    }

    #[test]
    fn events_for_run_returns_only_events_for_target_run() {
        // Given a journal with events for run 10 and run 20
        // When events_for_run is called for run 10
        // Then only run 10 events are returned
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run_a = RunId::new(10);
        let run_b = RunId::new(20);

        let event_a0 = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let event_b0 = JournalEvent::RunAccepted {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([2; 32]),
        };
        let event_a1 = JournalEvent::StepStarted {
            run: run_a,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };

        journal
            .append_journaled(&event_a0)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&event_b0)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&event_a1)
            .expect("journal.append_journaled must succeed");

        let events_a = journal
            .events_for_run(run_a)
            .expect("events_for_run should succeed");
        assert_eq!(events_a.len(), 2);
        assert_eq!(events_a[0], event_a0);
        assert_eq!(events_a[1], event_a1);

        let events_b = journal
            .events_for_run(run_b)
            .expect("events_for_run should succeed");
        assert_eq!(events_b.len(), 1);
        assert_eq!(events_b[0], event_b0);
    }

    #[test]
    fn event_seq_new_returns_correct_value() {
        // Given EventSeq::new(42)
        // When get is called
        // Then it returns 42
        let seq = EventSeq::new(42);
        assert_eq!(seq.get(), 42);
    }

    #[test]
    fn record_kind_id_returns_correct_wire_ids() {
        // Given each RecordKind variant
        // When id() is called
        // Then it returns the expected wire identifier
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
        assert_eq!(RecordKind::Blob.id(), 40);
        assert_eq!(RecordKind::IndexUpdate.id(), 50);
    }

    #[test]
    fn journal_event_run_id_returns_correct_run() {
        // Given a RunAccepted event for run 42
        // When run_id() is called
        // Then it returns 42
        let event = JournalEvent::RunAccepted {
            run: RunId::new(42),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        assert_eq!(event.run_id(), RunId::new(42));
    }

    #[test]
    fn journal_event_seq_returns_correct_seq() {
        // Given a StepStarted event with seq 7
        // When seq() is called
        // Then it returns EventSeq(7)
        let event = JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(7),
            step: StepIdx::new(0),
            attempt: 1,
        };
        assert_eq!(event.seq(), EventSeq::new(7));
    }

    #[test]
    fn journal_event_record_kind_returns_correct_kind() {
        // Given a RunFinished event
        // When record_kind() is called
        // Then it returns RecordKind::RunFinished
        let event = JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            result: vb_core::SlotIdx::new(0),
            attempt: 1,
        };
        assert_eq!(event.record_kind(), RecordKind::RunFinished);
    }

    #[test]
    fn decode_record_returns_postcard_decode_failed_for_garbage_payload() {
        // Given an encoded record with a valid header but corrupted payload bytes
        // that no longer deserialize correctly
        // When decode_record is called
        // Then it returns PostcardDecodeFailed
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
        // Corrupt the payload bytes after the header but not the blake3 digest
        // We need to corrupt and re-hash, so instead we construct a manually
        // crafted header with valid CRC/digest pointing to garbage
        let payload_start = 60;
        if let Some(byte) = encoded.get_mut(payload_start) {
            *byte = 0xFF;
        }
        // Now recompute the blake3 digest in the header
        let payload = &encoded[60..];
        let digest = blake3::hash(payload);
        encoded[24..56].copy_from_slice(digest.as_bytes());
        // Recompute CRC
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        assert!(matches!(result, Err(JournalError::PostcardDecodeFailed(_))));
    }

    #[test]
    fn envelope_round_trips_workflow_source_record() {
        // Given a WorkflowSourceRecord
        // When encoded and decoded with MAGIC_WORKFLOW_SOURCE
        // Then the record survives the round trip
        let record = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([0xAA; 32]),
            source: vec![1, 2, 3],
        };
        let encoded = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &record,
            128,
        )
        .expect("encoding should succeed");

        let (envelope, decoded) =
            decode_record::<WorkflowSourceRecord>(&encoded, MAGIC_WORKFLOW_SOURCE, 128)
                .expect("decoding should succeed");
        assert_eq!(envelope.magic, MAGIC_WORKFLOW_SOURCE);
        assert_eq!(envelope.record_kind, RecordKind::WorkflowSource.id());
        assert_eq!(decoded, record);
    }

    #[test]
    fn envelope_round_trips_compiled_ir_record() {
        // Given a CompiledIrRecord
        // When encoded and decoded with MAGIC_COMPILED_ARTIFACT
        // Then the record survives the round trip
        let record = CompiledIrRecord {
            digest: WorkflowDigest::from_bytes([0xBB; 32]),
            ir: vec![4, 5, 6],
        };
        let encoded = encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            0,
            &record,
            128,
        )
        .expect("encoding should succeed");

        let (envelope, decoded) =
            decode_record::<CompiledIrRecord>(&encoded, MAGIC_COMPILED_ARTIFACT, 128)
                .expect("decoding should succeed");
        assert_eq!(envelope.magic, MAGIC_COMPILED_ARTIFACT);
        assert_eq!(envelope.record_kind, RecordKind::CompiledIr.id());
        assert_eq!(decoded, record);
    }

    #[test]
    fn envelope_round_trips_blob_record() {
        // Given a BlobRecord
        // When encoded and decoded with MAGIC_BLOB
        // Then the record survives the round trip
        let record = BlobRecord {
            digest: [0xDD; 32],
            bytes: vec![7, 8, 9],
        };
        let encoded =
            encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, 128).expect("encoding ok");

        let (envelope, decoded) =
            decode_record::<BlobRecord>(&encoded, MAGIC_BLOB, 128).expect("decoding ok");
        assert_eq!(envelope.magic, MAGIC_BLOB);
        assert_eq!(envelope.record_kind, RecordKind::Blob.id());
        assert_eq!(decoded, record);
    }

    #[test]
    fn declared_keyspaces_returns_twelve_entries() {
        // Given FjallJournal::declared_keyspaces()
        // When called
        // Then it returns exactly 12 keyspace names including artifact_acceptance
        let keyspaces = FjallJournal::declared_keyspaces();
        assert_eq!(keyspaces.len(), 12);
        assert_eq!(keyspaces[0], "workflow_source");
        assert_eq!(keyspaces[1], "compiled_ir");
        assert_eq!(keyspaces[2], "run_header");
        assert_eq!(keyspaces[3], "run_event");
        assert_eq!(keyspaces[4], "run_snapshot");
        assert_eq!(keyspaces[5], "blob");
        assert_eq!(keyspaces[6], "index_status");
        assert_eq!(keyspaces[7], "index_workflow");
        assert_eq!(keyspaces[8], "index_action");
        assert_eq!(keyspaces.get(9), Some(&"run_inputs"));
        assert_eq!(keyspaces.get(10), Some(&"emergency_deny"));
        assert_eq!(keyspaces.get(11), Some(&"artifact_acceptance"));
    }

    #[test]
    fn run_header_returns_none_for_missing_run() {
        // Given an open journal with no stored headers
        // When run_header is called for an arbitrary run
        // Then it returns None
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let result = journal
            .run_header(RunId::new(999))
            .expect("lookup should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn compiled_ir_returns_none_for_missing_digest() {
        // Given an open journal with no stored IR
        // When compiled_ir is called
        // Then it returns None
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let result = journal
            .compiled_ir(WorkflowDigest::from_bytes([0; 32]))
            .expect("lookup should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn snapshot_returns_none_for_missing_entry() {
        // Given an open journal with no snapshots
        // When snapshot is called
        // Then it returns None
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let result = journal
            .snapshot(RunId::new(1), EventSeq::new(0))
            .expect("lookup should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn blob_returns_none_for_missing_digest() {
        // Given an open journal with no blobs
        // When blob is called
        // Then it returns None
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let result = journal.blob([0; 32]).expect("lookup should succeed");
        assert_eq!(result, None);
    }
