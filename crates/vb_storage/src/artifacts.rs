#![forbid(unsafe_code)]
//! Artifact storage operations for compiled IR and workflow source.
//!
//! Provides storage, retrieval, listing, and removal of compiled artifacts.

use crate::{
    constants::{DIGEST_BYTES, PREFIX_COMPILED_IR},
    error::JournalError,
    keys::compiled_ir_key,
    types::digests::CompiledIrDigest,
};

use crate::journal::FjallJournal;

impl FjallJournal {
    /// Returns all stored compiled IR artifact digests.
    pub fn list_artifacts(&self) -> Result<Vec<vb_core::WorkflowDigest>, JournalError> {
        let prefix = [PREFIX_COMPILED_IR];
        // CC-003 capacity hint: 4 covers the typical "few compiled
        // artifacts per workflow" case. Standard Vec doubling covers
        // larger deployments without over-allocating on small ones.
        let mut digests = Vec::with_capacity(4);
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
    pub fn remove_artifact(
        &self,
        digest: vb_core::WorkflowDigest,
    ) -> Result<(), JournalError> {
        let key = compiled_ir_key(CompiledIrDigest::from_bytes(digest.as_bytes()))?;
        let exists = self.compiled_ir.contains_key(key.as_slice())?;
        if !exists {
            return Err(JournalError::ArtifactNotFound { digest });
        }
        self.compiled_ir.remove(key.as_slice())?;
        Ok(())
    }

    /// Returns whether a compiled IR artifact is stored for the given digest.
    pub fn artifact_exists(
        &self,
        digest: vb_core::WorkflowDigest,
    ) -> Result<bool, JournalError> {
        let key = compiled_ir_key(CompiledIrDigest::from_bytes(digest.as_bytes()))?;
        Ok(self.compiled_ir.contains_key(key.as_slice())?)
    }
}
