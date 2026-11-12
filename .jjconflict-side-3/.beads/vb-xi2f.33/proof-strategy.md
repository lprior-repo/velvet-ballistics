# Proof Strategy — vb-xi2f.33: Digest Covers Ask Semantics

**Bead**: `vb-xi2f.33` / P1: digest covers ask semantics
**State**: 4 (proof-planner)
**Scope**: Proportional to P1 — a 3-line match-arm addition in two duplicate files.

## Risk Classification

| Risk | Classification | Primary Lane |
|------|---------------|--------------|
| semantic-integrity (digest ignore ask fields) | Rust-local invariant, bounded state | Kani + proptest |
| panic-freedom (holzman-rust) | Bounded state, no unsafe | Kani |
| determinism (field ordering) | Rust-local invariant | Kani + proptest |
| edge-case (empty prompt, None vs Some("")) | Bounded edge case | Kani + unit-test |
| regression (Set/Finish unchanged) | Defensive regression | unit-test |
| code-duplication (two copies) | Maintenance/parity | unit-test |
| security (digest collision enables workflow substitution) | Input boundary | cargo-fuzz |
| public-api (digest part of CompiledWorkflow) | API contract | proptest |

## Defense-in-Depth Architecture

```
Layer 0 (compile-time): Static analysis — explicit match arm exhaustiveness (PS-ASK-010)
Layer 1 (bounded proof):  Kani — panic-freedom, field sensitivity, edge cases (PS-ASK-001/002/004/005/008/009)
Layer 2 (broad property): proptest — digest sensitivity across random inputs (PS-ASK-001/002/003/008)
Layer 3 (adversarial):    cargo-fuzz — fuzz canonical_digest with generated sources (PS-ASK-001)
Layer 4 (behavior test):  State 8 test-planner — Set/Finish regression, duplicate parity, explicit arm, end-to-end digest correctness (PS-ASK-006/007/010 + traceability scenarios)
```

## Verifier Lane Summary

| Verifier | Status | Obligations | Rationale |
|----------|--------|-------------|-----------|
| **Kani** | required | 6 obligations | Primary formal lane: bounded state, panic-proof, hash collision proofs |
| **proptest** | required | 4 obligations | Broad input space for prompt/timeout sensitivity |
| **cargo-fuzz** | required | 1 obligation | Adversarial input boundary (YAML sources with Ask) |
| **behavior-test** | delegated (S8) | test-planner | Regression, parity, explicit arm, end-to-end scenarios from traceability-matrix.jsonl |
| **TLA+** | not_applicable | 0 | No temporal/state-machine/distributed properties; pure deterministic hash |
| **Verus** | not_applicable | 0 | P1 scope; 3-line fix; full Verus hash-state proof is disproportionate |
| **Flux** | not_applicable | 0 | No refinement-type properties; fix is structural (match arm), not numeric |
| **Loom** | not_applicable | 0 | No concurrency, threads, channels, or async in digest path |
| **Miri** | not_applicable | 0 | No unsafe code, FFI, raw pointers, or interior mutability in digest path |
| **mutation** | not_applicable | 0 | Covered by CI `cargo-mutants` on the broaden scope; not seed-specific |

## Proof Seeds Coverage

All 10 proof seeds from `proof-seeds.jsonl` are covered by at least one required obligation:

| Seed | Risk | Kani | proptest | fuzz | Behavior (S8) |
|------|------|------|----------|------|----------------|
| PS-ASK-001 (prompt sensitivity) | semantic-integrity | PO-KANI-001 | PO-PROPTEST-001 | PO-FUZZ-001 | delegated |
| PS-ASK-002 (timeout sensitivity) | semantic-integrity | PO-KANI-002 | PO-PROPTEST-002 | — | delegated |
| PS-ASK-003 (determinism) | determinism | — | PO-PROPTEST-003 | — | delegated |
| PS-ASK-004 (empty prompt) | edge-case | PO-KANI-003 | — | — | delegated |
| PS-ASK-005 (None vs Some("")) | edge-case | PO-KANI-004 | — | — | delegated |
| PS-ASK-006 (duplicate parity) | code-duplication | — | — | — | delegated (primary) |
| PS-ASK-007 (Set/Finish regression) | regression | — | — | — | delegated (primary) |
| PS-ASK-008 (field ordering) | determinism | PO-KANI-005 | PO-PROPTEST-004 | — | delegated |
| PS-ASK-009 (panic-freedom) | holzman-rust | PO-KANI-006 | — | — | — |
| PS-ASK-010 (explicit arm) | exhaustiveness | — | — | — | delegated + static review |

## Trusted Base

| Artifact | Kind | Reason |
|----------|------|--------|
| blake3::Hasher | trusted dependency | Cryptographic hash determinism; foundational assumption (HAZ-005) |
| YAML parser (vb_yaml) | trusted boundary | Validates prompt/timeout types before digest receives them (A2) |
| String::as_bytes() | stdlib trust | Bytes are deterministic for same String value (A4) |
| b"no_timeout" sentinel | design assumption | Non-collision with valid timeout string bytes (A5); verified by Kani PO-KANI-004 |
| Fix applied to both copies | process assumption | Duplicate parity enforced by PO-UT-003 (A3) |

## Non-Applicable Waiver Candidates

| Verifier | Reason |
|----------|--------|
| TLA+ | No temporal or distributed state; digest is pure deterministic function |
| Verus | P1 scope; 3-line match-arm fix; Kani covers bounded invariants at appropriate cost |
| Flux | No refinement-type properties; fix is structural not numeric |
| Loom | No concurrent code in digest path |
| Miri | No unsafe code in digest path; all operations are safe Rust |

## Execution Constraints

- All Kani proofs must use bounded unwinding (max prompt length limited by Kani performance).
- Proptest strategies must generate valid WorkflowSource values (not arbitrary bytes).
- Cargo-fuzz must target `canonical_digest` directly with well-formed WorkflowSource inputs.
- Behavior tests (State 8/9) must cover both `part_05.rs` and `compile/mod.rs` implementations per traceability-matrix.jsonl scenarios.
