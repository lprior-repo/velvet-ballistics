#![forbid(unsafe_code)]
//! Policy binding and gate count constants for admission.

use crate::error::JournalError;

/// Number of verification gates in the accepted artifact v1 admission flow.
/// This must match `vb_runtime::admission::REQUIRED_GATE_COUNT` (15).
pub(crate) const ADMISSION_GATE_COUNT: u8 = 15;

/// Checks whether a gate count is acceptable.
///
/// Gate count `0` is valid for the relaxed policy; `ADMISSION_GATE_COUNT` (15)
/// is valid for journaled/strict policies.
pub(crate) fn is_accepted_gate_count(gate_count: u8) -> bool {
    gate_count == 0 || gate_count == ADMISSION_GATE_COUNT
}

/// Computes the policy digest from a workflow's resource contract.
///
/// GAP-003 FIX: Added per review finding that `AcceptedArtifact` must bind
/// to the policy digest that governed admission. The policy digest is derived
/// from the resource contract by hashing its canonical serialization.
pub fn compute_policy_digest(
    workflow: &vb_core::CompiledWorkflow,
) -> Result<vb_core::WorkflowDigest, JournalError> {
    let contract_bytes = postcard::to_allocvec(&workflow.resource_contract())
        .map_err(|_| JournalError::ArtifactMalformed)?;
    let hash = blake3::hash(&contract_bytes);
    Ok(vb_core::WorkflowDigest::from_bytes(*hash.as_bytes()))
}
