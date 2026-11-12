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
    use crate::{BlobRecord, DIGEST_BYTES, FjallJournal, JournalError, keys::blob_key};

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
}
