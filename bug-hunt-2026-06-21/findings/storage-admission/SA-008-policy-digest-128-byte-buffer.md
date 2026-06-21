# SA-008: `compute_policy_digest` uses a hardcoded 128-byte buffer that rejects valid resource contracts

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_storage/src/admission/policy.rs:11, 26-35`
- **Confidence**: likely

## Description

`compute_policy_digest` serializes the workflow's `ResourceContract` via `postcard::to_slice` into a fixed `[0u8; 128]` buffer. If the serialized form exceeds 128 bytes, `postcard::to_slice` returns `Err(Flatten)` and the function maps this to `JournalError::ArtifactMalformed`. There is no static guarantee that every valid `ResourceContract` serializes into 128 bytes — the type is `#[non_exhaustive]` in vb_core and may grow.

## Evidence

```rust
// crates/vb_storage/src/admission/policy.rs:10-35
#[cfg(not(kani))]
const RESOURCE_CONTRACT_POLICY_BYTES: usize = 128;

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
```

The Kani-modeled variant (line 37-128) folds fields manually with XOR, which is a totally different digest that does not match production — so Kani proofs of policy-digest equality are vacuous. That is a separate defect (GOD RULE 2 violation in the proof artifact) but it underscores that no test actually verifies the 128-byte assumption against the real `ResourceContract` shape.

## Adversarial Check

`ResourceContract` in `crates/vb_core` is `#[non_exhaustive]` and contains 16+ numeric fields plus a `bool`. Current postcard encoding of these primitives fits well under 128 bytes (each `u64` is at most 9 bytes varint, each `u32` at most 5, giving a generous bound around 100 bytes). So today, the function works. But the bound is a magic number unrelated to any `ResourceContract::ENCODED_MAX_SIZE` constant. The next time vb_core adds a field to `ResourceContract` (e.g. `max_concurrent_actions: u64`), this function will start returning `ArtifactMalformed` for valid workflows, and the failure will be silent (the error variant is shared with actual corruption). The brittleness is the defect.

## Suggested Fix

Either (a) use `postcard::to_allocvec` and hash the resulting `Vec<u8>` (one allocation on a non-hot path — admission is per-workflow, not per-event), or (b) define a `pub const RESOURCE_CONTRACT_MAX_ENCODED_BYTES: usize` in vb_core derived from the type's actual layout, and use that constant here. Option (a) is simpler and removes the magic number entirely.
