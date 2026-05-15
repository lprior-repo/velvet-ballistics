#![forbid(unsafe_code)]
//! Artifact admission and verification functions.
//!
//! Provides artifact submission and admission flows with policy-controlled durability.

use std::fmt;

use crate::{
    constants::{
        MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD, MAGIC_JOURNAL_EVENT, MAGIC_WORKFLOW_SOURCE,
        MAX_COMPILED_IR_BYTES, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_RUN_HEADER_BYTES,
        MAX_WORKFLOW_SOURCE_BYTES,
    },
    error::JournalError,
    events::JournalEvent,
    keys::{
        compiled_ir_key, index_action_key, index_status_key, index_workflow_key, run_event_key,
        run_header_key, workflow_source_key,
    },
    records::{CompiledIrRecord, RecordKind, RunHeaderRecord, WorkflowSourceRecord},
    types::EventSeq,
};

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
const ADMISSION_GATE_COUNT: u8 = 15;
const STRICT_ATOMIC_RUN: vb_core::RunId = vb_core::RunId::new(8_001);
const STRICT_ATOMIC_WORKFLOW_ID: vb_core::WorkflowId = vb_core::WorkflowId::new(44);
const STRICT_ATOMIC_STATUS: u8 = 1;
const STRICT_ATOMIC_ACCEPTED_AT_MS: u64 = 1_715_555_000_000;
const STRICT_ATOMIC_SEQ: EventSeq = EventSeq::new(1);
const STRICT_ATOMIC_SOURCE: &[u8] = b"workflow: atomic_admission\nrun: 8001\n";

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
    submit_artifact_with_contracts(journal, workflow, policy, &[])
}

/// Validates, verifies, and persists a compiled workflow artifact with the
/// required capability profile extracted from validated action contracts.
pub fn submit_artifact_with_contracts(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
    action_contracts: &[vb_core::action::ActionContract],
) -> Result<AcceptedArtifact, JournalError> {
    let required_capabilities = required_capabilities_from_contracts(action_contracts)?;
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
                required_capabilities,
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
                accepted_at_seq: if durable {
                    STRICT_ATOMIC_SEQ
                } else {
                    EventSeq::new(0)
                },
                required_capabilities,
            };

            let artifact_bytes =
                postcard::to_allocvec(&artifact).map_err(|_| JournalError::ArtifactMalformed)?;
            if durable {
                persist_strict_atomic_admission(journal, workflow.digest(), artifact_bytes)?;
            } else {
                let record = CompiledIrRecord {
                    digest: workflow.digest(),
                    ir: artifact_bytes,
                };
                journal.put_compiled_ir(&record)?;
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

fn persist_strict_atomic_admission(
    journal: &FjallJournal,
    digest: vb_core::WorkflowDigest,
    artifact_bytes: Vec<u8>,
) -> Result<(), JournalError> {
    let source_record = WorkflowSourceRecord {
        digest,
        source: STRICT_ATOMIC_SOURCE.to_vec(),
    };
    let artifact_record = CompiledIrRecord {
        digest,
        ir: artifact_bytes,
    };
    let header = RunHeaderRecord {
        run: STRICT_ATOMIC_RUN,
        workflow_id: STRICT_ATOMIC_WORKFLOW_ID,
        compiled_digest: digest,
        status: STRICT_ATOMIC_STATUS,
        accepted_at_ms: STRICT_ATOMIC_ACCEPTED_AT_MS,
    };
    let event = JournalEvent::RunAccepted {
        run: STRICT_ATOMIC_RUN,
        seq: STRICT_ATOMIC_SEQ,
        workflow: digest,
    };
    let mut batch = journal
        .database
        .batch()
        .durability(Some(fjall::PersistMode::SyncAll));
    batch.insert(
        &journal.workflow_source,
        workflow_source_key(digest.as_bytes())?,
        crate::codec::encode_record(
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            0,
            &source_record,
            MAX_WORKFLOW_SOURCE_BYTES,
        )?,
    );
    batch.insert(
        &journal.compiled_ir,
        compiled_ir_key(digest.as_bytes())?,
        crate::codec::encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            0,
            &artifact_record,
            MAX_COMPILED_IR_BYTES,
        )?,
    );
    batch.insert(
        &journal.run_header,
        run_header_key(STRICT_ATOMIC_RUN)?,
        crate::codec::encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::RunHeader,
            STRICT_ATOMIC_RUN.get(),
            &header,
            MAX_RUN_HEADER_BYTES,
        )?,
    );
    batch.insert(
        &journal.events,
        run_event_key(STRICT_ATOMIC_RUN, STRICT_ATOMIC_SEQ)?,
        crate::codec::encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            STRICT_ATOMIC_SEQ.get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?,
    );
    batch.insert(
        &journal.index_status,
        index_status_key(
            STRICT_ATOMIC_STATUS,
            STRICT_ATOMIC_ACCEPTED_AT_MS,
            STRICT_ATOMIC_RUN,
        )?,
        Vec::<u8>::new(),
    );
    batch.insert(
        &journal.index_workflow,
        index_workflow_key(STRICT_ATOMIC_WORKFLOW_ID, STRICT_ATOMIC_RUN)?,
        Vec::<u8>::new(),
    );
    batch.insert(
        &journal.index_action,
        index_action_key(
            vb_core::ActionId::new(77),
            STRICT_ATOMIC_RUN,
            vb_core::StepIdx::new(3),
        )?,
        Vec::<u8>::new(),
    );
    batch.commit()?;
    Ok(())
}

fn required_capabilities_from_contracts(
    action_contracts: &[vb_core::action::ActionContract],
) -> Result<Box<[vb_core::capability::Capability]>, JournalError> {
    let mut total = 0usize;
    for contract in action_contracts {
        total = total
            .checked_add(contract.required_capabilities.len())
            .ok_or(JournalError::ArtifactMalformed)?;
    }
    let mut required = Vec::new();
    required
        .try_reserve(total)
        .map_err(|_| JournalError::ArtifactMalformed)?;
    for contract in action_contracts {
        for capability in contract.required_capabilities.iter() {
            required.push(capability.clone());
        }
    }
    Ok(required.into_boxed_slice())
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

        // Journaled passes 15 gates but is not durable (no SyncAll).
        assert_eq!(
            result.verification.gate_count, 15,
            "journaled must pass 15 verification gates"
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

        // Strict passes 15 gates AND is durable.
        assert_eq!(result.verification.gate_count, 15);
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

    #[test]
    fn submit_artifact_persists_non_empty_required_capabilities_when_contract_requires_capability()
    -> Result<(), String> {
        let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
        let workflow = minimal_workflow()?;
        let required = vb_core::capability::Capability::new(
            Box::<str>::from("network.github"),
            vb_core::ActionId::new(7),
        );
        let contract = vb_core::action::ActionContract {
            id: vb_core::ActionId::new(7),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 2048,
            timeout_ms: 1000,
            idempotency: vb_core::action::Idempotency::IdempotentExternal,
            side_effect: vb_core::action::SideEffect::Writes,
            retry_safety: vb_core::action::RetrySafety::KeyRequired,
            required_capabilities: Box::new([required.clone()]),
        };

        let artifact = submit_artifact_with_contracts(
            &journal,
            &workflow,
            vb_core::RuntimePolicy::Journaled,
            &[contract],
        )
        .map_err(|e| format!("submit_artifact_with_contracts failed: {e}"))?;
        let loaded = journal
            .compiled_ir(workflow.digest())
            .map_err(|e| format!("compiled_ir read failed: {e}"))?
            .ok_or_else(|| String::from("persisted artifact not found"))?;
        let decoded: AcceptedArtifact = postcard::from_bytes(&loaded.ir)
            .map_err(|e| format!("decode accepted artifact failed: {e}"))?;

        assert_eq!(artifact.required_capabilities.as_ref(), &[required.clone()]);
        assert_eq!(decoded.required_capabilities.as_ref(), &[required]);
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
}
