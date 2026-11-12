# Proof-to-Implementation Input — vb-xi2f.33

**Bead**: `vb-xi2f.33` / P1: digest covers ask semantics
**Purpose**: Bridge mapping proof claims to Rust source refs, behavior tests, and refinement harness refs.
**Target State**: State 7 (proof-to-implementation)

## Implementation Fix Required

### Source Files to Modify

| File | Function | Change |
|------|----------|--------|
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs` lines 140-162 | `digest_step_primitive` | Add explicit `Ask { prompt, timeout }` arm before the catch-all `other` arm |
| `crates/vb_compile/src/compile/mod.rs` lines 243-261 | `digest_step_primitive` | Apply identical fix to duplicate implementation |

### Required Fix: Ask Match Arm

In both files, add the following arm between the `Finish` arm and the `other` catch-all:

```rust
vb_yaml::ast::StepPrimitive::Ask { prompt, timeout } => {
    hasher.update(b"ask");
    hasher.update(prompt.as_bytes());
    match timeout {
        Some(t) => {
            hasher.update(b"timeout");
            hasher.update(t.as_bytes());
        }
        None => {
            hasher.update(b"no_timeout");
        }
    }
}
```

### Contract: Type Contracts (TC-001 through TC-007)

| Contract Clause | Implementation Check |
|----------------|---------------------|
| TC-001 (explicit Ask arm) | Verify the source has `Ask { prompt, timeout }` pattern, not relying on catch-all |
| TC-002 (deterministic ordering) | Verify update ordering: `b"ask"` → `prompt.as_bytes()` → `timeout` sentinel/value |
| TC-003 (empty prompt) | `hasher.update(b"")` is valid for empty prompt |
| TC-004 (timeout sentinel) | `None` → `b"no_timeout"`; `Some` → `b"timeout"` + value |
| TC-005 (no Set/Finish regression) | Existing Set/Finish arms unchanged |
| TC-006 (duplicate parity) | Both copies receive identical fix |
| TC-007 (no panic) | No `unwrap`, `expect`, `panic`, `todo`, `unimplemented` in Ask arm |

## Proof Claim → Source Ref Mapping

### Kani Proofs → Source Code

| Obligation | Proof Claim | Source Ref | Test Ref |
|-----------|------------|------------|----------|
| PO-KANI-001 | Prompt sensitivity | `part_05.rs` lines 140-162 (`digest_step_primitive`) | `verification/kani/digest_ask_prompt_sensitivity.rs` |
| PO-KANI-002 | Timeout sensitivity | `part_05.rs` lines 140-162 | `verification/kani/digest_ask_timeout_sensitivity.rs` |
| PO-KANI-003 | Empty prompt distinct | `part_05.rs` lines 140-162 | `verification/kani/digest_ask_empty_prompt.rs` |
| PO-KANI-004 | Sentinel distinction | `part_05.rs` lines 140-162 | `verification/kani/digest_ask_timeout_sentinel.rs` |
| PO-KANI-005 | Field ordering | `part_05.rs` lines 140-162 | `verification/kani/digest_ask_field_ordering.rs` |
| PO-KANI-006 | Panic-freedom | `part_05.rs` lines 140-162 (all arms) | `verification/kani/digest_step_primitive_no_panic.rs` |

### Proptest → Source Code

| Obligation | Proof Claim | Source Ref | Test Ref |
|-----------|------------|------------|----------|
| PO-PROPTEST-001 | Prompt sensitivity (random) | `part_05.rs` lines 116-138 (`canonical_digest`) | `crates/vb_compile/tests/proptest_digest_ask_prompt_sensitivity.rs` |
| PO-PROPTEST-002 | Timeout sensitivity (random) | `part_05.rs` lines 116-138 | `crates/vb_compile/tests/proptest_digest_ask_timeout_sensitivity.rs` |
| PO-PROPTEST-003 | Determinism | `part_05.rs` lines 116-138 | `crates/vb_compile/tests/proptest_digest_determinism.rs` |
| PO-PROPTEST-004 | Field ordering (random) | `part_05.rs` lines 116-138 | `crates/vb_compile/tests/proptest_digest_ask_ordering.rs` |

### Fuzz → Source Code

| Obligation | Proof Claim | Source Ref | Test Ref |
|-----------|------------|------------|----------|
| PO-FUZZ-001 | Adversarial input robustness | `part_05.rs` lines 116-138 | `fuzz/fuzz_targets/canonical_digest_ask.rs` |

### Behavior Tests → Source Code (delegated to State 8/9 test-planner/test-writer)

| Seed | Test Scenario (from traceability-matrix.jsonl) | Source Ref |
|------|----------------------------------------------|------------|
| PS-ASK-006 | `given_workflow_source_with_ask_when_compiled_via_active_path_and_legacy_path_then_digests_match` | `part_05.rs` + `compile/mod.rs` |
| PS-ASK-007 | `given_source_with_only_set_and_finish_when_digested_before_and_after_fix_then_digests_identical` | `part_05.rs` lines 144-161 |
| PS-ASK-010 | `given_code_review_of_digest_step_primitive_when_inspected_then_ask_has_explicit_arm_not_catch_all` | `part_05.rs` lines 140-162 |

## Independent Behavior Tests

These are not verifier harnesses — they are Rust behavior tests that verify the fix works end-to-end:

| Behavior Test (from traceability-matrix.jsonl) | Maps To |
|------------------------------------------------|---------|
| `given_two_workflow_sources_differing_only_by_ask_prompt_when_canonical_digest_computed_then_digests_are_different` | PO-PROPTEST-001, PO-KANI-001 |
| `given_two_workflow_sources_differing_only_by_ask_timeout_when_canonical_digest_computed_then_digests_are_different` | PO-PROPTEST-002, PO-KANI-002 |
| `given_workflow_source_when_canonical_digest_called_twice_then_same_result` | PO-PROPTEST-003 |
| `given_ask_with_empty_prompt_when_digested_then_digest_differs_from_any_non_empty_prompt` | PO-KANI-003 |
| `given_ask_with_timeout_none_when_digested_then_different_from_some_empty_string` | PO-KANI-004 |
| `given_source_with_only_set_and_finish_when_digested_before_and_after_fix_then_digests_identical` | PO-UT-002 |
| `given_code_review_of_digest_step_primitive_when_inspected_then_ask_has_explicit_arm_not_catch_all` | PO-UT-001 |

## Refinement Harness Ref Mapping

| Proof Claim | Refinement Harness | Status |
|------------|-------------------|--------|
| PO-KANI-001 (prompt sensitivity) | Kani harness: `check_ask_prompt_sensitivity` | planned |
| PO-KANI-002 (timeout sensitivity) | Kani harness: `check_ask_timeout_sensitivity` | planned |
| PO-KANI-003 (empty prompt) | Kani harness: `check_empty_prompt_distinct` | planned |
| PO-KANI-004 (sentinel distinction) | Kani harness: `check_timeout_sentinel_distinction` | planned |
| PO-KANI-005 (field ordering) | Kani harness: `check_ask_field_ordering_deterministic` | planned |
| PO-KANI-006 (panic-freedom) | Kani harness: `check_digest_step_primitive_no_panic` | planned |

## Exact Evidence Commands

All evidence commands are specified in `proof-obligations.planned.jsonl` as the `command` field. Summary:

```bash
# Kani proofs (State 5: proof-writer creates harnesses; State 6/12: execute)
cargo kani --harness check_ask_prompt_sensitivity --unwind 10
cargo kani --harness check_ask_timeout_sensitivity --unwind 10
cargo kani --harness check_empty_prompt_distinct --unwind 5
cargo kani --harness check_timeout_sentinel_distinction --unwind 5
cargo kani --harness check_ask_field_ordering_deterministic --unwind 10
cargo kani --harness check_digest_step_primitive_no_panic --unwind 10

# Proptest (State 5: proof-writer creates; State 6/12: execute)
cargo test --test proptest_digest_ask_prompt_sensitivity
cargo test --test proptest_digest_ask_timeout_sensitivity
cargo test --test proptest_digest_determinism
cargo test --test proptest_digest_ask_ordering

# Fuzz (State 5: proof-writer creates; State 6/12: execute)
cargo fuzz run canonical_digest_ask -- -max_len=65536 -runs=100000

# Unit tests (State 9: test-writer creates after implementation)
# Delegated to test-planner (State 8): see traceability-matrix.jsonl for test scenarios.
# Expected test artifacts:
#   crates/vb_compile/tests/digest_ask_explicit_arm.rs
#   crates/vb_compile/tests/digest_set_finish_regression.rs
#   crates/vb_compile/tests/digest_duplicate_parity.rs
```

## Implementation Order

1. **State 5 (proof-writer)**: Write Kani harnesses, proptest properties, and fuzz target. These will initially FAIL on the unfixed code (proving the bug).
2. **State 11 (holzman-rust)**: Apply the fix to both `part_05.rs` and `compile/mod.rs`. Kani/proptest should now pass.
3. **State 8 (test-planner)**: Plan behavior tests for PS-ASK-006/007/010 and all traceability-matrix.jsonl scenarios.
4. **State 9 (test-writer)**: Write behavior tests as planned.
5. **State 12 (formal-verifier)**: Execute all proof commands and collect evidence.

## Open Bridge Questions

1. Should the fix be applied ONLY to `part_05.rs` and the legacy `compile/mod.rs::digest_step_primitive` marked deprecated, or must both be kept synchronized? (Answer: both must be fixed for now; future bead may deprecate legacy path.)
2. Should the `Other` catch-all arm be removed to make `digest_step_primitive` exhaustive, forcing compiler errors on new primitives? (Answer: outside P1 scope, but recommended as a separate bead.)
3. Should Kani harnesses use `kani::Arbitrary` for `StepPrimitive` or generate Ask variants directly? (Answer: generate Ask variants directly to avoid Kani exploring all primitives, which would blow up state space.)
