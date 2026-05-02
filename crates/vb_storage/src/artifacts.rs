//! Artifact storage operations for compiled IR and workflow source.
//!
//! Provides storage, retrieval, listing, and removal of compiled artifacts.

use crate::{
    codec::decode_record,
    constants::{
        MAGIC_COMPILED_ARTIFACT, MAGIC_WORKFLOW_SOURCE, MAX_COMPILED_IR_BYTES,
        MAX_WORKFLOW_SOURCE_BYTES, PREFIX_COMPILED_IR,
    },
    error::JournalError,
    keys::{compiled_ir_key, workflow_source_key},
    records::{CompiledIrRecord, WorkflowSourceRecord},
};

use crate::journal::FjallJournal;

impl FjallJournal {
    /// Returns all stored compiled IR artifact digests.
    pub fn list_artifacts(&self) -> Result<Vec<vb_core::WorkflowDigest>, JournalError> {
        let prefix = [crate::constants::PREFIX_COMPILED_IR];
        let mut digests = Vec::new();
        for item in self.compiled_ir.prefix(prefix) {
            let raw_key = item.key()?;
            let digest_bytes =
                raw_key.get(1..).ok_or(JournalError::UnexpectedEof)?;
            let digest_array =
                <[u8; crate::constants::DIGEST_BYTES]>::try_from(digest_bytes)
                    .map_err(|_| JournalError::UnexpectedEof)?;
            digests.push(vb_core::WorkflowDigest::from_bytes(digest_array));
        }
        Ok(digests)
    }

    /// Removes a compiled IR artifact by digest.
    pub fn remove_artifact(
        &self,
        digest: vb_core::WorkflowDigest,
    ) -> Result<(), JournalError> {
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
