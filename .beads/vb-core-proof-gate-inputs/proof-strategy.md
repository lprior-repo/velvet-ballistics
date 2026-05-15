# Proof Strategy — vb-core-proof-gate-inputs

## Bead
- **id**: vb-core-proof-gate-inputs
- **state**: 4
- **scope**: `crates/vb_storage/src/admission.rs`, `crates/vb_core/src/compiled_workflow.rs`, `crates/vb_core/src/validation.rs`

---

## Gate Derivation (core claim)

| Gate | Source | Method | Trust boundary |
|------|--------|--------|----------------|
| Gate 1 — Structure | `CompiledWorkflow::try_from_parts(parts.clone())` | `validate_parts` (9 sub-checks) + `validate_budget` | `validate_parts` is pure; blake3/postcard are externals |
| Gate 2 — Checksum | `blake3::hash(&postcard::to_allocvec(...))` vs `workflow.digest()` | Hash comparison | blake3 and postcard are stubbed in Kani harness |

**gate_count**: Relaxed→0, Journaled/Strict→2
**durable**: Strict→true, others→false

---

## Proof Flag Derivation

| Flag | Derivation | Current status |
|------|------------|----------------|
| `bounded` | `BoundednessPolicy::DEFAULT.validate(&budget)` success → `true` | Default `true` |
| `taint_safe` | Not yet derived from ActionContract | Default `true` (WAIVER) |
| `retry_safe` | Not yet derived from ActionContract | Default `true` (WAIVER) |
| `replayable` | Not yet derived from ActionContract | Default `true` (WAIVER) |
| `idempotency_keyed` | Not yet populated | Empty |
| `idempotency_attested` | Not yet populated | Empty |

**WAIVER-FLAG-DERIV** applies to all six flags. Compensating evidence: BDD tests cover policy behavior; gate_count and durable are primary admission signals.

---

## Verifier Lane Selection

| Obligation | Verifier | Rationale |
|------------|----------|-----------|
| V-PF-001: VerificationProof::new field correspondence | Verus | Pure constructor; Rust-local invariant |
| V-PF-002: VerificationWarning::is_valid gate range | Verus | Simple const-bound invariant; cheaper than Kani |
| V-G1-001: try_from_parts postconditions | Verus | Pure validation; Rust-local |
| V-G1-002: validate_budget boundedness | Verus | Policy struct is pure; bounded arithmetic |
| V-G2-001: checksum postconditions | Verus | Hash equality is pure spec |
| V-POL-001: policy dispatch (Relaxed/Journaled/Strict) | Verus | Enum dispatch correctness |
| K-G2-001: checksum mismatch → error (no panic/UB) | Kani | Bounded input space (byte slice); codec bug → panic risk |
| K-G1-001: invalid parts → no panic in try_from_parts | Kani | Bounded set of invalid inputs |
| TEST-POL-001/002/003: policy gate_count/durable | cargo test | Happy-path coverage; fast feedback |
| TEST-WARN-001: VerificationWarning::is_valid range | cargo test | Boundary values 0,1,2,3 |
| TEST-BDD-001: BDD policy scenarios | cargo test | Scenario coverage |
| MIRI-001: admission path UB check | Miri | Postcard serialization + blake3 |
| PROP-G1-001: structure validation edge cases | proptest | Broad input space |

---

## Verification Architecture

```
VerificationProof (struct with 10 fields)
├── digest         — from workflow.digest() input
├── gate_count     — policy-derived: Relaxed=0, Journaled/Strict=2
├── durable        — policy-derived: Strict=true, others=false
├── bounded        — from validate_budget success (V-G1-002)
├── taint_safe     — default true (WAIVER-FLAG-DERIV)
├── retry_safe     — default true (WAIVER-FLAG-DERIV)
├── replayable     — default true (WAIVER-FLAG-DERIV)
├── idempotency_keyed    — empty (WAIVER-FLAG-DERIV)
├── idempotency_attested — empty (WAIVER-FLAG-DERIV)
└── warnings       — Vec<VerificationWarning> (INV-002)

Gate 1 (structure):
  CompiledWorkflow::try_from_parts(parts.clone())
    └─ validate_parts (9 sub-checks, all pure)
    └─ validate_budget → BoundednessPolicy::DEFAULT.validate(&budget)

Gate 2 (checksum):
  blake3::hash(&postcard::to_allocvec(parts_with_zeroed_digest()))
    ≡ workflow.digest()
```

---

## Command Inventory

| # | Command | Evidence target |
|---|---------|----------------|
| 1 | `moon run :verify-proof` | verus-report.md |
| 2 | `cargo kani -p vb_storage --no-default-features --tests` | kani-report.md |
| 3 | `cargo kani -p vb_core --no-default-features --tests` | kani-report.md |
| 4 | `cargo test -p vb_storage submit_artifact_relaxed` | test-output.txt |
| 5 | `cargo test -p vb_storage submit_artifact_journaled` | test-output.txt |
| 6 | `cargo test -p vb_storage submit_artifact_strict` | test-output.txt |
| 7 | `cargo test -p vb_storage warning gate is_valid` | test-output.txt |
| 8 | `cargo test -p vb_storage bdd_relaxed bdd_journaled bdd_strict` | test-output.txt |
| 9 | `MIRIFLAGS="-Zmiri-tag-raw-pointers" cargo miri test -p vb_storage` | miri-report.md |
| 10 | `cargo test -p vb_core submit_artifact_property_tests` | proptest-report.md |

---

## Waiver Summary

| ID | Scope | Reason | Expiry |
|----|-------|--------|--------|
| WAIVER-FLAG-DERIV | taint_safe, retry_safe, replayable, idempotency_keyed, idempotency_attested | ActionContract flag derivation not yet wired; conservative defaults (all true) | When action-contract analysis is implemented |

Compensating evidence: BDD tests cover policy-level admission; gate_count and durable are the primary admission signals.

---

## State Machine

- owner_state: 4 (proof planning)
- rerun_from: 4 (proof execution)
- Status transitions: planned → executing → evidence-pending → reviewed
