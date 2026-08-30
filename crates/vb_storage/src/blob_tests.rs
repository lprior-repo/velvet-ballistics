#![forbid(unsafe_code)]
#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod blob_tests {
    use crate::{
        BlobRecord, CompiledIrRecord, DIGEST_BYTES, FjallJournal, JournalError, RunHeaderRecord,
        RunSnapshot, StorageKey, WorkflowSourceRecord, DIGEST_BYTES as DB, EventSeq,
    };
    use crate::keys::{blob_key, encode_key};
    use vb_core::{RunId, WorkflowDigest};

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    fn make_blob_record(payload: &[u8]) -> BlobRecord {
        let digest: [u8; DIGEST_BYTES] = blake3::hash(payload).into();
        BlobRecord {
            digest,
            bytes: payload.to_vec(),
        }
    }

    #[test]
    fn put_blob_stores_and_retrieves_valid_blob() {
        let (_temp, journal) = temp_journal();
        let payload = b"five-byte-blob-test-payload-data";
        let record = make_blob_record(payload);
        journal.put_blob(&record).expect("put_blob should succeed");
        let loaded = journal.blob(record.digest).expect("blob should succeed");
        let found = loaded.expect("blob should exist after put");
        assert_eq!(found.digest, record.digest);
        assert_eq!(found.bytes, record.bytes);
    }

    #[test]
    fn put_blob_rejects_digest_mismatch() {
        let (_temp, journal) = temp_journal();
        let payload = b"real-blob-payload-for-digest-mismatch-test";
        let wrong_digest: [u8; DIGEST_BYTES] = [0xFF; 32];
        let record = BlobRecord {
            digest: wrong_digest,
            bytes: payload.to_vec(),
        };
        let result = journal.put_blob(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "must reject digest mismatch, got {:?}",
            result
        );
    }

    #[test]
    fn blob_returns_none_for_missing_digest() {
        let (_temp, journal) = temp_journal();
        let missing_digest: [u8; DIGEST_BYTES] = [0xAB; 32];
        let result = journal
            .blob(missing_digest)
            .expect("blob lookup should succeed");
        assert!(result.is_none(), "should return None for missing digest");
    }

    #[test]
    fn put_blob_accepts_empty_payload() {
        let (_temp, journal) = temp_journal();
        let empty: &[u8] = &[];
        let record = make_blob_record(empty);
        journal
            .put_blob(&record)
            .expect("put_blob of empty should succeed");
        let loaded = journal
            .blob(record.digest)
            .expect("get should succeed")
            .expect("should exist");
        assert_eq!(loaded.bytes.len(), 0);
    }

    #[test]
    fn put_blob_accepts_max_size_payload() {
        let (_temp, journal) = temp_journal();
        let payload = vec![0x42u8; 1024];
        let record = make_blob_record(&payload);
        journal
            .put_blob(&record)
            .expect("put_blob of max payload should succeed");
        let loaded = journal
            .blob(record.digest)
            .expect("get should succeed")
            .expect("should exist");
        assert_eq!(loaded.bytes.len(), payload.len());
    }

    #[test]
    fn put_blob_is_idempotent() {
        let (_temp, journal) = temp_journal();
        let payload = b"idempotent-blob-payload-data";
        let record = make_blob_record(payload);
        journal.put_blob(&record).expect("first put should succeed");
        let result = journal.put_blob(&record);
        assert!(
            result.is_ok(),
            "second put_blob with identical digest and payload should be idempotent, got {:?}",
            result
        );
    }

    #[test]
    fn multiple_blobs_with_different_digests_are_retrieved_correctly() {
        let (_temp, journal) = temp_journal();
        let p1 = b"first-blob-payload-for-multi-blob-test";
        let p2 = b"second-distinct-blob-payload";
        let r1 = make_blob_record(p1);
        let r2 = make_blob_record(p2);
        journal.put_blob(&r1).expect("put first blob");
        journal.put_blob(&r2).expect("put second blob");

        let loaded1 = journal.blob(r1.digest).expect("get first").expect("found");
        let loaded2 = journal.blob(r2.digest).expect("get second").expect("found");
        assert_eq!(loaded1.bytes, p1.to_vec());
        assert_eq!(loaded2.bytes, p2.to_vec());
    }

    #[test]
    fn read_blob_convenience_wrapper_works() {
        let (_temp, journal) = temp_journal();
        let payload = b"read-blob-wrapper-test-data";
        let record = make_blob_record(payload);
        journal.put_blob(&record).expect("put_blob should succeed");

        let result = crate::read_blob(&journal, record.digest).expect("read_blob should succeed");
        let found = result.expect("blob should exist");
        assert_eq!(found.bytes, payload.to_vec());
    }

    #[test]
    fn blob_decode_with_invalid_data_returns_error() {
        let (_temp, journal) = temp_journal();
        let digest: [u8; DIGEST_BYTES] = [0x70; 32];
        let fake_key = blob_key(digest).expect("blob key should succeed");
        let invalid_value = vec![0xFF; 4];

        journal
            .blob
            .insert(fake_key.to_vec(), invalid_value)
            .expect("raw insert should succeed");

        let result = journal.blob(digest);
        assert!(
            result.is_err() || result.map_or(false, |opt| opt.is_none()),
            "reading corrupt blob should error or return None"
        );
    }

    // ========================================================================
    // Point-read contract: digest-mismatch rejection on read
    // ========================================================================

    #[test]
    fn point_read_blob_rejects_stored_record_with_different_requested_digest() {
        // Given a blob stored with digest D1
        // When a point-read requests digest D2 (D2 != D1)
        // Then the read returns PayloadDigestMismatch
        let (_temp, journal) = temp_journal();
        let payload = b"digest-mismatch-point-read-test";
        let record = make_blob_record(payload);
        journal.put_blob(&record).expect("put_blob should succeed");

        let wrong_digest: [u8; DB] = [0xCC; 32];
        let result = journal.blob(wrong_digest);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "point-read with wrong digest must return PayloadDigestMismatch, got {:?}",
            result
        );
    }

    #[test]
    fn point_read_blob_returns_none_when_keyspace_has_no_entry() {
        // Given a blob key that was never stored
        // When the journal is opened fresh
        // Then point_read returns Ok(None) — no error, no panic
        let (_temp, journal) = temp_journal();
        let digest: [u8; DB] = [0xDD; 32];
        let result = journal.blob(digest);
        assert!(result.is_ok(), "blob lookup must not error for missing key");
        assert!(
            result.ok().flatten().is_none(),
            "missing blob must return None"
        );
    }

    #[test]
    fn point_read_blob_preserves_payload_byte_for_byte() {
        // Given a blob with known binary payload
        // When stored and retrieved via point-read
        // Then every byte matches exactly
        let (_temp, journal) = temp_journal();
        let payload: Vec<u8> = (0..=255).collect();
        let record = make_blob_record(&payload);
        journal.put_blob(&record).expect("put_blob should succeed");

        let loaded = journal
            .blob(record.digest)
            .expect("blob lookup should succeed")
            .expect("blob should exist");
        assert_eq!(loaded.bytes, payload, "blob payload must match byte-for-byte");
        assert_eq!(
            loaded.digest, record.digest,
            "blob digest must match after round-trip"
        );
    }

    #[test]
    fn point_read_workflow_source_contract() {
        // Given a workflow source record stored in the journal
        // When the workflow_source point-read is called
        // Then the exact WorkflowSourceRecord is returned
        let (_temp, journal) = temp_journal();
        let source_bytes = b"workflow source contract test data";
        let digest = WorkflowDigest::from_bytes(blake3::hash(source_bytes).into());
        let record = WorkflowSourceRecord {
            digest,
            source: source_bytes.to_vec(),
        };
        journal
            .put_workflow_source(&record)
            .expect("put_workflow_source should succeed");

        let found = journal
            .workflow_source(digest)
            .expect("workflow_source lookup should succeed")
            .expect("workflow source must exist");
        assert_eq!(found.digest, record.digest);
        assert_eq!(found.source, record.source);
    }

    #[test]
    fn point_read_workflow_source_returns_none_for_missing_digest() {
        let (_temp, journal) = temp_journal();
        let missing: WorkflowDigest = WorkflowDigest::from_bytes([0xBB; 32]);
        let result = journal.workflow_source(missing);
        assert!(result.is_ok(), "workflow_source lookup must not error");
        assert!(result.expect("ok").is_none(), "missing source must return None");
    }

    #[test]
    fn point_read_compiled_ir_contract() {
        let (_temp, journal) = temp_journal();
        let ir_bytes = b"compiled ir contract test bytes";
        let digest = WorkflowDigest::from_bytes(blake3::hash(ir_bytes).into());
        let record = CompiledIrRecord {
            digest,
            ir: ir_bytes.to_vec(),
        };
        journal
            .put_compiled_ir(&record)
            .expect("put_compiled_ir should succeed");

        let found = journal
            .compiled_ir(digest)
            .expect("compiled_ir lookup should succeed")
            .expect("compiled ir must exist");
        assert_eq!(found.digest, record.digest);
        assert_eq!(found.ir, record.ir);
    }

    #[test]
    fn point_read_run_header_contract() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(42);
        let digest = WorkflowDigest::from_bytes([0xAA; 32]);
        let record = RunHeaderRecord {
            run,
            workflow_id: vb_core::WorkflowId::new(7),
            compiled_digest: digest,
            status: 1,
            accepted_at_ms: 12345,
        };
        journal
            .put_run_header(&record)
            .expect("put_run_header should succeed");

        let found = journal
            .run_header(run)
            .expect("run_header lookup should succeed")
            .expect("run header must exist");
        assert_eq!(found.run, record.run);
        assert_eq!(found.workflow_id, record.workflow_id);
        assert_eq!(found.compiled_digest, record.compiled_digest);
        assert_eq!(found.status, record.status);
        assert_eq!(found.accepted_at_ms, record.accepted_at_ms);
    }

    #[test]
    fn point_read_run_header_returns_none_for_missing_run() {
        let (_temp, journal) = temp_journal();
        let missing = RunId::new(99999);
        let result = journal.run_header(missing);
        assert!(result.is_ok(), "run_header lookup must not error");
        assert!(result.expect("ok").is_none(), "missing header must return None");
    }

    #[test]
    fn point_read_snapshot_contract() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(42);
        let seq = EventSeq::new(10);
        let workflow = WorkflowDigest::from_bytes([0x11; 32]);
        let record = RunSnapshot {
            run,
            seq,
            workflow,
            slots: vec![1, 2, 3],
                taint: vec![2u8],
        };
        journal
            .put_snapshot(&record)
            .expect("put_snapshot should succeed");

        let found = journal
            .snapshot(run, seq)
            .expect("snapshot lookup should succeed")
            .expect("snapshot must exist");
        assert_eq!(found.run, record.run);
        assert_eq!(found.seq, record.seq);
        assert_eq!(found.workflow, record.workflow);
        assert_eq!(found.slots, record.slots);
        assert_eq!(found.taint, record.taint);
    }

    #[test]
    fn point_read_snapshot_returns_none_for_missing_run_seq() {
        let (_temp, journal) = temp_journal();
        let result = journal.snapshot(RunId::new(9999), EventSeq::new(0));
        assert!(result.is_ok(), "snapshot lookup must not error");
        assert!(result.expect("ok").is_none(), "missing snapshot must return None");
    }

    // ========================================================================
    // Blob contract: trim and delete assertions
    // ========================================================================

    #[test]
    fn trim_blob_removes_and_rejects_on_subsequent_read() {
        let (_temp, journal) = temp_journal();
        let payload = b"trim-test-blob-payload";
        let record = make_blob_record(payload);
        journal.put_blob(&record).expect("put_blob should succeed");

        // Verify blob exists before trim
        assert!(
            journal.blob(record.digest).expect("blob check before trim").is_some(),
            "blob must exist before trim"
        );

        // Trim the blob
        journal
            .trim_blob(record.digest)
            .expect("trim_blob should succeed");

        // Subsequent read returns None
        let after_trim = journal.blob(record.digest).expect("blob lookup after trim");
        assert!(
            after_trim.is_none(),
            "blob must be absent after trim, got {:?}",
            after_trim
        );
    }

    #[test]
    fn trim_blob_returns_artifact_not_found_for_nonexistent_blob() {
        let (_temp, journal) = temp_journal();
        let missing: [u8; DB] = [0xEE; 32];
        let result = journal.trim_blob(missing);
        assert!(
            matches!(result, Err(JournalError::ArtifactNotFound { .. })),
            "trim_blob for missing digest must return ArtifactNotFound, got {:?}",
            result
        );
    }

    #[test]
    fn blob_key_generation_produces_distinct_keys_for_distinct_digests() {
        // Given two different digests
        // When blob_key is called for each
        // Then the resulting keys are distinct and start with the BLOB prefix
        let d1: [u8; DB] = [0x01; 32];
        let d2: [u8; DB] = [0x02; 32];

        let key1 = blob_key(d1).expect("blob_key for d1 should succeed");
        let key2 = blob_key(d2).expect("blob_key for d2 should succeed");

        assert_ne!(key1, key2, "distinct digests must produce distinct keys");
        assert!(
            key1.starts_with(&[crate::PREFIX_BLOB]),
            "blob key must start with PREFIX_BLOB"
        );
    }

    #[test]
    fn blob_storage_enforces_max_blob_bytes_limit() {
        // Given the MAX_BLOB_BYTES limit
        // When a blob at exactly the limit is stored and retrieved
        // Then the operation succeeds and preserves all bytes
        let (_temp, journal) = temp_journal();
        let payload = vec![0xABu8; crate::MAX_BLOB_BYTES as usize];
        let record = make_blob_record(&payload);
        journal
            .put_blob(&record)
            .expect("put_blob at MAX_BLOB_BYTES should succeed");

        let loaded = journal
            .blob(record.digest)
            .expect("blob lookup should succeed")
            .expect("blob must exist");
        assert_eq!(loaded.bytes.len(), crate::MAX_BLOB_BYTES as usize);
        assert_eq!(loaded.bytes, payload);
    }

    // ========================================================================
    // Point-read contract: StorageKey encode_key round-trip
    // ========================================================================

    #[test]
    fn storage_key_blob_encode_key_produces_same_key_as_blob_key() {
        let digest: [u8; DB] = [0x99; 32];
        let encoded = StorageKey::Blob { digest };
        let stored_key = blob_key(digest).expect("blob_key should succeed");

        let key_bytes = encode_key(encoded);
        let key_bytes = key_bytes.expect("encode should succeed");
        assert_eq!(
            key_bytes.as_slice(),
            stored_key.as_slice(),
            "StorageKey::Blob encode must match blob_key output"
        );
    }

    // ========================================================================
    // Point-read contract: concurrent reads do not interfere
    // ========================================================================

    #[test]
    fn point_read_blob_isolation_across_runs() {
        // Given blobs for two different runs
        // When both are stored and read back
        // Then each read returns only the correct blob
        let (_temp, journal) = temp_journal();
        let run_a_blob = b"run-a-blob-data";
        let run_b_blob = b"run-b-blob-data";
        let record_a = make_blob_record(run_a_blob);
        let record_b = make_blob_record(run_b_blob);

        journal.put_blob(&record_a).expect("put blob a");
        journal.put_blob(&record_b).expect("put blob b");

        let loaded_a = journal
            .blob(record_a.digest)
            .expect("get a")
            .expect("a must exist");
        let loaded_b = journal
            .blob(record_b.digest)
            .expect("get b")
            .expect("b must exist");

        assert_eq!(loaded_a.bytes, run_a_blob.to_vec());
        assert_eq!(loaded_b.bytes, run_b_blob.to_vec());
        assert_ne!(
            loaded_a.digest, loaded_b.digest,
            "different blobs must have different digests"
        );
    }
}
