# Verification Layers — VerificationProof Gate Inputs

## Boundary

- **Verus-owned kernel**: Gate 1 (structure validation), Gate 2 (checksum validation), VerificationProof constructor, VerificationWarning invariants, policy dispatch
- **TLA+ temporal model**: None — no temporal behavior in scope
- **Theorem projection**: None — Verus sufficient for all Rust-local pure logic
- **Runtime shell**: FjallJournal persistence, blake3 hashing, postcard serialization (trusted externals)
- **External systems excluded from formal proof**: None

---

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layer | Tertiary Layer |
|-----------------|---------------|----------------|----------------|
| POST-001 (VerificationProof::new) | verus | proptest | — |
| INV-002 (VerificationWarning::is_valid) | verus | unit | — |
| POST-002 (Relaxed policy) | unit + BDD test | miri | — |
| POST-003 (Journaled policy) | unit + BDD test | miri | — |
| POST-004 (Strict policy) | unit + BDD test | miri | — |
| Gate 1 structure validation | verus | kani | proptest |
| Gate 2 checksum validation | verus | kani | — |
| bounded flag derivation | verus | proptest | — |
| ERR-001 ArtifactMalformed | unit test | miri | — |
| ERR-001 ArtifactChecksumMismatch | unit test | kani | — |
| Warnings gate range | unit test | verus | — |

---

## Verus Scope

### Target: VerificationProof::new

- **Rust module**: `crates/vb_storage/src/admission.rs`
- **Spec/Proof function**: Postconditions on constructor
- **Invariants**: Field value correspondence (digest, gate_count, durable) + default values (bounded, taint_safe, retry_safe, replayable = true; idempotency lists empty; warnings empty)
- **Trusted boundary**: Plain struct, no interior mutability, no unsafe
- **Shell exclusions**: No I/O, async, storage, wall-clock time

### Target: VerificationWarning::is_valid

- **Rust module**: `crates/vb_storage/src/admission.rs`
- **Spec/Proof function**: Range check `gate ∈ [1, 2]`
- **Invariants**: None beyond the range check
- **Trusted boundary**: Const fields, no unsafe
- **Shell exclusions**: None

### Target: CompiledWorkflow::try_from_parts (Gate 1)

- **Rust module**: `crates/vb_core/src/compiled_workflow.rs`
- **Spec/Proof function**: `spec fn` postconditions on Ok/Err
- **Invariants**: Structural validity of reconstructed workflow
- **Trusted boundary**: `validate_parts` and `validate_budget` are pure validation functions
- **Shell exclusions**: No I/O, async, storage

### Target: Checksum validation block (Gate 2)

- **Rust module**: `crates/vb_storage/src/admission.rs`
- **Spec/Proof function**: Hash equality implies digest matches
- **Invariants**: blake3 hash purity, postcard serialization purity
- **Trusted boundary**: `blake3::hash`, `postcard::to_allocvec` are trusted
- **Shell exclusions**: blake3 and postcard are treated as trusted externals

---

## Kani Scope

### Target: submit_artifact_with_contracts

- **Claim**: Gate 2 (checksum mismatch) returns `Err(ArtifactChecksumMismatch)` when digest diverges
- **Bounded model**: 1 valid digest + 1 invalid digest → 2 states
- **Command**: `cargo kani --no-default-features --features=vb_storage/kal'll`
- **Note**: Run after verifying `VerificationProof` gate_count derivation

---

## Miri Scope

### Target: admission.rs pointer handling

- **Claim**: No undefined behavior in admission path
- **Command**: `MIRIFLAGS="-Zmiri-tag-raw-pointers" cargo miri test -p vb_storage`
- **Note**: Important for `postcard::to_allocvec` and `blake3::hash` usage

---

## Proptest Scope

### Target: CompiledWorkflow::try_from_parts

- **Claim**: Structure validation handles edge cases: empty nodes, max resource limits, boundary expressions
- **Command**: `cargo test -p vb_core submit_artifact_property_tests` (proptest suite)
- **Note**: Shrinking invalid inputs to identify missing validation

---

## Unit/BDD Test Scope

### Durability Gate Tests

- **File**: `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs`
- **Scenarios**:
  - `bdd_relaxed_policy_accepts_without_gate_validation` — gate_count=0, durable=false
  - `bdd_journaled_policy_enforces_both_gates` — gate_count=2, durable=false
  - `bdd_strict_policy_enforces_gates_and_syncall` — gate_count=2, durable=true
  - `gate_count_zero_for_relaxed`
  - `gate_count_two_for_journaled`
  - `gate_count_two_for_strict`

### VerificationWarning Tests

- **File**: `crates/vb_storage/src/admission.rs` (inline tests)
- **Scenarios**:
  - `is_valid` returns true for gate=1, gate=2
  - `is_valid` returns false for gate=0, gate=3
  - Serialization roundtrip

---

## Waivers

| Clause | Reason | Compensating Evidence |
|--------|--------|----------------------|
| Kani for Gate 1 structure | Verus covers pure function postconditions; Kani adds bounded model check only for mismatch paths | BDD unit tests + proptest |
| TLA+ for any temporal behavior | Admission flow is sequential Rust, not a state machine | N/A — non-applicable |
| Lean/Aeneas/Hax | All Rust-local pure behavior expressible in Verus | N/A |
