# Proof-to-Implementation Input — vb-xi2f.34: Finish Digest Coverage

**Bead**: vb-xi2f.34  
**Phase**: p4-proof-planner → p5-proof-writer / p7-proof-to-implementation  
**Date**: 2026-05-24  

---

## Purpose

This document bridges proof-planner obligations to implementation concerns. It maps each proof claim to exact Rust source references, test/harness file locations, and the evidence commands that will prove the obligations.

---

## Mapping: Proof Obligations → Rust Source

### Kani Obligations → Source Under Test

| Obligation | Source File | Function | Lines |
|---|---|---|---|
| PO-KANI-FINISH-001 | `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | `digest_step_primitive` (Finish arm) | 150–156 |
| PO-KANI-FINISH-002 | `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | `digest_step_primitive` (Finish arm) | 150–156 |
| PO-KANI-FINISH-003 | `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | `digest_step_primitive` (Finish arm) | 150–156 |

**Shared harness file**: `crates/vb_compile/src/kani_finish_digest.rs` (NEW)

**Implementation notes**:
- The Kani harness should NOT hardcode structural inputs (GOD RULE #1). Use `kani::any()` for String values and `kani::Arbitrary` if available.
- The harness tracks byte slices passed to `hasher.update()` rather than computing actual blake3 hashes. This models the property: distinct inputs → distinct hash inputs. The assumption that blake3 is collision-resistant is trusted (T-1 in trusted-base-plan.md).
- Add `#[cfg(kani)]` gate to the harness file.

### Proptest Obligations → Source Under Test

| Obligation | Source File | Function | Lines |
|---|---|---|---|
| PO-PROPTEST-FINISH-001 | `part_05.rs` | `canonical_digest` | 116–138 |
| PO-PROPTEST-FINISH-002 | `part_05.rs` | `canonical_digest` + `digest_step_primitive` | 116–162 |
| PO-PROPTEST-FINISH-003 | `part_05.rs` | `canonical_digest` (step loop) | 133–136 |
| PO-PROPTEST-FINISH-004 | `part_05.rs` | `canonical_digest` (signature) | 116 |

**Shared proptest file**: `crates/vb_compile/src/proptest_finish_digest.rs` (NEW)

**Implementation notes**:
- Proptest strategies must generate valid `WorkflowSource` with at least one `Finish` step.
- Reuse existing proptest strategies from `crates/vb_compile/src/proptest_error_parity.rs` for generating `StepPrimitive::Finish { result: ... }`.
- The `canonical_digest` function is `pub(super)` in canonical path — accessible from within crate. Legacy path `canonical_digest` is private — may need `#[cfg(test)]` re-export or `pub(super)` change for equivalence test.
- Tests should use `#[test]` with proptest macros OR be marked `#[ignore]` for CI performance.

### Integration Test Obligations → Source Under Test

| Obligation | Source File | Function | Lines |
|---|---|---|---|
| PO-INT-FINISH-001 | `part_01.rs` | `compile_source` (digest at line 46) + `CompiledWorkflow::digest()` | 16–58 |
| PO-INT-FINISH-002 | `part_05.rs` | `canonical_digest` (step.id as_bytes at line 134) | 133–134 |
| PO-INT-FINISH-003 | `part_05.rs` | `digest_step_primitive` (Finish arm, both variants) | 150–156 |
| PO-INT-FINISH-004 | `part_05.rs` vs `compile/mod.rs` | Both `canonical_digest` implementations | 116–138 / 220–241 |

**Integration test file**: `crates/workspace_tests/tests/finish_digest_integration.rs` (NEW)

**Implementation notes**:
- Integration tests require YAML test fixtures OR inline YAML strings parsed by `vb_yaml`.
- For PO-INT-FINISH-001: Compile a workflow, change finish result in source, recompile, compare digests.
- For PO-INT-FINISH-004: Need access to both `mod_compile_lowering::part_05::canonical_digest` and `compile::mod::canonical_digest`. The legacy path function is private; may need to add `#[cfg(test)]` visibility or test through the public `compile_source` API.
- Integration tests live in `crates/workspace_tests/` per workspace structure rules.

### Static Analysis Obligations → Source Under Test

| Obligation | Source File | Check |
|---|---|---|
| PO-STATIC-FINISH-001 | `part_05.rs` lines 152–155 | Exhaustiveness: _ arm unreachable for current ScalarValue |
| PO-STATIC-FINISH-002 | `part_05.rs` lines 116–162 | No unsafe, no IO, no random, no time |

**Static test file**: `crates/vb_compile/src/tests/finish_digest_tests.rs` (NEW)

**Implementation notes**:
- PO-STATIC-FINISH-001: Write a test that pattern-matches all current `ScalarValue` variants and asserts they produce non-`"unsupported"` hash inputs. When a new variant is added, this test must be updated.
- PO-STATIC-FINISH-002: Manual code review checklist item. Can also be a compile-time `static_assertions::assert_not_impl_any!`-style check or a simple test that compiles with `#![forbid(unsafe_code)]`.

---

## File Creation Plan

| File | Crate | Type | Status |
|---|---|---|---|
| `crates/vb_compile/src/kani_finish_digest.rs` | vb_compile | Kani harness | NEW |
| `crates/vb_compile/src/proptest_finish_digest.rs` | vb_compile | Proptest properties | NEW |
| `crates/vb_compile/src/tests/finish_digest_tests.rs` | vb_compile | Unit/static tests | NEW |
| `crates/workspace_tests/tests/finish_digest_integration.rs` | workspace_tests | Integration tests | NEW |

---

## Visibility Requirements

| Function | Current Visibility | Required for Proof | Action |
|---|---|---|---|
| `canonical_digest` (canonical) | `pub(super)` | Accessible from test within `vb_compile` | OK — same crate |
| `canonical_digest` (legacy) | `fn` (private) | Accessible from integration test | Add `#[cfg(test)]` re-export or use `pub(super)` |
| `digest_step_primitive` (canonical) | `pub(super)` | Accessible from Kani/proptest | OK — same crate |
| `digest_step_primitive` (legacy) | `fn` (private) | Accessible from integration test | Add `#[cfg(test)]` re-export or use `pub(super)` |
| `compile_source` (canonical) | `pub(super)` | Accessible from integration test | OK — via `vb_compile` crate public API |

---

## Expected Evidence Commands

```bash
# Kani harnesses
cargo kani --harness finish_string_result_injectivity --unwind 3
cargo kani --harness finish_integer_result_injectivity --unwind 2
cargo kani --harness finish_scalarvalue_variant_discrimination --unwind 3

# Proptest properties
cargo test --lib proptest_finish_digest::canonical_digest_is_deterministic -- --ignored
cargo test --lib proptest_finish_digest::finish_result_change_changes_digest -- --ignored
cargo test --lib proptest_finish_digest::finish_position_change_changes_digest -- --ignored
cargo test --lib proptest_finish_digest::digest_independent_of_ir_layout -- --ignored

# Integration tests
cargo test --test finish_digest_integration

# Static tests
cargo test --lib finish_digest_tests::scalarvalue_exhaustiveness_in_digest

# Unsafe audit (one-liner)
grep -r 'unsafe\|Instant\|SystemTime\|rand::' crates/vb_compile/src/mod_compile_lowering/part_05.rs && echo "FAIL" || echo "PASS: no unsafe/IO/random in digest path"
```

---

## Kani Mock Design (for proof-writer)

The Kani harness must replace `blake3::Hasher` with a tracking mock:

```rust
#[cfg(kani)]
mod kani_finish_digest {
    use kani::any;
    
    // Mock hasher that tracks byte slices fed to update()
    struct MockHasher {
        updates: Vec<Vec<u8>>,
    }
    
    impl MockHasher {
        fn new() -> Self { Self { updates: Vec::new() } }
        fn update(&mut self, data: &[u8]) {
            self.updates.push(data.to_vec());
        }
        fn get_updates(&self) -> &[Vec<u8>] {
            &self.updates
        }
    }
    
    // The harness calls digest_step_primitive with MockHasher
    // then asserts that distinct inputs produce distinct update sequences.
}
```

**GOD RULE compliance**: No hardcoded structural inputs. Use `kani::any::<String>()` with `kani::assume(s.len() <= 256)` for bounded String values. Use `kani::any::<i64>()` for integers.

---

## Implementation Order

1. **Static tests** (PO-STATIC-FINISH-001, 002) — quick wins, no tooling dependency
2. **Integration tests** (PO-INT-FINISH-001 through 004) — end-to-end validation
3. **Proptest properties** (PO-PROPTEST-FINISH-001 through 004) — statistical coverage
4. **Kani harnesses** (PO-KANI-FINISH-001 through 003) — formal bounded proofs

Each layer provides evidence before the next layer begins. If integration tests reveal a bug, fix it before writing Kani proofs.
