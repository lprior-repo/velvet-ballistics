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
            // Legacy or relaxed admission: no actual policy identity bound.
            // Accept as historical record; cannot authorize live effects.
            Ok(())
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
        return Ok(());
    };
    let required = vb_core::AggregateResourceBudget::from_workflow(workflow)
        .map_err(|_| cannot_resume_error("admission_budget_invalid"))?;
    validate_recorded_budget_limits(&recorded, &required)
}

fn validate_recorded_budget_limits(
    recorded: &vb_core::AggregateResourceBudget,
    required: &vb_core::AggregateResourceBudget,
) -> RuntimeResult<()> {
    let vb_core::AggregateResourceBudget {
        max_steps_executable,
        max_action_tickets,
        max_parallel_in_flight,
        max_retries_per_action,
        max_gather_pages,
        max_gather_items,
        max_for_each_iterations,
        max_together_branches,
        max_repeat_attempts,
        max_run_time_seconds,
        max_result_bytes,
        max_total_slots_written,
        max_timer_entries,
        max_trace_events,
        max_queue_depth,
        max_journal_batch_bytes,
        max_ipc_payload_bytes,
        max_blob_bytes,
        max_input_bytes,
        max_step_budget_per_tick,
        max_transitions_per_tick,
        max_open_gates,
        max_signal_subscriptions,
        max_incident_emissions,
        max_transitions,
        max_journal_bytes_total,
        max_value_store_entries,
        max_cost_units,
    } = required;
    if [
        *max_steps_executable <= recorded.max_steps_executable,
        *max_action_tickets <= recorded.max_action_tickets,
        *max_parallel_in_flight <= recorded.max_parallel_in_flight,
        *max_retries_per_action <= recorded.max_retries_per_action,
        *max_gather_pages <= recorded.max_gather_pages,
        *max_gather_items <= recorded.max_gather_items,
        *max_for_each_iterations <= recorded.max_for_each_iterations,
        *max_together_branches <= recorded.max_together_branches,
        *max_repeat_attempts <= recorded.max_repeat_attempts,
        *max_run_time_seconds <= recorded.max_run_time_seconds,
        *max_result_bytes <= recorded.max_result_bytes,
        *max_total_slots_written <= recorded.max_total_slots_written,
        *max_timer_entries <= recorded.max_timer_entries,
        *max_trace_events <= recorded.max_trace_events,
        *max_queue_depth <= recorded.max_queue_depth,
        *max_journal_batch_bytes <= recorded.max_journal_batch_bytes,
        *max_ipc_payload_bytes <= recorded.max_ipc_payload_bytes,
        *max_blob_bytes <= recorded.max_blob_bytes,
        *max_input_bytes <= recorded.max_input_bytes,
        *max_step_budget_per_tick <= recorded.max_step_budget_per_tick,
        *max_transitions_per_tick <= recorded.max_transitions_per_tick,
        *max_open_gates <= recorded.max_open_gates,
        *max_signal_subscriptions <= recorded.max_signal_subscriptions,
        *max_incident_emissions <= recorded.max_incident_emissions,
        *max_transitions <= recorded.max_transitions,
        *max_journal_bytes_total <= recorded.max_journal_bytes_total,
        *max_value_store_entries <= recorded.max_value_store_entries,
        *max_cost_units <= recorded.max_cost_units,
    ]
    .into_iter()
    .all(core::convert::identity)
    {
        Ok(())
    } else {
        Err(cannot_resume_error("admission_budget_insufficient"))
    }
}

#[cfg(test)]
#[path = "admission_tests.rs"]
mod admission_tests;
