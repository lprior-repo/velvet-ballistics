//! Independent cold-path IR verifier. No compiler or source-language dependency.

use super::*;
use crate::compiled_ir::{decode_compiled_ir_validated, encode_compiled_ir};
use vb_core::artifact::{
    ArtifactEvidence, ArtifactVerificationEvidence, AssumptionId, ObligationClosure,
    ObligationKind, VerificationLedgerEntry,
};
use vb_core::{CompiledWorkflow, RuntimePolicy, WorkflowDigest, WorkflowParts};

pub(crate) const EVIDENCE_VERSION: u16 = 3;

#[path = "verification/executor_profile.rs"]
mod executor_profile;
pub(crate) use executor_profile::ExecutorProfile;

#[path = "verification/ledger.rs"]
mod ledger;
use ledger::{effect_closure, obligation_ledger};

#[path = "verification/versioned.rs"]
mod versioned;
pub(crate) use versioned::{prepare_for_version, required_ir_version};

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
    prepare_for_version(
        workflow,
        policy,
        contracts,
        profile,
        versioned::writer_version(workflow),
    )
}

/// Recompute checks and evidence from hostile artifact bytes before runtime use.
/// Legacy claims require regeneration; a rehashed forged ledger is still rejected.
pub fn verify_accepted_artifact(
    artifact: &AcceptedArtifact,
) -> Result<CompiledWorkflow, JournalError> {
    let evidence = versioned::checked_artifact(artifact)?;
    let (workflow, expected) = decode_compiled_ir_validated(
        &artifact.ir,
        Some(artifact.artifact_version),
        |workflow| {
            prepare_for_version(
                workflow,
                evidence.admission_policy,
                &artifact.action_contracts,
                ExecutorProfile::Legacy,
                artifact.artifact_version,
            )
        },
        |error| error,
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
        ArtifactEvidence::Unverified => Err(classify_evidence_held(1)),
        ArtifactEvidence::Verified(evidence) if evidence.schema_version == EVIDENCE_VERSION => {
            Ok(evidence)
        }
        ArtifactEvidence::Verified(evidence) => {
            Err(classify_evidence_held(evidence.schema_version))
        }
    }
}

/// Routes one evidence schema version through the §77.17 algebra at the
/// [`crate::compat::CompatBoundary::ArtifactEnvelope`] boundary. Only
/// the known legacy lines (the v1 `Unverified` era and v2) carry a
/// migration path, and the path is explicit: the caller sees
/// `MigrationRequired` and must regenerate the evidence, never a silent
/// default substitution. Anything else unknown fails closed as
/// `Rejected`. No replay allowance is declared on this path, so the
/// wildcard arm fails `ReplayOnly` closed with every unknown future
/// class.
pub(crate) fn classify_evidence_held(held: u16) -> JournalError {
    let inputs = crate::compat::wire_version_inputs(
        crate::compat::CompatBoundary::ArtifactEnvelope,
        held,
        EVIDENCE_VERSION,
        held == 1 || held == 2,
        false,
    );
    match crate::compat::classify(inputs).class {
        crate::compat::CompatibilityClass::MigrationRequired => JournalError::MigrationRequired {
            from: held,
            to: EVIDENCE_VERSION,
        },
        _ => JournalError::UnsupportedSchemaVersion { version: held },
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
    let parts = workflow.to_parts();
    CompiledWorkflow::try_from_parts(parts.clone())
        .map_err(JournalError::WorkflowReconstruction)?;
    vb_validate::shared::validate_with_contracts(&parts, contracts)
        .map_err(JournalError::ArtifactValidation)?;
    vb_core::action::validate_action_registry(contracts)
        .map_err(JournalError::ArtifactActionRegistry)?;
    validate_idempotency_contracts(contracts)?;
    profile.check(policy, contracts)?;
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

fn digest(bytes: &[u8]) -> WorkflowDigest {
    WorkflowDigest::from_bytes(blake3::hash(bytes).into())
}

fn verifier_digest() -> WorkflowDigest {
    // Fingerprints verifier entrypoints, not a signed compiler/build identity.
    let mut hasher = blake3::Hasher::new();
    hasher.update(include_bytes!("verification.rs"));
    hasher.update(include_bytes!("verification/ledger.rs"));
    hasher.update(include_bytes!("verification/versioned.rs"));
    hasher.update(include_bytes!("../compiled_ir.rs"));
    hasher.update(include_bytes!("../../../vb_core/src/workflow/mod.rs"));
    hasher.update(include_bytes!("../../../vb_core/src/budget.rs"));
    hasher.update(include_bytes!("../../../vb_core/src/action/registry.rs"));
    hasher.update(include_bytes!("../../../vb_validate/src/shared.rs"));
    validation_fingerprint(&mut hasher);
    WorkflowDigest::from_bytes(hasher.finalize().into())
}

fn validation_fingerprint(hasher: &mut blake3::Hasher) {
    hasher.update(include_bytes!("../../../vb_validate/src/gates/mod.rs"));
    hasher.update(include_bytes!("../../../vb_validate/src/gates/gate_07.rs"));
    hasher.update(include_bytes!("../../../vb_validate/src/gates/gate_08.rs"));
    hasher.update(include_bytes!("../../../vb_validate/src/gates/gate_09.rs"));
    hasher.update(include_bytes!("../../../vb_validate/src/gates/gate_10.rs"));
    hasher.update(include_bytes!(
        "../../../vb_validate/src/gates/gate_10_refs.rs"
    ));
    hasher.update(include_bytes!("../../../vb_validate/src/gates/gate_11.rs"));
    hasher.update(include_bytes!(
        "../../../vb_validate/src/gates/gate_11_spans.rs"
    ));
    hasher.update(include_bytes!("../../../vb_validate/src/gates/gate_12.rs"));
    hasher.update(include_bytes!("../../../vb_validate/src/gates/gate_13.rs"));
    hasher.update(include_bytes!("../../../vb_validate/src/gates/gate_14.rs"));
    hasher.update(include_bytes!("../../../vb_validate/src/gates/gate_15.rs"));
}
