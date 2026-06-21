# SA-012: `submit_relaxed_artifact_with_evidence` writes `idempotency_keyed`/`idempotency_attested` into the proof even though `gate_count == 0` claims no verification was performed

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_storage/src/admission/flow.rs:80-82` (relaxed writes); `crates/vb_storage/src/admission/record.rs:119-140` (validation does not enforce emptiness)
- **Confidence**: confirmed

## Description

The relaxed admission path constructs a `VerificationProof` with `gate_count = 0`, signaling "no verification gates were run". It then overwrites `proof.idempotency_keyed` and `proof.idempotency_attested` with the contract-extracted evidence. The read-side validator `validate_verification_proof` enforces only that `durable == false` and `has_any_proof_flag == false` for `gate_count == 0`; it does NOT enforce that the idempotency collections are empty. A relaxed artifact therefore carries idempotency evidence in its proof record, contradicting the `gate_count = 0` claim that no verification was performed.

## Evidence

```rust
// crates/vb_storage/src/admission/flow.rs:79-86
let ir_bytes = validate_workflow_artifact_bytes(workflow)?;
let mut proof = VerificationProof::new(workflow.digest(), 0, false);   // gate_count = 0
proof.idempotency_keyed = idempotency_evidence.keyed.clone();          // <-- but writes evidence
proof.idempotency_attested = idempotency_evidence.attested.clone();
let artifact = accepted_artifact(workflow, ir_bytes, proof, required_capabilities)?;
```

```rust
// crates/vb_storage/src/admission/record.rs:119-140
fn validate_verification_proof(proof: &VerificationProof) -> Result<(), JournalError> {
    if !is_accepted_gate_count(proof.gate_count) { ... }
    if proof.gate_count == 0 {
        if proof.durable || has_any_proof_flag(proof) {                 // <-- checks flags only
            return Err(JournalError::ArtifactMalformed);
        }
    } else if let Some(flag) = missing_proof_flag(proof) { ... }
    ...
}
```

`has_any_proof_flag` (`crates/vb_storage/src/admission/record.rs:142-148`) inspects only the five boolean `*_claimed` fields, not the `idempotency_keyed`/`idempotency_attested` collections.

## Adversarial Check

The relaxed policy by definition does not run verification, so any `idempotency_*` data written into the proof is "evidence extracted from contracts" — not "evidence that the runtime actually verified anything". A consumer reading the proof sees populated idempotency lists alongside `gate_count = 0` and must decide which signal to trust. The metadata-hash function (`crates/vb_storage/src/admission/metadata.rs:32-38`) hashes these collections into the artifact's binding, so they become part of the durable contract even though the proof claims no verification. The simplest reading is that the relaxed path should leave these collections empty (the `VerificationProof::new` default) — they are populated for the Journaled/Strict paths where `gate_count = 15` actually means "verified".

## Suggested Fix

Either (a) remove the `proof.idempotency_keyed = ...` lines from `submit_relaxed_artifact_with_evidence` (leave the default empty `Box::new([])`), or (b) extend `validate_verification_proof` to enforce emptiness of both collections when `gate_count == 0`:

```rust
if proof.gate_count == 0 {
    if proof.durable
        || has_any_proof_flag(proof)
        || !proof.idempotency_keyed.is_empty()
        || !proof.idempotency_attested.is_empty()
    {
        return Err(JournalError::ArtifactMalformed);
    }
}
```

Option (b) is the more defensive fix because it makes the validation independent of the construction site.
