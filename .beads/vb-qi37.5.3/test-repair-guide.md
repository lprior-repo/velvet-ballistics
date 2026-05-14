# test-repair-guide.md — vb-qi37.5.3

## Blocking Issue

**MAJOR-1**: `crates/vb_storage/src/admission.rs` region coverage at 87.38% — need ~30 more covered regions to reach 90% threshold.

Current: 1004/1149 regions covered (87.38%)
Target: ≥1034 regions covered (90%)
Gap: ~30 uncovered regions

---

## Diagnostic Commands

```bash
# Show uncovered regions with line numbers
cargo llvm-cov report -p vb_storage -- crates/vb_storage/src/admission.rs --show-missing 2>&1 | grep admission.rs

# HTML coverage report for interactive inspection
cargo llvm-cov nextest -p vb_storage --html 2>&1
# Open target/llvm-cov/html/index.html

# List all tests in admission module
cargo test -p vb_storage --lib -- --list 2>&1 | grep "^admission::"

# Run only admission tests
cargo test -p vb_storage --lib admission:: 2>&1
```

---

## Strategy: Cover Untested Error Branches

The 145 missing regions in admission.rs are concentrated in error-handling paths. Key areas to target:

### 1. VerificationProof Construction Error Paths

Functions with low coverage:
- `VerificationProof::new` / constructors
- Field validation branches (empty vs populated idempotency slices)

### 2. ArtifactEnvelopeError Variants

The `ArtifactEnvelopeError` enum has multiple variants not fully exercised:
- `MalformedEnvelope` — structural validation failures
- `MissingVerificationProof`
- `IdempotencyKeyMismatch` (replay path)

### 3. submit_artifact Error Branches

Current tests cover the happy path and checksum mismatch. Need to cover:
- IR parsing failures (malformed compiled workflow)
- Policy validation errors
- Store write failures

### 4. admit_artifact_run Error Paths

Need error branch coverage for:
- Store load failures
- Proof validation failures
- Policy check failures

---

## Recommended Test Additions

### Test Set A: VerificationProof Edge Cases (target: +10 regions)

```rust
#[test]
fn verification_proof_with_empty_idempotency_keyed_succeeds() {
    let proof = VerificationProof {
        idempotency_keyed: Box::new([]),
        idempotency_attested: Box::new([ActionId::new(1)]),
        // ... other fields
    };
    assert!(proof.idempotency_keyed.is_empty());
    assert_eq!(proof.idempotency_attested.len(), 1);
}

#[test]
fn verification_proof_with_both_empty_idempotency_succeeds() {
    // Exercise both-empty branch
}

#[test]
fn verification_proof_rejects_invalid_gate_value() {
    // Exercise gate validation error path
}
```

### Test Set B: ArtifactEnvelopeError Variants (target: +8 regions)

```rust
#[test]
fn artifact_envelope_rejects_missing_verification_proof() {
    let env = ArtifactEnvelope { verification_proof: None, .. };
    let result = validate_envelope_structure(&env);
    assert!(matches!(result, Err(ArtifactEnvelopeError::MissingVerificationProof)));
}

#[test]
fn artifact_envelope_rejects_malformed_envelope_structures() {
    // Test each MalformedEnvelope sub-variant
}
```

### Test Set C: submit_artifact Error Branches (target: +7 regions)

```rust
#[test]
fn submit_artifact_rejects_malformed_ir() {
    let journal = temp_journal()?;
    let bad_ir = CompiledWorkflow { raw_ir: invalid_bytes(), .. };
    let result = submit_artifact(&journal, &bad_ir, RuntimePolicy::Strict);
    assert!(matches!(result, Err(AdmissionError::StoreError(_))));
}

#[test]
fn submit_artifact_handles_store_write_failure() {
    // Use a read-only journal to trigger write failure
}
```

### Test Set D: admit_artifact_run Error Paths (target: +5 regions)

```rust
#[test]
fn admit_artifact_run_fails_when_store_load_returns_wrong_type() {
    // Use mock store that returns wrong artifact type
}

#[test]
fn admit_artifact_run_fails_on_proof_validation_error() {
    // Corrupt proof in store
}
```

---

## Success Criteria

After additions, verify:
```bash
cargo llvm-cov report -p vb_storage -- crates/vb_storage/src/admission.rs | grep "admission.rs"
```

Expected output should show regions coverage ≥90%.

---

## Non-Blocking Notes

1. **vb_runtime missing chunk_001.rs**: This is a pre-existing DEFERRED_GLOBAL issue in contract.md. Not attributable to vb-qi37.5.3 and not a blocker for this bead's approval.

2. **keys.rs line coverage 89.61%**: Outside vb-qi37.5.3 delivery scope per contract.md. Not a blocker.

3. **Function coverage 50.46%**: While low, this is not a direct threshold in the skill — line and region (branch) coverage are the gates.
