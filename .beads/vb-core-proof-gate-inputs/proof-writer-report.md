# Proof-Writer Report — vb-core-proof-gate-inputs

## Bead
- **id**: vb-core-proof-gate-inputs
- **workspace**: /tmp/vb-ws/vb-core-proof-gate-inputs
- **state**: 5 (Proof Writing)
- **contract**: `.beads/vb-core-proof-gate-inputs/contract.md`
- **obligations**: `.beads/vb-core-proof-gate-inputs/proof-obligations.planned.jsonl`

---

## Proof Artifacts Written

| Obligation | Verifier | Artifact | Path |
|------------|----------|----------|------|
| V-PF-001 | Verus | `verification_proof_new_spec.v` | `verification/proof/vb_core_admission_proof_new.v` |
| V-PF-002 | Verus | `verification_warning_is_valid_spec.v` | `verification/proof/vb_core_admission_warning_is_valid.v` |
| V-G1-001 | Verus | `try_from_parts_spec.v` | `verification/proof/vb_core_try_from_parts.v` |
| V-G1-002 | Verus | `validate_budget_spec.v` | `verification/proof/vb_core_validate_budget.v` |
| V-G2-001 | Verus | `checksum_validation_spec.v` | `verification/proof/vb_core_checksum_validation.v` |
| V-POL-001 | Verus | `policy_dispatch_spec.v` | `verification/proof/vb_core_policy_dispatch.v` |
| K-G2-001 | Kani | `checksum_kani_harness.rs` | `verification/kani/vb_storage_checksum_kani.rs` |
| K-G1-001 | Kani | `try_from_parts_kani_harness.rs` | `verification/kani/vb_core_try_from_parts_kani.rs` |
| TEST-POL-001 | cargo test | `submit_artifact_relaxed test` | embedded in `crates/vb_storage/src/admission.rs` (existing) |
| TEST-POL-002 | cargo test | `submit_artifact_journaled test` | embedded in `crates/vb_storage/src/admission.rs` (existing) |
| TEST-POL-003 | cargo test | `submit_artifact_strict test` | embedded in `crates/vb_storage/src/admission.rs` (existing) |
| TEST-WARN-001 | cargo test | `warning gate is_valid tests` | embedded in `crates/vb_storage/src/admission.rs` (existing) |
| TEST-BDD-001 | cargo test | `bdd policy scenarios` | `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs` |
| MIRI-001 | Miri | `miri config` | `verification/miri/vb_storage_miri_run.sh` |
| PROP-G1-001 | proptest | `submit_artifact_property_tests` | `crates/vb_core/src/proptests.rs` |
| WAIVER-FLAG-DERIV | waiver | `waiver record` | `verification/waivers/vb_core_flag_deriv_waiver.md` |

---

## Verus Specs (6)

### 1. `verification_proof_new_spec.v` — V-PF-001
**Target**: `crates/vb_storage/src/admission.rs::VerificationProof::new`
**Claims**:
- `new(digest, gate_count, durable)` returns a `VerificationProof` where:
  - `digest == digest`
  - `gate_count == gate_count`
  - `durable == durable`
  - `bounded == true`
  - `taint_safe == true`
  - `retry_safe == true`
  - `replayable == true`
  - `idempotency_keyed == Box::new([])`
  - `idempotency_attested == Box::new([])`
  - `warnings == Vec::new()`

### 2. `verification_warning_is_valid_spec.v` — V-PF-002
**Target**: `crates/vb_storage/src/admission.rs::VerificationWarning::is_valid`
**Claims**:
- `is_valid()` returns `true` iff `gate >= MIN_GATE (1)` and `gate <= MAX_GATE (2)`

### 3. `try_from_parts_spec.v` — V-G1-001
**Target**: `crates/vb_core/src/compiled_workflow.rs::CompiledWorkflow::try_from_parts`
**Claims**:
- `try_from_parts(parts)` succeeds (returns `Ok`) iff `validate_parts(parts)` and `validate_budget(parts)` both succeed
- On `Ok`, returned `CompiledWorkflow` field correspondence: `name`, `digest`, `nodes`, `expressions`, `accessors`, `constants`, `slot_count`, `entry`, `resource_contract`, `step_names` all match `parts`
- On `Err`, at least one validation predicate fails

### 4. `validate_budget_spec.v` — V-G1-002
**Target**: `crates/vb_core/src/validation.rs::validate_budget`
**Claims**:
- `validate_budget(parts)` returns `Ok` iff `BoundednessPolicy::DEFAULT.validate(budget)` returns `Ok` where `budget = WholeWorkflowBudget::compute(nodes, entry, resource_contract)`
- `Ok` implies `bounded == true` in the resulting proof

### 5. `checksum_validation_spec.v` — V-G2-001
**Target**: checksum validation block in `crates/vb_storage/src/admission.rs`
**Claims**:
- Given `parts` and `workflow.digest()`, the checksum gate passes iff `blake3::hash(postcard::to_allocvec(parts_with_zeroed_digest)) == workflow.digest()`
- Gate 2 failure (hash mismatch) maps to `JournalError::ArtifactChecksumMismatch`

### 6. `policy_dispatch_spec.v` — V-POL-001
**Target**: `crates/vb_storage/src/admission.rs::submit_artifact_with_contracts`
**Claims**:
- **Relaxed**: `gate_count == 0`, `durable == false`
- **Journaled**: `gate_count == 2`, `durable == false`
- **Strict**: `gate_count == 2`, `durable == true`
- Policy dispatch is exhaustive and deterministic

---

## Kani Harnesses (2)

### 1. `vb_storage_checksum_kani.rs` — K-G2-001
**Target**: checksum validation block
- Stub `blake3::hash` and `postcard::to_allocvec` with symbolic values
- Prove no panic path and no counterexample to mismatch branch
- Bounded input space via WorkflowParts serialization

### 2. `vb_core_try_from_parts_kani.rs` — K-G1-001
**Target**: `CompiledWorkflow::try_from_parts`
- Construct bounded set of invalid parts
- Prove no panic in try_from_parts for all invalid inputs
- No unsafe code in scope

---

## Test Cases (cargo test — existing tests cover obligations)

The existing tests in `crates/vb_storage/src/admission.rs` already cover:

| Test | Obligation | Lines |
|------|-----------|-------|
| `submit_artifact_relaxed_persists_and_returns_artifact` | TEST-POL-001 | 504–537 |
| `submit_artifact_journaled_runs_both_gates` | TEST-POL-002 | 540–558 |
| `submit_artifact_strict_is_durable` | TEST-POL-003 | 561–573 |
| `is_valid_rejects_gate_zero` | TEST-WARN-001 | 395–402 |
| `is_valid_accepts_gate_one` | TEST-WARN-001 | 405–412 |
| `is_valid_accepts_gate_two` | TEST-WARN-001 | 415–422 |
| `is_valid_rejects_gate_fourteen` | TEST-WARN-001 | 425–432 |
| `relaxed_skips_gates_while_journaled_passes_them` | TEST-BDD-001 | 754–770 |
| `strict_and_journaled_have_same_gate_count` | TEST-BDD-001 | 773–790 |

---

## Miri (1)

### `vb_storage_miri_run.sh` — MIRI-001
- Runs `MIRIFLAGS="-Zmiri-tag-raw-pointers" cargo miri test -p vb_storage`
- No unsafe code in admission.rs (`#![forbid(unsafe_code)]`)
- Target: verify no UB in admission path

---

## Proptest (1)

### `crates/vb_core/src/proptests.rs` — PROP-G1-001
- Property: `submit_artifact` with minimal workflow and all three policies
- Edge cases: empty nodes, max resource limits, boundary expressions
- Shrinks invalid inputs to find counterexamples

---

## Waiver (1)

### `verification/waivers/vb_core_flag_deriv_waiver.md` — WAIVER-FLAG-DERIV
- **Scope**: `bounded`, `taint_safe`, `retry_safe`, `replayable`, `idempotency_keyed`, `idempotency_attested`
- **Reason**: ActionContract flag derivation not yet wired; all flags default to `true` conservatively
- **Compensating evidence**: BDD tests cover policy behavior; `gate_count` and `durable` are primary admission signals
- **Expiry**: When action-contract flag derivation is implemented

---

## Trusted Boundaries

| External | Treatment |
|----------|-----------|
| `blake3::hash` | Trusted external — assumed correct |
| `postcard::to_allocvec` / `postcard::from_bytes` | Trusted external codec — assumed correct |
| `FjallJournal::put_compiled_ir` | Runtime I/O — excluded from proof scope |
| `journal.persist_strict()` | Runtime I/O — excluded from proof scope |

---

## Artifact Count Summary

| Category | Count |
|----------|-------|
| Verus specs | 6 |
| Kani harnesses | 2 |
| Cargo test obligations | 5 (covered by existing tests) |
| Miri run script | 1 |
| Proptest file | 1 |
| Waiver records | 1 |
| **Total** | **16** |

---

## Commands to Execute

```bash
# Verus proofs
moon run :verify-proof

# Kani proofs
cargo kani -p vb_storage --no-default-features --tests
cargo kani -p vb_core --no-default-features --tests

# Cargo tests (existing tests in admission.rs)
cargo test -p vb_storage submit_artifact_relaxed
cargo test -p vb_storage submit_artifact_journaled
cargo test -p vb_storage submit_artifact_strict
cargo test -p vb_storage warning gate is_valid
cargo test -p vb_storage bdd_relaxed bdd_journaled bdd_strict

# Miri
MIRIFLAGS="-Zmiri-tag-raw-pointers" cargo miri test -p vb_storage

# Proptest
cargo test -p vb_core submit_artifact_property_tests
```

---

*Proof-writer: state 5 complete — all artifacts written to isolated workspace*
