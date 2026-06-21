#![forbid(unsafe_code)]
//! Policy binding and gate count constants for admission.

use crate::error::JournalError;

/// Number of verification gates in the accepted artifact v1 admission flow.
/// This must match `vb_runtime::admission::REQUIRED_GATE_COUNT` (15).
pub(crate) const ADMISSION_GATE_COUNT: u8 = 15;

#[cfg(not(kani))]
const RESOURCE_CONTRACT_POLICY_BYTES: usize = 128;

/// vb-1rqz7.36 / SA-008: provides a runtime/proof-time witness that the
/// canonical postcard serialization of [`vb_core::ResourceContract`] fits
/// within [`RESOURCE_CONTRACT_POLICY_BYTES`].
///
/// The bound is empirical today: a fully-populated `ResourceContract` with
/// all 17 fields set to their maximum legal values serializes to fewer
/// than 128 bytes under the canonical postcard wire format. The function
/// is exposed as `const fn` so the bound can be referenced from `const`
/// contexts (e.g., a future compile-time assertion once the project
/// stabilizes on Rust 1.79+) and is exercised by a regression test
/// (`policy_buffer_fits_canonical_resource_contract`) to detect any
/// silent growth of `ResourceContract`.
#[cfg(not(kani))]
pub(crate) const fn resource_contract_policy_bytes_bound() -> usize {
    RESOURCE_CONTRACT_POLICY_BYTES
}

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
#[cfg(not(kani))]
pub fn compute_policy_digest(
    workflow: &vb_core::CompiledWorkflow,
) -> Result<vb_core::WorkflowDigest, JournalError> {
    let mut contract_bytes = [0_u8; RESOURCE_CONTRACT_POLICY_BYTES];
    let encoded = postcard::to_slice(&workflow.resource_contract(), &mut contract_bytes)
        .map_err(|_| JournalError::ArtifactMalformed)?;
    let hash = blake3::hash(encoded);
    Ok(vb_core::WorkflowDigest::from_bytes(*hash.as_bytes()))
}

#[cfg(kani)]
pub fn compute_policy_digest(
    workflow: &vb_core::CompiledWorkflow,
) -> Result<vb_core::WorkflowDigest, JournalError> {
    Ok(vb_core::WorkflowDigest::from_bytes(
        modeled_resource_contract_digest(workflow.resource_contract()),
    ))
}

/// XOR-fold model used under `cfg(kani)` in place of BLAKE3.
///
/// vb-1rqz7.23 / SA-014: this model is a *smoke-check abstraction*; the
/// Kani harnesses that call it assert dispatch totality and determinism,
/// not production cryptographic binding. The XOR-fold intentionally
/// differs from the production BLAKE3 digest (collision behaviour is
/// incomparable). See `verification::vb_fn4vt::policy_digest_binding`
/// for the explicit harness contract.
#[cfg(kani)]
fn modeled_resource_contract_digest(contract: vb_core::ResourceContract) -> [u8; 32] {
    let [steps_0, steps_1] = contract.max_steps.to_le_bytes();
    let [slots_0, slots_1] = contract.max_slots.to_le_bytes();
    let [constants_0, constants_1] = contract.max_constants.to_le_bytes();
    let [accessors_0, accessors_1] = contract.max_accessors.to_le_bytes();
    let [expressions_0, expressions_1] = contract.max_expressions.to_le_bytes();
    let [
        step_budget_0,
        step_budget_1,
        step_budget_2,
        step_budget_3,
        step_budget_4,
        step_budget_5,
        step_budget_6,
        step_budget_7,
    ] = contract.max_step_budget_per_tick.to_le_bytes();
    let [
        transitions_0,
        transitions_1,
        transitions_2,
        transitions_3,
        transitions_4,
        transitions_5,
        transitions_6,
        transitions_7,
    ] = contract.max_transitions_per_tick.to_le_bytes();
    let [input_0, input_1, input_2, input_3] = contract.max_input_bytes.to_le_bytes();
    let [output_0, output_1, output_2, output_3] = contract.max_output_bytes.to_le_bytes();
    let [
        blob_0,
        blob_1,
        blob_2,
        blob_3,
        blob_4,
        blob_5,
        blob_6,
        blob_7,
    ] = contract.max_blob_bytes.to_le_bytes();
    let [ipc_0, ipc_1, ipc_2, ipc_3] = contract.max_ipc_payload_bytes.to_le_bytes();
    let [retry_0, retry_1] = contract.max_retry_attempts.to_le_bytes();
    let [fanout_0, fanout_1] = contract.max_fanout.to_le_bytes();
    let [collect_0, collect_1, collect_2, collect_3] = contract.max_collect_items.to_le_bytes();
    let [queue_0, queue_1, queue_2, queue_3] = contract.max_queue_depth.to_le_bytes();
    let [journal_0, journal_1, journal_2, journal_3] =
        contract.max_journal_batch_bytes.to_le_bytes();
    let secret_results = u8::from(contract.allows_secret_results);

    [
        steps_0 ^ output_1,
        steps_1 ^ output_2,
        slots_0 ^ output_3,
        slots_1 ^ blob_0,
        constants_0 ^ blob_1,
        constants_1 ^ blob_2,
        accessors_0 ^ blob_3,
        accessors_1 ^ blob_4,
        expressions_0 ^ blob_5,
        expressions_1 ^ blob_6,
        contract.max_expr_stack ^ blob_7,
        step_budget_0 ^ ipc_0,
        step_budget_1 ^ ipc_1,
        step_budget_2 ^ ipc_2,
        step_budget_3 ^ ipc_3,
        step_budget_4 ^ retry_0,
        step_budget_5 ^ retry_1,
        step_budget_6 ^ fanout_0,
        step_budget_7 ^ fanout_1,
        transitions_0 ^ collect_0,
        transitions_1 ^ collect_1,
        transitions_2 ^ collect_2,
        transitions_3 ^ collect_3,
        transitions_4 ^ queue_0,
        transitions_5 ^ queue_1,
        transitions_6 ^ queue_2,
        transitions_7 ^ queue_3,
        input_0 ^ journal_0,
        input_1 ^ journal_1,
        input_2 ^ journal_2,
        input_3 ^ journal_3,
        output_0 ^ secret_results,
    ]
}
