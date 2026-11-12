# Proof Strategy: Digest Covers Collect Semantics (vb-xi2f.38)

## Executive Summary

**Critical Bug**: `digest_step_primitive` uses catch-all `other => canonical_primitive_name(other)` for `StepPrimitive::Collect`, only hashing the string `"collect"` and ignoring all semantically significant fields: `variable`, `source`, `pages`, `items`, `body`.

**Proof Objective**: Demonstrate that two `Collect` primitives with different fields produce different digest contributions (post-fix), and that the current implementation fails this property (pre-fix).

---

## Core Problem

### Affected Functions (Identical Bug in Two Locations)
1. `vb_compile/src/mod_compile_lowering/part_05.rs` lines 158–160
2. `vb_compile/src/compile/mod.rs` lines 257–259

### Current (Buggy) Code
```rust
other => {
    hasher.update(canonical_primitive_name(other).as_bytes());
}
```

### Required Fix
```rust
StepPrimitive::Collect { variable, source, pages, items, body } => {
    hasher.update(b"collect");
    hasher.update(variable.as_bytes());
    hasher.update(source.as_bytes());
    pages.map_or(0u32, |p| hasher.update(&p.to_le_bytes()));
    items.map_or(0u32, |i| hasher.update(&i.to_le_bytes()));
    for step in body {
        hasher.update(step.id.as_bytes());
        digest_step_primitive(hasher, &step.primitive);
    }
}
```

---

## Verifier Lane Selection

### Primary Lane: Kani (Bounded Model Checking)
- **Rationale**: Bounded state space — `Collect` has fixed fields, all combinatorially exhaustible
- **Kills**: H-1 (Collect field coverage), H-2 (ForEach/Aggregate coverage), H-9 (GOD RULE: no hardcoded shapes)
- **Proof**: `kani::any::<StepPrimitive::Collect>()` harness proving field-differential digest

### Secondary Lane: Proptest (Property-Based Testing)
- **Rationale**: Broad input space, cross-run determinism, field-differential equality
- **Kills**: CC-DIGEST-002 (determinism), CC-DIGEST-007 (equality property), H-2 (ForEach/Aggregate)
- **Proof**: `proptest` generating arbitrary Collect pairs, asserting digest equality/inequality

### Tertiary Lane: TLA+ (Formal Invariant)
- **Rationale**: Collect digest coverage as formal invariant; step ordering
- **Kills**: CC-DIGEST-001 (invariant), CC-DIGEST-001b (step ID coverage)
- **Proof**: `CollectDigestCoverage` invariant in TLA+ spec

### Supporting Lane: Verus (Collect Lowering Correctness)
- **Rationale**: `lower_canonical_collect` must emit correct 4-node sequence
- **Kills**: CC-DIGEST-004 (lowering semantics)
- **Proof**: `lemma_lower_canonical_collect_emits_4_nodes`

### Not Applicable Lanes
- **Flux**: Digest coverage is not naturally a refinement/type-state property
- **Loom**: No concurrent interleavings in digest computation
- **Miri**: No unsafe code in `digest_step_primitive`
- **cargo-fuzz**: Digest function is deterministic pure function, not a parser/security boundary
- **Gauntlet**: Not applicable to this bead scope

---

## Risk Classification

| Risk | Severity | Category | Verifier | Priority |
|------|----------|----------|----------|----------|
| H-1: Collect fields not hashed | **CRITICAL** | digest-coverage | Kani + Proptest + TLA+ | P0 |
| H-2: ForEach/Aggregate same bug | **HIGH** | digest-coverage | Kani + Proptest | P1 |
| H-9: Hardcoded harness data | **CRITICAL** | GOD RULE | Kani | P0 |
| CC-DIGEST-002: Non-determinism | HIGH | determinism | Proptest + TLA+ | P1 |
| CC-DIGEST-004: Lowering semantics | MEDIUM | refinement | Verus | P2 |
| CC-DIGEST-005: Digest mismatch detection | MEDIUM | storage | Integration test | P2 |
| H-4: Lowering non-determinism | MEDIUM | refinement | TLA+ | P2 |
| H-5: Serialization non-determinism | MEDIUM | codec | Proptest | P2 |

---

## Proof Obligations Summary

| ID | Requirement | Verifier | Artifact | Mode |
|----|-------------|----------|----------|------|
| PO-001 | CC-DIGEST-001: Collect field coverage | TLA+ | `verification/tla/collect_body_model.tla` | model-check |
| PO-002 | CC-DIGEST-001: Collect field coverage (pre-fix bug) | Kani | `verification/kani/collect_field_coverage.rs` | bounded-proof |
| PO-003 | CC-DIGEST-001a: Variable field hashing | Proptest | `crates/vb_compile/src/tests/digest_collect_tests.rs` | property-test |
| PO-004 | CC-DIGEST-001a: Source field hashing | Proptest | `crates/vb_compile/src/tests/digest_collect_tests.rs` | property-test |
| PO-005 | CC-DIGEST-001a: Pages field hashing | Proptest | `crates/vb_compile/src/tests/digest_collect_tests.rs` | property-test |
| PO-006 | CC-DIGEST-001a: Items field hashing | Proptest | `crates/vb_compile/src/tests/digest_collect_tests.rs` | property-test |
| PO-007 | CC-DIGEST-001a: Body recursive hashing | Proptest | `crates/vb_compile/src/tests/digest_collect_tests.rs` | property-test |
| PO-008 | CC-DIGEST-001b: Step ID coverage | TLA+ | `verification/tla/collect_body_model.tla` | model-check |
| PO-009 | CC-DIGEST-002: Digest determinism | Proptest | `crates/vb_compile/src/tests/error_variant_tests.rs` | property-test |
| PO-010 | CC-DIGEST-003: Artifact digest dependency | Proptest | `crates/vb_compile/src/tests/error_variant_tests.rs` | property-test |
| PO-011 | CC-DIGEST-004: Lowering correctness | Verus | `verification/verus/collect_lowering.rs` | proof |
| PO-012 | CC-DIGEST-004: Lowering IR structure | TLA+ | `verification/tla/collect_body_model.tla` | model-check |
| PO-013 | CC-DIGEST-006: No panic on Collect | Kani | `verification/kani/collect_try_from_parts.rs` | bounded-proof |
| PO-014 | CC-DIGEST-007: Property equality | Proptest | `crates/vb_compile/src/tests/digest_collect_tests.rs` | property-test |
| PO-015 | H-2: ForEach field hashing | Kani | `verification/kani/foreach_field_coverage.rs` | bounded-proof |
| PO-016 | H-2: Aggregate field hashing | Kani | `verification/kani/aggregate_field_coverage.rs` | bounded-proof |
| PO-017 | H-4: Lowering determinism | TLA+ | `verification/tla/collect_body_model.tla` | model-check |
| PO-018 | H-5: Serialization determinism | Proptest | `crates/vb_compile/src/tests/error_variant_tests.rs` | property-test |
| PO-019 | H-9: GOD RULE — no hardcoded Collect | Kani | All Kani harnesses | bounded-proof |

---

## Trusted Base

### Explicitly Trusted Surfaces
1. **BLAKE3-256**: Trusted 256-bit hash function — no custom crypto
2. **postcard::serialize**: Deterministic CBOR serialization (proven by proptest)
3. **String::as_bytes**: Deterministic UTF-8 byte representation
4. **u32::to_le_bytes**: Deterministic little-endian encoding

### Explicitly Assumed Bounds
1. **Bounded Collect body**: `Vec<StepAst>` body is bounded by workflow compilation limits
2. **Bounded workflow steps**: `WorkflowSource.steps()` bounded by validation
3. **Bounded String lengths**: `variable` and `source` bounded by parser limits

### Model Reductions
1. **No concurrent digest computation**: Digest is single-threaded, no races
2. **No I/O in digest path**: Pure function, no storage/network
3. **Deterministic Option serialization**: `None → 0u32.to_le_bytes()`, `Some(p) → p.to_le_bytes()`

---

## Waiver Candidates

None at this time. All lanes are applicable. No behavior-affecting waivers are proposed.

---

## Pre-Fix vs Post-Fix Proof Strategy

### Pre-Fix (Bug Reproduction)
**Goal**: Demonstrate that current code FAILS the digest coverage property.

| Verifier | Harness | Expected Evidence |
|----------|---------|-------------------|
| Kani | `kani_collect_different_pages_same_digest` | `kani::cover!(digest_a == digest_b)` PASSES — proves bug exists |
| Proptest | `prop_collect_pages_different_same_digest` | `proptest::prop_assert_eq!(digest_a, digest_b)` PASSES — proves bug exists |

### Post-Fix (Correctness Verification)
**Goal**: Demonstrate that fixed code SATISFIES the digest coverage property.

| Verifier | Harness | Expected Evidence |
|----------|---------|-------------------|
| Kani | `kani_collect_field_coverage` | `kani::cover!(digest_a != digest_b)` PASSES — proves different fields → different digests |
| Proptest | `prop_collect_pages_different_digest_ne` | `proptest::prop_assert_ne!(digest_a, digest_b)` PASSES — proves fix |
| TLA+ | `CollectDigestCoverage` | TLC: invariant holds — no counterexample |

---

## Execution Order

1. **PO-013** (Kani: no-panic) — proves harness infrastructure works with `kani::any()`
2. **PO-002** (Kani: bug reproduction) — proves current code fails digest coverage
3. **PO-015/PO-016** (Kani: ForEach/Aggregate) — proves H-2 same-risk pattern
4. **PO-003–PO-007** (Proptest: field hashing) — proves each field contributes
5. **PO-009** (Proptest: determinism) — proves cross-run determinism
6. **PO-014** (Proptest: equality property) — proves a==b → digest_eq, a≠b → digest_ne
7. **PO-001/PO-008** (TLA+: invariants) — proves formal invariants hold
8. **PO-011** (Verus: lowering) — proves `lower_canonical_collect` correctness
9. **Full re-run** after fix to confirm all obligations pass
