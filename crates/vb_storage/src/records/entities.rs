#![forbid(unsafe_code)]
//! Record entities for storage.

use vb_core::{RunId, WorkflowDigest, WorkflowId};

use super::status::RunHeaderStatus;
use crate::types::EventSeq;

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

/// Recovery progress stamp (wire ID 7, magic `MAGIC_RECOVERY_STAMP`).
///
/// A small fixed-shape record written by the recovery path to mark how far
/// replay has progressed for a given run. The fields are intentionally
/// compact and bounded so a recovery stamp fits comfortably in
/// `MAX_RECOVERY_STAMP_BYTES` and decodes without allocation beyond the
/// postcard payload buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecoveryStampRecord {
    /// Run identifier the stamp applies to.
    pub run: RunId,
    /// Highest journal sequence number observed at stamp time.
    pub last_seq: EventSeq,
    /// Wall-clock millisecond timestamp when the stamp was written.
    pub written_at_ms: u64,
}
