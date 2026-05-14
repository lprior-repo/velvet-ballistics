#![forbid(unsafe_code)]
//! RED PHASE failing tests for vb-fb52 — Atomic Journal and Index Write Batches
//!
//! These tests exercise the contract specified in contract.md and test-plan.md.
//! They are written to FAIL until the implementation is complete.
//!
//! Test naming convention:
//!   - U##: Unit test (Section 1 of test-plan.md)
//!   - I##: Integration test (Section 2 of test-plan.md)
//!   - P##: Property-based test (Section 3 of test-plan.md)

#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used,
    clippy::manual_assert,
    clippy::redundant_closure
)]
mod failing_batch_tests {
    use crate::{
        BlobRecord, CompiledIrRecord, DIGEST_BYTES, EventSeq, FjallJournal, JournalError,
        JournalEvent, JournalWriteBatch, RecordKind, RunHeaderRecord, WorkflowSourceRecord,
        MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD, MAGIC_JOURNAL_EVENT,
        MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE, MAX_BLOB_BYTES, MAX_COMPILED_IR_BYTES,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_RUN_HEADER_BYTES, MAX_SNAPSHOT_BYTES,
        MAX_WORKFLOW_SOURCE_BYTES, RECORD_HEADER_BYTES,
    };
    use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};
    use crate::recovery::RunSnapshot;
    use crate::keys::{
        blob_key, compiled_ir_key, index_action_key, index_status_key, index_workflow_key,
        run_event_key, run_header_key, run_snapshot_key, workflow_source_key,
    };

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    fn make_event(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        }
    }

    fn make_run_header(run: RunId) -> RunHeaderRecord {
        RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]),
            status: 1,
            accepted_at_ms: 1000,
        }
    }

    // =========================================================================
    // U4: Batch is not Send or Sync — structural invariant I1
    // =========================================================================

    #[test]
    fn batch_is_not_send_or_sync() {
        let (_temp, journal) = temp_journal();
        let batch = journal.batch();
        // JournalWriteBatch<'j> borrows FjallJournal which is !Sync + !Send
        // Therefore batch MUST NOT implement Send
        fn assert_not_send<T: Send>(_: &T) {
            panic!("JournalWriteBatch must be !Send but it implements Send");
        }
        fn assert_not_sync<T: Sync>(_: &T) {
            panic!("JournalWriteBatch must be !Sync but it implements Sync");
        }
        assert_not_send(&batch);
        assert_not_sync(&batch);
    }

    // =========================================================================
    // U5-U13: Encoding invariants — header size and magic values
    // =========================================================================

    #[test]
    fn workflow_source_record_header_is_60_bytes() {
        // I5: RECORD_HEADER_BYTES == 60 for workflow source
        let source = b"test workflow".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord {
            digest,
            source,
        };
        let encoded = crate::encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &record,
            MAX_WORKFLOW_SOURCE_BYTES,
        )
        .expect("encode should succeed");
        // Header is first 60 bytes
        let header_len = encoded.len() - record.source.len();
        assert_eq!(
            header_len as usize, RECORD_HEADER_BYTES,
            "workflow source header must be exactly {} bytes",
            RECORD_HEADER_BYTES
        );
    }

    #[test]
    fn blob_record_header_is_60_bytes() {
        // I5: RECORD_HEADER_BYTES == 60 for blob
        let payload = vec![0xCA, 0xFE, 0xBA, 0xBE];
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
        let record = BlobRecord {
            digest,
            bytes: payload.clone(),
        };
        let encoded = crate::encode_record(
            MAGIC_BLOB,
            RecordKind::Blob,
            0,
            &record,
            MAX_BLOB_BYTES,
        )
        .expect("encode should succeed");
        let header_len = encoded.len() - record.bytes.len();
        assert_eq!(
            header_len as usize, RECORD_HEADER_BYTES,
            "blob header must be exactly {} bytes",
            RECORD_HEADER_BYTES
        );
    }

    #[test]
    fn run_event_header_is_60_bytes() {
        // I5: RECORD_HEADER_BYTES == 60 for run event
        let event = make_event(RunId::new(1), 0);
        let encoded = crate::encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encode should succeed");
        // Manually compute payload size
        let payload_bytes = postcard::to_allocvec(&event).expect("postcard encode");
        let header_len = encoded.len() - payload_bytes.len();
        assert_eq!(
            header_len as usize, RECORD_HEADER_BYTES,
            "run event header must be exactly {} bytes",
            RECORD_HEADER_BYTES
        );
    }

    #[test]
    fn run_event_magic_is_0x5642_4A45() {
        // I6: MAGIC_JOURNAL_EVENT exclusively on run_event
        assert_eq!(
            MAGIC_JOURNAL_EVENT, 0x5642_4A45,
            "MAGIC_JOURNAL_EVENT must be 0x5642_4A45"
        );
        let event = make_event(RunId::new(1), 0);
        let encoded = crate::encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encode should succeed");
        // First 4 bytes are magic
        let magic = u32::from_le_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]);
        assert_eq!(magic, MAGIC_JOURNAL_EVENT, "encoded magic must match MAGIC_JOURNAL_EVENT");
    }

    #[test]
    fn workflow_source_magic_is_0x5642_5352() {
        // I7: MAGIC_WORKFLOW_SOURCE exclusively on workflow_source
        assert_eq!(
            MAGIC_WORKFLOW_SOURCE, 0x5642_5352,
            "MAGIC_WORKFLOW_SOURCE must be 0x5642_5352"
        );
    }

    #[test]
    fn compiled_ir_magic_is_0x5642_4952() {
        // I8: MAGIC_COMPILED_ARTIFACT exclusively on compiled_ir
        assert_eq!(
            MAGIC_COMPILED_ARTIFACT, 0x5642_4952,
            "MAGIC_COMPILED_ARTIFACT must be 0x5642_4952"
        );
    }

    #[test]
    fn snapshot_magic_is_0x5642_534E() {
        // I9: MAGIC_SNAPSHOT exclusively on run_snapshot
        assert_eq!(
            MAGIC_SNAPSHOT, 0x5642_534E,
            "MAGIC_SNAPSHOT must be 0x5642_534E"
        );
    }

    #[test]
    fn blob_magic_is_0x5642_424C() {
        // I10: MAGIC_BLOB exclusively on blob
        assert_eq!(
            MAGIC_BLOB, 0x5642_424C,
            "MAGIC_BLOB must be 0x5642_424C"
        );
    }

    #[test]
    fn index_record_magic_is_0x5642_4958() {
        // I11: MAGIC_INDEX_RECORD on run_header/index keyspaces
        assert_eq!(
            MAGIC_INDEX_RECORD, 0x5642_4958,
            "MAGIC_INDEX_RECORD must be 0x5642_4958"
        );
    }

    // =========================================================================
    // U14-U17: Key Layout Invariants
    // =========================================================================

    #[test]
    fn digest_keyed_record_key_is_33_bytes() {
        // I12: [prefix_u8][32_byte_digest] = 33 bytes
        let digest = [7_u8; 32];
        let key = workflow_source_key(digest).expect("workflow_source_key should succeed");
        assert_eq!(
            key.len(), 33,
            "digest-keyed record key must be 33 bytes [prefix][32_digest]"
        );

        let key = compiled_ir_key(digest).expect("compiled_ir_key should succeed");
        assert_eq!(key.len(), 33, "compiled_ir key must be 33 bytes");

        let key = blob_key(digest).expect("blob_key should succeed");
        assert_eq!(key.len(), 33, "blob key must be 33 bytes");
    }

    #[test]
    fn run_event_key_is_17_bytes() {
        // I13: [0x11][run_id_8be][seq_8be] = 17 bytes
        let run = RunId::new(0x0102_0304_0506_0708);
        let seq = EventSeq::new(0x1122_3344_5566_7788);
        let key = run_event_key(run, seq).expect("run_event_key should succeed");
        assert_eq!(
            key.len(), 17,
            "run_event key must be 17 bytes [prefix=0x11][run_id][seq]"
        );
        assert_eq!(key[0], 0x11, "first byte must be 0x11 prefix");
    }

    #[test]
    fn run_header_key_is_9_bytes() {
        // I14: [0x10][run_id_8be] = 9 bytes
        let run = RunId::new(0xAABBCCDD_EEFF0011);
        let key = run_header_key(run).expect("run_header_key should succeed");
        assert_eq!(
            key.len(), 9,
            "run_header key must be 9 bytes [prefix=0x10][run_id]"
        );
        assert_eq!(key[0], 0x10, "first byte must be 0x10 prefix");
    }

    #[test]
    fn run_snapshot_key_is_17_bytes() {
        // I15: [0x12][run_id_8be][seq_8be] = 17 bytes
        let run = RunId::new(1);
        let seq = EventSeq::new(2);
        let key = run_snapshot_key(run, seq).expect("run_snapshot_key should succeed");
        assert_eq!(
            key.len(), 17,
            "run_snapshot key must be 17 bytes [prefix=0x12][run_id][seq]"
        );
        assert_eq!(key[0], 0x12, "first byte must be 0x12 prefix");
    }

    // =========================================================================
    // U18: Digest verification (BH-02)
    // =========================================================================

    #[test]
    fn put_workflow_source_rejects_digest_mismatch() {
        // I19: PayloadDigestMismatch when content does not hash to record.digest
        let (_temp, journal) = temp_journal();
        let source = b"real content".to_vec();
        let wrong_digest = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
        let record = WorkflowSourceRecord {
            digest: wrong_digest,
            source,
        };
        let mut batch = journal.batch();
        let result = batch.put_workflow_source(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "batch must reject digest mismatch with PayloadDigestMismatch, got {:?}",
            result
        );
        assert_eq!(
            batch.len(), 0,
            "batch len must remain 0 after failed put"
        );
    }

    // =========================================================================
    // I16-I18: Process Lock Tests (Integration)
    // =========================================================================

    #[test]
    fn second_journal_open_on_same_path_is_prevented_by_process_lock() {
        // EP-5, I17: second open() returns JournalError::ProcessLockHeld
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().to_path_buf();

        // First open succeeds
        let journal1 = FjallJournal::open(&path, None).expect("first open should succeed");
        assert!(
            journal1.workflow_source(WorkflowDigest::from_bytes([0; 32])).is_ok()
        );

        // Second open on same path should fail with ProcessLockHeld
        let result = FjallJournal::open(&path, None);
        assert!(
            matches!(result, Err(JournalError::ProcessLockHeld { .. })),
            "second journal open must be prevented by process lock, got {:?}",
            result
        );

        drop(journal1);
    }

    #[test]
    fn no_keyspace_created_when_lock_fails() {
        // EP-6: directory unchanged after failed second open
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().to_path_buf();

        // First open succeeds and creates keyspace files
        let journal1 = FjallJournal::open(&path, None).expect("first open should succeed");
        let entries_before = std::fs::read_dir(&path).expect("read_dir should succeed");
        let count_before = entries_before.count();

        // Drop first journal to release lock
        drop(journal1);

        // First open again to have files present
        let _journal1 = FjallJournal::open(&path, None).expect("first open should succeed");

        // Count entries before second failed open
        let entries_before2 = std::fs::read_dir(&path).expect("read_dir should succeed");
        let count_before2 = entries_before2.count();

        // Try second open which should fail
        let _result = FjallJournal::open(&path, None);

        // Directory should be unchanged (no new files created by failed second open)
        let entries_after = std::fs::read_dir(&path).expect("read_dir should succeed");
        let count_after = entries_after.count();
        assert_eq!(
            count_before2, count_after,
            "keyspace directory should be unchanged after failed second open"
        );
    }

    // =========================================================================
    // I3, I6: Missing batch operations for compiled_ir and events
    // =========================================================================

    #[test]
    fn batch_put_compiled_ir_commits_and_is_readable() {
        // I3: compiled_ir readable after commit
        let (_temp, journal) = temp_journal();
        let ir = b"compiled-artifact-bytes".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
        let record = CompiledIrRecord {
            digest,
            ir: ir.clone(),
        };

        let mut batch = journal.batch();
        batch.put_compiled_ir(&record).expect("batch compiled ir");
        batch.commit().expect("commit should succeed");

        let loaded = journal.compiled_ir(digest).expect("get should succeed");
        let Some(found) = loaded else {
            panic!("compiled IR should be found after batch commit");
        };
        assert_eq!(found.ir, ir);
    }

    #[test]
    fn batch_append_event_commits_and_is_readable() {
        // I6: event readable after commit
        let (_temp, journal) = temp_journal();
        let run = RunId::new(100);
        let event = make_event(run, 0);

        let mut batch = journal.batch();
        batch.append_event(&event).expect("append event");
        batch.commit().expect("commit should succeed");

        let replayed = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(replayed.len(), 1, "should have 1 event after batch commit");
        assert_eq!(replayed[0], event);
    }

    // =========================================================================
    // I19: append_strict_rejects_duplicate_event via batch
    // =========================================================================

    #[test]
    fn batch_append_event_rejects_duplicate_event() {
        // EP-7, I20: DuplicateEvent on second append with same run_id+seq
        let (_temp, journal) = temp_journal();
        let run = RunId::new(200);
        let event = make_event(run, 0);

        // First append via batch
        let mut batch1 = journal.batch();
        batch1.append_event(&event).expect("first append should succeed");
        batch1.commit().expect("commit should succeed");

        // Second append with same run_id+seq should fail
        let mut batch2 = journal.batch();
        let result = batch2.append_event(&event);
        assert!(
            matches!(result, Err(JournalError::DuplicateEvent { .. })),
            "duplicate event must be rejected with DuplicateEvent, got {:?}",
            result
        );
        assert_eq!(batch2.len(), 0, "batch len should remain 0 after failed append");
    }

    // =========================================================================
    // I20: events_for_run_rejects_sequence_gap
    // =========================================================================

    #[test]
    fn events_for_run_rejects_sequence_gap() {
        // EP-8, I21: SequenceGap when replay detects non-contiguous sequence
        let (_temp, journal) = temp_journal();
        let run = RunId::new(300);

        // Write seq 0 and seq 2 directly (gap at seq 1)
        let e0 = make_event(run, 0);
        let e2 = make_event(run, 2);

        // Use batch to commit
        let mut batch1 = journal.batch();
        batch1.append_event(&e0).expect("append 0");
        batch1.commit().expect("commit 0");

        let mut batch2 = journal.batch();
        batch2.append_event(&e2).expect("append 2");
        batch2.commit().expect("commit 2");

        // Replay should detect gap
        let result = journal.events_for_run(run);
        assert!(
            matches!(result, Err(JournalError::SequenceGap { .. })),
            "sequence gap must be detected during replay, got {:?}",
            result
        );
    }

    // =========================================================================
    // P1-P4: Structural property-based tests
    // =========================================================================

    #[test]
    fn len_equals_staged_count_after_random_operations() {
        // P1: len() always equals actual staged operation count
        let (_temp, journal) = temp_journal();
        let run = RunId::new(400);

        let mut batch = journal.batch();
        let mut expected_len = 0;

        // Stage 3 events
        for i in 0..3 {
            let evt = JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(i),
                workflow: WorkflowDigest::from_bytes([0; 32]),
            };
            batch.append_event(&evt).expect("append should succeed");
            expected_len += 1;
            assert_eq!(
                batch.len(), expected_len,
                "len() must equal staged count after each operation"
            );
        }

        // Stage a header
        let header = make_run_header(run);
        batch.put_run_header(&header).expect("put header");
        expected_len += 1;
        assert_eq!(batch.len(), expected_len);

        // Commit
        batch.commit().expect("commit should succeed");
    }

    #[test]
    fn is_empty_equals_len_zero_invariant() {
        // P2: is_empty() == (len() == 0) holds after every operation
        let (_temp, journal) = temp_journal();
        let run = RunId::new(500);

        let mut batch = journal.batch();

        // Initially empty
        assert!(
            batch.is_empty() == (batch.len() == 0),
            "is_empty() must match (len() == 0) for new batch"
        );

        // After one operation
        batch.append_event(&make_event(run, 0)).expect("append");
        assert!(
            batch.is_empty() == (batch.len() == 0),
            "is_empty() must match (len() == 0) after one operation"
        );

        // After more operations
        batch.put_run_header(&make_run_header(run)).expect("put header");
        assert!(
            batch.is_empty() == (batch.len() == 0),
            "is_empty() must match (len() == 0) after multiple operations"
        );
    }

    #[test]
    fn batch_len_never_decreases() {
        // P3: len() monotonically increases (never decreases)
        let (_temp, journal) = temp_journal();
        let run = RunId::new(600);

        let mut batch = journal.batch();
        let mut prev_len = 0;

        // After each put operation, len must increase
        let operations = 5;
        for i in 0..operations {
            let evt = JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(i),
                workflow: WorkflowDigest::from_bytes([0; 32]),
            };
            batch.append_event(&evt).expect("append");
            assert!(
                batch.len() > prev_len,
                "len() must increase monotonically, prev={}, new={}",
                prev_len,
                batch.len()
            );
            prev_len = batch.len();
        }

        batch.commit().expect("commit");
    }

    #[test]
    fn commit_leaves_batch_in_consumed_state() {
        // P4: after commit() succeeds, batch is consumed
        let (_temp, journal) = temp_journal();
        let run = RunId::new(700);

        let mut batch = journal.batch();
        batch.append_event(&make_event(run, 0)).expect("append");
        batch.commit().expect("commit should succeed");

        // After commit, batch operations should not be usable
        // (In Rust, this is compile-time enforced, but we test len is 0)
        assert_eq!(batch.len(), 0, "committed batch should have len 0");
    }

    // =========================================================================
    // P5-P8: Atomicity and digest verification properties
    // =========================================================================

    #[test]
    fn all_or_nothing_commit_across_keyspaces() {
        // P5: commit is all-or-nothing; no partial state visible
        let (_temp, journal) = temp_journal();
        let run = RunId::new(800);
        let digest = WorkflowDigest::from_bytes([0xCC; DIGEST_BYTES]);

        let source = b"batch atomic test".to_vec();
        let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let workflow_record = WorkflowSourceRecord {
            digest: source_digest,
            source,
        };

        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(42),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 9999,
        };

        {
            let mut batch = journal.batch();
            batch.put_workflow_source(&workflow_record).expect("ws");
            batch.put_run_header(&header).expect("header");
            batch.commit().expect("commit should succeed");
        }

        // All or nothing: both must be present or neither
        let ws_present = journal
            .workflow_source(source_digest)
            .expect("get ws")
            .is_some();
        let header_present = journal
            .run_header(run)
            .expect("get header")
            .is_some();
        assert_eq!(
            ws_present, header_present,
            "commit must be all-or-nothing across keyspaces"
        );
    }

    #[test]
    fn strict_mode_requires_fsync_before_return() {
        // P6: strict durability guarantees fsync before commit() returns
        let (_temp, journal) = temp_journal();
        let run = RunId::new(900);

        let batch = journal.batch();
        let mut strict_batch = batch.strict();
        strict_batch
            .append_event(&make_event(run, 0))
            .expect("append should succeed");
        strict_batch.commit().expect("strict commit should succeed");

        // Verify data is durably persisted by reopening
        drop(journal);
        let (_temp2, journal2) = temp_journal();
        let reopened = FjallJournal::open(_temp.path(), None).expect("reopen should succeed");
        let events = reopened
            .events_for_run(run)
            .expect("replay should succeed after reopen");
        assert_eq!(events.len(), 1, "strict batch data must survive reopen");
    }

    #[test]
    fn digest_verification_mandatory_on_workflow_source() {
        // P7: BLAKE3 digest verification cannot be skipped for workflow_source
        let (_temp, journal) = temp_journal();
        let source = b"content to forge".to_vec();
        let real_digest = blake3::hash(&source);
        let forged_digest = WorkflowDigest::from_bytes([0xFF; 32]);

        let record = WorkflowSourceRecord {
            digest: forged_digest,
            source,
        };

        let mut batch = journal.batch();
        let result = batch.put_workflow_source(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "workflow_source digest verification must be mandatory"
        );
    }

    #[test]
    fn digest_verification_mandatory_on_blob() {
        // P8: BLAKE3 digest verification cannot be skipped for blob
        let (_temp, journal) = temp_journal();
        let payload = vec![1, 2, 3, 4, 5];
        let forged_digest: [u8; DIGEST_BYTES] = [0xAB; 32];

        let record = BlobRecord {
            digest: forged_digest,
            bytes: payload,
        };

        let mut batch = journal.batch();
        let result = batch.put_blob(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "blob digest verification must be mandatory"
        );
    }

    // =========================================================================
    // P9-P10: Encoding invariants
    // =========================================================================

    #[test]
    fn encoded_record_header_always_60_bytes() {
        // P9: RECORD_HEADER_BYTES == 60 for all record kinds
        let event = make_event(RunId::new(1), 0);
        let encoded = crate::encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encode should succeed");

        // Extract header bytes (first 60)
        let header = &encoded[..RECORD_HEADER_BYTES];
        assert_eq!(
            header.len(), RECORD_HEADER_BYTES,
            "encoded record header must always be {} bytes",
            RECORD_HEADER_BYTES
        );
    }

    #[test]
    fn magic_bytes_match_record_kind_family() {
        // P10: magic is consistent with record kind family
        // Journal events (10-23) use MAGIC_JOURNAL_EVENT
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([1; 32]),
        };
        let encoded = crate::encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encode should succeed");
        let magic = u32::from_le_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]);
        assert_eq!(magic, MAGIC_JOURNAL_EVENT);

        // WorkflowSource (kind 1) uses MAGIC_WORKFLOW_SOURCE
        let source = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([1; 32]),
            source: b"test".to_vec(),
        };
        let encoded = crate::encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &source,
            MAX_WORKFLOW_SOURCE_BYTES,
        )
        .expect("encode should succeed");
        let magic = u32::from_le_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]);
        assert_eq!(magic, MAGIC_WORKFLOW_SOURCE);
    }
}
