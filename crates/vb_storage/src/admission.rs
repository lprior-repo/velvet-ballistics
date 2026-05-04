//! Artifact admission and verification functions.
//!
//! Provides artifact submission and admission flows with policy-controlled durability.

use std::fmt;

use crate::{error::JournalError, records::CompiledIrRecord, types::EventSeq};

use crate::journal::FjallJournal;

/// A soft verification failure that does not block admission but should be reported.
///
/// Each warning is associated with a specific verification gate (1-13 range) and
/// carries a numeric code and human-readable message.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VerificationWarning {
    /// Numeric code identifying the specific warning condition.
    pub code: u32,
    /// Human-readable description of the warning.
    pub message: Box<str>,
    /// Which verification gate produced this warning (1-13 range).
    pub gate: u8,
}

impl VerificationWarning {
    /// Minimum valid gate value (inclusive).
    pub const MIN_GATE: u8 = 1;
    /// Maximum valid gate value (inclusive).
    pub const MAX_GATE: u8 = 13;

    /// Returns `true` if the `gate` field falls within the valid 1-13 range.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.gate >= Self::MIN_GATE && self.gate <= Self::MAX_GATE
    }
}

impl fmt::Display for VerificationWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "gate {}: [{}] {}",
            self.gate, self.code, self.message
        )
    }
}

/// Proof that artifact verification passed at admission time.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VerificationProof {
    /// Confirmed digest of the verified artifact.
    pub digest: vb_core::WorkflowDigest,
    /// Number of verification gates that passed.
    pub gate_count: u8,
    /// Whether the proof was durably persisted (SyncAll).
    pub durable: bool,
    /// Soft verification failures encountered during admission.
    pub warnings: Vec<VerificationWarning>,
}

impl VerificationProof {
    /// Creates a new verification proof.
    #[must_use]
    pub fn new(digest: vb_core::WorkflowDigest, gate_count: u8, durable: bool) -> Self {
        Self {
            digest,
            gate_count,
            durable,
            warnings: Vec::new(),
        }
    }
}

/// Accepted artifact record produced by the admission flow.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AcceptedArtifact {
    /// The artifact's content hash.
    pub digest: vb_core::WorkflowDigest,
    /// Serialized compiled IR (postcard).
    pub ir: Vec<u8>,
    /// Proof that verification passed.
    pub verification: VerificationProof,
    /// Journal sequence when accepted.
    pub accepted_at_seq: EventSeq,
    /// Required capabilities for actions in this artifact.
    pub required_capabilities: Box<[vb_core::capability::Capability]>,
}

/// Number of verification gates in the admission flow.
const ADMISSION_GATE_COUNT: u8 = 2;

/// Validates, verifies, and persists a compiled workflow artifact with policy-controlled durability.
///
/// This is the full admission flow. It performs:
/// 1. Structure validation: re-parse the workflow from serialized parts.
/// 2. Checksum validation: serialized bytes must hash to the claimed digest.
/// 3. Persistence: store the artifact in the `compiled_ir` keyspace.
/// 4. Durability: under `Strict` policy, calls SyncAll before returning.
///
/// Returns the `AcceptedArtifact` on success.
pub fn submit_artifact(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
) -> Result<AcceptedArtifact, JournalError> {
    match policy {
        vb_core::RuntimePolicy::Relaxed => {
            // No verification required, just persist.
            let parts = workflow.to_parts();
            let bytes =
                postcard::to_allocvec(&parts).map_err(|_| JournalError::ArtifactMalformed)?;
            let record = CompiledIrRecord {
                digest: workflow.digest(),
                ir: bytes,
            };
            journal.put_compiled_ir(&record)?;
            Ok(AcceptedArtifact {
                digest: workflow.digest(),
                ir: record.ir,
                verification: VerificationProof::new(workflow.digest(), 0, false),
                accepted_at_seq: EventSeq::new(0),
                required_capabilities: Box::new([]),
            })
        }
        vb_core::RuntimePolicy::Journaled | vb_core::RuntimePolicy::Strict => {
            let parts = workflow.to_parts();

            // Gate 1: Structure validation — must reconstruct successfully.
            vb_core::CompiledWorkflow::try_from_parts(parts.clone())
                .map_err(|_| JournalError::ArtifactMalformed)?;

            // Gate 2: Checksum validation — hash the content fields (digest zeroed)
            // and compare to the claimed digest. This avoids the circular dependency
            // where the digest field is part of its own hash input.
            let mut parts_for_hash = parts.clone();
            parts_for_hash.digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
            let hash_bytes = postcard::to_allocvec(&parts_for_hash)
                .map_err(|_| JournalError::ArtifactMalformed)?;
            let computed = blake3::hash(&hash_bytes);
            if computed.as_bytes() != &workflow.digest().as_bytes() {
                return Err(JournalError::ArtifactChecksumMismatch);
            }

            // Full serialization for storage (includes correct digest).
            let bytes =
                postcard::to_allocvec(&parts).map_err(|_| JournalError::ArtifactMalformed)?;

            // Persist accepted artifact.
            let record = CompiledIrRecord {
                digest: workflow.digest(),
                ir: bytes,
            };
            journal.put_compiled_ir(&record)?;

            let durable = policy == vb_core::RuntimePolicy::Strict;
            if durable {
                journal.persist_strict()?;
            }

            let proof = VerificationProof::new(workflow.digest(), ADMISSION_GATE_COUNT, durable);
            let artifact = AcceptedArtifact {
                digest: workflow.digest(),
                ir: record.ir,
                verification: proof,
                accepted_at_seq: EventSeq::new(0),
                required_capabilities: Box::new([]),
            };

            Ok(artifact)
        }
    }
}

/// Validates and persists a compiled workflow artifact.
///
/// Structure validation ensures the workflow can be reconstructed from its parts.
/// Checksum validation recomputes the BLAKE3 digest from the serialized parts
/// and compares it to the digest claimed by the workflow.
///
/// On success, the artifact is stored in the `compiled_ir` keyspace and its
/// digest is returned. On failure, the storage is left unchanged.
pub fn admit_compiled_artifact(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
) -> Result<vb_core::WorkflowDigest, JournalError> {
    let parts = workflow.to_parts();

    // Structure validation: must reconstruct successfully.
    vb_core::CompiledWorkflow::try_from_parts(parts.clone())
        .map_err(|_| JournalError::ArtifactMalformed)?;

    // Checksum validation: hash content fields (digest zeroed) and compare
    // to the claimed digest to avoid the circular dependency where the digest
    // field is part of its own hash input.
    let mut parts_for_hash = parts.clone();
    parts_for_hash.digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
    let hash_bytes =
        postcard::to_allocvec(&parts_for_hash).map_err(|_| JournalError::ArtifactMalformed)?;
    let computed = blake3::hash(&hash_bytes);
    if computed.as_bytes() != &workflow.digest().as_bytes() {
        return Err(JournalError::ArtifactChecksumMismatch);
    }

    // Persist accepted artifact with full serialization (includes digest).
    let bytes = postcard::to_allocvec(&parts).map_err(|_| JournalError::ArtifactMalformed)?;
    let record = CompiledIrRecord {
        digest: workflow.digest(),
        ir: bytes,
    };
    journal.put_compiled_ir(&record)?;

    Ok(workflow.digest())
}

#[cfg(test)]
#[allow(
    clippy::assertions_on_constants,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod tests {
    use super::*;

    #[test]
    fn verification_warning_display_formats_gate_code_message() {
        let warning = VerificationWarning {
            code: 42,
            message: Box::from("deprecated action kind"),
            gate: 3,
        };
        assert_eq!(
            format!("{warning}"),
            "gate 3: [42] deprecated action kind"
        );
    }

    #[test]
    fn verification_warning_equality_works() {
        let a = VerificationWarning {
            code: 1,
            message: Box::from("alpha"),
            gate: 2,
        };
        let b = VerificationWarning {
            code: 1,
            message: Box::from("alpha"),
            gate: 2,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn verification_warning_inequality_different_code() {
        let a = VerificationWarning {
            code: 1,
            message: Box::from("alpha"),
            gate: 2,
        };
        let b = VerificationWarning {
            code: 99,
            message: Box::from("alpha"),
            gate: 2,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn verification_warning_inequality_different_gate() {
        let a = VerificationWarning {
            code: 1,
            message: Box::from("alpha"),
            gate: 1,
        };
        let b = VerificationWarning {
            code: 1,
            message: Box::from("alpha"),
            gate: 13,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn verification_proof_new_initializes_empty_warnings() {
        let digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
        let proof = VerificationProof::new(digest, 2, true);
        assert!(proof.warnings.is_empty());
        assert_eq!(proof.gate_count, 2);
        assert!(proof.durable);
    }

    #[test]
    fn verification_proof_warnings_can_be_populated() {
        let digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
        let mut proof = VerificationProof::new(digest, 5, false);
        proof.warnings.push(VerificationWarning {
            code: 100,
            message: Box::from("soft check advisory"),
            gate: 7,
        });
        proof.warnings.push(VerificationWarning {
            code: 200,
            message: Box::from("boundary advisory"),
            gate: 11,
        });
        assert_eq!(proof.warnings.len(), 2);
        assert_eq!(proof.warnings[0].gate, 7);
        assert_eq!(proof.warnings[1].code, 200);
    }

    #[test]
    fn verification_warning_clone_preserves_fields() {
        let original = VerificationWarning {
            code: 55,
            message: Box::from("cloneable warning"),
            gate: 9,
        };
        let cloned = original.clone();
        assert_eq!(original, cloned);
        assert_eq!(cloned.code, 55);
        assert_eq!(&*cloned.message, "cloneable warning");
        assert_eq!(cloned.gate, 9);
    }

    #[test]
    fn is_valid_rejects_gate_zero() {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("zero gate"),
            gate: 0,
        };
        assert!(!w.is_valid());
    }

    #[test]
    fn is_valid_accepts_gate_one() {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("min gate"),
            gate: VerificationWarning::MIN_GATE,
        };
        assert!(w.is_valid());
    }

    #[test]
    fn is_valid_accepts_gate_thirteen() {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("max gate"),
            gate: VerificationWarning::MAX_GATE,
        };
        assert!(w.is_valid());
    }

    #[test]
    fn is_valid_rejects_gate_fourteen() {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("above max gate"),
            gate: 14,
        };
        assert!(!w.is_valid());
    }
}
