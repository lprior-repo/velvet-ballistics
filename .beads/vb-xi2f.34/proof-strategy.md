# Proof Strategy — vb-xi2f.34: Finish Digest Coverage

**Bead**: vb-xi2f.34  
**Phase**: p4-proof-planner  
**Date**: 2026-05-24  
**Boundary**: `vb_compile::mod_compile_lowering::part_05` (canonical) + `vb_compile::compile::mod` (legacy)  

---

## Strategy Summary

The `Finish` primitive's `result: ScalarValue` field is hashed into `WorkflowDigest` via `digest_step_primitive()`. The bead's P1 scope is to prove that this hashing is correct, sensitive, and deterministic. The strategy uses a **defense-in-depth** approach combining:

1. **Kani bounded proofs** (primary formal layer) — prove injectivity of the encoding for `String` and `Integer` variants within bounded input spaces.
2. **Proptest property tests** (statistical defense layer) — verify determinism, sensitivity, and transition properties across generated ASTs.
3. **Integration tests** (end-to-end layer) — verify digest preservation through the full `compile_source` → `CompiledWorkflow::digest()` pipeline.
4. **Static analysis & code review** (structural layer) — enforce exhaustiveness, unsafe absence, and code consolidation.

This is proportional to P1 scope: the `digest_step_primitive` function is ~22 lines, the `Finish` arm is 8 lines. The strategy focuses on behavioral correctness of existing code, not re-architecture.

---

## Lane Decisions (Summary)

| Verifier | Decision | Seeds Covered | Rationale |
|---|---|---|---|
| **Kani** | REQUIRED | PS-DIGEST-001, PS-DIGEST-002, PS-DIGEST-009 | Bounded injectivity proof for Finish digest encoding |
| **Proptest** | REQUIRED | PS-DIGEST-001, PS-DIGEST-003, PS-DIGEST-010 | Statistical validation of determinism and sensitivity |
| **Integration Test** | REQUIRED | PS-DIGEST-002, PS-DIGEST-003, PS-DIGEST-006, PS-DIGEST-004 | End-to-end digest preservation + equivalence |
| **Static Analysis** | REQUIRED | PS-DIGEST-005, PS-DIGEST-008 | Exhaustiveness enforcement, unsafe audit |
| **TLA+** | NOT_APPLICABLE | — | Digest is pure, stateless; pipeline is sequential, no interleavings |
| **Verus** | NOT_APPLICABLE | — | Uses external `blake3::Hasher`; properties are behavioral not type-refinement |
| **Flux** | NOT_APPLICABLE | — | No data-level refinement constraints; types already sound |
| **Loom** | NOT_APPLICABLE | — | No concurrency in digest subsystem |
| **Miri** | NOT_APPLICABLE | — | `#![forbid(unsafe_code)]`; no unsafe in digest path |
| **cargo-fuzz** | NOT_APPLICABLE | — | Digest operates on typed AST, not raw bytes; parser fuzzing is separate concern |

---

## Proof Layering

```
┌──────────────────────────────────────────────────┐
│ LAYER 1: Kani Bounded Proofs (injectivity)        │
│  PO-KANI-001: String result injectivity           │
│  PO-KANI-002: Integer result injectivity          │
│  PO-KANI-003: ScalarValue variant discrimination  │
├──────────────────────────────────────────────────┤
│ LAYER 2: Proptest Statistical Validation           │
│  PO-PROPTEST-001: Determinism                     │
│  PO-PROPTEST-002: Finish result sensitivity       │
│  PO-PROPTEST-003: Step position sensitivity       │
├──────────────────────────────────────────────────┤
│ LAYER 3: Integration / End-to-End                  │
│  PO-INT-001: Finish value → compiled digest       │
│  PO-INT-002: Finish step ID → compiled digest     │
│  PO-INT-003: Finish result type → compiled digest │
│  PO-INT-004: Legacy/canonical equivalence         │
├──────────────────────────────────────────────────┤
│ LAYER 4: Structural / Static                       │
│  PO-STATIC-001: ScalarValue exhaustiveness        │
│  PO-STATIC-002: Unsafe audit                      │
└──────────────────────────────────────────────────┘
```

---

## Trusted Base

See `trusted-base-plan.md` for complete assumptions register. Key items:

- `blake3::Hasher` is deterministic and collision-resistant (not proven; assumed per crate specification)
- `i64::to_le_bytes()` is bijective (proven by Rust type system; accepted)
- `String::as_bytes()` returns deterministic UTF-8 bytes for a given `String` value
- `#[non_exhaustive]` enum match semantics are correct per Rust compiler (trusted)
- Parser (`vb_yaml`) produces consistent `WorkflowSource` AST for equivalent YAML (out of scope for this bead)

---

## Waiver Candidates

One non-behavior waiver candidate identified:

- **WC-001**: `canonical_primitive_name` known bugs (`Together` → `"parallel"`, `Aggregate` → `"aggregate"`) are out of scope for Finish digest bead. These bugs do not affect Finish (which has its own match arm in `digest_step_primitive`). See `waiver-candidates.jsonl`.

---

## Implementation Bridge

See `proof-to-implementation-input.md` for mapping proof claims to Rust source refs, test file locations, and exact commands.

---

## Risk Classification Summary

| Proof Seed | Primary Risk | Classification | Primary Lane |
|---|---|---|---|
| PS-FINISH-DIGEST-001 | digest-sensitivity | Invariant / Bounded | Kani |
| PS-FINISH-DIGEST-002 | hash-collision | Refinement / Bounded | Kani |
| PS-FINISH-DIGEST-003 | determinism | Invariant | Proptest |
| PS-FINISH-DIGEST-004 | duplicate-code | Release/API | Integration |
| PS-FINISH-DIGEST-005 | forward-compatibility | Refinement | Static Analysis |
| PS-FINISH-DIGEST-006 | digest-preservation | Invariant / Temporal | Integration |
| PS-FINISH-DIGEST-007 | digest-scope | Refinement | Proptest |
| PS-FINISH-DIGEST-008 | over-hashing | Invariant | Static Audit |
| PS-FINISH-DIGEST-009 | injective-hashing | Invariant / Bounded | Kani |
| PS-FINISH-DIGEST-010 | step-ordering | Invariant | Proptest |
