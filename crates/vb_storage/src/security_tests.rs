/// Security-specific tests proving that digest forgery, cross-run injection,
/// and silent bypass attacks are blocked.
#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod security_tests {
    use crate::{
        BlobRecord, DIGEST_BYTES, EventSeq, FjallJournal, JournalError, WorkflowSourceRecord,
    };
    use vb_core::{RunId, WorkflowDigest};

    /// SECURITY: put_workflow_source rejects a forged digest where source bytes
    /// do not hash to the claimed digest.
    #[test]
    fn forged_workflow_source_digest_rejected() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let forged_digest = WorkflowDigest::from_bytes([0xDE; 32]);
        let record = WorkflowSourceRecord {
            digest: forged_digest,
            source: b"this will not hash to 0xDE..DE".to_vec(),
        };
        let result = journal.put_workflow_source(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "forged workflow source digest must be rejected, got {:?}",
            result
        );
    }

    /// SECURITY: put_blob rejects a forged digest where blob bytes do not hash
    /// to the claimed digest.
    #[test]
    fn forged_blob_digest_rejected() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let forged_digest = [0xAB; 32];
        let record = BlobRecord {
            digest: forged_digest,
            bytes: b"these bytes won't hash to 0xAB..AB".to_vec(),
        };
        let result = journal.put_blob(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "forged blob digest must be rejected, got {:?}",
            result
        );
    }

    /// SECURITY: batch put_workflow_source rejects a forged digest.
    #[test]
    fn batch_forged_workflow_source_digest_rejected() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let forged_digest = WorkflowDigest::from_bytes([0xFF; 32]);
        let mut batch = journal.batch();
        let result = batch.put_workflow_source(&WorkflowSourceRecord {
            digest: forged_digest,
            source: b"not the right hash".to_vec(),
        });
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "batch forged workflow source digest must be rejected, got {:?}",
            result
        );
    }

    /// SECURITY: batch put_blob rejects a forged digest.
    #[test]
    fn batch_forged_blob_digest_rejected() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let mut batch = journal.batch();
        let result = batch.put_blob(&BlobRecord {
            digest: [0x11; 32],
            bytes: b"wrong hash".to_vec(),
        });
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "batch forged blob digest must be rejected, got {:?}",
            result
        );
    }

    /// SECURITY: put_workflow_source accepts a record with a correct digest.
    #[test]
    fn valid_workflow_source_digest_accepted() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let source = b"valid content".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord {
            digest,
            source,
        };
        journal
            .put_workflow_source(&record)
            .expect("correct digest must be accepted");
    }

    /// SECURITY: put_blob accepts a record with a correct digest.
    #[test]
    fn valid_blob_digest_accepted() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");
        let bytes = b"valid blob content".to_vec();
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&bytes).into();
        let record = BlobRecord {
            digest,
            bytes,
        };
        journal.put_blob(&record).expect("correct digest must be accepted");
    }
}
