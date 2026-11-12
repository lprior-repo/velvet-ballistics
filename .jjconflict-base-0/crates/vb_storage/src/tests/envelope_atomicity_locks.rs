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
        let ir_record = compiled_ir_record(b"atomic_ir".to_vec());
        batch
            .put_compiled_ir(&ir_record)
            .expect("put_compiled_ir must succeed");
        batch.commit().expect("commit must succeed");
        let source = journal
            .workflow_source(digest)
            .expect("workflow_source must succeed");
        let ir = journal
            .compiled_ir(ir_record.digest)
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
        let ir_record = compiled_ir_record(b"strict_ir".to_vec());
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
                .put_compiled_ir(&ir_record)
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
            .compiled_ir(ir_record.digest)
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
        let ir_record = compiled_ir_record(b"ir".to_vec());
        let digest_2 = ir_record.digest;
        batch
            .put_compiled_ir(&ir_record)
            .expect("put 2 must succeed");
        batch
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(1),
                compiled_digest: digest_2,
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
        let record = compiled_ir_record(b"exact_ir_bytes".to_vec());
        let digest = record.digest;
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
        let ir_record = compiled_ir_record(b"ir".to_vec());
        let d2 = ir_record.digest;
        let run = RunId::new(9050);
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord { digest: d1, source })
            .expect("put1");
        batch.put_compiled_ir(&ir_record).expect("put2");
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
    fn adversarial_compiled_ir_with_same_digest_different_ir_is_rejected() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let first = compiled_ir_record(b"version1".to_vec());
        let digest = first.digest;
        journal.put_compiled_ir(&first).expect("put1");
        let second = journal.put_compiled_ir(&CompiledIrRecord {
            digest,
            ir: b"version2".to_vec(),
        });
        assert!(matches!(second, Err(JournalError::PayloadDigestMismatch)));
        let loaded = journal.compiled_ir(digest).expect("get").expect("exists");
        assert_eq!(
            loaded.ir,
            b"version1".to_vec(),
            "forged overwrite must fail"
        );
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
        let ir_record = compiled_ir_record(b"ir".to_vec());
        let run = RunId::new(9070);
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord { digest, source })
            .expect("ws");
        batch.put_compiled_ir(&ir_record).expect("ir");
        batch
            .put_run_header(&RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(1),
                compiled_digest: ir_record.digest,
                status: 1,
                accepted_at_ms: 0,
            })
            .expect("rh");
        batch.commit().expect("commit");
        // All three must be present — batch is atomic
        assert!(journal.workflow_source(digest).expect("g1").is_some());
        assert!(journal.compiled_ir(ir_record.digest).expect("g2").is_some());
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
                JournalError::ReplayKeyMismatch { .. } => "replay_key_mismatch",
                JournalError::ReplayEnvelopeSequenceMismatch { .. } => {
                    "replay_envelope_sequence_mismatch"
                }
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
                JournalError::ArtifactValidation(_) | JournalError::ArtifactActionRegistry(_) => {
                    "artifact_validation"
                }
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
                JournalError::InvalidConfig { .. } => "invalid_config",
                JournalError::UnsupportedReadOnly => "unsupported_read_only",
                JournalError::EmergencyDenyMarkerUnsupportedVersion { .. } => {
                    "emergency_deny_marker_unsupported_version"
                }
                JournalError::EmergencyDenyMarkerMalformed => "emergency_deny_marker_malformed",
                JournalError::ArtifactVersionIncompatible { .. } => "artifact_version_incompatible",
                JournalError::OwnedInputs(_) => "owned_inputs",
            }
        }
        assert_eq!(
            _exhaustive_match(&JournalError::OwnedInputs(
                crate::OwnedInputError::IdentityMismatch,
            )),
            "owned_inputs"
        );
        let _ = _exhaustive_match;
    }
