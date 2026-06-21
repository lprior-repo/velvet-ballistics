# SA-014: `compute_policy_digest` kani model produces a digest unrelated to the production blake3 hash

- **Severity**: Info
- **Category**: correctness
- **Location**: `crates/vb_storage/src/admission/policy.rs:37-128` (`#[cfg(kani)]` branch) vs `crates/vb_storage/src/admission/policy.rs:26-35` (production)
- **Confidence**: confirmed

## Description

Under `cfg(kani)`, `compute_policy_digest` delegates to `modeled_resource_contract_digest`, which XOR-folds the contract's fields into a 32-byte array. Under `cfg(not(kani))`, the same function hashes the postcard-serialized contract with BLAKE3. The two implementations produce completely different digests for the same input. Any Kani harness that proves properties about policy-digest equality or binding is proving a property of the XOR-folded model, not of the actual production digest.

## Evidence

Production:
```rust
// crates/vb_storage/src/admission/policy.rs:26-35
#[cfg(not(kani))]
pub fn compute_policy_digest(workflow: &vb_core::CompiledWorkflow)
    -> Result<vb_core::WorkflowDigest, JournalError>
{
    let mut contract_bytes = [0_u8; RESOURCE_CONTRACT_POLICY_BYTES];
    let encoded = postcard::to_slice(&workflow.resource_contract(), &mut contract_bytes)
        .map_err(|_| JournalError::ArtifactMalformed)?;
    let hash = blake3::hash(encoded);
    Ok(vb_core::WorkflowDigest::from_bytes(*hash.as_bytes()))
}
```

Kani model:
```rust
// crates/vb_storage/src/admission/policy.rs:46-128
#[cfg(kani)]
fn modeled_resource_contract_digest(contract: vb_core::ResourceContract) -> [u8; 32] {
    let [steps_0, steps_1] = contract.max_steps.to_le_bytes();
    ...
    [
        steps_0 ^ output_1,
        steps_1 ^ output_2,
        ...
        output_0 ^ secret_results,
    ]
}
```

The model discards several fields (`max_expr_stack` is folded in but only one byte of it is used, etc.) and produces a deterministic but completely different 32-byte value. Cryptographic distinctness is irrelevant for Kani's purposes, but the model is not a faithful abstraction: it has different collision behaviour (XOR-fold collisions are common, BLAKE3 collisions are computationally infeasible), so any Kani proof that relies on "distinct contracts produce distinct policy digests" is vacuous in production.

## Adversarial Check

This is a known pattern in the codebase (see `modeled_header_crc32c` in `crates/vb_storage/src/codec/header.rs:110-122` which similarly replaces `crc32c::crc32c` with a rotate-XOR under Kani). The AGENTS.md `GOD RULES` section explicitly addresses this: "Verus `proof fn` and `spec fn` models MUST mathematically bind to the actual Rust implementations". The modeled digest does not bind to the production digest in any mathematical sense. Any Kani harness that uses policy-digest equality as a lemma (e.g., the `vb_fn4vt_policy_digest_binding` module referenced in `crates/vb_storage/src/lib.rs:130-132`) is proving a property of the wrong function. The finding is Info rather than Higher because the production code itself is correct; the defect is purely in the proof artifact's fidelity.

## Suggested Fix

Either (a) mark the Kani variant `#[cfg(kani)]` and remove the production divergence by always using BLAKE3 (Kani supports `blake3` via its `crate`-level model — verify this with a `cargo kani` smoke test), or (b) document explicitly in the Kani harness that the proof is about the model's collision-resistance properties, not about production digest equality, and stamp the harness as a smoke check only.
