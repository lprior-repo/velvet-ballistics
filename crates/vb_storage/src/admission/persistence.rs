#![forbid(unsafe_code)]
//! Artifact persistence: serialize, store, and verify accepted artifacts.

use crate::error::JournalError;
use crate::journal::FjallJournal;
use crate::records::CompiledIrRecord;

use super::metadata::compute_artifact_metadata_hash;
use super::types::AcceptedArtifact;

/// Persists an accepted artifact to the journal's compiled-IR keyspace.
pub(crate) fn persist_accepted_artifact_ir(
    journal: &FjallJournal,
    artifact: &AcceptedArtifact,
) -> Result<(), JournalError> {
    let envelope = serialize_accepted_artifact(artifact)?;
    let metadata_hash = compute_artifact_metadata_hash(artifact);
    let record = CompiledIrRecord {
        digest: artifact.digest,
        ir: envelope,
        metadata_hash: Some(metadata_hash),
    };
    journal.put_compiled_ir(&record)
}

/// Serializes an accepted artifact to postcard bytes.
pub(crate) fn serialize_accepted_artifact(
    artifact: &AcceptedArtifact,
) -> Result<Vec<u8>, JournalError> {
    postcard::to_allocvec(artifact).map_err(|_| JournalError::ArtifactMalformed)
}

/// Verifies that a persisted artifact is present in the journal.
pub(crate) fn verify_persisted_artifact_present(
    journal: &FjallJournal,
    digest: vb_core::WorkflowDigest,
) -> Result<(), JournalError> {
    let stored = journal
        .compiled_ir(digest)
        .map_err(|_| JournalError::ArtifactMalformed)?;
    if stored.is_some() {
        Ok(())
    } else {
        Err(JournalError::ArtifactMalformed)
    }
}
