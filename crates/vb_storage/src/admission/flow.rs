#![forbid(unsafe_code)]
//! Admission orchestration: policy dispatch and artifact submission flows.

use crate::error::JournalError;
use crate::journal::FjallJournal;
use crate::types::EventSeq;

use super::bytes::validate_workflow_artifact_bytes;
use super::contracts::{AdmissionInputs, IdempotencyEvidence};
use super::persistence::{persist_accepted_artifact_ir, verify_persisted_artifact_present};
use super::policy::ADMISSION_GATE_COUNT;
use super::types::{AcceptedArtifact, Durability, VerificationProof};

/// Validates, verifies, and persists a compiled workflow artifact with policy-controlled durability.
///
/// This is the full admission flow. It performs:
/// 1. Policy check: Relaxed is rejected when accepted artifacts are required.
/// 2. Structure validation: re-parse the workflow from serialized parts.
/// 3. Checksum validation: serialized bytes must hash to the claimed digest.
/// 4. Proof validation: gate count must be 15 and all proof flags must be true.
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
    let admission_inputs = super::contracts::admission_inputs_from_contracts(action_contracts)?;
    submit_artifact_for_policy(journal, workflow, policy, admission_inputs)
}

fn submit_artifact_for_policy(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
    admission_inputs: AdmissionInputs,
) -> Result<AcceptedArtifact, JournalError> {
    let artifact = match policy {
        vb_core::RuntimePolicy::Relaxed => submit_relaxed_artifact_with_evidence(
            journal,
            workflow,
            admission_inputs.required_capabilities,
            &admission_inputs.idempotency_evidence,
        )?,
        vb_core::RuntimePolicy::Journaled | vb_core::RuntimePolicy::Strict => {
            submit_checked_artifact_with_evidence(
                journal,
                workflow,
                policy,
                admission_inputs.required_capabilities,
                admission_inputs.idempotency_evidence,
            )?
        }
        // `RuntimePolicy` is `#[non_exhaustive]`; unknown variants
        // fail closed rather than silently accept malformed artifacts.
        _ => return Err(JournalError::ArtifactMalformed),
    };
    // Live read-back applies to every policy. A write that returned `Ok`
    // but is not immediately readable indicates an Fjall/Lsm anomaly or
    // corruption and must surface regardless of durability mode.
    verify_persisted_artifact_present(journal, workflow.digest())?;
    Ok(artifact)
}

fn submit_relaxed_artifact_with_evidence(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    required_capabilities: Box<[vb_core::capability::Capability]>,
    idempotency_evidence: &IdempotencyEvidence,
) -> Result<AcceptedArtifact, JournalError> {
    let ir_bytes = validate_workflow_artifact_bytes(workflow)?;
    let mut proof = VerificationProof::new_volatile(workflow.digest(), 0);
    proof.idempotency_keyed = idempotency_evidence.keyed.clone();
    proof.idempotency_attested = idempotency_evidence.attested.clone();
    let artifact = accepted_artifact(workflow, ir_bytes, proof, required_capabilities)?;
    persist_accepted_artifact_ir(journal, &artifact)?;
    Ok(artifact)
}

fn submit_checked_artifact_with_evidence(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
    required_capabilities: Box<[vb_core::capability::Capability]>,
    idempotency_evidence: IdempotencyEvidence,
) -> Result<AcceptedArtifact, JournalError> {
    let ir_bytes = validate_workflow_artifact_bytes(workflow)?;
    let durable = policy == vb_core::RuntimePolicy::Strict;
    let durability = Durability::from(durable);
    let mut proof = VerificationProof::new(workflow.digest(), ADMISSION_GATE_COUNT, durability);
    proof.idempotency_keyed = idempotency_evidence.keyed;
    proof.idempotency_attested = idempotency_evidence.attested;
    let artifact = accepted_artifact(workflow, ir_bytes, proof, required_capabilities)?;
    persist_accepted_artifact_ir(journal, &artifact)?;
    if durable {
        journal.persist_strict()?;
    }
    Ok(artifact)
}

fn accepted_artifact(
    workflow: &vb_core::CompiledWorkflow,
    ir: Vec<u8>,
    verification: VerificationProof,
    required_capabilities: Box<[vb_core::capability::Capability]>,
) -> Result<AcceptedArtifact, JournalError> {
    Ok(AcceptedArtifact {
        digest: workflow.digest(),
        source_digest: workflow.digest(),
        policy_digest: super::policy::compute_policy_digest(workflow)?,
        ir,
        verification,
        accepted_at_seq: EventSeq::new(0),
        required_capabilities,
    })
}
