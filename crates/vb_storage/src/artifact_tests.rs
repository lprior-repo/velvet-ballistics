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
mod artifact_tests {
    use crate::{DIGEST_BYTES, FjallJournal, JournalError};
    use vb_core::WorkflowDigest;

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    fn put_test_artifact(journal: &FjallJournal, seed: &[u8]) -> WorkflowDigest {
        let record = crate::accepted_compiled_ir_record_for_test(seed.to_vec());
        let digest = record.digest;
        journal
            .put_compiled_ir(&record)
            .expect("put should succeed");
        digest
    }

    #[test]
    fn list_artifacts_returns_empty_for_empty_journal() {
        let (_temp, journal) = temp_journal();
        let artifacts = journal.list_artifacts().expect("list should succeed");
        assert!(
            artifacts.is_empty(),
            "should return empty list for empty journal"
        );
    }

    #[test]
    fn list_artifacts_returns_all_stored_digests() {
        let (_temp, journal) = temp_journal();
        let d1 = put_test_artifact(&journal, &[0x11]);
        let d2 = put_test_artifact(&journal, &[0x22]);
        let d3 = put_test_artifact(&journal, &[0x33]);

        let artifacts = journal.list_artifacts().expect("list should succeed");
        assert_eq!(artifacts.len(), 3, "should list 3 artifacts");
        assert!(artifacts.contains(&d1));
        assert!(artifacts.contains(&d2));
        assert!(artifacts.contains(&d3));
    }

    #[test]
    fn artifact_exists_returns_true_for_stored_digest() {
        let (_temp, journal) = temp_journal();
        let digest = put_test_artifact(&journal, &[0x44]);

        let exists = journal
            .artifact_exists(digest)
            .expect("check should succeed");
        assert!(exists, "artifact should exist after put");
    }

    #[test]
    fn artifact_exists_returns_false_for_missing_digest() {
        let (_temp, journal) = temp_journal();
        let missing = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
        let exists = journal
            .artifact_exists(missing)
            .expect("check should succeed");
        assert!(!exists, "artifact should not exist for unknown digest");
    }

    #[test]
    fn remove_artifact_deletes_existing_artifact() {
        let (_temp, journal) = temp_journal();
        let digest = put_test_artifact(&journal, &[0x55]);

        assert!(
            journal
                .artifact_exists(digest)
                .expect("check before remove")
        );
        journal
            .remove_artifact(digest)
            .expect("remove should succeed");
        assert!(
            !journal.artifact_exists(digest).expect("check after remove"),
            "artifact should not exist after removal"
        );
    }

    #[test]
    fn remove_artifact_returns_error_for_missing_digest() {
        let (_temp, journal) = temp_journal();
        let missing = WorkflowDigest::from_bytes([0xEE; DIGEST_BYTES]);
        let result = journal.remove_artifact(missing);
        assert!(
            matches!(result, Err(JournalError::ArtifactNotFound { digest }) if digest == missing),
            "must return ArtifactNotFound for missing digest, got {:?}",
            result
        );
    }

    #[test]
    fn list_artifacts_reflects_removal() {
        let (_temp, journal) = temp_journal();
        let d1 = put_test_artifact(&journal, &[0x66]);
        let d2 = put_test_artifact(&journal, &[0x77]);

        journal
            .remove_artifact(d1)
            .expect("remove d1 should succeed");

        let artifacts = journal.list_artifacts().expect("list should succeed");
        assert_eq!(artifacts.len(), 1);
        assert!(artifacts.contains(&d2));
        assert!(!artifacts.contains(&d1));
    }

    #[test]
    fn artifact_exists_idempotent_after_removal() {
        let (_temp, journal) = temp_journal();
        let digest = put_test_artifact(&journal, &[0x88]);
        journal
            .remove_artifact(digest)
            .expect("first remove should succeed");

        let result = journal.remove_artifact(digest);
        assert!(
            matches!(result, Err(JournalError::ArtifactNotFound { .. })),
            "second remove should return ArtifactNotFound, got {:?}",
            result
        );
    }
}
