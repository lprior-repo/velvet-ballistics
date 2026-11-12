use super::*;
use crate::compat::{CURRENT_IR_VERSION, SchemaVersion};

pub(crate) fn required_ir_version(held: SchemaVersion) -> SchemaVersion {
    if held.major == 1 {
        CURRENT_IR_VERSION
    } else {
        SchemaVersion::new(2, 0, 0)
    }
}

pub(super) fn writer_version(workflow: &CompiledWorkflow) -> SchemaVersion {
    if workflow.symbol_payloads().is_some() {
        SchemaVersion::new(2, 0, 0)
    } else {
        CURRENT_IR_VERSION
    }
}

pub(crate) fn prepare_for_version(
    workflow: &CompiledWorkflow,
    policy: RuntimePolicy,
    contracts: &[ActionContract],
    profile: ExecutorProfile<'_>,
    version: SchemaVersion,
) -> Result<AcceptedArtifact, JournalError> {
    let parts = checked_parts(workflow, policy, contracts, &profile)?;
    let ir = encode_compiled_ir(workflow)?.into_vec();
    let evidence = build_evidence(&parts, &ir, policy, contracts)?;
    artifact_record(workflow, contracts, version, ir, proof(evidence, &profile))
}

fn proof(
    evidence: ArtifactVerificationEvidence,
    profile: &ExecutorProfile<'_>,
) -> VerificationProof {
    let mut proof = VerificationProof::new(zero_workflow_digest(), 0, false);
    proof.bounded_claimed = true;
    proof.idempotency_verified_claimed = true;
    proof.evidence = profile.wrap(evidence);
    proof
}

fn artifact_record(
    workflow: &CompiledWorkflow,
    contracts: &[ActionContract],
    version: SchemaVersion,
    ir: Vec<u8>,
    verification: VerificationProof,
) -> Result<AcceptedArtifact, JournalError> {
    let mut artifact = AcceptedArtifact {
        digest: zero_workflow_digest(),
        source_digest: workflow.digest(),
        policy_digest: compute_policy_digest(workflow)?,
        ir,
        verification,
        accepted_at_seq: EventSeq::new(0),
        required_capabilities: required_capabilities_from_contracts(contracts)?,
        action_contracts: contracts.to_vec().into_boxed_slice(),
        artifact_version: version,
    };
    bind_artifact_digest(&mut artifact)?;
    Ok(artifact)
}

pub(super) fn checked_artifact(
    artifact: &AcceptedArtifact,
) -> Result<&ArtifactVerificationEvidence, JournalError> {
    let len = u32::try_from(artifact.ir.len()).map_err(|_| JournalError::ArtifactMalformed)?;
    if len > crate::constants::MAX_COMPILED_IR_BYTES {
        return Err(JournalError::PayloadTooLarge {
            len,
            max: crate::constants::MAX_COMPILED_IR_BYTES,
        });
    }
    let evidence = checked_evidence_schema(&artifact.verification.evidence)?;
    if accepted_artifact_digest(artifact)? != artifact.digest
        || artifact.verification.digest != artifact.digest
    {
        return Err(JournalError::ArtifactChecksumMismatch);
    }
    Ok(evidence)
}
