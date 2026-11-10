use super::AcceptedArtifact;
use crate::error::JournalError;

pub(crate) fn zero_workflow_digest() -> vb_core::WorkflowDigest {
    vb_core::WorkflowDigest::from_bytes([0u8; 32])
}

/// Computes the content digest for an accepted artifact envelope.
///
/// The artifact carries its own digest and the proof repeats that digest, so
/// hashing the final postcard bytes directly would create an unsolvable
/// self-reference. This mirrors `CompiledWorkflow` digesting: the two
/// self-digest fields are zeroed, the canonical envelope bytes are serialized,
/// and BLAKE3 binds the remaining artifact payload, proof, policy, capability,
/// and compiled IR bytes.
pub fn accepted_artifact_digest(
    artifact: &AcceptedArtifact,
) -> Result<vb_core::WorkflowDigest, JournalError> {
    let mut canonical = artifact.clone();
    let zero = zero_workflow_digest();
    canonical.digest = zero;
    canonical.verification.digest = zero;
    let bytes = postcard::to_allocvec(&canonical).map_err(JournalError::PostcardEncodeFailed)?;
    Ok(vb_core::WorkflowDigest::from_bytes(
        blake3::hash(&bytes).into(),
    ))
}

pub(crate) fn bind_artifact_digest(artifact: &mut AcceptedArtifact) -> Result<(), JournalError> {
    let digest = accepted_artifact_digest(artifact)?;
    artifact.digest = digest;
    artifact.verification.digest = digest;
    Ok(())
}
/// Computes the policy digest from a workflow's resource contract.
///
/// GAP-003 FIX: Added per review finding that `AcceptedArtifact` must bind
/// to the policy digest that governed admission. The policy digest is derived
/// from the resource contract by hashing its canonical serialization.
///
/// YAGNI FIX: the serialization buffer is pre-sized to
/// `resource_contract_policy_bytes_bound` (115 bytes) as a single
/// `vec![0u8; bound]` allocation. `postcard::to_slice` honors the slice
/// length it is given, so the Vec must be initialized to its full bound
/// length (not just capacity) before the call. The bound is derived from
/// postcard's per-field upper bounds on `ResourceContract` and is verified
/// by `policy_buffer_fits_canonical_resource_contract` in `admission::tests`.
/// After `to_slice` returns, the Vec is truncated to the used prefix and
/// the prefix is BLAKE3-hashed. The BLAKE3 output is byte-identical to a
/// `to_allocvec` implementation for any given `ResourceContract` because
/// postcard's varint encoding is deterministic. The pre-sized bound
/// guarantees no reallocation occurs and the buffer is bounded to the
/// policy bound (no dynamic growth).
pub fn compute_policy_digest(
    workflow: &vb_core::CompiledWorkflow,
) -> Result<vb_core::WorkflowDigest, JournalError> {
    let bound = resource_contract_policy_bytes_bound();
    let mut contract_bytes: Vec<u8> = vec![0u8; bound];
    let used_len = match postcard::to_slice(&workflow.resource_contract(), &mut contract_bytes)
        .map_err(JournalError::Encode)
    {
        Ok(used) => used.len(),
        Err(_) => {
            // Fallback: serialize the entire workflow parts and extract resource_contract field.
            let parts = workflow.to_parts();
            postcard::to_slice(&parts.resource_contract, &mut contract_bytes)
                .map_err(JournalError::Encode)?
                .len()
        }
    };
    contract_bytes.truncate(used_len);
    let hash = blake3::hash(&contract_bytes);
    Ok(vb_core::WorkflowDigest::from_bytes(*hash.as_bytes()))
}

/// Returns the maximum serialized size of a `ResourceContract` in bytes.
///
/// This is the policy-buffer upper bound used by admission: any canonical
/// `ResourceContract` (postcard-encoded) must fit in this bound so that
/// strict-durability admission paths can compute the policy digest without
/// dynamic allocation.
///
/// The bound is derived from field-by-field postcard upper bounds on the
/// `ResourceContract` struct fields (see `vb_core::workflow::ResourceContract`).
/// Postcard's encoding (postcard 1.1.x):
///
/// - `u8` is NOT varint-encoded; `serialize_u8` writes a single raw byte
///   (`postcard-1.1.3/src/ser/serializer.rs:130-134`). So `u8::MAX = 1 byte`.
/// - `u16` is varint (LEB128); `u16::MAX` needs 3 bytes (`varint_max::<u16>()`).
/// - `u32` is varint; `u32::MAX` needs 5 bytes (`varint_max::<u32>()`).
/// - `u64` is varint; `u64::MAX` needs 10 bytes (LEB128: `ceil(64/7) = 10`,
///   `varint_max::<u64>() = (64 + 7 - 1) / 7 = 10`).
/// - `bool` is 1 byte (true=1, false=0).
///
/// Field-by-field upper bounds for `ResourceContract`:
///
/// - 7 × `u16::MAX`  → 7 × 3 bytes
/// - 1 × `u8::MAX`   → 1 × 1 byte  (raw, not varint)
/// - 3 × `u64::MAX`  → 3 × 10 bytes
/// - 6 × `u32::MAX`  → 6 × 5 bytes
/// - 1 × bool        → 1 × 1 byte
///
/// Sum: 21 + 1 + 30 + 30 + 1 = 83 bytes. A 32-byte headroom is reserved for
/// future field additions before this bound must be re-derived. Total
/// bound: 115 bytes.
#[must_use]
pub(crate) const fn resource_contract_policy_bytes_bound() -> usize {
    // Field counts and per-field upper bounds are kept as named constants
    // so future maintainers can audit the math against `ResourceContract`.
    const U16_FIELDS: usize = 7;
    const U8_FIELDS: usize = 1;
    const U64_FIELDS: usize = 3;
    const U32_FIELDS: usize = 6;
    const BOOL_FIELDS: usize = 1;
    const HEADROOM: usize = 32;

    // Postcard upper bounds (largest encoded width per integer width).
    // u8 is NOT varint-encoded — it serializes as a single raw byte
    // (postcard-1.1.3/src/ser/serializer.rs:130-134).
    const U16_VARINT_MAX: usize = 3;
    const U8_RAW_MAX: usize = 1;
    const U64_VARINT_MAX: usize = 10;
    const U32_VARINT_MAX: usize = 5;

    U16_FIELDS * U16_VARINT_MAX
        + U8_FIELDS * U8_RAW_MAX
        + U64_FIELDS * U64_VARINT_MAX
        + U32_FIELDS * U32_VARINT_MAX
        + BOOL_FIELDS
        + HEADROOM
}
