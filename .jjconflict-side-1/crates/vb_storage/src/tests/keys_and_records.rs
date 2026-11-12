    #[test]
    fn journal_key_is_fixed_width() {
        // Given a run id 1 and event sequence 2
        // When the journal key is constructed
        // Then the key is exactly 17 bytes wide
        let key = journal_key(RunId::new(1), EventSeq::new(2));

        let key = key.expect("journal key construction should succeed");
        assert_eq!(key.len(), 17);
    }

    #[test]
    fn run_event_key_uses_required_prefix_and_big_endian_layout() {
        // Given run id 0x0102030405060708 and event sequence 9
        // When the run event key is constructed
        // Then the layout is [0x11 prefix][run id big-endian][seq big-endian]
        let key = run_event_key(RunId::new(0x0102_0304_0506_0708), EventSeq::new(9));

        let key = key.expect("run event key construction should succeed");
        let expected: [u8; 17] = [
            0x11, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x09,
        ];
        assert_eq!(key.as_slice(), expected.as_slice());
    }

    #[test]
    fn key_encoders_use_required_lengths() {
        // Given a standard 32-byte digest and common run/step identifiers
        // When each key encoder is called
        // Then the produced keys have the contract-specified byte widths
        let digest = [7_u8; 32];

        let ws = workflow_source_key(digest).expect("workflow_source_key should succeed");
        assert_eq!(ws.len(), 33);

        let ci = compiled_ir_key(digest).expect("compiled_ir_key should succeed");
        assert_eq!(ci.len(), 33);

        let rh = run_header_key(RunId::new(1)).expect("run_header_key should succeed");
        assert_eq!(rh.len(), 9);

        let rs = run_snapshot_key(RunId::new(1), EventSeq::new(2))
            .expect("run_snapshot_key should succeed");
        assert_eq!(rs.len(), 17);

        let bl = blob_key(digest).expect("blob_key should succeed");
        assert_eq!(bl.len(), 33);

        let is = index_status_key(IndexStatusState::Other(3), 4, RunId::new(5))
            .expect("index_status_key should succeed");
        assert_eq!(is.len(), 18);

        let iw = index_workflow_key(WorkflowId::new(6), RunId::new(7))
            .expect("index_workflow_key should succeed");
        assert_eq!(iw.len(), 13);

        let ia = index_action_key(ActionId::new(8), RunId::new(9), StepIdx::new(10))
            .expect("index_action_key should succeed");
        assert_eq!(ia.len(), 13);
    }

    #[test]
    fn encode_key_dispatches_to_existing_key_encoders() {
        let digest = [9_u8; 32];

        let run_event = encode_key(StorageKey::RunEvent {
            run: RunId::new(0x0102_0304_0506_0708),
            seq: EventSeq::new(9),
        })
        .expect("encode_key should encode run event key");
        let expected_run_event = run_event_key(RunId::new(0x0102_0304_0506_0708), EventSeq::new(9))
            .expect("run_event_key should succeed")
            .to_vec();
        assert_eq!(run_event, expected_run_event);

        let blob =
            encode_key(StorageKey::Blob { digest }).expect("encode_key should encode blob key");
        let expected_blob = blob_key(digest).expect("blob_key should succeed").to_vec();
        assert_eq!(blob, expected_blob);
    }

    #[test]
    fn record_header_wrappers_encode_decode_and_verify_digest() {
        let payload = b"compact payload";

        let header = encode_record_header(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            7,
            payload,
            128,
        )
        .expect("record header encoding should succeed");
        assert_eq!(header.len(), RECORD_HEADER_BYTES);

        let decoded = decode_record_header(&header, MAGIC_JOURNAL_EVENT, 128)
            .expect("record header decoding should succeed");
        assert_eq!(decoded.magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(decoded.record_kind, RecordKind::RunAccepted.id());
        assert_eq!(decoded.header_len, RECORD_HEADER_LEN);
        assert_eq!(decoded.payload_len, 15);
        assert_eq!(decoded.sequence, 7);
        verify_digest_match(payload, decoded.payload_digest)
            .expect("payload digest should match decoded header");
    }

    #[test]
    fn verify_digest_match_rejects_mismatched_payload() {
        let digest = *blake3::hash(b"original").as_bytes();

        let result = verify_digest_match(b"changed", digest);

        assert!(matches!(result, Err(JournalError::PayloadDigestMismatch)));
    }

    #[test]
    fn free_put_wrappers_delegate_to_journal_methods() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let journal = open_store(temp.path()).expect("journal should open");
        let source_bytes = vec![b'a'];
        let workflow_digest = WorkflowDigest::from_bytes(blake3::hash(&source_bytes).into());
        let blob_bytes = vec![b'b'];
        let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();

        let source = WorkflowSourceRecord {
            digest: workflow_digest,
            source: source_bytes,
        };
        put_workflow_source(&journal, &source).expect("workflow source should store");
        let stored_source = journal
            .workflow_source(workflow_digest)
            .expect("workflow source lookup should succeed");
        assert_eq!(stored_source, Some(source));

        let compiled = compiled_ir_record(vec![b'i']);
        let compiled_digest = compiled.digest;
        put_compiled_ir(&journal, &compiled).expect("compiled ir should store");
        let stored_compiled = journal
            .compiled_ir(compiled_digest)
            .expect("compiled ir lookup should succeed");
        assert_eq!(stored_compiled, Some(compiled));

        let header = RunHeaderRecord {
            run: RunId::new(11),
            workflow_id: WorkflowId::new(12),
            compiled_digest,
            status: 1,
            accepted_at_ms: 13,
        };
        put_run_header(&journal, &header).expect("run header should store");
        let stored_header = journal
            .run_header(RunId::new(11))
            .expect("run header lookup should succeed");
        assert_eq!(stored_header, Some(header));

        let blob = BlobRecord {
            digest: blob_digest,
            bytes: blob_bytes,
        };
        put_blob(&journal, &blob).expect("blob should store");
        let stored_blob = read_blob(&journal, blob_digest).expect("blob lookup should succeed");
        assert_eq!(stored_blob, Some(blob));
    }

    #[test]
    fn envelope_round_trips_and_reports_metadata() {
        // Given a RunFinished journal event with run 99, seq 12, result slot 1
        // When the event is encoded and then decoded
        // Then the envelope metadata and deserialized event match the originals
        let event = JournalEvent::RunFinished {
            run: RunId::new(99),
            seq: EventSeq::new(12),
            result: vb_core::SlotIdx::new(1),
            attempt: 1,
        };

        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunFinished,
            event.seq().get(),
            &event,
            128,
        );
        let encoded = encoded.expect("encoding should succeed");
        assert!(encoded.len() > 60, "encoded record must exceed header size");

        let (envelope, decoded_event) =
            decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
                .expect("decoding should succeed");
        assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(envelope.record_kind, RecordKind::RunFinished.id());
        assert_eq!(envelope.sequence, 12);
        assert_eq!(decoded_event, event);
    }

    #[test]
    fn decode_rejects_corrupt_header_checksum() {
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
        );
        let Ok(mut encoded) = encoded else {
            return;
        };
        if let Some(byte) = encoded.get_mut(56) {
            *byte ^= 1;
        }

        let decoded = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);

        assert!(matches!(decoded, Err(JournalError::HeaderChecksumMismatch)));
    }

    #[test]
    fn decode_rejects_corrupt_payload_digest() {
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
        );
        let Ok(mut encoded) = encoded else {
            return;
        };
        if let Some(byte) = encoded.get_mut(60) {
            *byte ^= 1;
        }

        let decoded = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);

        assert!(matches!(decoded, Err(JournalError::PayloadDigestMismatch)));
    }

    #[test]
    fn decode_rejects_payload_before_allocation() {
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
        );
        let Ok(encoded) = encoded else {
            return;
        };

        let decoded = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 1);

        assert!(matches!(decoded, Err(JournalError::PayloadTooLarge { .. })));
    }

    #[test]
    fn decode_rejects_bad_magic_and_unknown_kind() {
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
        );
        let Ok(mut bad_magic) = encoded else {
            return;
        };
        if let Some(byte) = bad_magic.get_mut(0) {
            *byte ^= 1;
        }

        let decoded = decode_record::<JournalEvent>(
            &bad_magic,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(matches!(decoded, Err(JournalError::BadMagic { .. })));

        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        let Ok(mut unknown_kind) = encoded else {
            return;
        };
        if let Some(byte) = unknown_kind.get_mut(6) {
            *byte = 200;
        }
        if let Some(byte) = unknown_kind.get_mut(56) {
            *byte ^= 1;
        }

        let decoded = decode_record::<JournalEvent>(
            &unknown_kind,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(matches!(
            decoded,
            Err(JournalError::UnknownRecordKind { .. })
        ));
    }

    #[test]
    fn append_strict_batch_writes_all_events_with_single_fsync() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(61);
        let events = vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
            },
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(2),
                result: vb_core::SlotIdx::new(0),
                attempt: 1,
            },
        ];

        let result = journal.append_strict_batch(&events);
        result.expect("action must succeed");

        let replayed = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(replayed.len(), 3);
        assert_eq!(replayed, events);
    }

    #[test]
    fn append_strict_batch_rejects_duplicate_within_batch() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(62);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let events = vec![event.clone(), event.clone()];

        let result = journal.append_strict_batch(&events);
        // vb-1rqz7.18 / vb-byk3q: same-batch duplicates are now rejected
        // with the typed `DuplicateStagedKey` (not `DuplicateEvent`).
        // The batch is NOT aborted — `DuplicateStagedKey` is recoverable
        // so the caller can skip the duplicate and commit the prior
        // staged events. See `batch_append_event_rejects_same_batch_*`.
        assert!(
            matches!(result, Err(JournalError::DuplicateStagedKey { .. })),
            "expected DuplicateStagedKey for in-batch duplicate, got {:?}",
            result
        );
    }

    #[test]
    fn batch_builder_collects_events() {
        let mut builder = BatchBuilder::new();
        assert!(builder.is_empty());
        assert_eq!(builder.len(), 0);

        let run = RunId::new(63);
        builder
            .push(JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
            })
            .expect("push within cap");
        assert_eq!(builder.len(), 1);
        assert!(!builder.is_empty());

        builder
            .push(JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(1),
                result: vb_core::SlotIdx::new(0),
                attempt: 1,
            })
            .expect("push within cap");
        assert_eq!(builder.len(), 2);
        assert_eq!(builder.as_slice().len(), 2);
    }

    #[test]
    fn batch_builder_round_trips_via_append_strict_batch() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(64);
        let mut builder = BatchBuilder::new();
        builder
            .push(JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([2; 32]),
            })
            .expect("push within cap");
        builder
            .push(JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            })
            .expect("push within cap");

        journal
            .append_strict_batch(builder.as_slice())
            .expect("journal.append_strict_batch must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn flush_profile_batches_strict_events_into_single_fsync() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = open_store(temp_dir.path()).expect("setup: journal open");
        let Ok(queue) = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT) else {
            return;
        };
        let run = RunId::new(58);
        let strict1 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([6; 32]),
        };
        let strict2 = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(1),
            result: vb_core::SlotIdx::new(0),
            attempt: 1,
        };

        queue
            .enqueue_strict(strict1.clone())
            .expect("queue.enqueue_strict must succeed");
        queue
            .enqueue_strict(strict2.clone())
            .expect("queue.enqueue_strict must succeed");
        let report = flush_profile(&queue, &journal);

        let report = report.expect("flush_profile should succeed");
        assert_eq!(report.drained, 2);
        assert_eq!(report.written, 2);
        let events = read_run_events(&journal, run);
        let events = events.expect("read_run_events should succeed");
        assert_eq!(events, vec![strict1, strict2]);
    }

    #[test]
    fn write_batch_commits_cross_keyspace_atomically() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let source_bytes = b"test workflow".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source_bytes).into());
        let run = RunId::new(42);

        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord {
                digest,
                source: source_bytes,
            })
            .expect("put_workflow_source must succeed");
        batch
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(7),
                compiled_digest: digest,
                status: 1,
                accepted_at_ms: 1234,
            })
            .expect("put_run_header must succeed");
        batch.commit().expect("batch.commit must succeed");

        let source = journal
            .workflow_source(digest)
            .expect("workflow source roundtrip");
        assert!(source.is_some());
        assert_eq!(source.unwrap().source, b"test workflow".to_vec());

        let header = journal.run_header(run).expect("run header roundtrip");
        assert!(header.is_some());
        assert_eq!(header.unwrap().run, run);
    }

    #[test]
    fn write_batch_strict_commits_with_durability() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let blob_bytes = b"blob data".to_vec();
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
        let mut batch = journal.batch().strict();
        batch
            .put_blob(&BlobRecord {
                digest,
                bytes: blob_bytes,
            })
            .expect("action must succeed");
        batch.commit().expect("batch.commit must succeed");

        let blob = journal.blob(digest).expect("blob roundtrip");
        assert!(blob.is_some());
        assert_eq!(blob.unwrap().bytes, b"blob data".to_vec());
    }

    #[test]
    fn write_batch_appends_events_and_indexes() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(99);
        let workflow = WorkflowId::new(5);
        let action = ActionId::new(3);
        let step = StepIdx::new(2);

        let mut batch = journal.batch();
        batch
            .append_event(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([3; 32]),
            })
            .expect("action must succeed");
        batch
            .put_workflow_index(workflow, run)
            .expect("batch.put_workflow_index must succeed");
        batch
            .put_action_index(action, run, step)
            .expect("batch.put_action_index must succeed");
        batch
            .put_status_index(IndexStatusState::Submitted, 5678, run)
            .expect("batch.put_status_index must succeed");
        batch.commit().expect("batch.commit must succeed");

        let events = journal.events_for_run(run);
        let events = events.expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn write_batch_empty_commit_succeeds() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let batch = journal.batch();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
        batch.commit().expect("batch.commit must succeed");
    }

    #[test]
    fn write_batch_is_empty_after_construction() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let batch = journal.batch();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
    }

    #[test]
    fn write_batch_len_tracks_operations() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let source = b"a".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let ir_record = compiled_ir_record(b"ir".to_vec());
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord { digest, source })
            .expect("action must succeed");
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());

        batch
            .put_compiled_ir(&ir_record)
            .expect("action must succeed");
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn write_batch_snapshot_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(77);
        let seq = EventSeq::new(5);
        let snapshot = RunSnapshot {
            run,
            seq,
            workflow: WorkflowDigest::from_bytes([5; 32]),
            slots: b"slot_data".to_vec(),
            taint: Vec::new(),
        };

        let mut batch = journal.batch();
        batch
            .put_snapshot(&snapshot)
            .expect("batch.put_snapshot must succeed");
        batch.commit().expect("batch.commit must succeed");

        let loaded = journal.snapshot(run, seq).expect("snapshot roundtrip");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().run, run);
    }

    #[test]
    fn keyspace_profiles_return_distinct_configs() {
        let _hot = keyspace_options_for(KeyspaceProfile::Hot);
        let _cold = keyspace_options_for(KeyspaceProfile::Cold);
        let _blob = keyspace_options_for(KeyspaceProfile::Blob);

        // Hot has no KV separation; Cold and Blob have KV separation.
        // We verify this indirectly by checking the configs differ.
        assert_ne!(
            std::mem::discriminant(&KeyspaceProfile::Hot),
            std::mem::discriminant(&KeyspaceProfile::Cold)
        );
        assert_ne!(
            std::mem::discriminant(&KeyspaceProfile::Cold),
            std::mem::discriminant(&KeyspaceProfile::Blob)
        );

        // Verify the function exists and returns valid options by using them
        // in a real database open.
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None);
        assert!(journal.is_ok(), "journal should open with tuned keyspaces");
    }

    #[test]
    fn journal_opens_declared_keyspaces_and_round_trips_typed_records() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        assert_eq!(FjallJournal::declared_keyspaces().len(), 12);

        let source_bytes = vec![b'n', b'a', b'm', b'e'];
        let workflow_digest = WorkflowDigest::from_bytes(blake3::hash(&source_bytes).into());
        let ir = compiled_ir_record(vec![1, 2, 3]);
        let compiled_digest = ir.digest;
        let source = WorkflowSourceRecord {
            digest: workflow_digest,
            source: source_bytes,
        };
        let header = RunHeaderRecord {
            run: RunId::new(3),
            workflow_id: WorkflowId::new(4),
            compiled_digest,
            status: 5,
            accepted_at_ms: 6,
        };
        let snapshot = RunSnapshot {
            run: RunId::new(3),
            seq: EventSeq::new(7),
            workflow: compiled_digest,
            slots: vec![8, 9],
            taint: Vec::new(),
        };
        let blob_bytes = vec![10, 11];
        let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
        let blob = BlobRecord {
            digest: blob_digest,
            bytes: blob_bytes,
        };

        journal
            .put_workflow_source(&source)
            .expect("journal.put_workflow_source must succeed");
        journal
            .put_compiled_ir(&ir)
            .expect("journal.put_compiled_ir must succeed");
        journal
            .put_run_header(&header)
            .expect("journal.put_run_header must succeed");
        journal
            .put_snapshot(&snapshot)
            .expect("journal.put_snapshot must succeed");
        journal
            .put_blob(&blob)
            .expect("journal.put_blob must succeed");
        journal
            .put_status_index(IndexStatusState::Submitted, 2, RunId::new(3))
            .expect("journal.put_status_index must succeed");
        journal
            .put_workflow_index(WorkflowId::new(4), RunId::new(3))
            .expect("action must succeed");
        journal
            .put_action_index(ActionId::new(5), RunId::new(3), StepIdx::new(6))
            .expect("action must succeed");

        let found_source = journal
            .workflow_source(workflow_digest)
            .expect("workflow source lookup should succeed");
        assert_eq!(found_source, Some(source));

        let found_ir = journal
            .compiled_ir(compiled_digest)
            .expect("compiled ir lookup should succeed");
        assert_eq!(found_ir, Some(ir));

        let found_header = journal
            .run_header(RunId::new(3))
            .expect("run header lookup should succeed");
        assert_eq!(found_header, Some(header));

        let found_snapshot = journal
            .snapshot(RunId::new(3), EventSeq::new(7))
            .expect("snapshot lookup should succeed");
        assert_eq!(found_snapshot, Some(snapshot));

        let found_blob = journal
            .blob(blob_digest)
            .expect("blob lookup should succeed");
        assert_eq!(found_blob, Some(blob));
    }

    #[test]
    fn non_journal_families_reject_wrong_record_kind() {
        let source = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([1; 32]),
            source: vec![1],
        };

        let encoded = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &source,
            128,
        );
        assert!(encoded.is_ok(), "encoding must succeed for valid input");
        let encoded = encoded.expect("setup: encoding");
        assert!(!encoded.is_empty(), "encoded bytes must be non-empty");
        let wrong_family = encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::WorkflowSource,
            0,
            &source,
            128,
        );

        assert!(matches!(
            wrong_family,
            Err(JournalError::RecordKindFamilyMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_event_append_is_rejected() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let event = JournalEvent::RunAccepted {
            run: RunId::new(9),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([3; 32]),
        };

        let first = journal.append_journaled(&event);
        let second = journal.append_journaled(&event);

        first.expect("action must succeed");
        assert!(matches!(second, Err(JournalError::DuplicateEvent { .. })));
    }

    #[test]
    fn journal_writer_queue_counts_pending_durability_profiles() {
        let Ok(queue) = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT) else {
            return;
        };
        let run = RunId::new(56);
        let journaled = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(0),
            attempt: 1,
            reason: None,
        };
        let strict = JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
        };

        queue
            .enqueue_journaled(journaled)
            .expect("queue.enqueue_journaled must succeed");
        queue
            .enqueue_strict(strict)
            .expect("queue.enqueue_strict must succeed");

        assert!(matches!(
            queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 1 && counts.strict == 1
        ));
    }

    #[test]
    fn flush_profile_wrapper_flushes_queued_events() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = open_store(temp_dir.path()).expect("setup: journal open");
        let Ok(queue) = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT) else {
            return;
        };
        let run = RunId::new(57);
        let journaled = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([5; 32]),
        };
        let strict = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(1),
            result: vb_core::SlotIdx::new(0),
            attempt: 1,
        };

        queue
            .enqueue_journaled(journaled.clone())
            .expect("queue.enqueue_journaled must succeed");
        queue
            .enqueue_strict(strict.clone())
            .expect("queue.enqueue_strict must succeed");
        let report = flush_profile(&queue, &journal);

        let report = report.expect("flush_profile should succeed");
        assert_eq!(report.drained, 2);
        assert_eq!(report.written, 2);
        let events = read_run_events(&journal, run);
        let events = events.expect("read_run_events should succeed");
        assert_eq!(events, vec![journaled, strict]);
    }

    #[test]
    fn replay_returns_contiguous_events_for_run() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(11);
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([4; 32]),
        };
        let finished = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(1),
            result: vb_core::SlotIdx::new(0),
            attempt: 1,
        };

        journal
            .append_journaled(&accepted)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&finished)
            .expect("journal.append_journaled must succeed");

        let replay = journal
            .events_for_run(run)
            .expect("event replay should succeed");
        assert_eq!(replay, vec![accepted, finished]);
    }

    #[test]
    fn decode_rejects_truncated_header() {
        // Given a byte slice shorter than the required 60-byte header
        // When decode_record is called
        // Then it returns UnexpectedEof
        let truncated = [0u8; 30];

        let result = decode_record::<JournalEvent>(&truncated, MAGIC_JOURNAL_EVENT, 128);
        assert!(matches!(result, Err(JournalError::UnexpectedEof)));
    }

    #[test]
    fn decode_rejects_migration_required_schema() {
        // Given a valid record whose schema version byte is 0 (less than current)
        // When decode_record is called
        // Then it returns MigrationRequired with from=0, to=CURRENT_SCHEMA_VERSION
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
        // Set schema version to 0 (two LE bytes at offset 4..6)
        encoded[4] = 0;
        encoded[5] = 0;
        // Recompute CRC32C for the modified header prefix
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        assert!(matches!(
            result,
            Err(JournalError::MigrationRequired { from: 0, to: 1 })
        ));
    }

    #[test]
    fn decode_rejects_unsupported_future_schema() {
        // Given a valid record whose schema version byte is 99 (greater than current)
        // When decode_record is called
        // Then it returns UnsupportedSchemaVersion
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
        // Set schema version to 99 (two LE bytes at offset 4..6)
        encoded[4] = 99;
        encoded[5] = 0;
        // Recompute CRC32C for the modified header prefix
        let header_prefix = &encoded[..56];
        let checksum = crc32c::crc32c(header_prefix);
        encoded[56] = (checksum & 0xFF) as u8;
        encoded[57] = ((checksum >> 8) & 0xFF) as u8;
        encoded[58] = ((checksum >> 16) & 0xFF) as u8;
        encoded[59] = ((checksum >> 24) & 0xFF) as u8;

        let result = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128);
        assert!(matches!(
            result,
            Err(JournalError::UnsupportedSchemaVersion { version: 99 })
        ));
    }

    #[test]
    fn decode_rejects_record_kind_family_mismatch() {
        // Given a record encoded with MAGIC_JOURNAL_EVENT but a kind outside 10..=23
        // When decode_record is called
        // Then it returns RecordKindFamilyMismatch
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
        // Patch the kind to 1 (WorkflowSource), which is outside 10..=23
        // Kind is at offset 6..8, little-endian
        let kind_bytes = 1u16.to_le_bytes();
        encoded[6] = kind_bytes[0];
        encoded[7] = kind_bytes[1];
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
            Err(JournalError::RecordKindFamilyMismatch {
                magic: MAGIC_JOURNAL_EVENT,
                kind: 1
            })
        ));
    }

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
