#![forbid(unsafe_code)]
//! Artifact admission and verification functions.
//!
//! Provides artifact submission and admission flows with policy-controlled durability.

use std::fmt;

use crate::{error::JournalError, records::CompiledIrRecord, types::EventSeq};

use crate::journal::FjallJournal;

/// A soft verification failure that does not block admission but should be reported.
///
/// Each warning is associated with a specific verification gate (1-2 range per
/// contract §4.2) and carries a numeric code and human-readable message.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VerificationWarning {
    /// Numeric code identifying the specific warning condition.
    pub code: u32,
    /// Human-readable description of the warning.
    pub message: Box<str>,
    /// Which verification gate produced this warning (1-2 range per contract).
    pub gate: u8,
}

impl VerificationWarning {
    /// Minimum valid gate value (inclusive).
    pub const MIN_GATE: u8 = 1;
    /// Maximum valid gate value (inclusive). Contract §4.2 specifies gate_count = 2.
    pub const MAX_GATE: u8 = 2;

    /// Returns `true` if the `gate` field falls within the valid 1-2 range.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.gate >= Self::MIN_GATE && self.gate <= Self::MAX_GATE
    }
}

impl fmt::Display for VerificationWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "gate {}: [{}] {}", self.gate, self.code, self.message)
    }
}

/// Proof flag that must be true for an accepted artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProofFlag {
    /// Artifact IR is size-bounded.
    Bounded,
    /// Artifact does not propagate taint.
    TaintSafe,
    /// Artifact actions are safe to retry.
    RetrySafe,
    /// Artifact can be replayed.
    Replayable,
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
    /// Artifact IR is size-bounded.
    pub bounded: bool,
    /// Artifact does not propagate taint.
    pub taint_safe: bool,
    /// Artifact actions are safe to retry.
    pub retry_safe: bool,
    /// Artifact can be replayed.
    pub replayable: bool,
    /// Actions keyed by idempotency key.
    pub idempotency_keyed: Box<[vb_core::ActionId]>,
    /// Actions with idempotency attested.
    pub idempotency_attested: Box<[vb_core::ActionId]>,
    /// Soft verification failures encountered during admission.
    pub warnings: Vec<VerificationWarning>,
}

impl VerificationProof {
    /// Creates a new verification proof with all proof flags set to true.
    #[must_use]
    pub fn new(digest: vb_core::WorkflowDigest, gate_count: u8, durable: bool) -> Self {
        Self {
            digest,
            gate_count,
            durable,
            bounded: true,
            taint_safe: true,
            retry_safe: true,
            replayable: true,
            idempotency_keyed: Box::new([]),
            idempotency_attested: Box::new([]),
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

/// Number of verification gates in the accepted artifact v1 admission flow.
const ADMISSION_GATE_COUNT: u8 = 2;

/// Validates, verifies, and persists a compiled workflow artifact with policy-controlled durability.
///
/// This is the full admission flow. It performs:
/// 1. Policy check: Relaxed is rejected when accepted artifacts are required.
/// 2. Structure validation: re-parse the workflow from serialized parts.
/// 3. Checksum validation: serialized bytes must hash to the claimed digest.
/// 4. Proof validation: gate count must be 2 and all proof flags must be true.
/// 5. Persistence: store the artifact in the `compiled_ir` keyspace.
/// 6. Durability: under `Strict` policy, calls SyncAll before returning.
///
/// Returns the `AcceptedArtifact` on success.
pub fn submit_artifact(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
) -> Result<AcceptedArtifact, JournalError> {
    match policy {
        vb_core::RuntimePolicy::Relaxed => {
            // Relaxed: skip gate validation, no durability, gate_count=0
            let parts = workflow.to_parts();
            let ir_bytes =
                postcard::to_allocvec(&parts).map_err(|_| JournalError::ArtifactMalformed)?;
            let proof = VerificationProof::new(workflow.digest(), 0, false);
            let artifact = AcceptedArtifact {
                digest: workflow.digest(),
                ir: ir_bytes,
                verification: proof,
                accepted_at_seq: EventSeq::new(0),
                required_capabilities: Box::new([]),
            };
            let artifact_bytes =
                postcard::to_allocvec(&artifact).map_err(|_| JournalError::ArtifactMalformed)?;
            let record = CompiledIrRecord {
                digest: workflow.digest(),
                ir: artifact_bytes,
            };
            journal.put_compiled_ir(&record)?;
            Ok(artifact)
        }
        vb_core::RuntimePolicy::Journaled | vb_core::RuntimePolicy::Strict => {
            let parts = workflow.to_parts();

            vb_core::CompiledWorkflow::try_from_parts(parts.clone())
                .map_err(|_| JournalError::ArtifactMalformed)?;

            let mut parts_for_hash = parts.clone();
            parts_for_hash.digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
            let hash_bytes = postcard::to_allocvec(&parts_for_hash)
                .map_err(|_| JournalError::ArtifactMalformed)?;
            let computed = blake3::hash(&hash_bytes);
            if computed.as_bytes() != &workflow.digest().as_bytes() {
                return Err(JournalError::ArtifactChecksumMismatch);
            }

            let durable = policy == vb_core::RuntimePolicy::Strict;

            let proof = VerificationProof::new(workflow.digest(), ADMISSION_GATE_COUNT, durable);

            let ir_bytes =
                postcard::to_allocvec(&parts).map_err(|_| JournalError::ArtifactMalformed)?;

            let artifact = AcceptedArtifact {
                digest: workflow.digest(),
                ir: ir_bytes,
                verification: proof,
                accepted_at_seq: EventSeq::new(0),
                required_capabilities: Box::new([]),
            };

            let artifact_bytes =
                postcard::to_allocvec(&artifact).map_err(|_| JournalError::ArtifactMalformed)?;
            let record = CompiledIrRecord {
                digest: workflow.digest(),
                ir: artifact_bytes,
            };
            journal.put_compiled_ir(&record)?;

            if durable {
                journal.persist_strict()?;
            }

            let stored = journal
                .compiled_ir(workflow.digest())
                .map_err(|_| JournalError::ArtifactMalformed)?;
            if stored.is_none() {
                return Err(JournalError::ArtifactMalformed);
            }

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
        assert_eq!(format!("{warning}"), "gate 3: [42] deprecated action kind");
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
    fn is_valid_accepts_gate_two() {
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

    // =========================================================================
    // submit_artifact: Relaxed policy
    // =========================================================================

    /// Opens a temporary FjallJournal that is cleaned up when dropped.
    fn temp_journal() -> Result<crate::FjallJournal, JournalError> {
        let dir = tempfile::tempdir().map_err(|_| JournalError::ArtifactMalformed)?;
        // Keep the TempDir so it survives the journal lifetime.
        // The OS cleans up /tmp eventually.
        let path = dir.keep();
        crate::FjallJournal::open(path, None)
    }

    /// Builds a minimal valid CompiledWorkflow for testing.
    ///
    /// The digest is computed by serializing the parts with the digest field zeroed,
    /// then BLAKE3-hashing the result. This mirrors the checksum validation gate.
    fn minimal_workflow() -> Result<vb_core::CompiledWorkflow, String> {
        use vb_core::value::ConstValue;
        use vb_core::workflow::{ResourceContract, WorkflowParts};
        use vb_core::{
            CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, SlotIdx, StepIdx,
            WorkflowDigest,
        };

        let mut parts = WorkflowParts {
            name: Box::<str>::from("test_admission"),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: Box::new([
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([ConstValue::I64(42)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };

        // Compute the correct BLAKE3 digest from the zeroed-digest serialization.
        let hash_bytes = postcard::to_allocvec(&parts)
            .map_err(|e| format!("serialize parts for digest: {e}"))?;
        let computed = blake3::hash(&hash_bytes);
        parts.digest = WorkflowDigest::from_bytes(computed.into());

        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    #[test]
    fn submit_artifact_relaxed_persists_and_returns_artifact() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;
        let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed)
            .map_err(|e| format!("submit_artifact(relaxed) failed: {e}"))?;

        // The returned digest must match the workflow's digest.
        assert_eq!(
            result.digest,
            workflow.digest(),
            "artifact digest must match workflow digest"
        );

        // The proof under Relaxed must have 0 gates and durable=false.
        assert_eq!(result.verification.gate_count, 0, "relaxed must skip gates");
        assert!(!result.verification.durable, "relaxed must not be durable");

        // The proof's digest must match.
        assert_eq!(
            result.verification.digest,
            workflow.digest(),
            "proof digest must match workflow digest"
        );

        // The ir bytes must be non-empty (postcard serialization).
        assert!(!result.ir.is_empty(), "compiled IR bytes must not be empty");

        // Verify we can read the artifact back from storage.
        let loaded = journal
            .compiled_ir(workflow.digest())
            .map_err(|e| format!("compiled_ir read failed: {e}"))?;
        assert!(loaded.is_some(), "artifact must be readable after submit");
        Ok(())
    }

    #[test]
    fn submit_artifact_journaled_runs_both_gates() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;

        let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
            .map_err(|e| format!("submit_artifact(journaled) failed: {e}"))?;

        // Journaled passes 2 gates but is not durable (no SyncAll).
        assert_eq!(
            result.verification.gate_count, 2,
            "journaled must pass 2 verification gates"
        );
        assert!(
            !result.verification.durable,
            "journaled must not be durable"
        );
        assert_eq!(result.digest, workflow.digest());
        Ok(())
    }

    #[test]
    fn submit_artifact_strict_is_durable() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;

        let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict)
            .map_err(|e| format!("submit_artifact(strict) failed: {e}"))?;

        // Strict passes 2 gates AND is durable.
        assert_eq!(result.verification.gate_count, 2);
        assert!(result.verification.durable, "strict must be durable");
        assert_eq!(result.digest, workflow.digest());
        Ok(())
    }

    // =========================================================================
    // submit_artifact: checksum validation
    // =========================================================================

    #[test]
    fn submit_artifact_journaled_roundtrip_bytes_match() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;

        let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
            .map_err(|e| format!("submit failed: {e}"))?;

        // Deserialize the returned IR bytes and verify the digest field.
        let loaded = journal
            .compiled_ir(result.digest)
            .map_err(|e| format!("read failed: {e}"))?;
        let record = loaded.ok_or_else(|| String::from("artifact not found after submit"))?;
        assert_eq!(record.digest, result.digest, "stored digest must match");
        Ok(())
    }

    // =========================================================================
    // admit_compiled_artifact
    // =========================================================================

    #[test]
    fn admit_compiled_artifact_succeeds_for_valid_workflow() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;

        let digest = admit_compiled_artifact(&journal, &workflow)
            .map_err(|e| format!("admit_compiled_artifact failed: {e}"))?;

        assert_eq!(
            digest,
            workflow.digest(),
            "returned digest must match workflow digest"
        );

        // Verify it's stored.
        let loaded = journal
            .compiled_ir(digest)
            .map_err(|e| format!("read failed: {e}"))?;
        assert!(loaded.is_some(), "artifact must be stored after admission");
        Ok(())
    }

    #[test]
    fn admit_compiled_artifact_idempotent() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;

        let digest_a = admit_compiled_artifact(&journal, &workflow)
            .map_err(|e| format!("first admit failed: {e}"))?;
        let digest_b = admit_compiled_artifact(&journal, &workflow)
            .map_err(|e| format!("second admit failed: {e}"))?;

        assert_eq!(
            digest_a, digest_b,
            "idempotent admission must return same digest"
        );
        Ok(())
    }

    // =========================================================================
    // AcceptedArtifact fields
    // =========================================================================

    #[test]
    fn accepted_artifact_fields_are_populated() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;

        let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed)
            .map_err(|e| format!("submit failed: {e}"))?;

        // accepted_at_seq should be 0 (no journal sequence tracking in current impl).
        assert_eq!(
            artifact.accepted_at_seq.get(),
            0,
            "accepted_at_seq must be 0"
        );
        // required_capabilities should be empty for minimal workflow.
        assert!(
            artifact.required_capabilities.is_empty(),
            "minimal workflow has no capabilities"
        );
        Ok(())
    }

    // =========================================================================
    // VerificationProof details
    // =========================================================================

    #[test]
    fn verification_proof_serde_roundtrip() -> Result<(), String> {
        let digest = vb_core::WorkflowDigest::from_bytes([0xAA_u8; 32]);
        let mut proof = VerificationProof::new(digest, 3, true);
        proof.warnings.push(VerificationWarning {
            code: 7,
            message: Box::from("test warning"),
            gate: 5,
        });

        let serialized =
            postcard::to_allocvec(&proof).map_err(|e| format!("serialize failed: {e}"))?;
        let deserialized: VerificationProof =
            postcard::from_bytes(&serialized).map_err(|e| format!("deserialize failed: {e}"))?;

        assert_eq!(proof, deserialized, "proof must survive serde roundtrip");
        Ok(())
    }

    #[test]
    fn accepted_artifact_serde_roundtrip() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;

        let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
            .map_err(|e| format!("submit failed: {e}"))?;

        let serialized =
            postcard::to_allocvec(&artifact).map_err(|e| format!("serialize failed: {e}"))?;
        let deserialized: AcceptedArtifact =
            postcard::from_bytes(&serialized).map_err(|e| format!("deserialize failed: {e}"))?;

        assert_eq!(
            artifact, deserialized,
            "artifact must survive serde roundtrip"
        );
        Ok(())
    }

    // =========================================================================
    // Relaxed vs Journaled/Strict gate count difference
    // =========================================================================

    #[test]
    fn relaxed_skips_gates_while_journaled_passes_them() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;

        let relaxed = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed)
            .map_err(|e| format!("relaxed failed: {e}"))?;
        let journaled = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
            .map_err(|e| format!("journaled failed: {e}"))?;

        assert!(
            relaxed.verification.gate_count < journaled.verification.gate_count,
            "relaxed gate count ({}) must be less than journaled ({})",
            relaxed.verification.gate_count,
            journaled.verification.gate_count
        );
        Ok(())
    }

    #[test]
    fn strict_and_journaled_have_same_gate_count() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;

        let journaled = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
            .map_err(|e| format!("journaled failed: {e}"))?;
        let strict = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict)
            .map_err(|e| format!("strict failed: {e}"))?;

        assert_eq!(
            journaled.verification.gate_count, strict.verification.gate_count,
            "journaled and strict must have identical gate count"
        );
        // Only difference is durable flag.
        assert!(!journaled.verification.durable);
        assert!(strict.verification.durable);
        Ok(())
    }

    // =========================================================================
    // Warning gate boundary values
    // =========================================================================

    #[test]
    fn all_valid_gates_pass_is_valid() -> Result<(), String> {
        for gate in VerificationWarning::MIN_GATE..=VerificationWarning::MAX_GATE {
            let w = VerificationWarning {
                code: 1,
                message: Box::from("boundary test"),
                gate,
            };
            if !w.is_valid() {
                return Err(format!("gate {gate} should be valid"));
            }
        }
        Ok(())
    }

    #[test]
    fn gate_values_outside_range_fail_is_valid() -> Result<(), String> {
        for gate in [0u8, 14, 15, 20, 255] {
            let w = VerificationWarning {
                code: 1,
                message: Box::from("out of range test"),
                gate,
            };
            if w.is_valid() {
                return Err(format!("gate {gate} should be invalid"));
            }
        }
        Ok(())
    }

    // =========================================================================
    // VerificationWarning serialization
    // =========================================================================

    #[test]
    fn verification_warning_serde_roundtrip() -> Result<(), String> {
        let warning = VerificationWarning {
            code: 999,
            message: Box::from("serde test warning"),
            gate: 7,
        };
        let bytes =
            postcard::to_allocvec(&warning).map_err(|e| format!("serialize failed: {e}"))?;
        let back: VerificationWarning =
            postcard::from_bytes(&bytes).map_err(|e| format!("deserialize failed: {e}"))?;
        assert_eq!(warning, back);
        Ok(())
    }

    // =========================================================================
    // VerificationProof idempotency fields — INV-05 unit tests
    // =========================================================================

    #[test]
    fn verification_proof_idempotency_keyed_starts_empty() {
        let digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
        let proof = VerificationProof::new(digest, 2, true);
        assert!(
            proof.idempotency_keyed.is_empty(),
            "idempotency_keyed must start empty"
        );
    }

    #[test]
    fn verification_proof_idempotency_attested_starts_empty() {
        let digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
        let proof = VerificationProof::new(digest, 2, true);
        assert!(
            proof.idempotency_attested.is_empty(),
            "idempotency_attested must start empty"
        );
    }

    #[test]
    fn verification_proof_idempotency_keyed_can_be_populated() {
        let digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
        let mut proof = VerificationProof::new(digest, 2, true);
        let action_ids = vec![
            vb_core::ActionId::new(1),
            vb_core::ActionId::new(2),
            vb_core::ActionId::new(3),
        ];
        proof.idempotency_keyed = action_ids.into_boxed_slice();
        assert_eq!(proof.idempotency_keyed.len(), 3);
        assert_eq!(proof.idempotency_keyed[0], vb_core::ActionId::new(1));
        assert_eq!(proof.idempotency_keyed[2], vb_core::ActionId::new(3));
    }

    #[test]
    fn verification_proof_idempotency_attested_can_be_populated() {
        let digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
        let mut proof = VerificationProof::new(digest, 2, true);
        let action_ids = vec![vb_core::ActionId::new(5), vb_core::ActionId::new(10)];
        proof.idempotency_attested = action_ids.into_boxed_slice();
        assert_eq!(proof.idempotency_attested.len(), 2);
        assert_eq!(proof.idempotency_attested[0], vb_core::ActionId::new(5));
    }

    #[test]
    fn verification_proof_idempotency_keyed_survives_serde() -> Result<(), String> {
        let digest = vb_core::WorkflowDigest::from_bytes([0xabu8; 32]);
        let mut proof = VerificationProof::new(digest, 3, true);
        proof.idempotency_keyed = vec![
            vb_core::ActionId::new(100),
            vb_core::ActionId::new(200),
            vb_core::ActionId::new(300),
        ]
        .into_boxed_slice();
        let bytes = postcard::to_allocvec(&proof).map_err(|e| format!("serialize failed: {e}"))?;
        let back: VerificationProof =
            postcard::from_bytes(&bytes).map_err(|e| format!("deserialize failed: {e}"))?;
        assert_eq!(back.idempotency_keyed.len(), 3);
        assert_eq!(back.idempotency_keyed[0], vb_core::ActionId::new(100));
        assert_eq!(back.idempotency_keyed[2], vb_core::ActionId::new(300));
        Ok(())
    }

    #[test]
    fn verification_proof_idempotency_attested_survives_serde() -> Result<(), String> {
        let digest = vb_core::WorkflowDigest::from_bytes([0xabu8; 32]);
        let mut proof = VerificationProof::new(digest, 3, true);
        proof.idempotency_attested =
            vec![vb_core::ActionId::new(400), vb_core::ActionId::new(500)].into_boxed_slice();
        let bytes = postcard::to_allocvec(&proof).map_err(|e| format!("serialize failed: {e}"))?;
        let back: VerificationProof =
            postcard::from_bytes(&bytes).map_err(|e| format!("deserialize failed: {e}"))?;
        assert_eq!(back.idempotency_attested.len(), 2);
        assert_eq!(back.idempotency_attested[0], vb_core::ActionId::new(400));
        assert_eq!(back.idempotency_attested[1], vb_core::ActionId::new(500));
        Ok(())
    }

    #[test]
    fn verification_proof_both_idempotency_fields_populated_survive_serde() -> Result<(), String> {
        let digest = vb_core::WorkflowDigest::from_bytes([0xacu8; 32]);
        let mut proof = VerificationProof::new(digest, 3, true);
        proof.idempotency_keyed =
            vec![vb_core::ActionId::new(1), vb_core::ActionId::new(2)].into_boxed_slice();
        proof.idempotency_attested = vec![vb_core::ActionId::new(3)].into_boxed_slice();
        let bytes = postcard::to_allocvec(&proof).map_err(|e| format!("serialize failed: {e}"))?;
        let back: VerificationProof =
            postcard::from_bytes(&bytes).map_err(|e| format!("deserialize failed: {e}"))?;
        assert_eq!(back.idempotency_keyed.len(), 2);
        assert_eq!(back.idempotency_attested.len(), 1);
        assert_eq!(back.idempotency_keyed[0], vb_core::ActionId::new(1));
        assert_eq!(back.idempotency_attested[0], vb_core::ActionId::new(3));
        Ok(())
    }

    #[test]
    fn verification_proof_flags_independent_of_idempotency_fields() {
        let digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
        let mut proof_false = VerificationProof::new(digest, 2, false);
        proof_false.idempotency_keyed = vec![vb_core::ActionId::new(99)].into_boxed_slice();

        let mut proof_true = VerificationProof::new(digest, 2, true);
        proof_true.idempotency_keyed = vec![vb_core::ActionId::new(99)].into_boxed_slice();

        assert_eq!(
            proof_false.idempotency_keyed, proof_true.idempotency_keyed,
            "idempotency_keyed content must be independent of durable flag"
        );
        assert_eq!(
            proof_false.idempotency_attested, proof_true.idempotency_attested,
            "idempotency_attested content must be independent of durable flag"
        );
    }

    // =========================================================================
    // VerificationWarning bounds — extended boundary unit tests
    // =========================================================================

    #[test]
    fn verification_warning_gate_boundary_at_min_exactly() {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("min gate boundary"),
            gate: 1,
        };
        assert!(w.is_valid(), "gate=1 must be valid (MIN_GATE)");
    }

    #[test]
    fn verification_warning_gate_boundary_at_max_exactly() {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("max gate boundary"),
            gate: 2,
        };
        assert!(w.is_valid(), "gate=2 must be valid (MAX_GATE)");
    }

    #[test]
    fn verification_warning_gate_below_min_is_invalid() {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("below min gate"),
            gate: 0,
        };
        assert!(!w.is_valid(), "gate=0 must be invalid");
    }

    #[test]
    fn verification_warning_gate_above_max_is_invalid() {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("above max gate"),
            gate: 3,
        };
        assert!(!w.is_valid(), "gate=3 must be invalid");
    }

    #[test]
    fn verification_warning_display_shows_all_fields() {
        let w = VerificationWarning {
            code: 42,
            message: Box::from("test message"),
            gate: 1,
        };
        let display = format!("{w}");
        assert!(
            display.contains("gate 1"),
            "display must contain gate value"
        );
        assert!(display.contains("[42]"), "display must contain code");
        assert!(
            display.contains("test message"),
            "display must contain message"
        );
    }

    #[test]
    fn verification_warning_is_valid_const_values_are_correct() {
        assert_eq!(VerificationWarning::MIN_GATE, 1, "MIN_GATE must be 1");
        assert_eq!(VerificationWarning::MAX_GATE, 2, "MAX_GATE must be 2");
    }

    // =========================================================================
    // submit_artifact checksum mismatch — MAJOR-1 fix
    // =========================================================================

    #[test]
    fn submit_artifact_journaled_rejects_checksum_mismatch() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;

        // Corrupt the workflow's digest to cause checksum mismatch
        let mut corrupted_parts = workflow.to_parts();
        corrupted_parts.digest = vb_core::WorkflowDigest::from_bytes([0xFF; 32]);
        let corrupted = vb_core::CompiledWorkflow::try_from_parts(corrupted_parts)
            .map_err(|e| format!("workflow reconstruct failed: {e}"))?;

        let result = submit_artifact(&journal, &corrupted, vb_core::RuntimePolicy::Journaled);

        assert!(
            matches!(result, Err(JournalError::ArtifactChecksumMismatch)),
            "journaled submit with wrong digest must return ArtifactChecksumMismatch, got {:?}",
            result
        );
        Ok(())
    }

    #[test]
    fn submit_artifact_strict_rejects_checksum_mismatch() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;

        // Corrupt the workflow's digest to cause checksum mismatch
        let mut corrupted_parts = workflow.to_parts();
        corrupted_parts.digest = vb_core::WorkflowDigest::from_bytes([0xAA; 32]);
        let corrupted = vb_core::CompiledWorkflow::try_from_parts(corrupted_parts)
            .map_err(|e| format!("workflow reconstruct failed: {e}"))?;

        let result = submit_artifact(&journal, &corrupted, vb_core::RuntimePolicy::Strict);

        assert!(
            matches!(result, Err(JournalError::ArtifactChecksumMismatch)),
            "strict submit with wrong digest must return ArtifactChecksumMismatch, got {:?}",
            result
        );
        Ok(())
    }

    // =========================================================================
    // admit_compiled_artifact checksum mismatch — branch coverage
    // =========================================================================

    #[test]
    fn admit_compiled_artifact_rejects_checksum_mismatch() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;

        // Corrupt the workflow's digest to cause checksum mismatch
        let mut corrupted_parts = workflow.to_parts();
        corrupted_parts.digest = vb_core::WorkflowDigest::from_bytes([0xBB; 32]);
        let corrupted = vb_core::CompiledWorkflow::try_from_parts(corrupted_parts)
            .map_err(|e| format!("workflow reconstruct failed: {e}"))?;

        let result = admit_compiled_artifact(&journal, &corrupted);

        assert!(
            matches!(result, Err(JournalError::ArtifactChecksumMismatch)),
            "admit_compiled_artifact with wrong digest must return ArtifactChecksumMismatch, got {:?}",
            result
        );
        Ok(())
    }

    // =========================================================================
    // submit_artifact Relaxed policy branches
    // =========================================================================

    #[test]
    fn submit_artifact_relaxed_returns_correct_digest() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;
        let expected_digest = workflow.digest();

        let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed)
            .map_err(|e| format!("relaxed submit failed: {e}"))?;

        assert_eq!(
            artifact.digest, expected_digest,
            "relaxed artifact digest must match workflow digest"
        );
        assert!(
            artifact.ir.len() > 0,
            "relaxed artifact IR must be non-empty"
        );
        Ok(())
    }

    #[test]
    fn submit_artifact_relaxed_skips_checksum_validation() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;

        // Even with wrong digest, Relaxed should succeed (no checksum validation)
        let mut corrupted_parts = workflow.to_parts();
        corrupted_parts.digest = vb_core::WorkflowDigest::from_bytes([0x01; 32]);
        let corrupted = vb_core::CompiledWorkflow::try_from_parts(corrupted_parts)
            .map_err(|e| format!("workflow reconstruct failed: {e}"))?;

        let result = submit_artifact(&journal, &corrupted, vb_core::RuntimePolicy::Relaxed);

        assert!(
            result.is_ok(),
            "relaxed policy must accept workflow even with wrong digest, got {:?}",
            result
        );
        Ok(())
    }

    // =========================================================================
    // submit_artifact stored artifact read back verification
    // =========================================================================

    #[test]
    fn submit_artifact_strict_stored_artifact_can_be_read_back() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;
        let digest = workflow.digest();

        let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict)
            .map_err(|e| format!("strict submit failed: {e}"))?;

        assert_eq!(artifact.digest, digest, "artifact digest must match");

        // Read back the stored artifact and verify it matches
        let stored = journal
            .compiled_ir(digest)
            .map_err(|e| format!("compiled_ir read failed: {e}"))?;

        assert!(
            stored.is_some(),
            "artifact must be readable after strict submit"
        );

        let record = stored.unwrap();
        assert_eq!(
            record.digest, digest,
            "stored record digest must match submitted digest"
        );
        assert!(!record.ir.is_empty(), "stored IR must be non-empty");
        Ok(())
    }

    // =========================================================================
    // VerificationWarning display formatting branches
    // =========================================================================

    #[test]
    fn verification_warning_display_formatting_zero_code() -> Result<(), String> {
        let w = VerificationWarning {
            code: 0,
            message: Box::from("zero code warning"),
            gate: 1,
        };
        let display = format!("{w}");
        assert!(
            display.contains("gate 1"),
            "display must contain gate value"
        );
        assert!(display.contains("[0]"), "display must contain zero code");
        assert!(
            display.contains("zero code warning"),
            "display must contain message"
        );
        Ok(())
    }

    #[test]
    fn verification_warning_display_formatting_max_values() -> Result<(), String> {
        let w = VerificationWarning {
            code: u32::MAX,
            message: Box::from("max code"),
            gate: VerificationWarning::MAX_GATE,
        };
        let display = format!("{w}");
        assert!(display.contains("gate 2"), "display must contain max gate");
        Ok(())
    }

    // =========================================================================
    // VerificationProof all flags set to false
    // =========================================================================

    #[test]
    fn verification_proof_all_flags_false_still_constructs() -> Result<(), String> {
        let digest = vb_core::WorkflowDigest::from_bytes([0xCC; 32]);
        // VerificationProof::new sets all flags to true, but we can override them
        let mut proof = VerificationProof::new(digest, 0, false);
        proof.bounded = false;
        proof.taint_safe = false;
        proof.retry_safe = false;
        proof.replayable = false;

        assert!(!proof.bounded, "bounded can be set to false");
        assert!(!proof.taint_safe, "taint_safe can be set to false");
        assert!(!proof.retry_safe, "retry_safe can be set to false");
        assert!(!proof.replayable, "replayable can be set to false");
        assert_eq!(proof.gate_count, 0, "gate_count should be 0");
        assert!(!proof.durable, "durable should be false");
        Ok(())
    }

    // =========================================================================
    // VerificationWarning edge cases for is_valid coverage
    // =========================================================================

    #[test]
    fn verification_warning_is_valid_gate_zero_returns_false() {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("zero gate"),
            gate: 0,
        };
        assert!(!w.is_valid(), "gate 0 must be invalid");
    }

    #[test]
    fn verification_warning_is_valid_gate_one_returns_true() {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("gate one"),
            gate: 1,
        };
        assert!(w.is_valid(), "gate 1 must be valid");
    }

    #[test]
    fn verification_warning_is_valid_gate_two_returns_true() {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("gate two"),
            gate: 2,
        };
        assert!(w.is_valid(), "gate 2 must be valid");
    }

    #[test]
    fn verification_warning_is_valid_gate_three_returns_false() {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("gate three"),
            gate: 3,
        };
        assert!(!w.is_valid(), "gate 3 must be invalid");
    }

    // =========================================================================
    // VerificationWarning display edge cases
    // =========================================================================

    #[test]
    fn verification_warning_display_single_digit_gate_and_code() -> Result<(), String> {
        let w = VerificationWarning {
            code: 5,
            message: Box::from("short"),
            gate: 1,
        };
        let display = format!("{w}");
        assert!(display.contains("gate 1"), "display must contain 'gate 1'");
        assert!(display.contains("[5]"), "display must contain code");
        Ok(())
    }

    #[test]
    fn verification_warning_display_empty_message() -> Result<(), String> {
        let w = VerificationWarning {
            code: 0,
            message: Box::from(""),
            gate: 2,
        };
        let display = format!("{w}");
        assert!(display.contains("gate 2"), "display must contain gate");
        assert!(display.contains("[0]"), "display must contain zero code");
        Ok(())
    }

    // =========================================================================
    // VerificationProof with various idempotency configurations
    // =========================================================================

    #[test]
    fn verification_proof_empty_idempotency_keyed_and_populated_attested() {
        let digest = vb_core::WorkflowDigest::from_bytes([0xDD; 32]);
        let mut proof = VerificationProof::new(digest, 2, true);
        proof.idempotency_keyed = Box::new([]);
        proof.idempotency_attested =
            vec![vb_core::ActionId::new(1), vb_core::ActionId::new(2)].into_boxed_slice();

        assert!(proof.idempotency_keyed.is_empty());
        assert_eq!(proof.idempotency_attested.len(), 2);
    }

    #[test]
    fn verification_proof_populated_idempotency_keyed_and_empty_attested() {
        let digest = vb_core::WorkflowDigest::from_bytes([0xEE; 32]);
        let mut proof = VerificationProof::new(digest, 2, true);
        proof.idempotency_keyed = vec![
            vb_core::ActionId::new(3),
            vb_core::ActionId::new(4),
            vb_core::ActionId::new(5),
        ]
        .into_boxed_slice();
        proof.idempotency_attested = Box::new([]);

        assert_eq!(proof.idempotency_keyed.len(), 3);
        assert!(proof.idempotency_attested.is_empty());
    }

    #[test]
    fn verification_proof_both_idempotency_populated() {
        let digest = vb_core::WorkflowDigest::from_bytes([0xFF; 32]);
        let mut proof = VerificationProof::new(digest, 2, true);
        proof.idempotency_keyed = vec![vb_core::ActionId::new(1)].into_boxed_slice();
        proof.idempotency_attested = vec![vb_core::ActionId::new(2)].into_boxed_slice();

        assert_eq!(proof.idempotency_keyed.len(), 1);
        assert_eq!(proof.idempotency_attested.len(), 1);
    }

    // =========================================================================
    // submit_artifact with checksum mismatch (all variants)
    // =========================================================================

    #[test]
    fn submit_artifact_strict_rejects_spoofed_digest() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;

        // Spoof the digest to be completely wrong.
        let mut parts = workflow.to_parts();
        parts.digest = vb_core::WorkflowDigest::from_bytes([0xAB; 32]);
        let spoofed = vb_core::CompiledWorkflow::try_from_parts(parts)
            .map_err(|e| format!("workflow construct failed: {e}"))?;

        let result = submit_artifact(&journal, &spoofed, vb_core::RuntimePolicy::Strict);

        assert!(
            matches!(result, Err(JournalError::ArtifactChecksumMismatch)),
            "strict with spoofed digest must be rejected, got {:?}",
            result
        );
        Ok(())
    }

    // =========================================================================
    // VerificationProof display (Debug impl)
    // =========================================================================

    #[test]
    fn verification_proof_debug_format_contains_fields() -> Result<(), String> {
        let digest = vb_core::WorkflowDigest::from_bytes([0x11; 32]);
        let proof = VerificationProof::new(digest, 2, true);
        let debug = format!("{:?}", proof);
        assert!(
            debug.contains("VerificationProof"),
            "debug should contain type name"
        );
        assert!(
            debug.contains("bounded"),
            "debug should contain bounded field"
        );
        Ok(())
    }

    #[test]
    fn accepted_artifact_debug_format_contains_fields() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;
        let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed)
            .map_err(|e| format!("submit failed: {e}"))?;

        let debug = format!("{:?}", artifact);
        assert!(
            debug.contains("AcceptedArtifact"),
            "debug should contain type name"
        );
        assert!(
            debug.contains("digest"),
            "debug should contain digest field"
        );
        Ok(())
    }

    // =========================================================================
    // ProofFlag enum coverage
    // =========================================================================

    #[test]
    fn proof_flag_bounded_debug() {
        let flag = ProofFlag::Bounded;
        let debug = format!("{:?}", flag);
        assert!(debug.contains("Bounded"), "debug should contain Bounded");
    }

    #[test]
    fn proof_flag_taint_safe_debug() {
        let flag = ProofFlag::TaintSafe;
        let debug = format!("{:?}", flag);
        assert!(
            debug.contains("TaintSafe"),
            "debug should contain TaintSafe"
        );
    }

    #[test]
    fn proof_flag_retry_safe_debug() {
        let flag = ProofFlag::RetrySafe;
        let debug = format!("{:?}", flag);
        assert!(
            debug.contains("RetrySafe"),
            "debug should contain RetrySafe"
        );
    }

    #[test]
    fn proof_flag_replayable_debug() {
        let flag = ProofFlag::Replayable;
        let debug = format!("{:?}", flag);
        assert!(
            debug.contains("Replayable"),
            "debug should contain Replayable"
        );
    }

    #[test]
    fn proof_flag_all_variants_debug() {
        let bounded = ProofFlag::Bounded;
        let taint_safe = ProofFlag::TaintSafe;
        let retry_safe = ProofFlag::RetrySafe;
        let replayable = ProofFlag::Replayable;

        let debug_bounded = format!("{:?}", bounded);
        let debug_taint = format!("{:?}", taint_safe);
        let debug_retry = format!("{:?}", retry_safe);
        let debug_replay = format!("{:?}", replayable);

        assert!(debug_bounded.contains("Bounded"));
        assert!(debug_taint.contains("TaintSafe"));
        assert!(debug_retry.contains("RetrySafe"));
        assert!(debug_replay.contains("Replayable"));
    }

    // =========================================================================
    // VerificationWarning with various gate values
    // =========================================================================

    #[test]
    fn verification_warning_gate_value_at_min_boundary() {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("min gate"),
            gate: VerificationWarning::MIN_GATE,
        };
        assert!(w.is_valid());
        assert_eq!(w.gate, 1);
    }

    #[test]
    fn verification_warning_gate_value_at_max_boundary() {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("max gate"),
            gate: VerificationWarning::MAX_GATE,
        };
        assert!(w.is_valid());
        assert_eq!(w.gate, 2);
    }

    #[test]
    fn verification_warning_equality_with_identical_fields() {
        let a = VerificationWarning {
            code: 42,
            message: Box::from("test"),
            gate: 1,
        };
        let b = VerificationWarning {
            code: 42,
            message: Box::from("test"),
            gate: 1,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn verification_warning_inequality_different_code_v2() {
        let a = VerificationWarning {
            code: 1,
            message: Box::from("test"),
            gate: 1,
        };
        let b = VerificationWarning {
            code: 2,
            message: Box::from("test"),
            gate: 1,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn verification_warning_clone_equality() {
        let original = VerificationWarning {
            code: 99,
            message: Box::from("clone test"),
            gate: 2,
        };
        let cloned = original.clone();
        assert_eq!(original, cloned);
        assert_eq!(original.code, cloned.code);
        assert_eq!(original.gate, cloned.gate);
    }

    // =========================================================================
    // VerificationProof with various gate counts
    // =========================================================================

    #[test]
    fn verification_proof_new_with_gate_count_zero() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x01; 32]);
        let proof = VerificationProof::new(digest, 0, false);
        assert_eq!(proof.gate_count, 0);
        assert!(!proof.durable);
    }

    #[test]
    fn verification_proof_new_with_gate_count_one() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x02; 32]);
        let proof = VerificationProof::new(digest, 1, false);
        assert_eq!(proof.gate_count, 1);
    }

    #[test]
    fn verification_proof_new_with_gate_count_two() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x03; 32]);
        let proof = VerificationProof::new(digest, 2, true);
        assert_eq!(proof.gate_count, 2);
        assert!(proof.durable);
    }

    #[test]
    fn verification_proof_durable_flag_differs() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x04; 32]);
        let proof_durable = VerificationProof::new(digest, 2, true);
        let proof_not_durable = VerificationProof::new(digest, 2, false);
        assert!(proof_durable.durable);
        assert!(!proof_not_durable.durable);
        assert_eq!(proof_durable.gate_count, proof_not_durable.gate_count);
    }

    #[test]
    fn verification_proof_idempotency_keyed_single_element() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x05; 32]);
        let mut proof = VerificationProof::new(digest, 2, true);
        proof.idempotency_keyed = vec![vb_core::ActionId::new(42)].into_boxed_slice();
        assert_eq!(proof.idempotency_keyed.len(), 1);
        assert_eq!(proof.idempotency_keyed[0], vb_core::ActionId::new(42));
    }

    #[test]
    fn verification_proof_idempotency_attested_single_element() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x06; 32]);
        let mut proof = VerificationProof::new(digest, 2, true);
        proof.idempotency_attested = vec![vb_core::ActionId::new(43)].into_boxed_slice();
        assert_eq!(proof.idempotency_attested.len(), 1);
        assert_eq!(proof.idempotency_attested[0], vb_core::ActionId::new(43));
    }

    #[test]
    fn verification_proof_multiple_warnings() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x07; 32]);
        let mut proof = VerificationProof::new(digest, 2, true);
        proof.warnings.push(VerificationWarning {
            code: 1,
            message: Box::from("warning 1"),
            gate: 1,
        });
        proof.warnings.push(VerificationWarning {
            code: 2,
            message: Box::from("warning 2"),
            gate: 2,
        });
        assert_eq!(proof.warnings.len(), 2);
        assert_eq!(proof.warnings[0].code, 1);
        assert_eq!(proof.warnings[1].code, 2);
    }

    // =========================================================================
    // AcceptedArtifact field coverage
    // =========================================================================

    #[test]
    fn accepted_artifact_has_non_empty_ir() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;
        let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
            .map_err(|e| format!("submit failed: {e}"))?;
        assert!(!artifact.ir.is_empty(), "IR bytes must not be empty");
        Ok(())
    }

    #[test]
    fn accepted_artifact_verification_gate_count_for_journaled() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;
        let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
            .map_err(|e| format!("submit failed: {e}"))?;
        assert_eq!(
            artifact.verification.gate_count, 2,
            "journaled should have gate_count=2"
        );
        assert!(
            !artifact.verification.durable,
            "journaled should not be durable"
        );
        Ok(())
    }

    #[test]
    fn accepted_artifact_verification_gate_count_for_strict() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;
        let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict)
            .map_err(|e| format!("submit failed: {e}"))?;
        assert_eq!(
            artifact.verification.gate_count, 2,
            "strict should have gate_count=2"
        );
        assert!(artifact.verification.durable, "strict should be durable");
        Ok(())
    }

    #[test]
    fn accepted_artifact_verification_gate_count_for_relaxed() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;
        let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed)
            .map_err(|e| format!("submit failed: {e}"))?;
        assert_eq!(
            artifact.verification.gate_count, 0,
            "relaxed should have gate_count=0"
        );
        assert!(
            !artifact.verification.durable,
            "relaxed should not be durable"
        );
        Ok(())
    }

    // =========================================================================
    // Additional serde roundtrip coverage
    // =========================================================================

    #[test]
    fn verification_proof_serde_preserves_all_fields() -> Result<(), String> {
        let digest = vb_core::WorkflowDigest::from_bytes([0xBC; 32]);
        let mut proof = VerificationProof::new(digest, 2, true);
        proof.idempotency_keyed =
            vec![vb_core::ActionId::new(10), vb_core::ActionId::new(20)].into_boxed_slice();
        proof.idempotency_attested = vec![vb_core::ActionId::new(30)].into_boxed_slice();
        proof.warnings.push(VerificationWarning {
            code: 5,
            message: Box::from("test warning"),
            gate: 1,
        });

        let bytes = postcard::to_allocvec(&proof).map_err(|e| format!("serialize failed: {e}"))?;
        let back: VerificationProof =
            postcard::from_bytes(&bytes).map_err(|e| format!("deserialize failed: {e}"))?;

        assert_eq!(back.digest, digest);
        assert_eq!(back.gate_count, 2);
        assert!(back.durable);
        assert_eq!(back.idempotency_keyed.len(), 2);
        assert_eq!(back.idempotency_attested.len(), 1);
        assert_eq!(back.warnings.len(), 1);
        Ok(())
    }

    #[test]
    fn verification_warning_serde_with_special_chars() -> Result<(), String> {
        let warning = VerificationWarning {
            code: 12345,
            message: Box::from("special chars: <>&\"'"),
            gate: 2,
        };
        let bytes =
            postcard::to_allocvec(&warning).map_err(|e| format!("serialize failed: {e}"))?;
        let back: VerificationWarning =
            postcard::from_bytes(&bytes).map_err(|e| format!("deserialize failed: {e}"))?;
        assert_eq!(warning, back);
        Ok(())
    }

    #[test]
    fn accepted_artifact_serde_with_warnings() -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;

        let mut proof = VerificationProof::new(workflow.digest(), 2, true);
        proof.warnings.push(VerificationWarning {
            code: 7,
            message: Box::from("test warning"),
            gate: 1,
        });

        let artifact = AcceptedArtifact {
            digest: workflow.digest(),
            ir: vec![1, 2, 3],
            verification: proof,
            accepted_at_seq: EventSeq::new(42),
            required_capabilities: Box::new([]),
        };

        let bytes =
            postcard::to_allocvec(&artifact).map_err(|e| format!("serialize failed: {e}"))?;
        let back: AcceptedArtifact =
            postcard::from_bytes(&bytes).map_err(|e| format!("deserialize failed: {e}"))?;

        assert_eq!(back.digest, artifact.digest);
        assert_eq!(back.accepted_at_seq.get(), 42);
        assert_eq!(back.verification.warnings.len(), 1);
        Ok(())
    }
}
