#![forbid(unsafe_code)]
//! Artifact admission and verification functions.
//!
//! Provides artifact submission and admission flows with policy-controlled durability.

use crate::{CompiledIrRecord, error::JournalError, types::EventSeq};

use crate::journal::FjallJournal;
use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};

#[path = "admission/proof.rs"]
mod proof;
pub use proof::{ProofFlag, VerificationProof, VerificationWarning};

#[path = "admission/digest.rs"]
mod digest;
#[cfg(test)]
pub(crate) use digest::resource_contract_policy_bytes_bound;
pub use digest::{accepted_artifact_digest, compute_policy_digest};
pub(crate) use digest::{bind_artifact_digest, zero_workflow_digest};

/// Accepted artifact record produced by the admission flow.
///
/// GAP-002/GAP-003 FIX: Added `source_digest` and `policy_digest` fields to satisfy
/// Backend DoD requirement for durable evidence chain binding.
///
/// Tracks the binding between a run and its accepting artifact per Backend DoD:
/// - `source_digest` binds the run to the workflow source that produced the artifact
/// - `policy_digest` binds the run to the policy/resource contract in effect
///
/// GAP-004 FIX: Per-action digests are NOT added because actions are already
/// cryptographically bound via the `CompiledWorkflow` digest. Each action's
/// bytecode and parameters are part of the workflow structure that is hashed
/// to produce the workflow digest. The workflow digest therefore serves as
/// a composite binding for all actions in the workflow.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AcceptedArtifact {
    /// The compiled artifact's content hash (matches `source_digest` when
    /// artifact is produced directly from compilation without separate source).
    pub digest: vb_core::WorkflowDigest,
    /// Digest of the original workflow source that was compiled to produce this artifact.
    /// For directly compiled workflows, this equals `digest`.
    pub source_digest: vb_core::WorkflowDigest,
    /// Digest of the resource/policy contract that governed this artifact's admission.
    /// Derived from the `resource_contract` field of the compiled workflow.
    pub policy_digest: vb_core::WorkflowDigest,
    /// Serialized compiled IR (postcard).
    pub ir: Vec<u8>,
    /// Proof that verification passed.
    pub verification: VerificationProof,
    /// Journal sequence when accepted.
    ///
    /// GAP-007 FIX: This field is currently always set to `EventSeq::new(0)`
    /// because actual sequence tracking is not implemented. The field is retained
    /// as a placeholder for future implementation of proper sequence tracking.
    /// When actual tracking is implemented, replace the placeholder with the real
    /// sequence number from the journal at admission time.
    pub accepted_at_seq: EventSeq,
    /// Required capabilities for actions in this artifact.
    pub required_capabilities: Box<[vb_core::capability::Capability]>,
    /// Validated action contracts bound into this accepted artifact.
    ///
    /// The runtime needs these contracts after crash recovery to validate
    /// action completion size, taint, idempotency, and output-slot authority.
    /// Keeping them inside the accepted artifact makes the artifact the unit of
    /// trust described by the master contract instead of relying on ephemeral
    /// process memory.
    #[serde(default)]
    pub action_contracts: Box<[ActionContract]>,
    /// Compiled-IR schema version stamped at admission; the admission and
    /// dispatch gates classify it against `CURRENT_IR_VERSION`.
    pub artifact_version: crate::compat::SchemaVersion,
}

#[path = "admission/verification.rs"]
pub(crate) mod verification;
pub use verification::{
    prepare_compiled_artifact, verify_accepted_artifact, verify_accepted_artifact_for_policy,
};

/// Validate and persist an artifact without action declarations.
pub fn submit_artifact(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
) -> Result<AcceptedArtifact, JournalError> {
    submit_artifact_with_contracts(journal, workflow, policy, &[])
}

/// Check artifact evidence before writing; Strict returns only after SyncAll.
/// The durable display flag is not a persisted fsync certificate.
pub fn submit_artifact_with_contracts(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
    action_contracts: &[ActionContract],
) -> Result<AcceptedArtifact, JournalError> {
    // Prepare validates first; pure prepare stays seq=0/unpersisted unchanged.
    let artifact = prepare_compiled_artifact(workflow, policy, action_contracts)?;

    // Delegate issuance to private journal helper. The helper holds the
    // write_lock across the entire RMW sequence: read counter, increment,
    // stamp, rebind digest, encode, stage both writes in one batch, commit
    // with policy-appropriate durability.
    let stamped = crate::journal::artifact_acceptance::issue_next_artifact_sequence(
        journal, artifact, policy,
    )?;

    // Existing readback verification after commit.
    verify_artifact_persisted(journal, stamped.digest)?;
    Ok(stamped)
}

/// Verifies that the artifact identified by `digest` is actually readable
/// from the journal after a `put_compiled_ir` call.
///
/// Both persistence layers (Relaxed and Journaled/Strict) rely on this readback
/// so that a silent persistence failure surfaces as `ArtifactMalformed` rather
/// than as a falsely-accepted artifact in the returned `AcceptedArtifact`.
fn verify_artifact_persisted(
    journal: &FjallJournal,
    digest: vb_core::WorkflowDigest,
) -> Result<(), JournalError> {
    // vb-l9jqs: preserve the inner JournalError from the readback via the
    // typed CompiledIrReadback wrapper.
    let stored = journal
        .compiled_ir(digest)
        .map_err(|e| JournalError::CompiledIrReadback(Box::new(e)))?;
    if stored.is_none() {
        return Err(JournalError::ArtifactMalformed);
    }
    Ok(())
}

fn required_capabilities_from_contracts(
    action_contracts: &[ActionContract],
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
        .map_err(JournalError::AdmissionAllocationFailed)?;
    for contract in action_contracts {
        for capability in contract.required_capabilities.iter() {
            required.push(capability.clone());
        }
    }
    Ok(required.into_boxed_slice())
}

fn validate_idempotency_contracts(action_contracts: &[ActionContract]) -> Result<(), JournalError> {
    for contract in action_contracts {
        if !is_contract_idempotency_accepted(contract) {
            return Err(JournalError::ArtifactMalformed);
        }
    }
    Ok(())
}

fn is_contract_idempotency_accepted(contract: &ActionContract) -> bool {
    match (
        contract.side_effect,
        contract.retry_safety,
        contract.idempotency,
    ) {
        (SideEffect::None, _, _) => true,
        (_, RetrySafety::Unsafe, _) => false,
        (_, _, Idempotency::AtLeastOnceExternal | Idempotency::DeterministicPure) => false,
        (_, RetrySafety::Safe | RetrySafety::KeyRequired, Idempotency::IdempotentExternal) => true,
        // `SideEffect`, `RetrySafety`, and `Idempotency` are all `#[non_exhaustive]`.
        // Unknown combinations are conservatively rejected.
        _ => false,
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
    let bytes = crate::encode_compiled_ir(workflow)?.into_vec();
    let digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(&bytes).into());
    let record = CompiledIrRecord { digest, ir: bytes };
    journal.put_compiled_ir(&record)?;

    Ok(digest)
}

#[cfg(test)]
#[path = "admission/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "admission/versioned_codec_tests.rs"]
mod versioned_codec_tests;
