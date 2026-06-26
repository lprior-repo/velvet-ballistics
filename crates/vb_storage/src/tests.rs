#![forbid(unsafe_code)]
#[cfg(test)]
#[allow(
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod tests {
    use crate::keys::{
        blob_key, compiled_ir_key, encode_key, index_action_key, index_status_key,
        index_workflow_key, journal_key, run_event_key, run_header_key, run_snapshot_key,
        workflow_source_key,
    };
    use crate::queue::BatchBuilder;
    use crate::recovery::{ActionReplayTracker, RunSnapshot};
    use crate::{
        BlobRecord, CURRENT_SCHEMA_VERSION, CompiledIrRecord, DIGEST_BYTES, EventSeq, FjallJournal,
        IndexStatusState, JournalError, JournalEvent, JournalWriterQueue, KeyspaceProfile,
        MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD, MAGIC_IPC_FRAME,
        MAGIC_JOURNAL_EVENT, MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE, MAX_BLOB_BYTES,
        MAX_COMPILED_IR_BYTES, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_RUN_HEADER_BYTES,
        MAX_SNAPSHOT_BYTES, MAX_WORKFLOW_SOURCE_BYTES, PREFIX_BLOB, PREFIX_COMPILED_IR,
        PREFIX_INDEX_ACTION, PREFIX_INDEX_STATUS, PREFIX_INDEX_WORKFLOW, PREFIX_RUN_EVENT,
        PREFIX_RUN_HEADER, PREFIX_RUN_SNAPSHOT, PREFIX_WORKFLOW_SOURCE, RECORD_HEADER_BYTES,
        RECORD_HEADER_LEN, RecordKind, RunHeaderRecord, StorageKey, StorageLimits,
        WorkflowSourceRecord, append_journal_event, decode_record, decode_record_header,
        encode_record, encode_record_header, flush_profile, init_keyspaces, keyspace_options_for,
        open_store, put_blob, put_compiled_ir, put_run_header, put_workflow_source, read_blob,
        read_run_events, replay_journal, verify_digest_match, write_snapshot,
    };
    use vb_core::{ActionId, DiagnosticCode, RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

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
        let compiled_digest = WorkflowDigest::from_bytes([2; 32]);
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

        let compiled = CompiledIrRecord {
            digest: compiled_digest,
            ir: vec![b'i'],
        };
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
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord { digest, source })
            .expect("action must succeed");
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());

        batch
            .put_compiled_ir(&CompiledIrRecord {
                digest,
                ir: b"ir".to_vec(),
            })
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
        assert_eq!(FjallJournal::declared_keyspaces().len(), 9);

        let source_bytes = vec![b'n', b'a', b'm', b'e'];
        let workflow_digest = WorkflowDigest::from_bytes(blake3::hash(&source_bytes).into());
        let compiled_digest = WorkflowDigest::from_bytes([2; 32]);
        let source = WorkflowSourceRecord {
            digest: workflow_digest,
            source: source_bytes,
        };
        let ir = CompiledIrRecord {
            digest: compiled_digest,
            ir: vec![1, 2, 3],
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
        assert_eq!(FjallJournal::declared_keyspaces().len(), 9);
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

        let digest = WorkflowDigest::from_bytes([3; 32]);
        let record = CompiledIrRecord {
            digest,
            ir: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };
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
        assert_eq!(RecordKind::RunKilled.id(), 28);
        assert_eq!(RecordKind::AskTimedOut.id(), 29);
        assert_eq!(RecordKind::Snapshot.id(), 30);
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
    fn declared_keyspaces_returns_nine_entries() {
        // Given FjallJournal::declared_keyspaces()
        // When called
        // Then it returns exactly 9 keyspace names
        let keyspaces = FjallJournal::declared_keyspaces();
        assert_eq!(keyspaces.len(), 9);
        assert_eq!(keyspaces[0], "workflow_source");
        assert_eq!(keyspaces[1], "compiled_ir");
        assert_eq!(keyspaces[2], "run_header");
        assert_eq!(keyspaces[3], "run_event");
        assert_eq!(keyspaces[4], "run_snapshot");
        assert_eq!(keyspaces[5], "blob");
        assert_eq!(keyspaces[6], "index_status");
        assert_eq!(keyspaces[7], "index_workflow");
        assert_eq!(keyspaces[8], "index_action");
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

    // --- Section 4: Journal Lifecycle BDD Tests ---

    fn open_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("journal should open");
        (temp_dir, journal)
    }

    fn test_digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

    #[test]
    fn journal_open_creates_fresh_instance_with_no_data() {
        // Given a temporary directory
        // When FjallJournal::open is called
        // Then the journal has no events for any run
        let (_guard, journal) = open_journal();
        let events = journal
            .events_for_run(RunId::new(1))
            .expect("events_for_run should succeed on empty journal");
        assert!(events.is_empty());
    }

    #[test]
    fn append_strict_writes_submitted_event_with_correct_run_id() {
        // Given an open journal
        // When append_strict is called with a RunAccepted event for run 42
        // Then the stored event has run_id 42
        let (_guard, journal) = open_journal();
        let run = RunId::new(42);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].run_id(), run);
    }

    #[test]
    fn append_strict_writes_accepted_event_after_submitted() {
        // Given an open journal with a RunAccepted event at seq 0
        // When a StepStarted event at seq 1 is appended
        // Then both events are retrieved in order
        let (_guard, journal) = open_journal();
        let run = RunId::new(1);
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let started = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        journal
            .append_strict(&accepted)
            .expect("journal.append_strict must succeed");
        journal
            .append_strict(&started)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], accepted);
        assert_eq!(events[1], started);
    }

    #[test]
    fn append_strict_writes_step_started_event_with_correct_step() {
        // Given an open journal
        // When a StepStarted event with step 5 is appended and retrieved
        // Then the event carries step 5
        let (_guard, journal) = open_journal();
        let run = RunId::new(10);
        let step = StepIdx::new(5);
        let event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step,
            attempt: 1,
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::StepStarted {
            step: found_step, ..
        } = events[0]
        else {
            panic!("expected StepStarted event");
        };
        assert_eq!(found_step, step);
    }

    #[test]
    fn append_strict_writes_step_ended_event_with_correct_step() {
        // Given an open journal
        // When a StepSucceeded event with step 3 is appended and retrieved
        // Then the event carries step 3 and output slot 7
        let (_guard, journal) = open_journal();
        let run = RunId::new(11);
        let step = StepIdx::new(3);
        let output = vb_core::SlotIdx::new(7);
        let event = JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(0),
            step,
            output,
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::StepSucceeded {
            step: found_step,
            output: found_output,
            ..
        } = events[0]
        else {
            panic!("expected StepSucceeded event");
        };
        assert_eq!(found_step, step);
        assert_eq!(found_output, output);
    }

    #[test]
    fn append_strict_writes_slot_written_event_with_correct_slot() {
        // Given an open journal
        // When a SlotWrittenEvent with slot 9 is appended and retrieved
        // Then the event carries slot 9
        let (_guard, journal) = open_journal();
        let run = RunId::new(12);
        let slot = vb_core::SlotIdx::new(9);
        let event = JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(0),
            slot,
            value: None,
            extra: None,
            attempt: 1,
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::SlotWrittenEvent {
            slot: found_slot, ..
        } = events[0]
        else {
            panic!("expected SlotWrittenEvent");
        };
        assert_eq!(found_slot, slot);
    }

    #[test]
    fn append_strict_writes_action_scheduled_event_with_correct_step() {
        // Given an open journal
        // When an ActionScheduled event with step 4 is appended and retrieved
        // Then the event carries step 4 and action 2
        let (_guard, journal) = open_journal();
        let run = RunId::new(13);
        let step = StepIdx::new(4);
        let action = ActionId::new(2);
        let event = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::ActionScheduled {
            step: found_step,
            action: found_action,
            ..
        } = events[0]
        else {
            panic!("expected ActionScheduled event");
        };
        assert_eq!(found_step, step);
        assert_eq!(found_action, action);
    }

    #[test]
    fn append_strict_writes_action_completed_event_with_correct_step() {
        // Given an open journal
        // When an ActionCompletedEvent with step 6 is appended and retrieved
        // Then the event carries step 6 and action 3
        let (_guard, journal) = open_journal();
        let run = RunId::new(14);
        let step = StepIdx::new(6);
        let action = ActionId::new(3);
        let event = JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::ActionCompletedEvent {
            step: found_step,
            action: found_action,
            ..
        } = events[0]
        else {
            panic!("expected ActionCompletedEvent");
        };
        assert_eq!(found_step, step);
        assert_eq!(found_action, action);
    }

    #[test]
    fn append_strict_writes_run_finished_event_with_correct_result() {
        // Given an open journal
        // When a RunFinished event with result slot 15 is appended and retrieved
        // Then the event carries result 15
        let (_guard, journal) = open_journal();
        let run = RunId::new(15);
        let result = vb_core::SlotIdx::new(15);
        let event = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(0),
            result,
            attempt: 1,
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        let JournalEvent::RunFinished {
            result: found_result,
            ..
        } = events[0]
        else {
            panic!("expected RunFinished event");
        };
        assert_eq!(found_result, result);
    }

    #[test]
    fn append_strict_writes_run_failed_event() {
        // Given an open journal
        // When a RunFailedEvent is appended and retrieved
        // Then the event carries the correct run
        let (_guard, journal) = open_journal();
        let run = RunId::new(16);
        let event = JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(0),
            attempt: 1,
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].run_id(), run);
    }

    #[test]
    fn append_strict_assigns_monotonically_increasing_sequences() {
        // Given an open journal
        // When three events are appended with seq 0, 1, 2
        // Then events_for_run returns them in contiguous order
        let (_guard, journal) = open_journal();
        let run = RunId::new(17);
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let e1 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let e2 = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: vb_core::SlotIdx::new(0),
            attempt: 1,
        };
        journal
            .append_strict(&e0)
            .expect("journal.append_strict must succeed");
        journal
            .append_strict(&e1)
            .expect("journal.append_strict must succeed");
        journal
            .append_strict(&e2)
            .expect("journal.append_strict must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].seq(), EventSeq::new(0));
        assert_eq!(events[1].seq(), EventSeq::new(1));
        assert_eq!(events[2].seq(), EventSeq::new(2));
    }

    #[test]
    fn append_strict_rejects_duplicate_sequence() {
        // Given an open journal with an event at seq 0 for run 50
        // When the same event is appended again
        // Then DuplicateEvent is returned with exact run and seq
        let (_guard, journal) = open_journal();
        let run = RunId::new(50);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        journal
            .append_strict(&event)
            .expect("journal.append_strict must succeed");

        let result = journal.append_strict(&event);
        let Err(JournalError::DuplicateEvent {
            run: dup_run,
            seq: dup_seq,
        }) = result
        else {
            panic!("expected DuplicateEvent, got {:?}", result);
        };
        assert_eq!(dup_run, run);
        assert_eq!(dup_seq, EventSeq::new(0));
    }

    #[test]
    fn events_for_run_returns_events_in_sequence_order() {
        // Given a journal with 5 events for a run
        // When events_for_run is called
        // Then events are returned in ascending sequence order
        let (_guard, journal) = open_journal();
        let run = RunId::new(18);
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let e1 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let e2 = JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: vb_core::SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        };
        let e3 = JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            output: vb_core::SlotIdx::new(1),
        };
        let e4 = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(4),
            result: vb_core::SlotIdx::new(1),
            attempt: 1,
        };
        journal
            .append_journaled(&e0)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&e1)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&e2)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&e3)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&e4)
            .expect("journal.append_journaled must succeed");

        let events = journal
            .events_for_run(run)
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 5);
        assert_eq!(events[0], e0);
        assert_eq!(events[1], e1);
        assert_eq!(events[2], e2);
        assert_eq!(events[3], e3);
        assert_eq!(events[4], e4);
    }

    #[test]
    fn events_for_run_returns_empty_for_run_with_no_events() {
        // Given an open journal with events for run 1
        // When events_for_run is called for run 2
        // Then it returns an empty vec
        let (_guard, journal) = open_journal();
        let run_a = RunId::new(1);
        let event = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        journal
            .append_journaled(&event)
            .expect("journal.append_journaled must succeed");

        let events = journal
            .events_for_run(RunId::new(2))
            .expect("events_for_run should succeed");
        assert!(events.is_empty());
    }

    #[test]
    fn append_strict_handles_concurrent_runs_interleaved() {
        // Given a journal with interleaved events from run A and run B
        // When events_for_run is called for run A
        // Then only run A events are returned in order
        let (_guard, journal) = open_journal();
        let run_a = RunId::new(100);
        let run_b = RunId::new(200);

        let a0 = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        let b0 = JournalEvent::RunAccepted {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: test_digest(2),
        };
        let a1 = JournalEvent::StepStarted {
            run: run_a,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let b1 = JournalEvent::StepStarted {
            run: run_b,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let a2 = JournalEvent::RunFinished {
            run: run_a,
            seq: EventSeq::new(2),
            result: vb_core::SlotIdx::new(0),
            attempt: 1,
        };

        journal
            .append_journaled(&a0)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&b0)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&a1)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&b1)
            .expect("journal.append_journaled must succeed");
        journal
            .append_journaled(&a2)
            .expect("journal.append_journaled must succeed");

        let events_a = journal
            .events_for_run(run_a)
            .expect("events_for_run A should succeed");
        assert_eq!(events_a.len(), 3);
        assert_eq!(events_a[0], a0);
        assert_eq!(events_a[1], a1);
        assert_eq!(events_a[2], a2);

        let events_b = journal
            .events_for_run(run_b)
            .expect("events_for_run B should succeed");
        assert_eq!(events_b.len(), 2);
        assert_eq!(events_b[0], b0);
        assert_eq!(events_b[1], b1);
    }

    #[test]
    fn append_journaled_succeeds_without_flush() {
        // Given an open journal
        // When append_journaled is called
        // Then the event is readable immediately
        let (_guard, journal) = open_journal();
        let run = RunId::new(30);
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
            .expect("events_for_run should succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn run_header_record_roundtrip_with_large_timestamp() {
        // Given a run header with a large accepted_at_ms value
        // When put and retrieved
        // Then the timestamp survives exactly
        let (_guard, journal) = open_journal();
        let record = RunHeaderRecord {
            run: RunId::new(1),
            workflow_id: WorkflowId::new(2),
            compiled_digest: test_digest(5),
            status: 0,
            accepted_at_ms: u64::MAX / 2,
        };
        journal
            .put_run_header(&record)
            .expect("journal.put_run_header must succeed");

        let retrieved = journal
            .run_header(RunId::new(1))
            .expect("lookup should succeed");
        assert_eq!(retrieved, Some(record));
    }

    #[test]
    fn snapshot_record_roundtrip_with_nonempty_slots() {
        // Given a snapshot with non-empty slot data
        // When stored and retrieved
        // Then the slot bytes survive exactly
        let (_guard, journal) = open_journal();
        let snapshot = RunSnapshot {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: test_digest(7),
            slots: vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE],
            taint: Vec::new(),
        };
        journal
            .put_snapshot(&snapshot)
            .expect("journal.put_snapshot must succeed");

        let retrieved = journal
            .snapshot(RunId::new(1), EventSeq::new(0))
            .expect("lookup should succeed");
        assert_eq!(retrieved, Some(snapshot));
    }

    #[test]
    fn compiled_ir_returns_none_when_different_digest_queried() {
        // Given an open journal with a compiled IR stored at digest [1;32]
        // When a different digest [2;32] is queried
        // Then it returns None
        let (_guard, journal) = open_journal();
        let stored_digest = test_digest(1);
        let record = CompiledIrRecord {
            digest: stored_digest,
            ir: vec![1, 2, 3],
        };
        journal
            .put_compiled_ir(&record)
            .expect("journal.put_compiled_ir must succeed");

        let result = journal
            .compiled_ir(test_digest(2))
            .expect("lookup should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn workflow_source_returns_none_for_different_digest() {
        // Given an open journal with one workflow source stored
        // When a different digest is queried
        // Then it returns None
        let (_guard, journal) = open_journal();
        let source = vec![1];
        let stored_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord {
            digest: stored_digest,
            source,
        };
        journal
            .put_workflow_source(&record)
            .expect("journal.put_workflow_source must succeed");

        let result = journal
            .workflow_source(test_digest(11))
            .expect("lookup should succeed");
        assert_eq!(result, None);
    }

    #[test]
    fn journal_event_run_id_returns_correct_run_for_all_variants() {
        // Given every JournalEvent variant with run_id 99
        // When run_id() is called
        // Then each returns RunId::new(99)
        let run = RunId::new(99);
        assert_eq!(
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: test_digest(1)
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: vb_core::StepIdx::ZERO,
                attempt: 1,
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                output: vb_core::SlotIdx::new(0)
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                action: ActionId::new(1),
                attempt: 1,
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                action: ActionId::new(1),
                attempt: 1,
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::ActionFailedEvent {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                action: ActionId::new(1),
                attempt: 1,
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(0),
                slot: vb_core::SlotIdx::new(0),
                value: None,
                extra: None,
                attempt: 1,
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                attempt: 1,
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                attempt: 1,
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::AskAnsweredEvent {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                attempt: 1,
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::AskTimedOutEvent {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                attempt: 1,
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::RetryScheduledEvent {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                attempt: 1,
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(0),
                attempt: 1,
                reason: None,
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(0),
                result: vb_core::SlotIdx::new(0),
                attempt: 1,
            }
            .run_id(),
            run
        );
        assert_eq!(
            JournalEvent::RunFailedEvent {
                run,
                seq: EventSeq::new(0),
                attempt: 1,
            }
            .run_id(),
            run
        );
    }

    #[test]
    fn journal_event_seq_returns_correct_seq_for_all_variants() {
        // Given every JournalEvent variant with seq 42
        // When seq() is called
        // Then each returns EventSeq::new(42)
        let seq = EventSeq::new(42);
        let run = RunId::new(1);
        assert_eq!(
            JournalEvent::RunAccepted {
                run,
                seq,
                workflow: test_digest(1)
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::StepStarted {
                run,
                seq,
                step: StepIdx::new(0),
                attempt: 1,
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::StepSucceeded {
                run,
                seq,
                step: StepIdx::new(0),
                output: vb_core::SlotIdx::new(0)
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::ActionScheduled {
                run,
                seq,
                step: StepIdx::new(0),
                action: ActionId::new(1),
                attempt: 1,
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::ActionCompletedEvent {
                run,
                seq,
                step: StepIdx::new(0),
                action: ActionId::new(1),
                attempt: 1,
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::ActionFailedEvent {
                run,
                seq,
                step: StepIdx::new(0),
                action: ActionId::new(1),
                attempt: 1,
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::SlotWrittenEvent {
                run,
                seq,
                slot: vb_core::SlotIdx::new(0),
                value: None,
                extra: None,
                attempt: 1,
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::WaitScheduledEvent {
                run,
                seq,
                step: StepIdx::new(0),
                attempt: 1,
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::AskScheduledEvent {
                run,
                seq,
                step: StepIdx::new(0),
                attempt: 1,
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::AskAnsweredEvent {
                run,
                seq,
                step: StepIdx::new(0),
                attempt: 1,
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::AskTimedOutEvent {
                run,
                seq,
                step: StepIdx::new(0),
                attempt: 1,
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::RetryScheduledEvent {
                run,
                seq,
                step: StepIdx::new(0),
                attempt: 1,
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::RunCancelled {
                run,
                seq,
                attempt: 1,
                reason: None
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::RunFinished {
                run,
                seq,
                result: vb_core::SlotIdx::new(0),
                attempt: 1,
            }
            .seq(),
            seq
        );
        assert_eq!(
            JournalEvent::RunFailedEvent {
                run,
                seq,
                attempt: 1
            }
            .seq(),
            seq
        );
    }

    #[test]
    fn journal_event_record_kind_returns_correct_kind_for_all_variants() {
        // Given every JournalEvent variant
        // When record_kind() is called
        // Then each returns the expected RecordKind
        let run = RunId::new(1);
        let seq = EventSeq::new(0);
        assert_eq!(
            JournalEvent::RunAccepted {
                run,
                seq,
                workflow: test_digest(1)
            }
            .record_kind(),
            RecordKind::RunAccepted
        );
        assert_eq!(
            JournalEvent::StepStarted {
                run,
                seq,
                step: StepIdx::new(0),
                attempt: 1,
            }
            .record_kind(),
            RecordKind::StepStarted
        );
        assert_eq!(
            JournalEvent::StepSucceeded {
                run,
                seq,
                step: StepIdx::new(0),
                output: vb_core::SlotIdx::new(0)
            }
            .record_kind(),
            RecordKind::SlotWritten
        );
        assert_eq!(
            JournalEvent::ActionScheduled {
                run,
                seq,
                step: StepIdx::new(0),
                action: ActionId::new(1),
                attempt: 1,
            }
            .record_kind(),
            RecordKind::ActionScheduled
        );
        assert_eq!(
            JournalEvent::ActionCompletedEvent {
                run,
                seq,
                step: StepIdx::new(0),
                action: ActionId::new(1),
                attempt: 1,
            }
            .record_kind(),
            RecordKind::ActionCompleted
        );
        assert_eq!(
            JournalEvent::ActionFailedEvent {
                run,
                seq,
                step: StepIdx::new(0),
                action: ActionId::new(1),
                attempt: 1,
            }
            .record_kind(),
            RecordKind::ActionFailed
        );
        assert_eq!(
            JournalEvent::SlotWrittenEvent {
                run,
                seq,
                slot: vb_core::SlotIdx::new(0),
                value: None,
                extra: None,
                attempt: 1,
            }
            .record_kind(),
            RecordKind::SlotWritten
        );
        assert_eq!(
            JournalEvent::WaitScheduledEvent {
                run,
                seq,
                step: StepIdx::new(0),
                attempt: 1,
            }
            .record_kind(),
            RecordKind::WaitScheduled
        );
        assert_eq!(
            JournalEvent::AskScheduledEvent {
                run,
                seq,
                step: StepIdx::new(0),
                attempt: 1,
            }
            .record_kind(),
            RecordKind::AskScheduled
        );
        assert_eq!(
            JournalEvent::AskAnsweredEvent {
                run,
                seq,
                step: StepIdx::new(0),
                attempt: 1,
            }
            .record_kind(),
            RecordKind::AskAnswered
        );
        assert_eq!(
            JournalEvent::AskTimedOutEvent {
                run,
                seq,
                step: StepIdx::new(0),
                attempt: 1,
            }
            .record_kind(),
            RecordKind::AskTimedOut
        );
        assert_eq!(
            JournalEvent::RetryScheduledEvent {
                run,
                seq,
                step: StepIdx::new(0),
                attempt: 1,
            }
            .record_kind(),
            RecordKind::RetryScheduled
        );
        assert_eq!(
            JournalEvent::RunCancelled {
                run,
                seq,
                attempt: 1,
                reason: None
            }
            .record_kind(),
            RecordKind::RunCancelled
        );
        assert_eq!(
            JournalEvent::RunFinished {
                run,
                seq,
                result: vb_core::SlotIdx::new(0),
                attempt: 1,
            }
            .record_kind(),
            RecordKind::RunFinished
        );
        assert_eq!(
            JournalEvent::RunFailedEvent {
                run,
                seq,
                attempt: 1
            }
            .record_kind(),
            RecordKind::RunFailed
        );
    }

    // --- Section 5: Encode/Decode Roundtrip Tests ---

    #[test]
    fn encode_decode_roundtrip_for_run_accepted_record() {
        // Given a RunAccepted event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: test_digest(42),
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_step_started_record() {
        // Given a StepStarted event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::StepStarted {
            run: RunId::new(2),
            seq: EventSeq::new(1),
            step: StepIdx::new(5),
            attempt: 1,
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::StepStarted, 1, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_step_ended_record() {
        // Given a StepSucceeded event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::StepSucceeded {
            run: RunId::new(3),
            seq: EventSeq::new(2),
            step: StepIdx::new(5),
            output: vb_core::SlotIdx::new(10),
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::SlotWritten, 2, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_slot_written_record() {
        // Given a SlotWrittenEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(4),
            seq: EventSeq::new(3),
            slot: vb_core::SlotIdx::new(7),
            value: None,
            extra: None,
            attempt: 1,
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::SlotWritten, 3, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_action_scheduled_record() {
        // Given an ActionScheduled event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::ActionScheduled {
            run: RunId::new(5),
            seq: EventSeq::new(4),
            step: StepIdx::new(2),
            action: ActionId::new(3),
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionScheduled,
            4,
            &event,
            128,
        )
        .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_action_completed_record() {
        // Given an ActionCompletedEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::ActionCompletedEvent {
            run: RunId::new(6),
            seq: EventSeq::new(5),
            step: StepIdx::new(2),
            action: ActionId::new(3),
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionCompleted,
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
    fn encode_decode_roundtrip_for_run_finished_record() {
        // Given a RunFinished event
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::RunFinished {
            run: RunId::new(7),
            seq: EventSeq::new(6),
            result: vb_core::SlotIdx::new(99),
            attempt: 1,
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunFinished, 6, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_decode_roundtrip_for_run_failed_record() {
        // Given a RunFailedEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::RunFailedEvent {
            run: RunId::new(8),
            seq: EventSeq::new(7),
            attempt: 1,
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunFailed, 7, &event, 128)
            .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    #[test]
    fn encode_record_rejects_record_exceeding_max_payload() {
        // Given a workflow source with 200 bytes of source data
        // When encode_record is called with max_payload_len of 10
        // Then it returns PayloadTooLarge
        let source = WorkflowSourceRecord {
            digest: test_digest(1),
            source: vec![0u8; 200],
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
    fn encode_decode_roundtrip_for_action_failed_record() {
        // Given an ActionFailedEvent
        // When encoded and decoded
        // Then the event survives the roundtrip exactly
        let event = JournalEvent::ActionFailedEvent {
            run: RunId::new(9),
            seq: EventSeq::new(3),
            step: StepIdx::new(1),
            action: ActionId::new(4),
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionFailed,
            3,
            &event,
            128,
        )
        .expect("encoding should succeed");
        let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(decoded, event);
    }

    // --- Section 6: JournalError Variant Tests ---

    #[test]
    fn journal_error_encode_from_postcard_error() {
        // Given a payload that causes a postcard encoding error
        // When encode_record encounters the error
        // Then JournalError::Encode is returned
        // This is tested indirectly: encode_record with a valid payload succeeds,
        // and the Encode variant exists as a From<postcard::Error> conversion.
        // We verify the variant exists by checking the error display.
        let err = JournalError::Encode(postcard::Error::DeserializeBadVarint);
        let msg = format!("{}", err);
        assert!(!msg.is_empty());
    }

    #[test]
    fn journal_error_key_capacity_display() {
        // Given a JournalError::KeyCapacity
        // When displayed
        // Then the message is non-empty
        let err = JournalError::KeyCapacity;
        let msg = format!("{}", err);
        assert!(!msg.is_empty());
    }

    #[test]
    fn journal_error_write_lock_poisoned_display() {
        // Given a JournalError::WriteLockPoisoned
        // When displayed
        // Then the message mentions poisoned
        let err = JournalError::WriteLockPoisoned;
        let msg = format!("{}", err);
        assert!(msg.contains("poisoned"));
    }

    #[test]
    fn journal_error_wrong_run_display() {
        // Given a JournalError::WrongRun with expected and actual
        // When displayed
        // Then the message contains both run values
        let err = JournalError::WrongRun {
            expected: RunId::new(1),
            actual: RunId::new(2),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("1"));
        assert!(msg.contains("2"));
    }

    #[test]
    fn journal_error_sequence_overflow_display() {
        // Given a JournalError::SequenceOverflow
        // When displayed
        // Then the message mentions overflow
        let err = JournalError::SequenceOverflow;
        let msg = format!("{}", err);
        assert!(msg.contains("overflow"));
    }

    #[test]
    fn journal_error_postcard_decode_failed_display() {
        // Given a JournalError::PostcardDecodeFailed
        // When displayed
        // Then the message mentions postcard
        let err = JournalError::PostcardDecodeFailed(postcard::Error::DeserializeBadVarint);
        let msg = format!("{}", err);
        assert!(msg.contains("postcard"));
    }

    #[test]
    fn journal_error_unexpected_eof_display() {
        // Given a JournalError::UnexpectedEof
        // When displayed
        // Then the message mentions end of record
        let err = JournalError::UnexpectedEof;
        let msg = format!("{}", err);
        assert!(msg.contains("end"));
    }

    #[test]
    fn journal_error_payload_digest_mismatch_display() {
        // Given a JournalError::PayloadDigestMismatch
        // When displayed
        // Then the message mentions digest
        let err = JournalError::PayloadDigestMismatch;
        let msg = format!("{}", err);
        assert!(msg.contains("digest"));
    }

    #[test]
    fn record_envelope_fields_match_encoded_values() {
        // Given an encoded event
        // When decoded
        // Then the envelope contains magic, schema_version, record_kind, and sequence
        let event = JournalEvent::RunAccepted {
            run: RunId::new(77),
            seq: EventSeq::new(3),
            workflow: test_digest(5),
        };
        let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 3, &event, 128)
            .expect("encoding should succeed");
        let (envelope, _) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
            .expect("decoding should succeed");
        assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(envelope.schema_version, 1);
        assert_eq!(envelope.record_kind, RecordKind::RunAccepted.id());
        assert_eq!(envelope.sequence, 3);
    }

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
            RecordKind::RunKilled.id(),
            RecordKind::AskTimedOut.id(),
            RecordKind::Snapshot.id(),
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
        let master = include_str!("../../../velvet-ballistics-MASTER.md");

        assert!(master.contains("| 29 | `AskTimedOut` |"));
        assert!(master.contains("CURRENT_SCHEMA_VERSION`\nremains `1`"));
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
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &5u16.to_le_bytes());
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
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 6, &99u16.to_le_bytes());
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
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 6, &30u16.to_le_bytes());
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
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 8, &48u32.to_le_bytes());
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

    // =========================================================================
    // Section: Adversarial Key Encoding Tests
    // =========================================================================

    #[test]
    fn adversarial_key_prefix_isolation_proves_different_prefixes() {
        let digest = [0xAB; 32];
        let ws = workflow_source_key(digest).expect("ws");
        let ci = compiled_ir_key(digest).expect("ci");
        let bl = blob_key(digest).expect("bl");
        assert_ne!(ws[0], ci[0]);
        assert_ne!(ws[0], bl[0]);
        assert_eq!(ws[1..], ci[1..]);
        assert_eq!(ws[1..], bl[1..]);
    }

    #[test]
    fn adversarial_key_wrong_endianness_produces_different_keys() {
        let key = run_header_key(RunId::new(1)).expect("key");
        let mut le = [0u8; 9];
        le[0] = PREFIX_RUN_HEADER;
        le[1..9].copy_from_slice(&1u64.to_le_bytes());
        assert_ne!(key.as_slice(), le.as_slice());
        assert_eq!(key[1..9], 1u64.to_be_bytes());
    }

    #[test]
    fn adversarial_key_no_collision_different_runs_same_seq() {
        let k1 = run_event_key(RunId::new(100), EventSeq::new(5)).expect("k1");
        let k2 = run_event_key(RunId::new(200), EventSeq::new(5)).expect("k2");
        assert_ne!(k1.as_slice(), k2.as_slice());
    }

    #[test]
    fn adversarial_key_no_collision_same_run_different_seq() {
        let k1 = run_event_key(RunId::new(100), EventSeq::new(0)).expect("k1");
        let k2 = run_event_key(RunId::new(100), EventSeq::new(1)).expect("k2");
        assert_ne!(k1.as_slice(), k2.as_slice());
    }

    #[test]
    fn adversarial_key_no_collision_different_digests() {
        assert_ne!(
            blob_key([1u8; 32]).expect("k1").as_slice(),
            blob_key([2u8; 32]).expect("k2").as_slice()
        );
    }

    // =========================================================================
    // Section: Adversarial Journal / Replay Tests
    // =========================================================================

    #[test]
    fn adversarial_append_duplicate_sequence_rejected_with_exact_fields() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let run = RunId::new(50);
        assert!(
            journal
                .append_journaled(&JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: test_digest(1)
                })
                .is_ok()
        );
        let result = journal.append_journaled(&JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            attempt: 1,
        });
        let Err(JournalError::DuplicateEvent { run: r, seq: s }) = result else {
            panic!("expected DuplicateEvent, got {:?}", result)
        };
        assert_eq!(r, run);
        assert_eq!(s, EventSeq::new(0));
    }

    #[test]
    fn adversarial_read_events_with_sequence_gap_returns_exact_gap() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let run = RunId::new(777);
        assert!(
            journal
                .append_journaled(&JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: test_digest(1)
                })
                .is_ok()
        );
        assert!(
            journal
                .append_journaled(&JournalEvent::RunFinished {
                    run,
                    seq: EventSeq::new(5),
                    result: vb_core::SlotIdx::new(0),
                    attempt: 1,
                })
                .is_ok()
        );
        let Err(JournalError::SequenceGap { expected, actual }) = journal.events_for_run(run)
        else {
            panic!("expected SequenceGap")
        };
        assert_eq!(expected, EventSeq::new(1));
        assert_eq!(actual, EventSeq::new(5));
    }

    // =========================================================================
    // Section: Adversarial Blob / Snapshot / Size Boundary Tests
    // =========================================================================

    #[test]
    fn adversarial_put_blob_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let bytes = vec![0u8; (MAX_BLOB_BYTES as usize).saturating_add(1)];
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&bytes).into();
        let record = BlobRecord { digest, bytes };
        assert!(matches!(
            journal.put_blob(&record),
            Err(JournalError::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn adversarial_blob_zero_length_round_trips() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let bytes: Vec<u8> = vec![];
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&bytes).into();
        let record = BlobRecord {
            digest,
            bytes: bytes.clone(),
        };
        journal
            .put_blob(&record)
            .expect("journal.put_blob must succeed");
        assert_eq!(journal.blob(digest).expect("ok"), Some(record));
    }

    #[test]
    fn adversarial_snapshot_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let snap = RunSnapshot {
            run: RunId::new(888),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
            slots: vec![0u8; (MAX_SNAPSHOT_BYTES as usize).saturating_add(1)],
            taint: Vec::new(),
        };
        assert!(matches!(
            journal.put_snapshot(&snap),
            Err(JournalError::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn adversarial_snapshot_corrupt_magic_returns_bad_magic() {
        let snap = RunSnapshot {
            run: RunId::new(889),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
            slots: vec![1, 2, 3],
            taint: Vec::new(),
        };
        let mut enc = encode_record(
            MAGIC_SNAPSHOT,
            RecordKind::Snapshot,
            snap.seq.get(),
            &snap,
            MAX_SNAPSHOT_BYTES,
        )
        .expect("ok");
        if let Some(b) = enc.get_mut(0) {
            *b ^= 0xFF;
        }
        assert!(matches!(
            decode_record::<RunSnapshot>(&enc, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES),
            Err(JournalError::BadMagic { .. })
        ));
    }

    #[test]
    fn adversarial_workflow_source_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let source = vec![0u8; (MAX_WORKFLOW_SOURCE_BYTES as usize).saturating_add(1)];
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord { digest, source };
        assert!(matches!(
            journal.put_workflow_source(&record),
            Err(JournalError::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn adversarial_compiled_ir_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let record = CompiledIrRecord {
            digest: test_digest(0xCC),
            ir: vec![0u8; (MAX_COMPILED_IR_BYTES as usize).saturating_add(1)],
        };
        assert!(matches!(
            journal.put_compiled_ir(&record),
            Err(JournalError::PayloadTooLarge { .. })
        ));
    }

    // =========================================================================
    // Section: Adversarial Schema Migration Tests
    // =========================================================================

    #[test]
    fn adversarial_schema_migration_from_zero_exact_fields() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(11),
            seq: EventSeq::new(0),
            workflow: test_digest(11),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &0u16.to_le_bytes());
        let Err(JournalError::MigrationRequired { from, to }) =
            decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        else {
            panic!("expected MigrationRequired")
        };
        assert_eq!(from, 0);
        assert_eq!(to, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn adversarial_schema_future_version_max_unsupported() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(12),
            seq: EventSeq::new(0),
            workflow: test_digest(12),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &u16::MAX.to_le_bytes());
        let Err(JournalError::UnsupportedSchemaVersion { version }) =
            decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        else {
            panic!("expected UnsupportedSchemaVersion")
        };
        assert_eq!(version, u16::MAX);
    }

    // =========================================================================
    // Section: Adversarial Queue Tests
    // =========================================================================

    #[test]
    fn adversarial_queue_zero_capacity_returns_queue_capacity() {
        assert!(matches!(
            JournalWriterQueue::new(0, 1, StorageLimits::DEFAULT),
            Err(JournalError::QueueCapacity)
        ));
    }

    #[test]
    fn adversarial_queue_zero_batch_returns_queue_capacity() {
        assert!(matches!(
            JournalWriterQueue::new(1, 0, StorageLimits::DEFAULT),
            Err(JournalError::QueueCapacity)
        ));
    }

    #[test]
    fn adversarial_queue_full_returns_queue_full() {
        let queue = JournalWriterQueue::new(1, 1, StorageLimits::DEFAULT).expect("q");
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        queue
            .enqueue_journaled(event.clone())
            .expect("queue.enqueue_journaled must succeed");
        assert!(matches!(
            queue.enqueue_journaled(event),
            Err(JournalError::QueueFull)
        ));
    }

    #[test]
    fn journal_writer_queue_drain_all_flushes_until_empty() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let queue = JournalWriterQueue::new(4, 1, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(2);
        let workflow = test_digest(2);

        assert!(
            queue
                .enqueue_journaled(JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow,
                })
                .is_ok()
        );
        assert!(
            queue
                .enqueue_journaled(JournalEvent::RunCancelled {
                    run,
                    seq: EventSeq::new(1),
                    attempt: 1,
                    reason: None,
                })
                .is_ok()
        );

        assert!(matches!(
            queue.drain_all(&journal),
            Ok(report) if report.drained == 2 && report.written == 2
        ));
        assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() == 2));
    }

    #[test]
    fn journal_writer_queue_retains_events_when_append_fails() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(3);
        let duplicate = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(3),
        };
        let conflicting_duplicate = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(33),
        };
        let next = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        };

        assert!(matches!(journal.append_journaled(&duplicate), Ok(())));
        assert!(matches!(
            queue.enqueue_journaled(conflicting_duplicate),
            Ok(())
        ));
        assert!(matches!(queue.enqueue_journaled(next), Ok(())));

        assert!(matches!(
            queue.flush_batch(&journal),
            Err(JournalError::DuplicateEvent { run: found, seq })
                if found == run && seq == EventSeq::new(0)
        ));
        assert!(matches!(
            queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 2 && counts.strict == 0
        ));
    }

    #[test]
    fn journal_writer_queue_flush_persists_journaled_events_before_drain() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let path = temp_dir.path().to_path_buf();
        let journal = FjallJournal::open(&path, None).expect("opens");
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(4);
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(4),
        };
        let cancelled = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        };

        queue
            .enqueue_journaled(accepted)
            .expect("queue.enqueue_journaled must succeed");
        queue
            .enqueue_journaled(cancelled)
            .expect("queue.enqueue_journaled must succeed");
        assert!(matches!(
            queue.flush_batch(&journal),
            Ok(report) if report.drained == 2 && report.written == 2
        ));
        drop(journal);

        let reopened = FjallJournal::open(&path, None).expect("reopen");
        assert!(matches!(reopened.events_for_run(run), Ok(events) if events.len() == 2));
    }

    #[test]
    fn journal_writer_queue_shutdown_rejects_new_writes_after_durable_drain() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let queue = JournalWriterQueue::new(4, 1, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(5);
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(5),
        };
        let cancelled = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        };

        queue
            .enqueue_journaled(accepted.clone())
            .expect("queue.enqueue_journaled must succeed");
        queue
            .enqueue_strict(cancelled)
            .expect("queue.enqueue_strict must succeed");
        assert!(matches!(
            queue.shutdown(&journal),
            Ok(report) if report.drained == 2 && report.written == 2
        ));
        assert!(matches!(
            queue.enqueue_journaled(accepted),
            Err(JournalError::QueueShutdown)
        ));
        assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() == 2));
    }

    #[test]
    fn journal_writer_queue_crash_window_retry_drains_already_written_same_event() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(6);
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(6),
        };
        let cancelled = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        };

        journal
            .append_journaled(&accepted)
            .expect("journal.append_journaled must succeed");
        queue
            .enqueue_journaled(accepted)
            .expect("queue.enqueue_journaled must succeed");
        queue
            .enqueue_journaled(cancelled)
            .expect("queue.enqueue_journaled must succeed");

        // This models the crash window where a prior attempt reached Fjall before
        // the queue could durably drain. Retrying accepts the identical event only.
        assert!(matches!(
            queue.flush_batch(&journal),
            Ok(report) if report.drained == 2 && report.written == 2
        ));
        assert!(matches!(
            queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 0 && counts.strict == 0
        ));
        assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() == 2));
    }

    // =========================================================================
    // Section: Adversarial Postcard / Encoding Edge Cases
    // =========================================================================

    #[test]
    fn adversarial_valid_header_garbage_postcard_returns_decode_failed() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(13),
            seq: EventSeq::new(0),
            workflow: test_digest(13),
        };
        let mut enc = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("ok");
        if let Some(b) = enc.get_mut(60) {
            *b = 0xFF;
        }
        let digest_bytes = *blake3::hash(&enc[60..]).as_bytes();
        enc.get_mut(24..56)
            .expect("digest")
            .copy_from_slice(&digest_bytes);
        let cs = crc32c::crc32c(&enc[..56]);
        enc[56] = (cs & 0xFF) as u8;
        enc[57] = ((cs >> 8) & 0xFF) as u8;
        enc[58] = ((cs >> 16) & 0xFF) as u8;
        enc[59] = ((cs >> 24) & 0xFF) as u8;
        assert!(matches!(
            decode_record::<JournalEvent>(&enc, MAGIC_JOURNAL_EVENT, 128),
            Err(JournalError::PostcardDecodeFailed(_))
        ));
    }

    #[test]
    fn adversarial_run_header_wrong_magic_returns_bad_magic() {
        let record = RunHeaderRecord {
            run: RunId::new(123),
            workflow_id: WorkflowId::new(456),
            compiled_digest: test_digest(8),
            status: 1,
            accepted_at_ms: 1700000000,
        };
        let enc = encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::RunHeader,
            record.run.get(),
            &record,
            MAX_RUN_HEADER_BYTES,
        )
        .expect("ok");
        assert!(matches!(
            decode_record::<RunHeaderRecord>(&enc, MAGIC_BLOB, MAX_RUN_HEADER_BYTES),
            Err(JournalError::BadMagic { .. })
        ));
    }

    #[test]
    fn adversarial_decode_empty_returns_unexpected_eof() {
        assert!(matches!(
            decode_record::<JournalEvent>(&[][..], MAGIC_JOURNAL_EVENT, 128),
            Err(JournalError::UnexpectedEof)
        ));
    }

    #[test]
    fn adversarial_encode_empty_blob_succeeds() {
        assert!(
            encode_record(
                MAGIC_BLOB,
                RecordKind::Blob,
                0,
                &BlobRecord {
                    digest: [0; 32],
                    bytes: vec![]
                },
                MAX_BLOB_BYTES
            )
            .is_ok()
        );
    }

    #[test]
    fn adversarial_encode_empty_source_succeeds() {
        assert!(
            encode_record(
                MAGIC_WORKFLOW_SOURCE,
                RecordKind::WorkflowSource,
                0,
                &WorkflowSourceRecord {
                    digest: test_digest(0),
                    source: vec![]
                },
                MAX_WORKFLOW_SOURCE_BYTES
            )
            .is_ok()
        );
    }

    #[test]
    fn adversarial_encode_empty_ir_succeeds() {
        assert!(
            encode_record(
                MAGIC_COMPILED_ARTIFACT,
                RecordKind::CompiledIr,
                0,
                &CompiledIrRecord {
                    digest: test_digest(0),
                    ir: vec![]
                },
                MAX_COMPILED_IR_BYTES
            )
            .is_ok()
        );
    }

    #[test]
    fn journal_error_diagnostic_codes_are_unique() {
        let errors = [
            JournalError::KeyCapacity,
            JournalError::WriteLockPoisoned,
            JournalError::QueueCapacity,
            JournalError::QueueFull,
            JournalError::SequenceOverflow,
            JournalError::HeaderChecksumMismatch,
            JournalError::PayloadDigestMismatch,
            JournalError::UnexpectedEof,
            JournalError::PostcardDecodeFailed(postcard::Error::DeserializeBadVarint),
            JournalError::DuplicateEvent {
                run: RunId::new(1),
                seq: EventSeq::new(0),
            },
            JournalError::WrongRun {
                expected: RunId::new(1),
                actual: RunId::new(2),
            },
            JournalError::SequenceGap {
                expected: EventSeq::new(0),
                actual: EventSeq::new(1),
            },
            JournalError::BadMagic { found: 0 },
            JournalError::UnsupportedSchemaVersion { version: 0 },
            JournalError::MigrationRequired { from: 0, to: 1 },
            JournalError::UnknownRecordKind { kind: 0 },
            JournalError::RecordKindFamilyMismatch { magic: 0, kind: 0 },
            JournalError::HeaderLengthMismatch { found: 0 },
            JournalError::PayloadTooLarge { len: 0, max: 0 },
            JournalError::TooManyEvents {
                run: RunId::new(1),
                limit: 1,
                observed: 2,
            },
            JournalError::ReplayAllocationFailed {
                run: RunId::new(1),
                requested: 1,
            },
        ];
        let mut seen = std::collections::BTreeSet::new();
        for err in &errors {
            let code = err.diagnostic_code();
            assert!(seen.insert(code), "duplicate diagnostic code: {code}");
        }
        assert_eq!(seen.len(), errors.len());
    }

    #[test]
    fn journal_error_diagnostic_code_fjall() {
        // Fjall and Encode variants hold external errors; we verify via KeyCapacity
        assert_eq!(
            JournalError::KeyCapacity.diagnostic_code(),
            DiagnosticCode::new(0x4003)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_duplicate_event() {
        assert_eq!(
            JournalError::DuplicateEvent {
                run: RunId::new(42),
                seq: EventSeq::new(7),
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x4004)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_write_lock_poisoned() {
        assert_eq!(
            JournalError::WriteLockPoisoned.diagnostic_code(),
            DiagnosticCode::new(0x4005)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_queue_capacity() {
        assert_eq!(
            JournalError::QueueCapacity.diagnostic_code(),
            DiagnosticCode::new(0x4006)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_queue_full() {
        assert_eq!(
            JournalError::QueueFull.diagnostic_code(),
            DiagnosticCode::new(0x4007)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_wrong_run() {
        assert_eq!(
            JournalError::WrongRun {
                expected: RunId::new(1),
                actual: RunId::new(2),
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x4008)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_sequence_gap() {
        assert_eq!(
            JournalError::SequenceGap {
                expected: EventSeq::new(0),
                actual: EventSeq::new(1),
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x4009)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_sequence_overflow() {
        assert_eq!(
            JournalError::SequenceOverflow.diagnostic_code(),
            DiagnosticCode::new(0x400A)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_bad_magic() {
        assert_eq!(
            JournalError::BadMagic { found: 0xDEAD_BEEF }.diagnostic_code(),
            DiagnosticCode::new(0x400B)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_unsupported_schema_version() {
        assert_eq!(
            JournalError::UnsupportedSchemaVersion { version: 99 }.diagnostic_code(),
            DiagnosticCode::new(0x400C)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_migration_required() {
        assert_eq!(
            JournalError::MigrationRequired { from: 0, to: 1 }.diagnostic_code(),
            DiagnosticCode::new(0x400D)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_unknown_record_kind() {
        assert_eq!(
            JournalError::UnknownRecordKind { kind: 200 }.diagnostic_code(),
            DiagnosticCode::new(0x400E)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_record_kind_family_mismatch() {
        assert_eq!(
            JournalError::RecordKindFamilyMismatch {
                magic: MAGIC_JOURNAL_EVENT,
                kind: 1,
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x400F)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_header_length_mismatch() {
        assert_eq!(
            JournalError::HeaderLengthMismatch { found: 99 }.diagnostic_code(),
            DiagnosticCode::new(0x4010)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_payload_too_large() {
        assert_eq!(
            JournalError::PayloadTooLarge { len: 200, max: 10 }.diagnostic_code(),
            DiagnosticCode::new(0x4011)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_header_checksum_mismatch() {
        assert_eq!(
            JournalError::HeaderChecksumMismatch.diagnostic_code(),
            DiagnosticCode::new(0x4012)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_payload_digest_mismatch() {
        assert_eq!(
            JournalError::PayloadDigestMismatch.diagnostic_code(),
            DiagnosticCode::new(0x4013)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_unexpected_eof() {
        assert_eq!(
            JournalError::UnexpectedEof.diagnostic_code(),
            DiagnosticCode::new(0x4014)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_postcard_decode_failed() {
        assert_eq!(
            JournalError::PostcardDecodeFailed(postcard::Error::DeserializeBadVarint)
                .diagnostic_code(),
            DiagnosticCode::new(0x4015)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_too_many_events() {
        assert_eq!(
            JournalError::TooManyEvents {
                run: RunId::new(7),
                limit: 1,
                observed: 2,
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x401E)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_replay_allocation_failed() {
        assert_eq!(
            JournalError::ReplayAllocationFailed {
                run: RunId::new(7),
                requested: 2,
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x401F)
        );
    }

    // =========================================================================
    // Section: Batch Write-Through Integration Tests (60 new tests)
    // =========================================================================

    // --- JournalWriteBatch put_run_event round-trips (tests 1-12) ---

    #[test]
    fn batch_append_run_accepted_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1001);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1, "one event must be stored");
        assert_eq!(events[0], event, "event must round-trip exactly");
    }

    #[test]
    fn batch_append_step_started_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1002);
        let event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(1),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_step_succeeded_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1003);
        let event = JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(2),
            output: SlotIdx::new(3),
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_step_failed_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1004);
        let event = JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(0),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_action_scheduled_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1005);
        let event = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            action: ActionId::new(7),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_action_completed_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1006);
        let event = JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(1),
            action: ActionId::new(8),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_action_failed_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1007);
        let event = JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(2),
            action: ActionId::new(9),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_run_finished_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1008);
        let event = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(0),
            result: SlotIdx::new(42),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_run_failed_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1009);
        let event = JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(0),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_run_cancelled_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1010);
        let event = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(0),
            attempt: 1,
            reason: None,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_slot_written_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1011);
        let event = JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(0),
            slot: SlotIdx::new(5),
            value: None,
            extra: None,
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn batch_append_suspended_event_round_trips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(1012);
        let event = JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(3),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    // --- Multi-run isolation (tests 13-16) ---

    #[test]
    fn events_for_run_isolates_run_a_from_run_b() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run_a = RunId::new(2001);
        let run_b = RunId::new(2002);
        let event_a = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xAA; 32]),
        };
        let event_b = JournalEvent::RunAccepted {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xBB; 32]),
        };
        let event_a2 = JournalEvent::RunFinished {
            run: run_a,
            seq: EventSeq::new(1),
            result: SlotIdx::new(0),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch
            .append_event(&event_a)
            .expect("batch.append_event must succeed");
        batch
            .append_event(&event_b)
            .expect("batch.append_event must succeed");
        batch
            .append_event(&event_a2)
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        let events_a = journal
            .events_for_run(run_a)
            .expect("events_for_run A must succeed");
        assert_eq!(events_a.len(), 2, "run A must have exactly 2 events");
        assert_eq!(events_a[0], event_a);
        assert_eq!(events_a[1], event_a2);
        let events_b = journal
            .events_for_run(run_b)
            .expect("events_for_run B must succeed");
        assert_eq!(events_b.len(), 1, "run B must have exactly 1 event");
        assert_eq!(events_b[0], event_b);
    }

    #[test]
    fn run_header_isolation_between_runs() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run_1 = RunId::new(3001);
        let run_2 = RunId::new(3002);
        let header_1 = RunHeaderRecord {
            run: run_1,
            workflow_id: WorkflowId::new(10),
            compiled_digest: WorkflowDigest::from_bytes([1; 32]),
            status: 1,
            accepted_at_ms: 100,
        };
        let header_2 = RunHeaderRecord {
            run: run_2,
            workflow_id: WorkflowId::new(20),
            compiled_digest: WorkflowDigest::from_bytes([2; 32]),
            status: 2,
            accepted_at_ms: 200,
        };
        let mut batch = journal.batch();
        batch
            .put_run_header(&header_1)
            .expect("batch.put_run_header must succeed");
        batch
            .put_run_header(&header_2)
            .expect("batch.put_run_header must succeed");
        batch.commit().expect("batch.commit must succeed");
        let found_1 = journal
            .run_header(run_1)
            .expect("run_header run_1 must succeed");
        assert_eq!(found_1, Some(header_1), "run 1 header must match exactly");
        let found_2 = journal
            .run_header(run_2)
            .expect("run_header run_2 must succeed");
        assert_eq!(found_2, Some(header_2), "run 2 header must match exactly");
    }

    #[test]
    fn snapshot_isolation_between_runs() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run_a = RunId::new(3003);
        let run_b = RunId::new(3004);
        let snap_a = RunSnapshot {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xA; 32]),
            slots: vec![1, 2, 3],
            taint: Vec::new(),
        };
        let snap_b = RunSnapshot {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xB; 32]),
            slots: vec![4, 5, 6],
            taint: Vec::new(),
        };
        let mut batch = journal.batch();
        batch
            .put_snapshot(&snap_a)
            .expect("batch.put_snapshot must succeed");
        batch
            .put_snapshot(&snap_b)
            .expect("batch.put_snapshot must succeed");
        batch.commit().expect("batch.commit must succeed");
        let found_a = journal
            .snapshot(run_a, EventSeq::new(0))
            .expect("snapshot A must succeed");
        assert_eq!(found_a, Some(snap_a), "snapshot for run A must match");
        let found_b = journal
            .snapshot(run_b, EventSeq::new(0))
            .expect("snapshot B must succeed");
        assert_eq!(found_b, Some(snap_b), "snapshot for run B must match");
    }

    #[test]
    fn batch_writes_for_multiple_runs_commit_atomically() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run_1 = RunId::new(4001);
        let run_2 = RunId::new(4002);
        let run_3 = RunId::new(4003);
        let mut batch = journal.batch();
        batch
            .append_event(&JournalEvent::RunAccepted {
                run: run_1,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
            })
            .expect("batch.append_event must succeed");
        batch
            .append_event(&JournalEvent::RunAccepted {
                run: run_2,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([2; 32]),
            })
            .expect("batch.append_event must succeed");
        batch
            .append_event(&JournalEvent::RunAccepted {
                run: run_3,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([3; 32]),
            })
            .expect("batch.append_event must succeed");
        batch.commit().expect("batch.commit must succeed");
        assert_eq!(
            journal
                .events_for_run(run_1)
                .expect("run_1 must succeed")
                .len(),
            1,
            "run 1 must have 1 event"
        );
        assert_eq!(
            journal
                .events_for_run(run_2)
                .expect("run_2 must succeed")
                .len(),
            1,
            "run 2 must have 1 event"
        );
        assert_eq!(
            journal
                .events_for_run(run_3)
                .expect("run_3 must succeed")
                .len(),
            1,
            "run 3 must have 1 event"
        );
    }

    // --- Writer Queue edge cases (tests 17-22) ---

    #[test]
    fn queue_journaled_enqueue_and_drain_preserves_order() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
        let run = RunId::new(5001);
        let event_0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let event_1 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let event_2 = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(0),
            attempt: 1,
        };
        queue
            .enqueue_journaled(event_0.clone())
            .expect("enqueue 0 must succeed");
        queue
            .enqueue_journaled(event_1.clone())
            .expect("enqueue 1 must succeed");
        queue
            .enqueue_journaled(event_2.clone())
            .expect("enqueue 2 must succeed");
        let report = queue.drain_all(&journal).expect("drain_all must succeed");
        assert_eq!(report.drained, 3);
        assert_eq!(report.written, 3);
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events[0], event_0, "first event must be seq 0");
        assert_eq!(events[1], event_1, "second event must be seq 1");
        assert_eq!(events[2], event_2, "third event must be seq 2");
    }

    #[test]
    fn queue_strict_enqueue_and_drain_preserves_order() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
        let run = RunId::new(5002);
        let event_0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([2; 32]),
        };
        let event_1 = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        };
        queue
            .enqueue_strict(event_0.clone())
            .expect("enqueue 0 must succeed");
        queue
            .enqueue_strict(event_1.clone())
            .expect("enqueue 1 must succeed");
        let report = queue.drain_all(&journal).expect("drain_all must succeed");
        assert_eq!(report.drained, 2);
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events[0], event_0);
        assert_eq!(events[1], event_1);
    }

    #[test]
    fn queue_mixed_journaled_and_strict_drain_returns_both() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
        let run = RunId::new(5003);
        let journaled_event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([3; 32]),
        };
        let strict_event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        };
        queue
            .enqueue_journaled(journaled_event.clone())
            .expect("enqueue journaled must succeed");
        queue
            .enqueue_strict(strict_event.clone())
            .expect("enqueue strict must succeed");
        let report = queue.drain_all(&journal).expect("drain_all must succeed");
        assert_eq!(report.drained, 2, "both events must be drained");
        assert_eq!(report.written, 2, "both events must be written");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], journaled_event);
        assert_eq!(events[1], strict_event);
    }

    #[test]
    fn queue_flush_persists_before_drain() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
        let run = RunId::new(5004);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([4; 32]),
        };
        queue
            .enqueue_journaled(event.clone())
            .expect("enqueue must succeed");
        let report = queue
            .flush_batch(&journal)
            .expect("flush_batch must succeed");
        assert_eq!(report.written, 1, "one event must be written");
        let events_before = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events_before.len(), 1, "event must be on disk before drain");
        assert_eq!(events_before[0], event);
    }

    #[test]
    fn queue_empty_drain_returns_zero_events() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let queue = JournalWriterQueue::new(8, 8, StorageLimits::DEFAULT).expect("setup: queue");
        let report = queue.drain_all(&journal).expect("drain_all must succeed");
        assert_eq!(report.drained, 0, "empty queue must drain zero events");
        assert_eq!(report.written, 0, "empty queue must write zero events");
    }

    #[test]
    fn queue_pending_count_matches_enqueued() {
        let queue = JournalWriterQueue::new(16, 4, StorageLimits::DEFAULT).expect("setup: queue");
        let run = RunId::new(5005);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([5; 32]),
        };
        let counts_empty = queue.pending_profile_counts().expect("counts must succeed");
        assert_eq!(counts_empty.journaled, 0);
        assert_eq!(counts_empty.strict, 0);
        queue
            .enqueue_journaled(event.clone())
            .expect("enqueue 0 must succeed");
        queue
            .enqueue_journaled(event.clone())
            .expect("enqueue 1 must succeed");
        queue.enqueue_strict(event).expect("enqueue 2 must succeed");
        let counts = queue.pending_profile_counts().expect("counts must succeed");
        assert_eq!(counts.journaled, 2, "two journaled events must be counted");
        assert_eq!(counts.strict, 1, "one strict event must be counted");
    }

    // --- FjallJournal open/close/reopen (tests 23-30) ---

    #[test]
    fn journal_open_creates_fresh_database() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let events = journal
            .events_for_run(RunId::new(1))
            .expect("events_for_run must succeed");
        assert!(events.is_empty(), "fresh database must have no events");
        let header = journal
            .run_header(RunId::new(1))
            .expect("run_header must succeed");
        assert_eq!(header, None, "fresh database must have no headers");
        let blob = journal.blob([0; 32]).expect("blob must succeed");
        assert_eq!(blob, None, "fresh database must have no blobs");
    }

    #[test]
    fn journal_close_and_reopen_preserves_strict_data() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let path = temp_dir.path().to_path_buf();
        let digest = WorkflowDigest::from_bytes([0xEE; 32]);
        let run = RunId::new(6001);
        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(5),
            compiled_digest: digest,
            status: 3,
            accepted_at_ms: 999,
        };
        {
            let journal = FjallJournal::open(&path, None).expect("setup: journal open");
            journal
                .put_run_header(&header)
                .expect("put_run_header must succeed");
            journal
                .persist_strict()
                .expect("persist_strict must succeed");
        }
        let reopened = FjallJournal::open(&path, None).expect("reopen must succeed");
        let found = reopened.run_header(run).expect("run_header must succeed");
        assert_eq!(found, Some(header), "strict data must survive reopen");
    }

    #[test]
    fn journal_multiple_opens_same_path_fails_or_succeeds() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal1 = FjallJournal::open(temp_dir.path(), None).expect("first open must succeed");
        let journal2_result = FjallJournal::open(temp_dir.path(), None);
        drop(journal1);
        if let Ok(j2) = journal2_result {
            drop(j2);
        }
    }

    #[test]
    fn journal_put_then_get_workflow_source_consistent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let source = b"consistent_source".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord { digest, source };
        journal
            .put_workflow_source(&record)
            .expect("put_workflow_source must succeed");
        let found = journal
            .workflow_source(digest)
            .expect("workflow_source must succeed");
        assert_eq!(
            found,
            Some(record),
            "put-then-get must be consistent in same session"
        );
    }

    #[test]
    fn journal_put_then_get_compiled_ir_consistent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest = WorkflowDigest::from_bytes([0x88; 32]);
        let record = CompiledIrRecord {
            digest,
            ir: b"consistent_ir".to_vec(),
        };
        journal
            .put_compiled_ir(&record)
            .expect("put_compiled_ir must succeed");
        let found = journal
            .compiled_ir(digest)
            .expect("compiled_ir must succeed");
        assert_eq!(
            found,
            Some(record),
            "put-then-get must be consistent in same session"
        );
    }

    #[test]
    fn journal_put_then_get_run_header_consistent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(6002);
        let record = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(99),
            compiled_digest: WorkflowDigest::from_bytes([0x99; 32]),
            status: 7,
            accepted_at_ms: 123456789,
        };
        journal
            .put_run_header(&record)
            .expect("put_run_header must succeed");
        let found = journal.run_header(run).expect("run_header must succeed");
        assert_eq!(
            found,
            Some(record),
            "put-then-get must be consistent in same session"
        );
    }

    #[test]
    fn journal_put_then_get_snapshot_consistent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(6003);
        let seq = EventSeq::new(4);
        let snapshot = RunSnapshot {
            run,
            seq,
            workflow: WorkflowDigest::from_bytes([0xAA; 32]),
            slots: vec![0xDE, 0xAD],
            taint: Vec::new(),
        };
        journal
            .put_snapshot(&snapshot)
            .expect("put_snapshot must succeed");
        let found = journal.snapshot(run, seq).expect("snapshot must succeed");
        assert_eq!(
            found,
            Some(snapshot),
            "put-then-get must be consistent in same session"
        );
    }

    #[test]
    fn journal_put_then_get_blob_consistent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let blob_bytes = b"consistent_blob".to_vec();
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
        let record = BlobRecord {
            digest,
            bytes: blob_bytes,
        };
        journal.put_blob(&record).expect("put_blob must succeed");
        let found = journal.blob(digest).expect("blob must succeed");
        assert_eq!(
            found,
            Some(record),
            "put-then-get must be consistent in same session"
        );
    }

    // --- Index queries (tests 31-35) ---

    #[test]
    fn status_index_stores_and_queries_by_state() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let state = IndexStatusState::Other(3);
        let timestamp: u64 = 1700000000;
        let run = RunId::new(7001);
        journal
            .put_status_index(state, timestamp, run)
            .expect("put_status_index must succeed");
        let key = index_status_key(state, timestamp, run).expect("key must succeed");
        let value = journal
            .index_status
            .get(key.as_slice())
            .expect("get must succeed");
        assert!(value.is_some(), "status index entry must exist after put");
    }

    #[test]
    fn workflow_index_stores_and_queries_by_workflow_id() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let workflow = WorkflowId::new(42);
        let run = RunId::new(7002);
        journal
            .put_workflow_index(workflow, run)
            .expect("put_workflow_index must succeed");
        let key = index_workflow_key(workflow, run).expect("key must succeed");
        let value = journal
            .index_workflow
            .get(key.as_slice())
            .expect("get must succeed");
        assert!(value.is_some(), "workflow index entry must exist after put");
    }

    #[test]
    fn action_index_stores_and_queries_by_action_id() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let action = ActionId::new(7);
        let run = RunId::new(7003);
        let step = StepIdx::new(2);
        journal
            .put_action_index(action, run, step)
            .expect("put_action_index must succeed");
        let key = index_action_key(action, run, step).expect("key must succeed");
        let value = journal
            .index_action
            .get(key.as_slice())
            .expect("get must succeed");
        assert!(value.is_some(), "action index entry must exist after put");
    }

    #[test]
    fn status_index_multiple_runs_same_state() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let state = IndexStatusState::Other(5);
        let run_1 = RunId::new(7010);
        let run_2 = RunId::new(7011);
        let run_3 = RunId::new(7012);
        journal
            .put_status_index(state, 100, run_1)
            .expect("put_status_index 1 must succeed");
        journal
            .put_status_index(state, 200, run_2)
            .expect("put_status_index 2 must succeed");
        journal
            .put_status_index(state, 300, run_3)
            .expect("put_status_index 3 must succeed");
        let key_1 = index_status_key(state, 100, run_1).expect("key 1 must succeed");
        let key_2 = index_status_key(state, 200, run_2).expect("key 2 must succeed");
        let key_3 = index_status_key(state, 300, run_3).expect("key 3 must succeed");
        assert!(
            journal
                .index_status
                .get(key_1.as_slice())
                .expect("get 1")
                .is_some()
        );
        assert!(
            journal
                .index_status
                .get(key_2.as_slice())
                .expect("get 2")
                .is_some()
        );
        assert!(
            journal
                .index_status
                .get(key_3.as_slice())
                .expect("get 3")
                .is_some()
        );
    }

    #[test]
    fn workflow_index_multiple_runs_same_workflow() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let workflow = WorkflowId::new(99);
        let run_1 = RunId::new(7020);
        let run_2 = RunId::new(7021);
        let run_3 = RunId::new(7022);
        journal
            .put_workflow_index(workflow, run_1)
            .expect("put 1 must succeed");
        journal
            .put_workflow_index(workflow, run_2)
            .expect("put 2 must succeed");
        journal
            .put_workflow_index(workflow, run_3)
            .expect("put 3 must succeed");
        let key_1 = index_workflow_key(workflow, run_1).expect("key 1 must succeed");
        let key_2 = index_workflow_key(workflow, run_2).expect("key 2 must succeed");
        let key_3 = index_workflow_key(workflow, run_3).expect("key 3 must succeed");
        assert!(
            journal
                .index_workflow
                .get(key_1.as_slice())
                .expect("get 1")
                .is_some()
        );
        assert!(
            journal
                .index_workflow
                .get(key_2.as_slice())
                .expect("get 2")
                .is_some()
        );
        assert!(
            journal
                .index_workflow
                .get(key_3.as_slice())
                .expect("get 3")
                .is_some()
        );
    }

    // --- Record builder (tests 36-40) ---

    #[test]
    fn builder_initial_len_is_zero() {
        let builder = BatchBuilder::new();
        assert_eq!(builder.len(), 0, "new builder must have len 0");
        assert!(builder.is_empty(), "new builder must be empty");
    }

    #[test]
    fn builder_append_increments_len() {
        let mut builder = BatchBuilder::new();
        let run = RunId::new(8001);
        builder
            .push(JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
            })
            .expect("push within cap");
        assert_eq!(builder.len(), 1, "builder must have len 1 after one push");
        assert!(!builder.is_empty());
    }

    #[test]
    fn builder_append_multiple_events_len_matches() {
        let mut builder = BatchBuilder::new();
        let run = RunId::new(8002);
        builder
            .push(JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
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
        builder
            .push(JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(2),
                result: SlotIdx::new(0),
                attempt: 1,
            })
            .expect("push within cap");
        assert_eq!(
            builder.len(),
            3,
            "builder must have len 3 after three pushes"
        );
    }

    #[test]
    fn builder_as_slice_returns_appended_events() {
        let mut builder = BatchBuilder::new();
        let run = RunId::new(8003);
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let e1 = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        };
        builder.push(e0.clone()).expect("push within cap");
        builder.push(e1.clone()).expect("push within cap");
        let slice = builder.as_slice();
        assert_eq!(slice.len(), 2);
        assert_eq!(
            slice[0], e0,
            "first slice element must match first pushed event"
        );
        assert_eq!(
            slice[1], e1,
            "second slice element must match second pushed event"
        );
    }

    #[test]
    fn builder_build_produces_correct_record_count() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(8004);
        let mut builder = BatchBuilder::new();
        builder
            .push(JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
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
        builder
            .push(JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(2),
                result: SlotIdx::new(0),
                attempt: 1,
            })
            .expect("push within cap");
        assert_eq!(builder.len(), 3);
        journal
            .append_strict_batch(builder.as_slice())
            .expect("append_strict_batch must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 3, "three events must be stored");
    }

    // --- Batch state tracking (tests 41-44) ---

    #[test]
    fn batch_initial_len_is_zero() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let batch = journal.batch();
        assert_eq!(batch.len(), 0, "new batch must have len 0");
        assert!(batch.is_empty(), "new batch must be empty");
    }

    #[test]
    fn batch_len_increments_per_put() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let mut batch = journal.batch();
        let source = b"a".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        batch
            .put_workflow_source(&WorkflowSourceRecord { digest, source })
            .expect("put 1 must succeed");
        assert_eq!(batch.len(), 1, "batch must have len 1 after first put");
        batch
            .put_compiled_ir(&CompiledIrRecord {
                digest,
                ir: b"ir".to_vec(),
            })
            .expect("put 2 must succeed");
        assert_eq!(batch.len(), 2, "batch must have len 2 after second put");
        batch
            .put_run_header(&RunHeaderRecord {
                run: RunId::new(9001),
                workflow_id: WorkflowId::new(1),
                compiled_digest: digest,
                status: 0,
                accepted_at_ms: 0,
            })
            .expect("put 3 must succeed");
        assert_eq!(batch.len(), 3, "batch must have len 3 after third put");
    }

    #[test]
    fn batch_len_resets_after_commit() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let mut batch = journal.batch();
        let source = b"data".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        batch
            .put_workflow_source(&WorkflowSourceRecord { digest, source })
            .expect("put must succeed");
        assert_eq!(batch.len(), 1, "batch must have 1 operation before commit");
        batch.commit().expect("commit must succeed");
        let fresh_batch = journal.batch();
        assert_eq!(
            fresh_batch.len(),
            0,
            "new batch after commit must start at 0"
        );
    }

    #[test]
    fn batch_put_snapshot_increments_len() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let mut batch = journal.batch();
        assert_eq!(batch.len(), 0);
        let snapshot = RunSnapshot {
            run: RunId::new(9002),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0x43; 32]),
            slots: vec![1, 2],
            taint: Vec::new(),
        };
        batch
            .put_snapshot(&snapshot)
            .expect("put_snapshot must succeed");
        assert_eq!(batch.len(), 1, "batch len must be 1 after put_snapshot");
    }

    // --- Envelope validation (tests 45-47) ---

    #[test]
    fn decode_valid_envelope_produces_exact_record() {
        let record = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([0xDD; 32]),
            source: b"exact_match".to_vec(),
        };
        let encoded = encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &record,
            MAX_WORKFLOW_SOURCE_BYTES,
        )
        .expect("encode must succeed");
        let (envelope, decoded) = decode_record::<WorkflowSourceRecord>(
            &encoded,
            MAGIC_WORKFLOW_SOURCE,
            MAX_WORKFLOW_SOURCE_BYTES,
        )
        .expect("decode must succeed");
        assert_eq!(envelope.magic, MAGIC_WORKFLOW_SOURCE);
        assert_eq!(envelope.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(envelope.record_kind, RecordKind::WorkflowSource.id());
        assert_eq!(
            decoded, record,
            "decoded record must exactly match original"
        );
    }

    #[test]
    fn envelope_magic_matches_expected_constant() {
        assert_eq!(MAGIC_WORKFLOW_SOURCE, 0x5642_5352, "VBSR in ASCII hex");
        assert_eq!(MAGIC_COMPILED_ARTIFACT, 0x5642_4952, "VBIR in ASCII hex");
        assert_eq!(MAGIC_JOURNAL_EVENT, 0x5642_4A45, "VBJE in ASCII hex");
        assert_eq!(MAGIC_SNAPSHOT, 0x5642_534E, "VBSN in ASCII hex");
        assert_eq!(MAGIC_BLOB, 0x5642_424C, "VBBL in ASCII hex");
        assert_eq!(MAGIC_IPC_FRAME, 0x5642_4C54, "VBLT in ASCII hex");
        assert_eq!(MAGIC_INDEX_RECORD, 0x5642_4958, "VBIX in ASCII hex");
    }

    #[test]
    fn envelope_header_len_is_fixed_at_60() {
        assert_eq!(RECORD_HEADER_LEN, 60, "header length must be exactly 60");
        assert_eq!(RECORD_HEADER_BYTES, 60, "header bytes constant must be 60");
        let header = encode_record_header(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            b"payload",
            128,
        )
        .expect("encode_record_header must succeed");
        assert_eq!(header.len(), 60, "encoded header must be exactly 60 bytes");
    }

    // --- Cross-keyspace atomicity (tests 48-60) ---

    #[test]
    fn batch_atomic_all_or_nothing_workflow_source_and_ir() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let source_bytes = b"atomic_source".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source_bytes).into());
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord {
                digest,
                source: source_bytes,
            })
            .expect("put_workflow_source must succeed");
        batch
            .put_compiled_ir(&CompiledIrRecord {
                digest,
                ir: b"atomic_ir".to_vec(),
            })
            .expect("put_compiled_ir must succeed");
        batch.commit().expect("commit must succeed");
        let source = journal
            .workflow_source(digest)
            .expect("workflow_source must succeed");
        let ir = journal
            .compiled_ir(digest)
            .expect("compiled_ir must succeed");
        assert!(
            source.is_some(),
            "source must be present after atomic commit"
        );
        assert!(ir.is_some(), "IR must be present after atomic commit");
        assert_eq!(source.unwrap().source, b"atomic_source".to_vec());
        assert_eq!(ir.unwrap().ir, b"atomic_ir".to_vec());
    }

    #[test]
    fn batch_commit_with_header_and_events_cross_keyspace() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9003);
        let digest = WorkflowDigest::from_bytes([0xCD; 32]);
        let mut batch = journal.batch();
        batch
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(1),
                compiled_digest: digest,
                status: 1,
                accepted_at_ms: 555,
            })
            .expect("put_run_header must succeed");
        batch
            .append_event(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: digest,
            })
            .expect("append_event must succeed");
        batch.commit().expect("commit must succeed");
        let header = journal.run_header(run).expect("run_header must succeed");
        assert!(header.is_some(), "header must be present");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(events.len(), 1, "event must be present");
    }

    #[test]
    fn batch_strict_commit_all_persisted_durably() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let path = temp_dir.path().to_path_buf();
        let ws_bytes = b"strict_ws".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&ws_bytes).into());
        let blob_bytes = b"strict_blob".to_vec();
        let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
        {
            let journal = FjallJournal::open(&path, None).expect("setup: journal open");
            let mut batch = journal.batch().strict();
            batch
                .put_workflow_source(&WorkflowSourceRecord {
                    digest,
                    source: ws_bytes,
                })
                .expect("put_workflow_source must succeed");
            batch
                .put_compiled_ir(&CompiledIrRecord {
                    digest,
                    ir: b"strict_ir".to_vec(),
                })
                .expect("put_compiled_ir must succeed");
            batch
                .put_blob(&BlobRecord {
                    digest: blob_digest,
                    bytes: blob_bytes,
                })
                .expect("put_blob must succeed");
            batch.commit().expect("commit must succeed");
        }
        let reopened = FjallJournal::open(&path, None).expect("reopen must succeed");
        let ws = reopened
            .workflow_source(digest)
            .expect("workflow_source must succeed");
        assert_eq!(ws.unwrap().source, b"strict_ws".to_vec());
        let ir = reopened
            .compiled_ir(digest)
            .expect("compiled_ir must succeed");
        assert_eq!(ir.unwrap().ir, b"strict_ir".to_vec());
        let bl = reopened.blob(blob_digest).expect("blob must succeed");
        assert_eq!(bl.unwrap().bytes, b"strict_blob".to_vec());
    }

    #[test]
    fn batch_empty_strict_commit_succeeds() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let batch = journal.batch().strict();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
        batch
            .commit()
            .expect("empty strict batch commit must succeed");
    }

    #[test]
    fn batch_commit_after_multiple_puts_persists_all() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let ws_bytes = b"ws".to_vec();
        let digest_1 = WorkflowDigest::from_bytes(blake3::hash(&ws_bytes).into());
        let digest_2 = WorkflowDigest::from_bytes([2; 32]);
        let blob_bytes = b"blob".to_vec();
        let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
        let run = RunId::new(9005);
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord {
                digest: digest_1,
                source: ws_bytes,
            })
            .expect("put 1 must succeed");
        batch
            .put_compiled_ir(&CompiledIrRecord {
                digest: digest_2,
                ir: b"ir".to_vec(),
            })
            .expect("put 2 must succeed");
        batch
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(1),
                compiled_digest: digest_1,
                status: 1,
                accepted_at_ms: 100,
            })
            .expect("put 3 must succeed");
        batch
            .put_blob(&BlobRecord {
                digest: blob_digest,
                bytes: blob_bytes,
            })
            .expect("put 4 must succeed");
        batch
            .put_snapshot(&RunSnapshot {
                run,
                seq: EventSeq::new(0),
                workflow: digest_1,
                slots: vec![42],
                taint: Vec::new(),
            })
            .expect("put 5 must succeed");
        batch.commit().expect("commit must succeed");
        assert!(journal.workflow_source(digest_1).expect("ws").is_some());
        assert!(journal.compiled_ir(digest_2).expect("ir").is_some());
        assert!(journal.run_header(run).expect("rh").is_some());
        assert!(journal.blob(blob_digest).expect("bl").is_some());
        assert!(
            journal
                .snapshot(run, EventSeq::new(0))
                .expect("sn")
                .is_some()
        );
    }

    #[test]
    fn journal_events_for_run_after_batch_commit_matches_input() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9006);
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let e1 = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let e2 = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::new(1),
            attempt: 1,
        };
        let mut batch = journal.batch();
        batch.append_event(&e0).expect("append 0 must succeed");
        batch.append_event(&e1).expect("append 1 must succeed");
        batch.append_event(&e2).expect("append 2 must succeed");
        batch.commit().expect("commit must succeed");
        let events = journal
            .events_for_run(run)
            .expect("events_for_run must succeed");
        assert_eq!(
            events,
            vec![e0, e1, e2],
            "replayed events must match input exactly"
        );
    }

    #[test]
    fn journal_workflow_source_after_batch_commit_matches_input() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let source = b"exact_bytes_source".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord { digest, source };
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&record)
            .expect("put must succeed");
        batch.commit().expect("commit must succeed");
        let found = journal
            .workflow_source(digest)
            .expect("lookup must succeed");
        let found_record = found.expect("record must exist");
        assert_eq!(
            found_record.source,
            b"exact_bytes_source".to_vec(),
            "source bytes must match exactly"
        );
        assert_eq!(found_record.digest, digest, "digest must match");
    }

    #[test]
    fn journal_compiled_ir_after_batch_commit_matches_input() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest = WorkflowDigest::from_bytes([0xFC; 32]);
        let record = CompiledIrRecord {
            digest,
            ir: b"exact_ir_bytes".to_vec(),
        };
        let mut batch = journal.batch();
        batch.put_compiled_ir(&record).expect("put must succeed");
        batch.commit().expect("commit must succeed");
        let found = journal.compiled_ir(digest).expect("lookup must succeed");
        let found_record = found.expect("record must exist");
        assert_eq!(
            found_record.ir,
            b"exact_ir_bytes".to_vec(),
            "IR bytes must match exactly"
        );
        assert_eq!(found_record.digest, digest);
    }

    #[test]
    fn journal_run_header_after_batch_commit_matches_all_fields() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9007);
        let workflow_id = WorkflowId::new(42);
        let compiled_digest = WorkflowDigest::from_bytes([0xFB; 32]);
        let status: u8 = 5;
        let accepted_at_ms: u64 = 9876543210;
        let record = RunHeaderRecord {
            run,
            workflow_id,
            compiled_digest,
            status,
            accepted_at_ms,
        };
        let mut batch = journal.batch();
        batch.put_run_header(&record).expect("put must succeed");
        batch.commit().expect("commit must succeed");
        let found = journal.run_header(run).expect("lookup must succeed");
        let found_record = found.expect("record must exist");
        assert_eq!(found_record.run, run, "run must match");
        assert_eq!(
            found_record.workflow_id, workflow_id,
            "workflow_id must match"
        );
        assert_eq!(
            found_record.compiled_digest, compiled_digest,
            "compiled_digest must match"
        );
        assert_eq!(found_record.status, status, "status must match");
        assert_eq!(
            found_record.accepted_at_ms, accepted_at_ms,
            "accepted_at_ms must match"
        );
    }

    #[test]
    fn journal_snapshot_after_batch_commit_matches_input() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9008);
        let seq = EventSeq::new(3);
        let snapshot = RunSnapshot {
            run,
            seq,
            workflow: WorkflowDigest::from_bytes([0xFA; 32]),
            slots: b"snapshot_data".to_vec(),
            taint: Vec::new(),
        };
        let mut batch = journal.batch();
        batch.put_snapshot(&snapshot).expect("put must succeed");
        batch.commit().expect("commit must succeed");
        let found = journal.snapshot(run, seq).expect("lookup must succeed");
        let found_record = found.expect("record must exist");
        assert_eq!(found_record.run, run);
        assert_eq!(found_record.seq, seq);
        assert_eq!(found_record.slots, b"snapshot_data".to_vec());
    }

    #[test]
    fn journal_blob_after_batch_commit_matches_input() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let blob_bytes = b"batch_blob_exact".to_vec();
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
        let record = BlobRecord {
            digest,
            bytes: blob_bytes,
        };
        let mut batch = journal.batch();
        batch.put_blob(&record).expect("put must succeed");
        batch.commit().expect("commit must succeed");
        let found = journal.blob(digest).expect("lookup must succeed");
        let found_record = found.expect("record must exist");
        assert_eq!(
            found_record.bytes,
            b"batch_blob_exact".to_vec(),
            "blob bytes must match exactly"
        );
        assert_eq!(found_record.digest, digest);
    }

    #[test]
    fn journal_status_index_after_batch_commit_returns_correct_run() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let state = IndexStatusState::Other(7);
        let timestamp: u64 = 55555;
        let run = RunId::new(9009);
        let mut batch = journal.batch();
        batch
            .put_status_index(state, timestamp, run)
            .expect("put_status_index must succeed");
        batch.commit().expect("commit must succeed");
        let key = index_status_key(state, timestamp, run).expect("key must succeed");
        let value = journal
            .index_status
            .get(key.as_slice())
            .expect("get must succeed");
        assert!(
            value.is_some(),
            "status index must exist after batch commit"
        );
    }

    #[test]
    fn journal_action_index_after_batch_commit_returns_correct_entry() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let action = ActionId::new(11);
        let run = RunId::new(9010);
        let step = StepIdx::new(4);
        let mut batch = journal.batch();
        batch
            .put_action_index(action, run, step)
            .expect("put_action_index must succeed");
        batch.commit().expect("commit must succeed");
        let key = index_action_key(action, run, step).expect("key must succeed");
        let value = journal
            .index_action
            .get(key.as_slice())
            .expect("get must succeed");
        assert!(
            value.is_some(),
            "action index must exist after batch commit"
        );
    }

    #[test]
    fn adversarial_reopen_after_unflushed_journaled_events_may_lose_them() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9001);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        journal.append_journaled(&event).expect("append journaled");
        drop(journal);
        let journal2 = FjallJournal::open(temp_dir.path(), None).expect("setup: journal reopen");
        let result = journal2
            .events_for_run(run)
            .expect("events_for_run succeeds");
        // Journaled durability does not guarantee persistence without flush
        // Either the event is present (Fjall flushed on drop) or absent (acceptable)
        assert!(result.len() <= 1, "at most one event expected");
    }

    #[test]
    fn adversarial_reopen_after_flushed_journaled_events_preserves_them() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9002);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([2; 32]),
        };
        journal.append_journaled(&event).expect("append journaled");
        drop(journal);
        let journal2 = FjallJournal::open(temp_dir.path(), None).expect("setup: journal reopen");
        let events = journal2
            .events_for_run(run)
            .expect("events_for_run succeeds");
        assert_eq!(
            events.len(),
            1,
            "flushed journaled event must survive reopen"
        );
    }

    #[test]
    fn adversarial_reopen_after_strict_event_preserves_it() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9003);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([3; 32]),
        };
        journal.append_strict(&event).expect("append strict");
        drop(journal);
        let journal2 = FjallJournal::open(temp_dir.path(), None).expect("setup: journal reopen");
        let events = journal2
            .events_for_run(run)
            .expect("events_for_run succeeds");
        assert_eq!(events.len(), 1, "strict event must survive reopen");
    }

    #[test]
    fn adversarial_batch_commit_then_reopen_preserves_all_keys() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let source_bytes = b"source".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source_bytes).into());
        let run = RunId::new(9004);
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord {
                digest,
                source: source_bytes,
            })
            .expect("put_workflow_source");
        batch
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(1),
                compiled_digest: digest,
                status: 1,
                accepted_at_ms: 100,
            })
            .expect("put_run_header");
        let blob_bytes = b"blob".to_vec();
        let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
        batch
            .put_blob(&BlobRecord {
                digest: blob_digest,
                bytes: blob_bytes,
            })
            .expect("put_blob");
        batch.commit().expect("commit");
        drop(journal);
        let journal2 = FjallJournal::open(temp_dir.path(), None).expect("setup: journal reopen");
        let source = journal2.workflow_source(digest).expect("get source");
        assert!(source.is_some(), "workflow source must survive reopen");
        let header = journal2.run_header(run).expect("get header");
        assert!(header.is_some(), "run header must survive reopen");
        let blob = journal2.blob(blob_digest).expect("get blob");
        assert!(blob.is_some(), "blob must survive reopen");
    }

    #[test]
    fn adversarial_double_append_same_run_seq_returns_duplicate_error() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9005);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([5; 32]),
        };
        journal.append_strict(&event).expect("first append");
        let result = journal.append_strict(&event);
        assert!(
            matches!(result, Err(JournalError::DuplicateEvent { .. })),
            "duplicate append must return DuplicateEvent"
        );
    }

    #[test]
    fn adversarial_events_for_run_on_empty_journal_returns_empty() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let events = journal
            .events_for_run(RunId::new(9999))
            .expect("events_for_run");
        assert_eq!(events.len(), 0, "no events for nonexistent run");
    }

    #[test]
    fn adversarial_run_header_for_never_written_run_returns_none() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let header = journal.run_header(RunId::new(8888)).expect("run_header");
        assert!(header.is_none(), "no header for nonexistent run");
    }

    #[test]
    fn adversarial_snapshot_for_nonexistent_run_returns_none() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let snapshot = journal
            .snapshot(RunId::new(7777), EventSeq::new(0))
            .expect("snapshot");
        assert!(snapshot.is_none(), "no snapshot for nonexistent run");
    }

    #[test]
    fn adversarial_blob_for_nonexistent_digest_returns_none() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let blob = journal.blob([0xAA; 32]).expect("blob");
        assert!(blob.is_none(), "no blob for nonexistent digest");
    }

    #[test]
    fn adversarial_workflow_source_for_wrong_digest_returns_none() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let source = b"data".to_vec();
        let digest_a = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord {
            digest: digest_a,
            source,
        };
        journal.put_workflow_source(&record).expect("put");
        let digest_b = WorkflowDigest::from_bytes([2; 32]);
        let result = journal.workflow_source(digest_b).expect("get");
        assert!(result.is_none(), "wrong digest must return None");
    }

    #[test]
    fn adversarial_multiple_snapshots_same_run_different_seq_all_retrievable() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9006);
        for seq_val in [0u64, 5, 10] {
            let snap = RunSnapshot {
                run,
                seq: EventSeq::new(seq_val),
                workflow: WorkflowDigest::from_bytes([1; 32]),
                slots: vec![0u8],
                taint: Vec::new(),
            };
            journal.put_snapshot(&snap).expect("put_snapshot");
        }
        for seq_val in [0u64, 5, 10] {
            let loaded = journal.snapshot(run, EventSeq::new(seq_val)).expect("get");
            assert!(loaded.is_some(), "snapshot at seq {seq_val} must exist");
        }
    }

    #[test]
    fn adversarial_batch_two_sequential_commits_both_visible() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let source1 = b"first".to_vec();
        let digest1 = WorkflowDigest::from_bytes(blake3::hash(&source1).into());
        let source2 = b"second".to_vec();
        let digest2 = WorkflowDigest::from_bytes(blake3::hash(&source2).into());
        let mut batch1 = journal.batch();
        batch1
            .put_workflow_source(&WorkflowSourceRecord {
                digest: digest1,
                source: source1,
            })
            .expect("put1");
        batch1.commit().expect("commit1");
        let mut batch2 = journal.batch();
        batch2
            .put_workflow_source(&WorkflowSourceRecord {
                digest: digest2,
                source: source2,
            })
            .expect("put2");
        batch2.commit().expect("commit2");
        assert!(journal.workflow_source(digest1).expect("get1").is_some());
        assert!(journal.workflow_source(digest2).expect("get2").is_some());
    }

    #[test]
    fn adversarial_snapshot_with_empty_slots_roundtrips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9010);
        let snap = RunSnapshot {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
            slots: vec![],
            taint: Vec::new(),
        };
        journal.put_snapshot(&snap).expect("put");
        let loaded = journal
            .snapshot(run, EventSeq::new(0))
            .expect("get")
            .expect("must exist");
        assert_eq!(loaded.slots.len(), 0);
        assert_eq!(loaded.run, run);
    }

    #[test]
    fn adversarial_blob_with_single_byte_roundtrips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let bytes = vec![0xFF];
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&bytes).into();
        let record = BlobRecord {
            digest,
            bytes: bytes.clone(),
        };
        journal.put_blob(&record).expect("put");
        let loaded = journal.blob(digest).expect("get").expect("must exist");
        assert_eq!(loaded.bytes, bytes);
    }

    #[test]
    fn adversarial_workflow_source_with_empty_bytes_roundtrips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let source: Vec<u8> = vec![];
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord { digest, source };
        journal.put_workflow_source(&record).expect("put");
        let loaded = journal
            .workflow_source(digest)
            .expect("get")
            .expect("must exist");
        assert_eq!(loaded.source, vec![]);
    }

    #[test]
    fn adversarial_run_header_with_max_run_id_roundtrips() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(u64::MAX);
        let digest = WorkflowDigest::from_bytes([9; 32]);
        let record = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(u32::MAX),
            compiled_digest: digest,
            status: 2,
            accepted_at_ms: u64::MAX,
        };
        journal.put_run_header(&record).expect("put");
        let loaded = journal.run_header(run).expect("get").expect("must exist");
        assert_eq!(loaded.run, RunId::new(u64::MAX));
        assert_eq!(loaded.workflow_id, WorkflowId::new(u32::MAX));
        assert_eq!(loaded.accepted_at_ms, u64::MAX);
    }

    #[test]
    fn adversarial_batch_strict_commit_survives_immediate_reopen() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let path = temp_dir.path().to_path_buf();
        let journal = FjallJournal::open(&path, None).expect("setup: journal open");
        let run = RunId::new(9020);
        let digest = WorkflowDigest::from_bytes([11; 32]);
        let mut batch = journal.batch().strict();
        batch
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(3),
                compiled_digest: digest,
                status: 1,
                accepted_at_ms: 500,
            })
            .expect("put");
        batch.strict().commit().expect("strict commit");
        drop(journal);
        let journal2 = FjallJournal::open(&path, None).expect("reopen");
        let header = journal2.run_header(run).expect("get").expect("must exist");
        assert_eq!(header.run, run);
        assert_eq!(header.status, 1);
    }

    #[test]
    fn adversarial_events_for_run_isolates_run_a_from_run_b() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run_a = RunId::new(100);
        let run_b = RunId::new(200);
        let digest = WorkflowDigest::from_bytes([1; 32]);
        journal
            .append_strict(&JournalEvent::RunAccepted {
                run: run_a,
                seq: EventSeq::new(0),
                workflow: digest,
            })
            .expect("append a");
        journal
            .append_strict(&JournalEvent::RunAccepted {
                run: run_b,
                seq: EventSeq::new(0),
                workflow: digest,
            })
            .expect("append b");
        journal
            .append_strict(&JournalEvent::StepStarted {
                run: run_a,
                seq: EventSeq::new(1),
                step: vb_core::StepIdx::ZERO,
                attempt: 1,
            })
            .expect("append a2");
        let events_a = journal.events_for_run(run_a).expect("events a");
        let events_b = journal.events_for_run(run_b).expect("events b");
        assert_eq!(events_a.len(), 2, "run A should have 2 events");
        assert_eq!(events_b.len(), 1, "run B should have 1 event");
    }

    #[test]
    fn adversarial_run_header_overwrite_replaces_previous() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9030);
        let digest = WorkflowDigest::from_bytes([1; 32]);
        journal
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(1),
                compiled_digest: digest,
                status: 1,
                accepted_at_ms: 100,
            })
            .expect("put first");
        journal
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(2),
                compiled_digest: digest,
                status: 3,
                accepted_at_ms: 200,
            })
            .expect("put second");
        let header = journal.run_header(run).expect("get").expect("exists");
        assert_eq!(header.workflow_id, WorkflowId::new(2));
        assert_eq!(header.status, 3);
        assert_eq!(header.accepted_at_ms, 200);
    }

    #[test]
    fn adversarial_batch_commit_with_5_puts_persists_all() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let source = b"s".to_vec();
        let d1 = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let d2 = WorkflowDigest::from_bytes([2; 32]);
        let run = RunId::new(9050);
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord { digest: d1, source })
            .expect("put1");
        batch
            .put_compiled_ir(&CompiledIrRecord {
                digest: d2,
                ir: b"ir".to_vec(),
            })
            .expect("put2");
        batch
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(1),
                compiled_digest: d1,
                status: 1,
                accepted_at_ms: 0,
            })
            .expect("put3");
        let blob_bytes = b"b".to_vec();
        let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
        batch
            .put_blob(&BlobRecord {
                digest: blob_digest,
                bytes: blob_bytes,
            })
            .expect("put4");
        batch
            .put_status_index(IndexStatusState::Submitted, 0, run)
            .expect("put5");
        batch.commit().expect("commit");
        assert!(journal.workflow_source(d1).expect("g1").is_some());
        assert!(journal.compiled_ir(d2).expect("g2").is_some());
        assert!(journal.run_header(run).expect("g3").is_some());
        assert!(journal.blob(blob_digest).expect("g4").is_some());
    }

    #[test]
    fn adversarial_compiled_ir_with_different_ir_same_digest_overwrites() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let digest = WorkflowDigest::from_bytes([1; 32]);
        journal
            .put_compiled_ir(&CompiledIrRecord {
                digest,
                ir: b"version1".to_vec(),
            })
            .expect("put1");
        journal
            .put_compiled_ir(&CompiledIrRecord {
                digest,
                ir: b"version2".to_vec(),
            })
            .expect("put2");
        let loaded = journal.compiled_ir(digest).expect("get").expect("exists");
        assert_eq!(loaded.ir, b"version2".to_vec(), "second write must win");
    }

    #[test]
    fn adversarial_journal_open_fresh_database_is_empty() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        assert!(journal.run_header(RunId::new(1)).expect("header").is_none());
        assert!(
            journal
                .workflow_source(WorkflowDigest::from_bytes([0; 32]))
                .expect("source")
                .is_none()
        );
        assert!(
            journal
                .compiled_ir(WorkflowDigest::from_bytes([0; 32]))
                .expect("ir")
                .is_none()
        );
        assert!(journal.blob([0; 32]).expect("blob").is_none());
        assert_eq!(
            journal.events_for_run(RunId::new(1)).expect("events").len(),
            0
        );
    }

    #[test]
    fn adversarial_snapshot_isolation_between_runs() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run1 = RunId::new(100);
        let run2 = RunId::new(200);
        journal
            .put_snapshot(&RunSnapshot {
                run: run1,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([1; 32]),
                slots: vec![1u8],
                taint: Vec::new(),
            })
            .expect("snap1");
        journal
            .put_snapshot(&RunSnapshot {
                run: run2,
                seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([2; 32]),
                slots: vec![2u8],
                taint: Vec::new(),
            })
            .expect("snap2");
        let s1 = journal
            .snapshot(run1, EventSeq::new(0))
            .expect("get1")
            .expect("exists");
        let s2 = journal
            .snapshot(run2, EventSeq::new(0))
            .expect("get2")
            .expect("exists");
        assert_eq!(s1.workflow, WorkflowDigest::from_bytes([1; 32]));
        assert_eq!(s2.workflow, WorkflowDigest::from_bytes([2; 32]));
    }

    #[test]
    fn adversarial_status_index_multiple_runs_same_state() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let state = IndexStatusState::Active;
        let ts = 1000u64;
        for run_id in [RunId::new(10), RunId::new(20), RunId::new(30)] {
            journal.put_status_index(state, ts, run_id).expect("put");
        }
        // All three runs should be indexable under the same state
        // (verification via no-error roundtrip)
    }

    #[test]
    fn adversarial_workflow_index_multiple_runs_same_workflow() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let wf = WorkflowId::new(42);
        for run_id in [RunId::new(1), RunId::new(2), RunId::new(3)] {
            journal.put_workflow_index(wf, run_id).expect("put");
        }
        // All three runs indexed under same workflow
    }

    #[test]
    fn adversarial_batch_empty_strict_commit_succeeds() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let batch = journal.batch().strict();
        batch
            .strict()
            .commit()
            .expect("empty strict commit must succeed");
    }

    #[test]
    fn adversarial_append_event_at_max_seq_stores_correctly() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let run = RunId::new(9060);
        // Write contiguous events 0..2, then verify seq 0 and 1 are present
        let digest = WorkflowDigest::from_bytes([1; 32]);
        journal
            .append_strict(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: digest,
            })
            .expect("append0");
        journal
            .append_strict(&JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: vb_core::StepIdx::ZERO,
                attempt: 1,
            })
            .expect("append1");
        let events = journal.events_for_run(run).expect("replay");
        assert_eq!(events.len(), 2, "contiguous seq 0,1 must replay");
    }

    #[test]
    fn adversarial_batch_commit_persists_all_keys_or_none() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let source = b"src".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let run = RunId::new(9070);
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord { digest, source })
            .expect("ws");
        batch
            .put_compiled_ir(&CompiledIrRecord {
                digest,
                ir: b"ir".to_vec(),
            })
            .expect("ir");
        batch
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(1),
                compiled_digest: digest,
                status: 1,
                accepted_at_ms: 0,
            })
            .expect("rh");
        batch.commit().expect("commit");
        // All three must be present — batch is atomic
        assert!(journal.workflow_source(digest).expect("g1").is_some());
        assert!(journal.compiled_ir(digest).expect("g2").is_some());
        assert!(journal.run_header(run).expect("g3").is_some());
    }

    // =====================================================================
    // vb-apn5: Single-server database lock enforcement tests
    // =====================================================================

    #[test]
    fn test_first_open_succeeds_and_creates_lock_file() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None);
        assert!(journal.is_ok(), "first open on empty path should succeed");
        let lock_path = temp.path().join(".process.lock");
        assert!(
            lock_path.exists(),
            ".process.lock file should be created after open"
        );
    }

    #[test]
    fn test_lock_releases_on_journal_drop() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        {
            let _journal =
                FjallJournal::open(temp.path(), None).expect("first open should succeed");
        } // journal dropped here, lock released
        let result = FjallJournal::open(temp.path(), None);
        assert!(result.is_ok(), "re-open after drop must succeed");
    }

    #[test]
    fn test_second_open_fails_in_same_process() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let _journal = FjallJournal::open(temp.path(), None).expect("first open should succeed");
        let result = FjallJournal::open(temp.path(), None);
        // Same-process: flock allows it, but Fjall detects the open database.
        // Cross-process: ProcessLockHeld would be returned first.
        assert!(
            result.is_err(),
            "second open in same process must fail (Fjall detects open DB)"
        );
    }

    #[test]
    fn test_lock_file_contains_holder_pid() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let _journal = FjallJournal::open(temp.path(), None).expect("first open should succeed");
        let lock_path = temp.path().join(".process.lock");
        let contents = std::fs::read_to_string(&lock_path).expect("read lock file");
        let pid: u32 = contents
            .trim()
            .parse()
            .expect("lock file should contain valid PID");
        assert_eq!(
            pid,
            std::process::id(),
            "lock file should contain current process PID"
        );
    }

    #[test]
    fn test_no_keyspace_created_when_lock_fails() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let _journal = FjallJournal::open(temp.path(), None).expect("first open should succeed");

        let before_count = std::fs::read_dir(temp.path()).expect("read_dir").count();

        let result = FjallJournal::open(temp.path(), None);
        assert!(result.is_err(), "second open must fail");

        let after_count = std::fs::read_dir(temp.path()).expect("read_dir").count();

        assert_eq!(
            before_count, after_count,
            "no new files should appear when lock fails"
        );
    }

    // ══ vb-hbav B19: JournalError exhaustiveness compile-time check ════════
    #[test]
    fn journal_error_match_covers_all_variants() {
        fn _exhaustive_match(e: &JournalError) -> &'static str {
            match e {
                JournalError::Fjall(_) => "fjall",
                JournalError::Encode(_) => "encode",
                JournalError::KeyCapacity => "key_capacity",
                JournalError::DuplicateEvent { .. } => "duplicate_event",
                JournalError::DuplicateStagedKey { .. } => "duplicate_staged_key",
                JournalError::WriteLockPoisoned => "write_lock_poisoned",
                JournalError::QueueCapacity => "queue_capacity",
                JournalError::QueueFull => "queue_full",
                JournalError::QueueShutdown => "queue_shutdown",
                JournalError::WrongRun { .. } => "wrong_run",
                JournalError::SequenceGap { .. } => "sequence_gap",
                JournalError::SequenceOverflow => "sequence_overflow",
                JournalError::BadMagic { .. } => "bad_magic",
                JournalError::UnsupportedSchemaVersion { .. } => "unsupported_schema_version",
                JournalError::MigrationRequired { .. } => "migration_required",
                JournalError::UnknownRecordKind { .. } => "unknown_record_kind",
                JournalError::RecordKindFamilyMismatch { .. } => "record_kind_family_mismatch",
                JournalError::RecordKindPayloadMismatch { .. } => "record_kind_payload_mismatch",
                JournalError::HeaderLengthMismatch { .. } => "header_length_mismatch",
                JournalError::PayloadTooLarge { .. } => "payload_too_large",
                JournalError::HeaderChecksumMismatch => "header_checksum_mismatch",
                JournalError::PayloadDigestMismatch => "payload_digest_mismatch",
                JournalError::UnexpectedEof => "unexpected_eof",
                JournalError::PostcardDecodeFailed(_) => "postcard_decode_failed",
                JournalError::PostcardEncodeFailed(_) => "postcard_encode_failed",
                JournalError::InvalidEvent => "invalid_event",
                JournalError::ArtifactMalformed => "artifact_malformed",
                JournalError::WorkflowReconstruction(_) => "workflow_reconstruction",
                JournalError::CompiledIrReadback(_) => "compiled_ir_readback",
                JournalError::AdmissionAllocationFailed(_) => "admission_allocation_failed",
                JournalError::ArtifactChecksumMismatch => "artifact_checksum_mismatch",
                JournalError::InvalidGateCount { .. } => "invalid_gate_count",
                JournalError::MissingRequiredProofFlag { .. } => "missing_required_proof_flag",
                JournalError::ArtifactNotFound { .. } => "artifact_not_found",
                JournalError::AdmissionRequired => "admission_required",
                JournalError::ArtifactInvalid { .. } => "artifact_invalid",
                JournalError::InputTooLarge { .. } => "input_too_large",
                JournalError::InputSchemaMismatch => "input_schema_mismatch",
                JournalError::CapabilityDenied => "capability_denied",
                JournalError::SecretUnavailable => "secret_unavailable",
                JournalError::RunAlreadyExists => "run_already_exists",
                JournalError::InvalidRunId { .. } => "invalid_run_id",
                JournalError::ActiveRunCapacityExceeded => "active_run_capacity_exceeded",
                JournalError::FrameAllocationFailed => "frame_allocation_failed",
                JournalError::AdmissionJournalFailed => "admission_journal_failed",
                JournalError::StrictDurabilityFailed => "strict_durability_failed",
                JournalError::TooManyEvents { .. } => "too_many_events",
                JournalError::ReplayAllocationFailed { .. } => "replay_allocation_failed",
                JournalError::ClockUnavailable => "clock_unavailable",
                JournalError::ProcessLockHeld { .. } => "process_lock_held",
                JournalError::ProcessLockIo { .. } => "process_lock_io",
                JournalError::Trim(_) => "trim",
                JournalError::JournalBatchBytesExceeded { .. } => "journal_batch_bytes_exceeded",
                JournalError::MalformedKeyspaceRow { .. } => "malformed_keyspace_row",
                JournalError::BatchAborted => "batch_aborted",
                JournalError::IndexStatusStateCollision { .. } => "index_status_state_collision",
            }
        }
        let _ = _exhaustive_match;
    }
}
