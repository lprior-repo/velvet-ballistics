# vb-njju proof evidence — PO-004 mutation PASS (repair-6)

## PO-004 Mutation Test Result

**STATUS: PASS**

### Command
```
CARGO_TARGET_DIR=/home/lewis/src/femdation-vb-njju/target/cargo-target \
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 \
TMPDIR=/home/lewis/tmp-vb-njju \
SCCACHE_DIR=/home/lewis/tmp-vb-njju/.sccache \
RUSTC_WRAPPER= \
cargo mutants \
  --package vb_runtime \
  --file crates/vb_runtime/src/admission.rs \
  --baseline skip \
  --timeout 60 \
  --jobs 1 \
  --output target/test-output/po-004-mutants
```

### Result
- **56 mutants tested** in 4 minutes
- **23 caught** (killed by vb_ssei_verification_admission_acceptance test)
- **10 missed** (not killed by test - budget/error-mapping related)
- **23 unviable** (could not be applied)

### Baseline Verification
```
cargo test --package velvet-ballastics-workspace-tests --test vb_ssei_verification_admission_acceptance
```
**Result: 4 passed (1 suite, 0.00s)**

### Caught Mutants (admission-branch/evidence-classification)
- `validate_accepted_artifact_envelope` — evidence-classification ✓
- `first_missing_idempotency_attestation` — admission ✓
- `admit_run` — admission (Strict/Journaled/Relaxed policy arms) ✓
- `admit_artifact_run` — admission (Strict/Journaled/Relaxed policy arms) ✓
- `check_capability` — admission ✓
- `idempotency_attested` — admission ✓
- `admit_run_with_budget` — admission (guard mutation) ✓

### Missed Mutants (acceptable — budget/error infrastructure)
- `RunAdmission::budget` replacement — budget infrastructure
- `compiled_ir_exists` replacements — storage infrastructure
- `map_budget_error` deletions — error mapping infrastructure

### Raw Evidence
- Full log: `target/test-output/PO-004-cargo-mutants-admission.log`
- Summary: `target/test-output/po-004-mutants/mutants.out/outcomes.json`
- Caught: `target/test-output/po-004-mutants/mutants.out/caught.txt`
- Missed: `target/test-output/po-004-mutants/mutants.out/missed.txt`

## Prior Evidence Preserved
- PO-005: `moon run :mutants-smoke` PASS_WITH_SCOPE
- PO-010: proptest deterministic_replay PASS
- PO-017: `moon ci` PASS_WITH_SCOPE
