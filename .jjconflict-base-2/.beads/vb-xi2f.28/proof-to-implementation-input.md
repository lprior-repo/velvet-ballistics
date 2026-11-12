# Proof-to-Implementation Input — Digest Coverage of `for_each` Semantics

**Bead:** vb-xi2f.28
**State:** 4 (proof-planner)
**Date:** 2026-05-25
**Status:** PLANNED

---

This document provides the bridge input from proof planning to the eventual `proof-to-implementation` state (State 7). It maps each proof claim to the Rust implementation artifacts that must satisfy those claims, along with behavior test and refinement harness references. The `proof-to-implementation` agent will materialize these into `rust-refinement-obligation/v1` rows.

## 1. Implementation Target Summary

| Implementation File | Symbol(s) | Actions Required |
|---|---|---|
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | `digest_step_primitive()`, `canonical_digest()` | Add `StepPrimitive::ForEach { variable, input, at_once, body }` match arm before `other` catch-all; hash all four fields in canonical order with `b":"` delimiters |
| `crates/vb_compile/src/compile/mod.rs` | `digest_step_primitive()`, `canonical_digest()` | Identical change to part_05.rs; must match field order and canonical representations exactly |

## 2. Proof Claim → Rust Source Mapping

### 2.1 Field Sensitivity Claims

| Proof Obligation | Rust Target | Source File | Line(s) | Claim |
|---|---|---|---|---|
| PO-K-FE-01, PO-P-FE-01 | `digest_step_primitive` ForEach arm | `mod_compile_lowering/part_05.rs` | ~140-162 | `hasher.update(input.as_bytes())` is called and contributes to digest |
| PO-K-FE-02, PO-P-FE-02 | `digest_step_primitive` ForEach arm | `mod_compile_lowering/part_05.rs` | ~140-162 | `hasher.update(&at_once.unwrap_or(1).to_le_bytes())` is called and contributes to digest |
| PO-K-FE-03, PO-P-FE-03 | `digest_step_primitive` ForEach arm | `mod_compile_lowering/part_05.rs` | ~140-162 | `hasher.update(variable.as_bytes())` is called and contributes to digest |
| PO-K-FE-04, PO-P-FE-04 | `digest_step_primitive` ForEach arm | `mod_compile_lowering/part_05.rs` | ~140-162 | Loop over body: `hasher.update(step.id.as_bytes())` + recursive `digest_step_primitive(&mut hasher, &step.primitive)` |
| PO-K-FE-01 through PO-K-FE-04 (duplicate) | `digest_step_primitive` ForEach arm | `compile/mod.rs` | ~243-261 | Identical ForEach hashing logic |

### 2.2 Determinism Claim

| Proof Obligation | Rust Target | Source File | Line(s) | Claim |
|---|---|---|---|---|
| PO-K-FE-05, PO-P-FE-05 | `canonical_digest()` | `mod_compile_lowering/part_05.rs` | ~116-138 | Pure function: no time, no rand, no HashMap. Same input → same WorkflowDigest always |
| PO-K-FE-05, PO-P-FE-05 (duplicate) | `canonical_digest()` | `compile/mod.rs` | ~220-241 | Identical pure function guarantee |

### 2.3 Dual-Path Equivalence Claim

| Proof Obligation | Rust Target | Source File | Claim |
|---|---|---|---|
| PO-P-FE-06 | Both `canonical_digest()` functions | `mod_compile_lowering/part_05.rs` AND `compile/mod.rs` | Both functions produce identical `WorkflowDigest` for identical `WorkflowSource` input |

### 2.4 Semantic Equivalence Claim

| Proof Obligation | Rust Target | Source File | Claim |
|---|---|---|---|
| PO-K-FE-07 | `digest_step_primitive` ForEach arm | `mod_compile_lowering/part_05.rs` | `at_once=None` and `at_once=Some(1)` both produce `hasher.update(&1u32.to_le_bytes())` |

### 2.5 Non-Regression Claim

| Proof Obligation | Rust Target | Source File | Claim |
|---|---|---|---|
| PO-P-FE-08 | `digest_step_primitive` Set/Finish arms | `mod_compile_lowering/part_05.rs` AND `compile/mod.rs` | Set and Finish hashing behavior unchanged after ForEach fix. ForEach arm does not alter Set/Finish digest computation. |

### 2.6 Exhaustiveness Claim

| Proof Obligation | Rust Target | Source File | Claim |
|---|---|---|---|
| PO-K-FE-09 | `digest_step_primitive` ForEach arm | `mod_compile_lowering/part_05.rs` | All four fields (`variable`, `input`, `at_once`, `body`) are consumed by `hasher.update()` in the ForEach match arm. |

### 2.7 Delimiter Safety Claim

| Proof Obligation | Rust Target | Source File | Claim |
|---|---|---|---|
| PO-K-FE-10 | `digest_step_primitive` ForEach arm | `mod_compile_lowering/part_05.rs` | Delimiter byte `0x3A` (`b':'`) never appears within YAML identifier field values, ensuring unambiguous field boundaries. |

## 3. Behavior Test References

Tests are planned by `test-planner` (State 8), not by `proof-planner`. The proof strategy identifies the following expected test targets for bridge mapping:

| Contract Clause | Expected Test(s) | Test Framework | Note |
|---|---|---|---|
| AC-FE-01 | `TST-FE-01`: input change → digest change | Integration (cargo test) | Per contract.md §3.1 |
| AC-FE-02 | `TST-FE-02`: at_once change → digest change | Integration | Per contract.md §3.1 |
| AC-FE-03 | `TST-FE-03`: variable change → digest change | Integration | Per contract.md §3.1 |
| AC-FE-04 | `TST-FE-04`: body change → digest change | Integration | Per contract.md §3.1 |
| AC-FE-05 | `TST-FE-05`: determinism | Integration | Extend existing `compiled_digest_is_deterministic` test |
| AC-FE-06 | `TST-FE-06`: dual-path equivalence | Cross-path integration | New test comparing both compilation paths |
| AC-FE-07 | `TST-FE-07`: None/Some(1) equivalence | Unit | Specific equivalence case |
| AC-FE-08 | `TST-FE-08`: regression Set/Finish | Regression | Verify existing tests still pass |

## 4. Existing Test Files for Extension

| File | Existing Tests | Extension Needed |
|---|---|---|
| `crates/vb_compile/src/tests/error_variant_tests.rs` | `compiled_digest_is_deterministic`, `different_sources_produce_different_digests` | Add ForEach field variation cases |
| `crates/vb_compile/tests/vb_a001_for_each_topology.rs` | 11 structural tests | Add digest sensitivity assertions |
| `crates/vb_compile/tests/v1_primitive_lowering.rs` | `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir`, `PRIMITIVE_CASES[for_each]` | Extend proptest for ForEach digest variation |

## 5. Bridge Implementation Notes

### 5.1 Exact Code Pattern for Both Files

```rust
// ADD after StepPrimitive::Finish arm, BEFORE 'other' catch-all:
vb_yaml::ast::StepPrimitive::ForEach { variable, input, at_once, body } => {
    hasher.update(b"for_each");
    hasher.update(b"variable:");
    hasher.update(variable.as_bytes());
    hasher.update(b"input:");
    hasher.update(input.as_bytes());
    hasher.update(b"at_once:");
    let limit = at_once.unwrap_or(1);
    hasher.update(&limit.to_le_bytes());
    hasher.update(b"body:");
    for step in body {
        hasher.update(step.id.as_bytes());
        digest_step_primitive(hasher, &step.primitive);
    }
}
```

### 5.2 Critical Alignment Points

1. Both copies MUST use the same field order: variable, input, at_once, body
2. Both copies MUST use the same delimiter: `b":"`
3. Both copies MUST use the same at_once canonical form: `at_once.unwrap_or(1).to_le_bytes()`
4. Both copies MUST recursively hash body steps via `digest_step_primitive`
5. Both copies MUST include body step IDs before primitives

### 5.3 Known Pre-existing Divergence Between Copies

| Item | mod_compile_lowering/part_05.rs | compile/mod.rs | Impact on ForEach fix |
|---|---|---|---|
| Together name | `"together"` | `"parallel"` | None (Together out of scope) |
| Aggregate name | `"reduce"` | `"aggregate"` | None (Aggregate out of scope) |
| Wildcard arm | `_ => "unknown"` | (exhaustive match) | None |

These divergences are pre-existing and do not affect the ForEach fix. They are documented here for the `proof-to-implementation` agent to be aware of during bridge review.

### 5.4 Instrumentation for Verification

For Kani harnesses to access `digest_step_primitive`, the function may need to be made `pub` or `pub(crate)` in both files, or Kani harnesses must live in the same crate. The `proof-writer` agent will determine the appropriate visibility or harness placement.

## 6. Out-of-Scope for Bridge

- Consolidating the two `canonical_digest` copies (separate refactoring bead)
- Adding ForEach field hashing to `compute_compiled_digest` (already correct)
- Adding field hashing for other primitives (Collect, Aggregate, etc.)
- Modifying `lower_steps_to_ir`, `lower_canonical_for_each`, or any lowering logic
- Changing `WorkflowDigest`, `WorkflowParts`, or `CompiledWorkflow` types
