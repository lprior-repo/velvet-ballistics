#![forbid(unsafe_code)]
//! Artifact storage operations for compiled IR and workflow source.
//!
//! Provides storage, retrieval, listing, and removal of compiled artifacts.

use crate::{
    constants::{DIGEST_BYTES, PREFIX_COMPILED_IR},
    error::JournalError,
    keys::compiled_ir_key,
};

use crate::journal::FjallJournal;

impl FjallJournal {
    /// Returns all stored compiled IR artifact digests.
    pub fn list_artifacts(&self) -> Result<Vec<vb_core::WorkflowDigest>, JournalError> {
        let prefix = [PREFIX_COMPILED_IR];
        let mut digests = Vec::new();
        for item in self.compiled_ir.prefix(prefix) {
            let raw_key = item.key()?;
            let digest_bytes = raw_key.get(1..).ok_or(JournalError::UnexpectedEof)?;
            let digest_array = <[u8; DIGEST_BYTES]>::try_from(digest_bytes)
                .map_err(|_| JournalError::UnexpectedEof)?;
            digests.push(vb_core::WorkflowDigest::from_bytes(digest_array));
        }
        Ok(digests)
    }

    /// Removes a compiled IR artifact by digest.
    pub fn remove_artifact(&self, digest: vb_core::WorkflowDigest) -> Result<(), JournalError> {
        let key = compiled_ir_key(digest.as_bytes())?;
        let exists = self.compiled_ir.contains_key(key.as_slice())?;
        if !exists {
            return Err(JournalError::ArtifactNotFound { digest });
        }
        self.compiled_ir.remove(key.as_slice())?;
        Ok(())
    }

    /// Returns whether a compiled IR artifact is stored for the given digest.
    pub fn artifact_exists(&self, digest: vb_core::WorkflowDigest) -> Result<bool, JournalError> {
        let key = compiled_ir_key(digest.as_bytes())?;
        Ok(self.compiled_ir.contains_key(key.as_slice())?)
    }
}

#[cfg(test)]
mod tests {
    use crate::records::CompiledIrRecord;

    fn temp_journal() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir creation should succeed")
    }

    fn make_digest(byte: u8) -> vb_core::WorkflowDigest {
        vb_core::WorkflowDigest::from_bytes([byte; 32])
    }

    fn make_ir_record(digest: vb_core::WorkflowDigest, ir_bytes: &[u8]) -> CompiledIrRecord {
        CompiledIrRecord {
            digest,
            ir: ir_bytes.to_vec(),
        }
    }

    // =========================================================================
    // list_artifacts
    // =========================================================================

    #[test]
    fn list_artifacts_returns_empty_when_no_artifacts_stored() {
        let temp = temp_journal();
        let journal =
            crate::FjallJournal::open(temp.path(), None).expect("journal open should succeed");

        let digests = journal
            .list_artifacts()
            .expect("list_artifacts should succeed");
        assert!(
            digests.is_empty(),
            "list_artifacts should return empty vec when no artifacts stored"
        );
    }

    #[test]
    fn list_artifacts_returns_stored_digests() {
        let temp = temp_journal();
        let journal =
            crate::FjallJournal::open(temp.path(), None).expect("journal open should succeed");

        let digest1 = make_digest(0xAB);
        let digest2 = make_digest(0xCD);

        let record1 = make_ir_record(digest1, b"ir_v1");
        let record2 = make_ir_record(digest2, b"ir_v2");

        journal
            .put_compiled_ir(&record1)
            .expect("put_compiled_ir v1 should succeed");
        journal
            .put_compiled_ir(&record2)
            .expect("put_compiled_ir v2 should succeed");

        let digests = journal
            .list_artifacts()
            .expect("list_artifacts should succeed");
        assert_eq!(digests.len(), 2, "list_artifacts should return 2 digests");
        assert!(
            digests.contains(&digest1),
            "list_artifacts should contain digest1"
        );
        assert!(
            digests.contains(&digest2),
            "list_artifacts should contain digest2"
        );
    }

    #[test]
    fn list_artifacts_returns_single_digest() {
        let temp = temp_journal();
        let journal =
            crate::FjallJournal::open(temp.path(), None).expect("journal open should succeed");

        let digest = make_digest(0x42);
        let record = make_ir_record(digest, b"ir_single");

        journal
            .put_compiled_ir(&record)
            .expect("put_compiled_ir should succeed");

        let digests = journal
            .list_artifacts()
            .expect("list_artifacts should succeed");
        assert_eq!(digests.len(), 1, "list_artifacts should return 1 digest");
        assert!(
            digests.contains(&digest),
            "list_artifacts should contain the stored digest"
        );
    }

    // =========================================================================
    // remove_artifact
    // =========================================================================

    #[test]
    fn remove_artifact_succeeds_when_artifact_exists() {
        let temp = temp_journal();
        let journal =
            crate::FjallJournal::open(temp.path(), None).expect("journal open should succeed");

        let digest = make_digest(0xEF);
        let record = make_ir_record(digest, b"ir_to_remove");

        journal
            .put_compiled_ir(&record)
            .expect("put_compiled_ir should succeed");

        let result = journal.remove_artifact(digest);
        assert!(
            result.is_ok(),
            "remove_artifact should succeed for existing artifact"
        );

        // Verify artifact is gone
        let exists = journal
            .artifact_exists(digest)
            .expect("artifact_exists should succeed");
        assert!(!exists, "artifact should no longer exist after removal");
    }

    #[test]
    fn remove_artifact_fails_with_artifact_not_found_when_missing() {
        let temp = temp_journal();
        let journal =
            crate::FjallJournal::open(temp.path(), None).expect("journal open should succeed");

        let digest = make_digest(0x99);

        let result = journal.remove_artifact(digest);
        assert!(
            result.is_err(),
            "remove_artifact should fail for non-existent artifact"
        );
        match result {
            Err(crate::JournalError::ArtifactNotFound {
                digest: returned_digest,
            }) => {
                assert_eq!(
                    returned_digest, digest,
                    "error digest should match requested digest"
                );
            }
            _ => panic!("Expected ArtifactNotFound error, got {:?}", result),
        }
    }

    #[test]
    fn remove_artifact_one_of_two_artifacts() {
        let temp = temp_journal();
        let journal =
            crate::FjallJournal::open(temp.path(), None).expect("journal open should succeed");

        let digest1 = make_digest(0x11);
        let digest2 = make_digest(0x22);

        let record1 = make_ir_record(digest1, b"ir_1");
        let record2 = make_ir_record(digest2, b"ir_2");

        journal
            .put_compiled_ir(&record1)
            .expect("put_compiled_ir v1 should succeed");
        journal
            .put_compiled_ir(&record2)
            .expect("put_compiled_ir v2 should succeed");

        journal
            .remove_artifact(digest1)
            .expect("remove_artifact should succeed");

        // digest1 should be gone, digest2 should remain
        let exists1 = journal
            .artifact_exists(digest1)
            .expect("artifact_exists should succeed");
        let exists2 = journal
            .artifact_exists(digest2)
            .expect("artifact_exists should succeed");

        assert!(!exists1, "digest1 should not exist after removal");
        assert!(exists2, "digest2 should still exist");

        let digests = journal
            .list_artifacts()
            .expect("list_artifacts should succeed");
        assert_eq!(
            digests.len(),
            1,
            "list_artifacts should return 1 digest after removal"
        );
        assert!(
            digests.contains(&digest2),
            "list_artifacts should contain remaining digest2"
        );
    }

    // =========================================================================
    // artifact_exists
    // =========================================================================

    #[test]
    fn artifact_exists_returns_true_when_artifact_is_stored() {
        let temp = temp_journal();
        let journal =
            crate::FjallJournal::open(temp.path(), None).expect("journal open should succeed");

        let digest = make_digest(0x77);
        let record = make_ir_record(digest, b"ir_exists");

        journal
            .put_compiled_ir(&record)
            .expect("put_compiled_ir should succeed");

        let exists = journal
            .artifact_exists(digest)
            .expect("artifact_exists should succeed");
        assert!(
            exists,
            "artifact_exists should return true for stored artifact"
        );
    }

    #[test]
    fn artifact_exists_returns_false_when_artifact_is_not_stored() {
        let temp = temp_journal();
        let journal =
            crate::FjallJournal::open(temp.path(), None).expect("journal open should succeed");

        let digest = make_digest(0x88);

        let exists = journal
            .artifact_exists(digest)
            .expect("artifact_exists should succeed");
        assert!(
            !exists,
            "artifact_exists should return false for non-existent artifact"
        );
    }

    #[test]
    fn artifact_exists_returns_false_after_remove() {
        let temp = temp_journal();
        let journal =
            crate::FjallJournal::open(temp.path(), None).expect("journal open should succeed");

        let digest = make_digest(0x55);
        let record = make_ir_record(digest, b"ir_to_remove");

        journal
            .put_compiled_ir(&record)
            .expect("put_compiled_ir should succeed");

        journal
            .remove_artifact(digest)
            .expect("remove_artifact should succeed");

        let exists = journal
            .artifact_exists(digest)
            .expect("artifact_exists should succeed");
        assert!(!exists, "artifact_exists should return false after removal");
    }
}
