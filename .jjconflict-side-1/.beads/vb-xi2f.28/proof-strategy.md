# Proof Strategy — Digest Coverage of `for_each` Semantics

**Bead:** vb-xi2f.28
**State:** 4 (proof-planner)
**Date:** 2026-05-25
**Status:** PLANNED

---

## 1. Executive Summary

This bead fixes a digest coverage gap in `canonical_digest()` where `StepPrimitive::ForEach` fields (`variable`, `input`, `at_once`, `body`) are not hashed. The fix adds an explicit `ForEach` match arm in `digest_step_primitive()` in two files. The proof strategy targets **four risk dimensions**:

1. **Field sensitivity** (AC-FE-01 through AC-FE-04): Prove that changing each ForEach field changes the digest.
2. **Determinism** (AC-FE-05): Prove the pure function remains deterministic.
3. **Dual-path equivalence** (AC-FE-06): Prove both compilation paths produce identical digests.
4. **Non-regression** (AC-FE-08): Prove existing Set/Finish behavior is unchanged.

The strategy uses **Kani** for bounded verification of field coverage, determinism, and boundary-collision resistance; **proptest** for broad input-space validation of sensitivity, dual-path equivalence, and non-regression.

## 2. Risk Classification

| Risk | Severity | Verifier | Rationale |
|---|---|---|---|
| Digest coverage gap (field sensitivity) | P0 | Kani + proptest | Kani proves field reaches hasher; proptest validates across random inputs |
| Determinism regression | P0 | Kani + proptest | Kani proves determinism for bounded inputs; proptest validates across 1000+ runs |
| Duplicate code divergence | P0 | proptest | Cross-path equivalence test; both copies compiled and compared |
| Non-exhaustive catch-all (silent gap) | P0 | Kani (compile-time guard via destructuring) | Rust match destructuring enforces field exhaustiveness |
| Boundary collision | P1 | Kani | Bounded proof that delimiter byte `:` is absent from YAML identifiers |
| Semantic equivalence | P2 | Kani | Bounded proof that `None` and `Some(1)` hash identically |

## 3. Verifier Lane Summary

### 3.1 Required Lanes (8 Kani + 7 proptest = 15 obligations)

| Verifier | Obligations | Covers Seeds |
|---|---|---|
| **kani** | 8 | PS-FE-01, PS-FE-02, PS-FE-03, PS-FE-04, PS-FE-05, PS-FE-07, PS-FE-09, PS-FE-10 |
| **proptest** | 7 | PS-FE-01, PS-FE-02, PS-FE-03, PS-FE-04, PS-FE-05, PS-FE-06, PS-FE-08 |

### 3.2 Non-Applicable Lanes (with evidence)

| Verifier | Evidence |
|---|---|
| **tla-plus** | No temporal or distributed state; `canonical_digest` is a pure, stateless function (see boundary-map.md §1) |
| **verus** | No deep mathematical invariants; properties are behavioral equality and determinism on bounded inputs, not complex type-level theorems |
| **flux-rs** | Rust destructuring match enforces field exhaustiveness at compile time; Flux would need to model `blake3::Hasher` state which is infeasible |
| **loom** | No concurrency, atomics, channels, or shared state; pure function with immutable references (see hazard-analysis.md §5, HZ-C01) |
| **miri** | No unsafe code, FFI, raw pointers, or provenance risk in digest computation (see boundary-map.md §4) |
| **cargo-fuzz** | `canonical_digest` consumes a parsed AST, not raw bytes; fuzzing at the byte level would fuzz the parser (out of scope). Structured random input is covered by proptest. |

## 4. Proof Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    PROOF ARCHITECTURE                         │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  Layer 1: Bounded Verification (Kani)                        │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ PO-K-FE-01: input.as_bytes() reaches hasher             │ │
│  │ PO-K-FE-02: variable.as_bytes() reaches hasher          │ │
│  │ PO-K-FE-03: at_once.unwrap_or(1) reaches hasher          │ │
│  │ PO-K-FE-04: body steps reach hasher recursively          │ │
│  │ PO-K-FE-05: determinism (same input → same hasher state) │ │
│  │ PO-K-FE-07: None/Some(1) equivalence                     │ │
│  │ PO-K-FE-09: exhaustive field coverage (all 4 fields)     │ │
│  │ PO-K-FE-10: delimiter collision resistance               │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                               │
│  Layer 2: Broad Input Validation (proptest)                  │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ PO-P-FE-01: Random input variations → digest uniqueness  │ │
│  │ PO-P-FE-02: Random at_once variations → digest uniqueness│ │
│  │ PO-P-FE-03: Random variable names → digest uniqueness    │ │
│  │ PO-P-FE-04: Random body content → digest uniqueness      │ │
│  │ PO-P-FE-05: Determinism across 1000+ re-compiles         │ │
│  │ PO-P-FE-06: Dual-path equivalence (both copies match)    │ │
│  │ PO-P-FE-08: Non-regression: Set/Finish digests unchanged │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                               │
│  Compile-Time Defense (Rust destructuring)                    │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ StepPrimitive::ForEach { variable, input, at_once, body }│ │
│  │ ⇒ compiler forces all fields to be mentioned             │ │
│  │ ⇒ #![deny(unused_variables)] in test module              │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

## 5. Trusted Base

| Component | Trust Level | Reason |
|---|---|---|
| `blake3::Hasher` | Trusted | Known-correct cryptographic library; deterministic by design |
| `WorkflowDigest` | Trusted | Simple newtype; no logic beyond byte storage |
| Rust compiler (match exhaustiveness) | Trusted | Language-level guarantee |
| YAML parser (`vb_yaml`) | Trusted (out of scope) | Parser correctness is independent of digest sensitivity |

## 6. What Is NOT Proved

- Other primitives (Collect, Aggregate, Repeat, etc.) retain digest gaps — out of scope per DD-01
- The two `canonical_digest` copies remain duplicated — maintenance risk accepted for this bead (separate refactoring bead)
- Performance of digest computation with large bodies — not a correctness concern
- The `compute_compiled_digest` function — already correct, out of scope

## 7. Waiver Candidates

No behavior-affecting waivers. All behavior-affecting claims (field sensitivity, determinism, dual-path equivalence) are fully covered by Kani or proptest obligations.

The one non-behavior waiver candidate:
- **WC-FE-01**: Tooling availability. If `cargo kani` cannot be installed in CI, the Kani obligations may need to be downgraded to proptest-only with explicit acceptance of reduced bounded-proof coverage. This is a tooling/environment concern, not a behavioral one.

## 8. Proportionality Statement

This is a P1 digest sensitivity bead with narrow scope (two files, one function, one primitive variant). The proof plan is proportional:
- 15 planned obligations across 2 verifiers
- 6 non-applicable lanes with concrete evidence
- 1 non-behavior waiver candidate for tooling
- No TLA+, Verus, Flux, Loom, Miri, or fuzz required (all justified)

The most expensive obligation is PO-P-FE-06 (dual-path equivalence proptest) which requires compiling through both code paths. All other obligations are lightweight Kani harnesses or proptest properties.
