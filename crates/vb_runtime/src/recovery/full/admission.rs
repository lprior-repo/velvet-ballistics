use vb_core::action::ActionContract;
use vb_core::ids::{ActionId, RunId, WorkflowDigest};
use vb_core::{CapabilitySet, CompiledWorkflow, RuntimePolicy};

use crate::{RuntimeError, RuntimeResult};

pub(crate) struct RecoveredAdmissionContext {
    pub(crate) run: RunId,
    pub(crate) expected_workflow: WorkflowDigest,
    pub(crate) runtime_policy: RuntimePolicy,
}

pub(crate) struct RecoveredAdmissionEvidence<'a> {
    pub(crate) context: RecoveredAdmissionContext,
    pub(crate) admission: &'a crate::admission::RunAdmission,
    pub(crate) artifact: &'a vb_storage::admission::AcceptedArtifact,
    pub(crate) workflow: &'a CompiledWorkflow,
}

pub(crate) fn action_abi_digests_from_contracts(
    contracts: &[ActionContract],
) -> RuntimeResult<Box<[(ActionId, WorkflowDigest)]>> {
    let mut digests = Vec::new();
    digests
        .try_reserve(contracts.len())
        .map_err(|_| cannot_resume_error("action_abi_digests_missing"))?;
    for contract in contracts {
        digests.push((contract.id, action_contract_abi_digest(contract)?));
    }
    Ok(digests.into_boxed_slice())
}

fn action_contract_abi_digest(contract: &ActionContract) -> RuntimeResult<WorkflowDigest> {
    let bytes = postcard::to_allocvec(contract)
        .map_err(|_| cannot_resume_error("action_abi_digests_missing"))?;
    Ok(WorkflowDigest::from_bytes(blake3::hash(&bytes).into()))
}

pub(crate) fn validate_recovered_admission_evidence(
    evidence: RecoveredAdmissionEvidence<'_>,
) -> RuntimeResult<()> {
    if evidence.admission.run_id() != evidence.context.run {
        return Err(cannot_resume_error("admission_run_mismatch"));
    }
    if evidence.admission.policy() != evidence.context.runtime_policy {
        return Err(cannot_resume_error("admission_policy_mismatch"));
    }
    if evidence.artifact.digest != evidence.admission.artifact_digest() {
        return Err(cannot_resume_error("artifact_digest_mismatch"));
    }
    if evidence.artifact.source_digest != evidence.context.expected_workflow
        || evidence.workflow.digest() != evidence.context.expected_workflow
    {
        return Err(cannot_resume_error("workflow_digest_mismatch"));
    }
    validate_recovered_policy_digest(evidence.artifact, evidence.workflow)?;
    validate_recovered_capabilities(evidence.admission.granted_capabilities(), evidence.artifact)
}

fn validate_recovered_policy_digest(
    artifact: &vb_storage::admission::AcceptedArtifact,
    workflow: &CompiledWorkflow,
) -> RuntimeResult<()> {
    let expected = vb_storage::admission::compute_policy_digest(workflow)
        .map_err(|_| cannot_resume_error("admission_policy_mismatch"))?;
    if artifact.policy_digest == expected {
        Ok(())
    } else {
        Err(cannot_resume_error("admission_policy_mismatch"))
    }
}

fn validate_recovered_capabilities(
    granted: &CapabilitySet,
    artifact: &vb_storage::admission::AcceptedArtifact,
) -> RuntimeResult<()> {
    if artifact.required_capabilities.len() != granted.len() {
        return Err(cannot_resume_error("admission_capabilities_mismatch"));
    }
    for required in artifact.required_capabilities.iter() {
        if !granted.grants(required) {
            return Err(cannot_resume_error("admission_capabilities_mismatch"));
        }
    }
    Ok(())
}

fn cannot_resume_error(reason: &'static str) -> RuntimeError {
    RuntimeError::RecoveryCannotResume {
        reason: String::from(reason),
    }
}
