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

pub(crate) fn action_contract_abi_digest(
    contract: &ActionContract,
) -> RuntimeResult<WorkflowDigest> {
    let bytes = postcard::to_allocvec(contract)
        .map_err(|_| cannot_resume_error("action_abi_digests_missing"))?;
    Ok(WorkflowDigest::from_bytes(blake3::hash(&bytes).into()))
}

pub(crate) fn validate_recovered_admission_evidence(
    evidence: RecoveredAdmissionEvidence<'_>,
) -> RuntimeResult<()> {
    validate_current_admission(evidence.admission)?;
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
    validate_recovered_policy_pin(evidence.admission, evidence.artifact)?;
    validate_recovered_capabilities(evidence.admission.granted_capabilities(), evidence.artifact)?;
    validate_recovered_budget(evidence.admission, evidence.workflow)
}

pub(crate) fn validate_current_admission(
    admission: &crate::admission::RunAdmission,
) -> RuntimeResult<()> {
    if admission.budget().is_none() {
        return Err(cannot_resume_error("admission_budget_missing"));
    }
    if !matches!(
        admission.input_digest(),
        vb_storage::RunInputBinding::Digest(_)
    ) {
        return Err(cannot_resume_error("admission_input_binding_missing"));
    }
    if !matches!(admission.policy_pin(), vb_storage::RunPolicyPin::Pinned(_)) {
        return Err(cannot_resume_error("admission_policy_pin_missing"));
    }
    Ok(())
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

pub(crate) fn validate_recovered_policy_pin(
    admission: &crate::admission::RunAdmission,
    artifact: &vb_storage::admission::AcceptedArtifact,
) -> RuntimeResult<()> {
    match admission.policy_pin() {
        vb_storage::RunPolicyPin::Unverified => {
            Err(cannot_resume_error("admission_policy_pin_missing"))
        }
        vb_storage::RunPolicyPin::Pinned(pin_digest) => {
            // Actual policy identity was bound. Validate it matches the artifact.
            if pin_digest == artifact.policy_digest {
                Ok(())
            } else {
                Err(cannot_resume_error("admission_policy_pin_mismatch"))
            }
        }
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

fn validate_recovered_budget(
    admission: &crate::admission::RunAdmission,
    workflow: &CompiledWorkflow,
) -> RuntimeResult<()> {
    let Some(recorded) = admission.budget() else {
        return Err(cannot_resume_error("admission_budget_missing"));
    };
    let required = vb_core::AggregateResourceBudget::from_workflow(workflow)
        .map_err(|_| cannot_resume_error("admission_budget_invalid"))?;
    validate_recorded_budget_limits(&recorded, &required)
}

fn validate_recorded_budget_limits(
    recorded: &vb_core::AggregateResourceBudget,
    required: &vb_core::AggregateResourceBudget,
) -> RuntimeResult<()> {
    if recorded.max_run_time_seconds == 0
        || recorded.max_run_time_seconds > required.max_run_time_seconds
    {
        return Err(cannot_resume_error("admission_budget_lifetime_invalid"));
    }
    if execution_budget_covers(recorded, required)
        && storage_budget_covers(recorded, required)
        && runtime_budget_covers(recorded, required)
    {
        Ok(())
    } else {
        Err(cannot_resume_error("admission_budget_insufficient"))
    }
}

fn execution_budget_covers(
    recorded: &vb_core::AggregateResourceBudget,
    required: &vb_core::AggregateResourceBudget,
) -> bool {
    required.max_steps_executable <= recorded.max_steps_executable
        && required.max_action_tickets <= recorded.max_action_tickets
        && required.max_parallel_in_flight <= recorded.max_parallel_in_flight
        && required.max_retries_per_action <= recorded.max_retries_per_action
        && required.max_gather_pages <= recorded.max_gather_pages
        && required.max_gather_items <= recorded.max_gather_items
        && required.max_for_each_iterations <= recorded.max_for_each_iterations
        && required.max_together_branches <= recorded.max_together_branches
        && required.max_repeat_attempts <= recorded.max_repeat_attempts
}

fn storage_budget_covers(
    recorded: &vb_core::AggregateResourceBudget,
    required: &vb_core::AggregateResourceBudget,
) -> bool {
    required.max_result_bytes <= recorded.max_result_bytes
        && required.max_total_slots_written <= recorded.max_total_slots_written
        && required.max_timer_entries <= recorded.max_timer_entries
        && required.max_trace_events <= recorded.max_trace_events
        && required.max_queue_depth <= recorded.max_queue_depth
        && required.max_journal_batch_bytes <= recorded.max_journal_batch_bytes
        && required.max_ipc_payload_bytes <= recorded.max_ipc_payload_bytes
        && required.max_blob_bytes <= recorded.max_blob_bytes
        && required.max_input_bytes <= recorded.max_input_bytes
}

fn runtime_budget_covers(
    recorded: &vb_core::AggregateResourceBudget,
    required: &vb_core::AggregateResourceBudget,
) -> bool {
    required.max_step_budget_per_tick <= recorded.max_step_budget_per_tick
        && required.max_transitions_per_tick <= recorded.max_transitions_per_tick
        && required.max_open_gates <= recorded.max_open_gates
        && required.max_signal_subscriptions <= recorded.max_signal_subscriptions
        && required.max_incident_emissions <= recorded.max_incident_emissions
        && required.max_transitions <= recorded.max_transitions
        && required.max_journal_bytes_total <= recorded.max_journal_bytes_total
        && required.max_value_store_entries <= recorded.max_value_store_entries
        && required.max_cost_units <= recorded.max_cost_units
}

#[cfg(test)]
#[path = "admission_tests.rs"]
mod admission_tests;
