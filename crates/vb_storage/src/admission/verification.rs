//! Independent cold-path IR verifier. No compiler or source-language dependency.

use super::*;
use vb_core::artifact::{
    ArtifactEvidence, ArtifactVerificationEvidence, AssumptionId, ObligationClosure,
    ObligationKind, VerificationLedgerEntry,
};
use vb_core::{CompiledWorkflow, RuntimePolicy, WorkflowDigest, WorkflowParts};

pub(crate) const EVIDENCE_VERSION: u16 = 3;

#[path = "verification/executor_profile.rs"]
mod executor_profile;
pub(crate) use executor_profile::ExecutorProfile;

/// Prepare a checked envelope without writing a journal or claiming a durable ack.
/// External assumptions in the ledger remain assumptions, not executed evidence.
pub fn prepare_compiled_artifact(
    workflow: &CompiledWorkflow,
    policy: RuntimePolicy,
    contracts: &[ActionContract],
) -> Result<AcceptedArtifact, JournalError> {
    prepare_with_profile(workflow, policy, contracts, ExecutorProfile::Legacy)
}

pub(crate) fn prepare_with_profile(
    workflow: &CompiledWorkflow,
    policy: RuntimePolicy,
    contracts: &[ActionContract],
    profile: ExecutorProfile<'_>,
) -> Result<AcceptedArtifact, JournalError> {
    let parts = checked_parts(workflow, policy, contracts, &profile)?;
    let ir = postcard::to_allocvec(&parts).map_err(JournalError::PostcardEncodeFailed)?;
    let evidence = build_evidence(&parts, &ir, policy, contracts)?;
    let mut proof = VerificationProof::new(zero_workflow_digest(), 0, false);
    proof.bounded_claimed = true;
    proof.idempotency_verified_claimed = true;
    proof.evidence = profile.wrap(evidence);
    let mut artifact = AcceptedArtifact {
        digest: zero_workflow_digest(),
        source_digest: workflow.digest(),
        policy_digest: compute_policy_digest(workflow)?,
        ir,
        verification: proof,
        accepted_at_seq: EventSeq::new(0),
        required_capabilities: required_capabilities_from_contracts(contracts)?,
        action_contracts: contracts.to_vec().into_boxed_slice(),
    };
    bind_artifact_digest(&mut artifact)?;
    Ok(artifact)
}

/// Recompute checks and evidence from hostile artifact bytes before runtime use.
/// Legacy claims require regeneration; a rehashed forged ledger is still rejected.
pub fn verify_accepted_artifact(
    artifact: &AcceptedArtifact,
) -> Result<CompiledWorkflow, JournalError> {
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
    let (parts, trailing) = postcard::take_from_bytes::<WorkflowParts>(&artifact.ir)
        .map_err(JournalError::PostcardDecodeFailed)?;
    if !trailing.is_empty() {
        return Err(JournalError::ArtifactMalformed);
    }
    let workflow =
        CompiledWorkflow::try_from_parts(parts).map_err(JournalError::WorkflowReconstruction)?;
    let expected = prepare_compiled_artifact(
        &workflow,
        evidence.admission_policy,
        &artifact.action_contracts,
    )?;
    verify_expected_bindings(artifact, &expected)?;
    Ok(workflow)
}

/// Verify that a caller cannot upgrade a weaker artifact to a stronger policy.
pub fn verify_accepted_artifact_for_policy(
    artifact: &AcceptedArtifact,
    policy: RuntimePolicy,
) -> Result<CompiledWorkflow, JournalError> {
    let evidence = checked_evidence_schema(&artifact.verification.evidence)?;
    if evidence.admission_policy != policy {
        return Err(JournalError::ArtifactMalformed);
    }
    verify_accepted_artifact(artifact)
}

fn checked_evidence_schema(
    evidence: &ArtifactEvidence,
) -> Result<&ArtifactVerificationEvidence, JournalError> {
    match evidence {
        ArtifactEvidence::Packed(_) => Err(JournalError::ArtifactMalformed),
        ArtifactEvidence::Unverified => Err(JournalError::MigrationRequired {
            from: 1,
            to: EVIDENCE_VERSION,
        }),
        ArtifactEvidence::Verified(evidence) if evidence.schema_version == 2 => {
            Err(JournalError::MigrationRequired {
                from: 2,
                to: EVIDENCE_VERSION,
            })
        }
        ArtifactEvidence::Verified(evidence) if evidence.schema_version == EVIDENCE_VERSION => {
            Ok(evidence)
        }
        ArtifactEvidence::Verified(evidence) => Err(JournalError::UnsupportedSchemaVersion {
            version: evidence.schema_version,
        }),
    }
}

pub(crate) fn verify_expected_bindings(
    actual: &AcceptedArtifact,
    expected: &AcceptedArtifact,
) -> Result<(), JournalError> {
    let mut proof = actual.verification.clone();
    proof.digest = expected.verification.digest;
    if actual.source_digest != expected.source_digest
        || actual.policy_digest != expected.policy_digest
        || actual.ir != expected.ir
        || proof != expected.verification
        || actual.required_capabilities != expected.required_capabilities
    {
        return Err(JournalError::ArtifactMalformed);
    }
    Ok(())
}

fn checked_parts(
    workflow: &CompiledWorkflow,
    policy: RuntimePolicy,
    contracts: &[ActionContract],
    profile: &ExecutorProfile<'_>,
) -> Result<WorkflowParts, JournalError> {
    match policy {
        RuntimePolicy::Relaxed | RuntimePolicy::Journaled | RuntimePolicy::Strict => {}
        _ => return Err(JournalError::ArtifactMalformed),
    }
    let mut parts = workflow.to_parts();
    CompiledWorkflow::try_from_parts(parts.clone())
        .map_err(JournalError::WorkflowReconstruction)?;
    vb_validate::shared::validate_with_contracts(&parts, contracts)
        .map_err(JournalError::ArtifactValidation)?;
    vb_core::action::validate_action_registry(contracts)
        .map_err(JournalError::ArtifactActionRegistry)?;
    validate_idempotency_contracts(contracts)?;
    profile.check(policy, contracts)?;
    parts.digest = zero_workflow_digest();
    let canonical = postcard::to_allocvec(&parts).map_err(JournalError::PostcardEncodeFailed)?;
    if digest(&canonical) != workflow.digest() {
        return Err(JournalError::ArtifactChecksumMismatch);
    }
    parts.digest = workflow.digest();
    Ok(parts)
}

fn build_evidence(
    parts: &WorkflowParts,
    ir: &[u8],
    policy: RuntimePolicy,
    contracts: &[ActionContract],
) -> Result<ArtifactVerificationEvidence, JournalError> {
    let policy_bytes = postcard::to_allocvec(&parts.resource_contract)
        .map_err(JournalError::PostcardEncodeFailed)?;
    let contract_bytes =
        postcard::to_allocvec(contracts).map_err(JournalError::PostcardEncodeFailed)?;
    let policy_digest = digest(&policy_bytes);
    let verifier_digest = verifier_digest();
    Ok(ArtifactVerificationEvidence {
        schema_version: EVIDENCE_VERSION,
        payload_digest: digest(ir),
        workflow_digest: parts.digest,
        action_contract_digest: digest(&contract_bytes),
        policy_digest,
        admission_policy: policy,
        verifier_digest,
        ledger: obligation_ledger(
            policy_digest,
            verifier_digest,
            effect_closure(contracts, verifier_digest)?,
        )?,
    })
}

fn effect_closure(
    contracts: &[ActionContract],
    verifier_digest: WorkflowDigest,
) -> Result<ObligationClosure, JournalError> {
    if contracts.is_empty() {
        return Ok(ObligationClosure::RuntimeValidated { verifier_digest });
    }
    let declarations =
        postcard::to_allocvec(contracts).map_err(JournalError::PostcardEncodeFailed)?;
    Ok(ObligationClosure::ExternalAssumption {
        assumption_id: AssumptionId::ExecutorHonorsActionContracts,
        evidence_digest: digest(&declarations),
    })
}

fn obligation_ledger(
    policy_digest: WorkflowDigest,
    verifier: WorkflowDigest,
    effect_safety: ObligationClosure,
) -> Result<[VerificationLedgerEntry; 10], JournalError> {
    use ObligationKind::*;
    let validated = ObligationClosure::RuntimeValidated {
        verifier_digest: verifier,
    };
    let entry = |obligation, closure| VerificationLedgerEntry {
        obligation,
        closure,
        policy_digest,
    };
    Ok([
        entry(Determinism, validated),
        entry(EffectSafety, effect_safety),
        entry(CapabilityNonMinting, validated),
        entry(Boundedness, validated),
        entry(
            ReplayEffectSeparation,
            assumption(AssumptionId::RuntimeReplayEffectSeparation)?,
        ),
        entry(ArtifactValidity, validated),
        entry(IdempotencyClassification, validated),
        entry(
            RecoveryClassification,
            assumption(AssumptionId::RuntimeRecoverySemantics)?,
        ),
        entry(
            TaintHandling,
            assumption(AssumptionId::RuntimeTaintEnforcement)?,
        ),
        entry(PolicyBinding, validated),
    ])
}

fn assumption(assumption_id: AssumptionId) -> Result<ObligationClosure, JournalError> {
    // Identifies this explicit declaration; it is not a verifier or test result.
    let declaration = ("velvet-ballistics/external-assumption/v1", assumption_id);
    let bytes = postcard::to_allocvec(&declaration).map_err(JournalError::PostcardEncodeFailed)?;
    Ok(ObligationClosure::ExternalAssumption {
        assumption_id,
        evidence_digest: digest(&bytes),
    })
}

fn digest(bytes: &[u8]) -> WorkflowDigest {
    WorkflowDigest::from_bytes(blake3::hash(bytes).into())
}

fn verifier_digest() -> WorkflowDigest {
    // Fingerprints verifier entrypoints, not a signed compiler/build identity.
    let mut hasher = blake3::Hasher::new();
    hasher.update(include_bytes!("verification.rs"));
    hasher.update(include_bytes!("../../../vb_core/src/workflow/mod.rs"));
    hasher.update(include_bytes!("../../../vb_core/src/budget.rs"));
    hasher.update(include_bytes!("../../../vb_core/src/action/registry.rs"));
    hasher.update(include_bytes!("../../../vb_validate/src/shared.rs"));
    hasher.update(include_bytes!("../../../vb_validate/src/gates.rs"));
    WorkflowDigest::from_bytes(hasher.finalize().into())
}
