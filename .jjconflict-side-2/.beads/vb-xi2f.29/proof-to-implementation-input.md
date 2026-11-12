# Proof-to-Implementation Bridge Input: vb-xi2f.29

**Bead**: vb-xi2f.29 — Digest Covers Together Semantics
**Planner invocation**: p4-plan-vb-xi2f.29-001
**Date**: 2026-05-24

## Overview

This document provides the proof-to-implementation bridge with the mapping from planned proof obligations to Rust source locations, test harnesses, and verification commands. The implementation surface is small: one new `Together` arm in `digest_step_primitive` and one new recursive `digest_sub_step` function.

## Source Changes Required

### 1. `crates/vb_compile/src/mod_compile_lowering/part_05.rs`

**Line 140-161 (digest_step_primitive)**: Add explicit `Together` arm before the `other` wildcard:

```rust
vb_yaml::ast::StepPrimitive::Together { branches } => {
    hasher.update(b"together");
    hasher.update(&(branches.len() as u16).to_le_bytes());
    for branch in branches {
        hasher.update(branch.label.as_bytes());
        for step in &branch.steps {
            digest_sub_step(hasher, step);
        }
    }
}
```

**New function**: `digest_sub_step` that recursively hashes a `StepAst`:

```rust
fn digest_sub_step(hasher: &mut blake3::Hasher, step: &vb_yaml::ast::StepAst) {
    hasher.update(step.id.as_bytes());
    digest_step_primitive(hasher, &step.primitive);
}
```

**Note**: `canonical_primitive_name(Together)` at line 105 has been fixed (REPAIR-2). The production code now returns `"together"`. This was tracked as the CANONICAL_NAME_BUG (C-01) and is now resolved per Kani evidence (PO-001 VERIFIED, 0/432 failed).

### 2. No changes to `crates/vb_compile/src/kani_canonical_name.rs`

Existing harnesses are already correct. They expect `"together"` which is already returned by line 105.

## Proof Claims → Implementation Mapping

| Proof Obligation | Proof Claim | Rust Source Target | Behavior Test | Refinement Harness |
|---|---|---|---|---|
| PO-xi2f29-001 | canonical_primitive_name(Together) == "together" | part_05.rs:105 | PO-015 (unit) | kani_canonical_name.rs:42-62 |
| PO-xi2f29-002 | Branch count affects digest | part_05.rs:140-162 Together arm | together_digest_sensitivity.rs (proptest) | PO-010 (kani) |
| PO-xi2f29-003 | Branch labels affect digest | part_05.rs:140-162 Together arm | together_digest_sensitivity.rs (proptest) | — |
| PO-xi2f29-004 | Sub-step contents affect digest | new digest_sub_step fn | together_digest_sensitivity.rs (proptest) | PO-010 (kani) |
| PO-xi2f29-005 | Branch ordering affects digest | part_05.rs:140-162 Together arm (loop) | together_digest_sensitivity.rs (proptest) | — |
| PO-xi2f29-006 | Digest determinism preserved | part_05.rs:116-138 canonical_digest | v1_primitive_lowering.rs (existing proptest) | — |
| PO-xi2f29-007 | Non-Together digests don't regress | part_05.rs:98-162 (all paths unchanged except Together arm) | v1_primitive_lowering.rs (existing proptest) | — |
| PO-xi2f29-008 | Exhaustive variant match | part_05.rs:98-114 canonical_primitive_name | — | kani_canonical_name.rs:121-175 |
| PO-xi2f29-009 | Recursion bounded | new digest_sub_step fn | — | together_digest_kani.rs (new) |
| PO-xi2f29-010 | Together arm deterministic | part_05.rs:140-162 Together arm | — | together_digest_kani.rs (new) |
| PO-xi2f29-011 | Empty branches deterministic | new digest_sub_step fn | error_variant_tests.rs (unit) | — |
| PO-xi2f29-012 | Nested together recursion | new digest_sub_step fn | error_variant_tests.rs (unit) | — |
| PO-xi2f29-013 | Digest idempotency | part_05.rs:116-138 canonical_digest | error_variant_tests.rs (unit) | — |
| PO-xi2f29-014 | Together structural coverage | part_05.rs:140-162 Together arm + digest_sub_step | error_variant_tests.rs (unit) | — |
| PO-xi2f29-015 | canonical_primitive_name Together unit | part_05.rs:105 | error_variant_tests.rs (unit) | — |

## New Verification Artifacts Needed

| Artifact | Type | Location | Description |
|---|---|---|---|
| together_digest_sensitivity.rs | proptest (integration test) | crates/vb_compile/tests/ | Proptest file with 6 property functions (PO-002 through PO-007) |
| together_digest_kani.rs | kani harness | crates/vb_compile/src/ | Kani harness file with 2 harness functions (PO-009, PO-010) |
| error_variant_tests.rs | unit tests (extend) | crates/vb_compile/src/tests/ | 5 new unit tests (PO-011 through PO-015) |

## Evidence Commands (for proof-writer)

```bash
# Kani - canonical name regression gate
TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile --harness canonical_name_together_harness --no-unwind

# Kani - exhaustive name verification
TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile --harness canonical_name_all_harness --no-unwind

# Kani - recursion bound verification
TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile --harness together_digest_sub_step_recursion_bounded_kani

# Kani - digest determinism with symbolic together
TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile --harness together_digest_step_deterministic_kani

# Proptest - all together sensitivity properties
cargo test -p vb_compile --test together_digest_sensitivity -- --nocapture

# Proptest - existing regression gate
cargo test -p vb_compile --test v1_primitive_lowering -- --nocapture

# Unit - all 5 together digest unit tests
cargo test -p vb_compile --lib tests::error_variant_tests -- --nocapture
```

## Trusted Base Bindings

| Trust Marker | Required Binding | Evidence |
|---|---|---|
| TB-xi2f29-003 (blake3) | blake3 version unchanged; deterministic | Cargo.lock hash |
| TB-xi2f29-004 (name fix) | part_05.rs:105 returns "together" | PO-001 + PO-015 evidence |
| TB-xi2f29-005 (nesting depth) | MAX_LANGUAGE_NESTING_DEPTH = 8 | vb_core/src/limits.rs:63 |
| TB-xi2f29-006 (recursion bound) | digest_sub_step depth ≤ MAX_LANGUAGE_NESTING_DEPTH | PO-009 evidence |

## Out-of-Scope for Implementation

- Do NOT add Together arm handling to `compile/mod.rs` dead code
- Do NOT change other primitives' digest paths (for_each, collect, aggregate, repeat)
- Do NOT change `Aggregate` canonical name (`"aggregate"` → not in scope)
- Do NOT add `StepAst.field`-level hashing beyond `id` and `primitive`
- Do NOT change `compute_compiled_digest` (byte-level digest, different function)
