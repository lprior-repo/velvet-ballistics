# Theorem Kernel Projection — VerificationProof Gate Inputs

## Boundary

- **TLA+-owned temporal model**: None — no temporal, protocol, or distributed state machine in scope.
- **Verus-owned Rust core**: All gate validation, proof flag defaults, policy dispatch, and codec invariants are Rust-local pure functions proven by Verus.
- **Theorem-owned kernel**: None — no algebraic transition proofs, parser grammar theorems, codec theorems, or arithmetic bounds require Lean/Aeneas/Hax extraction beyond Verus.
- **Rust/runtime shell**: The admission flow is a Rust function calling into vb_storage for persistence and vb_core for validation — no I/O boundaries require separate theorem treatment.
- **External systems excluded from theorem proof**: None.

---

## Verus-Owned Clauses

### V-PF-001: VerificationProof Constructor Postconditions

- **Contract clause**: POST-001 (contract.md)
- **Rust target**: `crates/vb_storage/src/admission.rs::VerificationProof::new`
- **Spec/proof surface**: Verus `spec fn` for postconditions, `proof fn` for field invariants
- **Claim**: Constructor sets all fields to specified values; proof flags default to `true`, idempotency lists default to empty, warnings default to empty.
- **Trusted boundary**: `VerificationProof` is a plain struct with no interior mutability.
- **Shell exclusions**: No I/O, async, or external calls in constructor.
- **Evidence command**: `moon run :verify-proof` (Verus lane)

### V-PF-002: VerificationWarning Gate Range Invariant

- **Contract clause**: INV-002 (contract.md)
- **Rust target**: `crates/vb_storage/src/admission.rs::VerificationWarning::is_valid`
- **Spec/proof surface**: `spec fn` for invariant, `proof fn` for range check
- **Claim**: `is_valid()` returns true iff `gate ∈ [1, 2]`
- **Trusted boundary**: `VerificationWarning` fields are const-constructible.
- **Evidence command**: `moon run :verify-proof` (Verus lane)

### V-G1-001: Gate 1 — Structure Validation Postconditions

- **Contract clause**: POST-002, POST-003, POST-004
- **Rust target**: `crates/vb_core/src/compiled_workflow.rs::CompiledWorkflow::try_from_parts`
- **Spec/proof surface**: `spec fn` for `Result<Self, WorkflowError>`, `proof fn` for preconditions/postconditions
- **Claim**: On `Ok`, the reconstructed workflow is valid; on `Err`, the error is a semantic validation failure (not a codec bug).
- **Trusted boundary**: `CompiledWorkflow` fields are validated on construction.
- **Evidence command**: `moon run :verify-proof` (Verus lane), Kani bounded model check

### V-G1-002: Gate 1 — validate_budget Boundedness

- **Contract clause**: POST-001 `bounded == true`
- **Rust target**: `crates/vb_core/src/validation.rs::validate_budget`
- **Spec/proof surface**: `spec fn` postcondition that `bounded` is true when validation passes
- **Claim**: `BoundednessPolicy::DEFAULT.validate(&budget)` succeeding implies the IR is size-bounded.
- **Evidence command**: `moon run :verify-proof` (Verus lane)

### V-G2-001: Gate 2 — Checksum Validation Correctness

- **Contract clause**: POST-002, POST-003, POST-004
- **Rust target**: `crates/vb_storage/src/admission.rs` (checksum validation block lines 177-184)
- **Spec/proof surface**: `spec fn` for hash computation, `proof fn` for equality check
- **Claim**: If the returned `AcceptedArtifact.verification.gate_count == 2`, then the BLAKE3 hash of the serialized parts (with digest zeroed) equals the claimed workflow digest.
- **Trusted boundary**: `blake3::hash` and `postcard::to_allocvec` are trusted external functions.
- **Evidence command**: `moon run :verify-proof` (Verus lane), Kani bounded model check for mismatch path

### V-POL-001: Policy-Gated Admission Dispatch

- **Contract clause**: POST-002, POST-003, POST-004
- **Rust target**: `crates/vb_storage/src/admission.rs::submit_artifact_with_contracts`
- **Spec/proof surface**: Match expression on `RuntimePolicy` dispatch, postconditions per arm
- **Claim**:
  - Relaxed arm: `gate_count == 0`, `durable == false`
  - Journaled arm: `gate_count == 2`, `durable == false`
  - Strict arm: `gate_count == 2`, `durable == true`
- **Evidence command**: `moon run :verify-proof` (Verus lane), BDD unit tests

---

## Theorem-Owned Clauses

**None** — no clause requires Lean/Aeneas/Hax extraction. All critical behavior is expressible in Verus:

- Gate validation is pure codec/validation functions (Verus-suited)
- Proof flag defaults are constructor postconditions (Verus-suited)
- Policy dispatch is a match on an enum (Verus-suited)
- No algebraic state transition lattice, no protocol refinement, no parser grammar theorem

---

## Waivers

| Clause | Waiver Reason | Owner | Expiry |
|--------|--------------|-------|--------|
| Lean/Aeneas/Hax for any clause | All Rust-local pure behavior is expressible in Verus; no algebraic kernel extraction needed | vb-core-proof-gate-inputs | N/A |

---

## Non-goals

- Lean/Aeneas/Hax theorem projection
- TLA+ temporal modeling (non-applicable by design)
- Theorem kernel extraction beyond Verus
