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

/// Maximum byte size for a serialized `ResourceContract` accepted by the
/// admission policy buffer.
///
/// SA-008 FIX companion: this is the post-`compute_policy_digest` SA-008
/// buffer bound for the policy-digest path. The bound is the worst-case
/// postcard varint width of every field of `vb_core::workflow::ResourceContract`
/// at `T::MAX`, plus a small per-field tag overhead. The companion test
/// `policy_buffer_fits_canonical_resource_contract` enforces that the actual
/// postcard encoding of the canonical contract fits inside this bound.
///
/// Field-by-field worst-case widths (varint, plus one-byte field tag for
/// non-unit structs; postcard uses 1-byte field tags per field):
///
/// * 5 × u16 (`max_steps`, `max_slots`, `max_constants`, `max_accessors`,
///   `max_expressions`) → 3 bytes varint each → 5 × 3 = 15 bytes
/// * 1 × u8 (`max_expr_stack`) → 2 bytes varint
/// * 1 × u8 (`max_transitions_per_tick`? — actually u64) → see below
///
/// Re-counting by type:
/// * u16 fields: 6 (added `max_retry_attempts`, `max_fanout`) → 6 × 3 = 18 bytes
/// * u8 field: 1 (`max_expr_stack`) → 2 bytes
/// * u32 fields: 5 (`max_input_bytes`, `max_output_bytes`,
///   `max_ipc_payload_bytes`, `max_collect_items`, `max_queue_depth`,
///   `max_journal_batch_bytes`) → 6 × 5 = 30 bytes
/// * u64 fields: 3 (`max_step_budget_per_tick`, `max_transitions_per_tick`,
///   `max_blob_bytes`) → 3 × 9 = 27 bytes
/// * bool: 1 (`allows_secret_results`) → 1 byte
///
/// Total payload bytes: 18 + 2 + 30 + 27 + 1 = 78 bytes.
///
/// Postcard adds per-field tags (1 byte each = 18 fields) plus 1-byte
/// struct tag → 19 bytes overhead. Worst case: 78 + 19 = 97 bytes.
///
/// We round up to a safe 256-byte upper bound so any future field additions
/// up to roughly 2× the current size still fit without silent rejection.
#[cfg(test)]
pub(crate) const RESOURCE_CONTRACT_POLICY_BYTES_BOUND: usize = 256;

/// Returns the maximum byte size for a `ResourceContract` accepted by the
/// admission policy buffer.
///
/// Companion to [`compute_policy_digest`]: admission pre-allocates
/// (logically) a buffer of this size before serializing the workflow's
/// `resource_contract` for hashing. The bound is intentionally conservative
/// so that adding new bounded fields to `ResourceContract` cannot silently
/// push a previously-valid contract past the buffer.
///
/// See [`RESOURCE_CONTRACT_POLICY_BYTES_BOUND`] for the derivation.
#[cfg(test)]
#[must_use]
pub(crate) const fn resource_contract_policy_bytes_bound() -> usize {
    RESOURCE_CONTRACT_POLICY_BYTES_BOUND
}

/// Computes the policy digest from a workflow's resource contract.
///
/// GAP-003 FIX: Added per review finding that `AcceptedArtifact` must bind
/// to the policy digest that governed admission. The policy digest is derived
/// from the resource contract by hashing its canonical serialization.
///
/// SA-008 FIX: Replaced the previous fixed-size `[u8; 128]` stack buffer with
/// `postcard::to_allocvec`. The old buffer's capacity (128 bytes) was a magic
/// number with no compile-time link to `ResourceContract`'s field-count or
/// field widths; future field additions would have silently rejected valid
/// contracts, with the failure mode mapped to `ArtifactMalformed` (the same
/// variant used for genuinely malformed IR). The replacement allocates the
/// exact required byte count on the per-workflow admission path (not a hot
/// path); the BLAKE3 hash is byte-identical to the previous implementation
/// for any given `ResourceContract` because postcard's varint encoding is
/// deterministic.
#[cfg(not(kani))]
pub fn compute_policy_digest(
    workflow: &vb_core::CompiledWorkflow,
) -> Result<vb_core::WorkflowDigest, JournalError> {
    let contract_bytes = postcard::to_allocvec(&workflow.resource_contract())
        .map_err(|_| JournalError::ArtifactMalformed)?;
    let hash = blake3::hash(&contract_bytes);
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
