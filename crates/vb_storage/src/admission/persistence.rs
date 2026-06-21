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
///
/// The underlying postcard encode error is preserved via the `Encode`
/// variant of `JournalError` so callers can distinguish serialization
/// failures from unrelated artifact-corruption cases.
pub(crate) fn serialize_accepted_artifact(
    artifact: &AcceptedArtifact,
) -> Result<Vec<u8>, JournalError> {
    postcard::to_allocvec(artifact).map_err(JournalError::from)
}

/// Verifies that a persisted artifact is present in the journal.
///
/// The underlying journal error is propagated unchanged; only the
/// absent-key case is translated into the typed `ArtifactNotFound`
/// variant so callers can distinguish "row missing" from "row corrupt
/// or unreadable".
pub(crate) fn verify_persisted_artifact_present(
    journal: &FjallJournal,
    digest: vb_core::WorkflowDigest,
) -> Result<(), JournalError> {
    match journal.compiled_ir(digest)? {
        Some(_) => Ok(()),
        None => Err(JournalError::ArtifactNotFound { digest }),
    }
}
