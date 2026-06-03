#![forbid(unsafe_code)]
//! Record entities for storage.

use vb_core::{RunId, WorkflowDigest, WorkflowId};

use super::status::RunHeaderStatus;

/// Immutable workflow source bytes bound to their digest.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkflowSourceRecord {
    /// Source digest key.
    pub digest: WorkflowDigest,
    /// Original strict YAML authoring bytes.
    pub source: Vec<u8>,
}

/// Compiled IR artifact bytes bound to their digest.
///
/// Includes a metadata hash to prevent same-digest metadata mutation attacks.
/// The metadata hash is computed from the `AcceptedArtifact` fields that should
/// not change after admission: `source_digest`, `policy_digest`, `ir`,
/// `verification` (excluding its own `digest` field), `accepted_at_seq`,
/// and `required_capabilities`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CompiledIrRecord {
    /// Compiled IR digest key.
    pub digest: WorkflowDigest,
    /// Postcard-compatible compiled artifact bytes.
    pub ir: Vec<u8>,
    /// BLAKE3 hash of the artifact metadata fields that must remain immutable
    /// after admission. `None` indicates a pre-mutation-protection record
    /// (backward compatibility); such records are accepted on read but any
    /// subsequent write to the same digest must provide a matching hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_hash: Option<[u8; 32]>,
}

#[cfg(test)]
impl CompiledIrRecord {
    /// Test-only constructor that creates a record with no metadata hash.
    /// The metadata hash will be computed and set by `journal.put_compiled_ir`.
    pub(crate) fn test_new(digest: WorkflowDigest, ir: Vec<u8>) -> Self {
        Self {
            digest,
            ir,
            metadata_hash: None,
        }
    }
}

impl Default for CompiledIrRecord {
    fn default() -> Self {
        Self {
            digest: WorkflowDigest::from_bytes([0; 32]),
            ir: Vec::new(),
            metadata_hash: None,
        }
    }
}

/// Minimal run metadata record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunHeaderRecord {
    /// Run identifier.
    pub run: RunId,
    /// Workflow identifier.
    pub workflow_id: WorkflowId,
    /// Compiled workflow digest bound at run acceptance.
    pub compiled_digest: WorkflowDigest,
    /// Persisted status byte owned by the runtime status model.
    ///
    /// Use [`RunHeaderRecord::run_header_status`] and
    /// [`RunHeaderRecord::set_run_header_status`] at typed boundaries. This
    /// field remains a `u8` to preserve the existing storage wire format.
    pub status: u8,
    /// Admission timestamp in milliseconds supplied by the caller.
    pub accepted_at_ms: u64,
}

impl RunHeaderRecord {
    /// Returns the status as a typed, lossless value.
    #[must_use]
    pub const fn run_header_status(&self) -> RunHeaderStatus {
        RunHeaderStatus::from_byte(self.status)
    }

    /// Replaces the persisted status byte from a typed status value.
    pub fn set_run_header_status(&mut self, status: RunHeaderStatus) {
        self.status = status.as_byte();
    }
}

/// Large payload blob record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BlobRecord {
    /// Blob digest key.
    pub digest: [u8; crate::constants::DIGEST_BYTES],
    /// Bounded blob payload.
    pub bytes: Vec<u8>,
}
