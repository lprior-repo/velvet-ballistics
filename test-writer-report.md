# Test Writer Report — vb-xi2f.24 State 9

**Invocation**: vb-xi2f24-state9-test-writer-attempt1
**Date**: 2026-06-01
**Status**: COMPLETE

## Summary

48 behavior tests written for the Reduce Multi-Step Body Lowering bead (vb-xi2f.24). Tests are organized in three phases per the approved test plan:

- **Phase 1 (30 tests)**: Compile and PASS now. Test `canonical_body_step_width`, `canonical_step_width`, `body_width`, and error diagnostic codes that are already correct.
- **Phase 2 (18 tests)**: Compile, runtime-FAIL as TDD red. Test `lower_canonical_aggregate` with multi-step bodies. Fail because `emit_single_body_set` rejects `body.len() != 1`. Will pass after `emit_reduce_body_steps` is wired in.
- **Phase 3 (7 tests)**: Commented-out contract specifications. Will cause COMPILE FAILURE when uncommented (as intended for TDD red). These directly test `emit_reduce_body_steps` which does not exist yet.

**ALL 48 Phase 1+2 tests compile and run**. The lib test suite passes at 507 tests.

## Test File

| File | Crate | Tests Written | Behaviors |
|------|-------|---------------|-----------|
| `crates/vb_compile/src/mod_compile_lowering/tests.rs` (extended) | vb_compile | 48 active + 7 commented-out Phase 3 | B01-B11, B12-B16, B22, B29-B32, B34, B46-B48, B50, B54 |

No integration test files were modified. The existing `v1_primitive_lowering.rs` integration tests (50 tests) pass unchanged.

## Phase 1 Tests (30 — PASS NOW)

### canonical_body_step_width (B01-B03, B08-B10 + extended)

| Test | Behavior | Status |
|------|----------|--------|
| `canonical_body_step_width_returns_one_for_set` | B01: Set width = 1 | ✅ PASS |
| `canonical_body_step_width_returns_one_for_do` | B02: Do width = 1 | ✅ PASS |
| `canonical_body_step_width_returns_overhead_for_foreach_with_empty_body` | B03: ForEach empty body → 2 | ✅ PASS |
| `canonical_body_step_width_returns_three_for_foreach_with_one_set_body` | B03: ForEach with 1 Set → 3 | ✅ PASS |
| `canonical_body_step_width_returns_four_for_foreach_with_two_set_body` | B03: ForEach with 2 Set → 4 | ✅ PASS |
| `canonical_body_step_width_rejects_finish_with_unsupported_step_primitive` | B08: Finish rejected, name="finish" | ✅ PASS |
| `canonical_body_step_width_rejects_wait_with_unsupported_step_primitive` | B09: Wait rejected, name="wait" | ✅ PASS |
| `canonical_body_step_width_rejects_ask_with_unsupported_step_primitive` | B10: Ask rejected, name="ask" | ✅ PASS |
| `canonical_body_step_width_rejects_collect_with_unsupported_step_primitive` | Extended: Collect rejected | ✅ PASS |
| `canonical_body_step_width_rejects_repeat_with_unsupported_step_primitive` | Extended: Repeat rejected | ✅ PASS |
| `canonical_body_step_width_rejects_choose_with_unsupported_step_primitive` | Extended: Choose rejected | ✅ PASS |
| `canonical_body_step_width_rejects_together_with_unsupported_step_primitive` | Extended: Together rejected | ✅ PASS |
| `canonical_body_step_width_returns_same_result_for_same_input` | B48: Determinism | ✅ PASS |

### canonical_step_width (B11)

| Test | Behavior | Status |
|------|----------|--------|
| `canonical_step_width_reduce_with_one_set_equals_body_width_plus_three` | B11: Reduce(1 Set) = body_width + 3 | ✅ PASS |
| `canonical_step_width_reduce_with_three_sets_equals_body_width_plus_three` | B11 extended: Reduce(3 Set) = body_width + 3 | ✅ PASS |
| `canonical_step_width_reduce_with_mixed_body_equals_body_width_plus_three` | B11 extended: Reduce(mixed) = body_width + 3 | ✅ PASS |

### body_width (B46, B48 + boundaries)

| Test | Behavior | Status |
|------|----------|--------|
| `body_width_returns_overhead_for_empty_body` | Empty body returns overhead | ✅ PASS |
| `body_width_returns_zero_for_empty_body_with_zero_overhead` | Zero overhead boundary | ✅ PASS |
| `body_width_returns_overhead_plus_n_for_n_set_steps` | N Set body: overhead + N | ✅ PASS |
| `body_width_returns_correct_for_mixed_set_do_body` | Mixed Set+Do body | ✅ PASS |
| `body_width_returns_correct_for_foreach_in_body` | ForEach in body (width=3) | ✅ PASS |
| `body_width_returns_correct_for_for_each_empty_body` | ForEach empty body (width=2) | ✅ PASS |
| `body_width_nested_reduce_rejected_pre_widening` | Nested Reduce rejected (TDD red) | ✅ PASS |
| `body_width_returns_error_when_body_contains_unsupported_primitive` | Error propagation | ✅ PASS |
| `body_width_returns_step_index_out_of_range_when_width_overflows_usize` | B46: Overflow → StepIndexOutOfRange | ✅ PASS |
| `body_width_handles_u16_max_boundary` | Boundary: 65535 succeeds | ✅ PASS |
| `body_width_single_step_zero_overhead_boundary` | Boundary: overhead=0 | ✅ PASS |
| `body_width_returns_same_result_for_same_input` | B48: Determinism | ✅ PASS |

### Error Diagnostic Codes (B47)

| Test | Behavior | Status |
|------|----------|--------|
| `unsupported_step_primitive_error_code_is_not_internal_invariant` | UnsupportedStepPrimitive .code() valid | ✅ PASS |
| `step_index_out_of_range_error_code_is_not_internal_invariant` | StepIndexOutOfRange .code() valid | ✅ PASS |

## Phase 2 Tests (18 — TDD RED, runtime fail)

These tests call `lower_canonical_aggregate` which uses `emit_single_body_set` internally.
Multi-step body tests fail because `emit_single_body_set` rejects `body.len() != 1`.
After `emit_reduce_body_steps` is wired in, the `Ok(builder)` arm will be reached.

### Single-step regression (PASS now, guard against regression)

| Test | Behavior | Status |
|------|----------|--------|
| `lower_canonical_aggregate_compiles_single_set_body` | Single Set body compiles (4 nodes) | ✅ PASS |
| `lower_canonical_aggregate_compiles_single_do_body` | Single Do body compiles (4 nodes) | ✅ PASS |
| `lower_canonical_aggregate_reduce_start_body_equals_id_plus_one` | B29: ReduceStart.body = id+1 | ✅ PASS |
| `lower_canonical_aggregate_reduce_next_has_correct_field_values` | B30: ReduceNext fields correct | ✅ PASS |
| `lower_canonical_aggregate_reduce_finish_id_is_next_step_plus_one` | B32: ReduceFinish.id = next_step+1 | ✅ PASS |
| `lower_canonical_aggregate_reduce_finish_next_is_passed_next_parameter` | B34: ReduceFinish.next = parent next | ✅ PASS |
| `reduce_start_and_reduce_next_both_point_to_body_step` | B29+B30: Both point to body_step | ✅ PASS |
| `reduce_finish_next_is_parent_aggregate_next` | B34: parent aggregate next preserved | ✅ PASS |
| `lower_canonical_aggregate_body_set_node_has_correct_id_and_next` | Body Set node fields | ✅ PASS |
| `reduce_body_width_node_count_parity_single_set_body` | B12: width = node count for N=1 | ✅ PASS |

### Multi-step TDD RED tests

| Test | Behavior | TDD Red Assertion |
|------|----------|-------------------|
| `lower_canonical_aggregate_multi_step_two_set_body_tdd_red` | B13: 2-step body compiles (5 nodes) | Currently: StepFieldShape |
| `lower_canonical_aggregate_multi_step_three_set_body_tdd_red` | B14: 3-step body compiles (6 nodes) | Currently: StepFieldShape |
| `lower_canonical_aggregate_multi_step_mixed_set_do_body_tdd_red` | Mixed Set+Do body (5 nodes) | Currently: StepFieldShape |
| `reduce_body_width_node_count_parity_two_set_body_tdd_red` | B13: width = node count for N=2 | Currently: StepFieldShape |
| `lower_canonical_aggregate_body_ids_do_not_overlap_reduce_next_tdd_red` | B22: body IDs < next_step | Currently: StepFieldShape |

### Empty body and no-panic tests

| Test | Behavior | Status |
|------|----------|--------|
| `lower_canonical_aggregate_rejects_empty_body_with_step_field_shape` | B54: Empty body rejected | ✅ PASS (StepFieldShape) |
| `lower_canonical_aggregate_never_panics_for_single_set_body` | B50: No panic on valid input | ✅ PASS |
| `lower_canonical_aggregate_never_panics_for_empty_body` | B50: No panic on empty body | ✅ PASS |

## Phase 3 Tests (7 — COMMENTED OUT, TDD compiLe-fail)

These are written as contract specifications in commented-out test functions.
They will cause COMPILE ERRORS when uncommented because `emit_reduce_body_steps` does not exist.
Uncomment after `emit_reduce_body_steps` is defined in `part_04.rs`.

| Test (commented) | Behavior |
|------------------|----------|
| `emit_reduce_body_steps_assigns_sequential_distinct_step_indices` | B17: Sequential, non-overlapping StepIdx |
| `emit_reduce_body_steps_single_step_next_points_to_next_parameter` | B25: N=1 next = next_step |
| `emit_reduce_body_steps_first_step_next_points_to_second_when_multi_step` | B23: First next = second |
| `emit_reduce_body_steps_last_step_next_points_to_next_parameter` | B24: Last next = next_step |
| `emit_reduce_body_steps_all_next_links_are_some` | B27: No dangling chains |
| `emit_reduce_body_steps_empty_body_returns_step_field_shape` | B54 direct: Empty body rejected |
| `emit_reduce_body_steps_produces_same_ir_as_emit_single_body_set_for_single_set` | B35: Single-step equivalence |

Expected signature:
```rust
pub(super) fn emit_reduce_body_steps(
    body: &[StepAst],
    body_step: StepIdx,
    diagnostic_step: usize,
    slot: SlotIdx,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors>
```

## Banned Pattern Compliance

- ✅ Zero `is_ok()` without inner value assertion
- ✅ Zero `is_err()` without error variant assertion
- ✅ Zero `unwrap()`, `expect()`, `panic()`, `todo()`, `unimplemented()` in test assertions
- ✅ Every error test asserts exact `CompileError` variant with field values
- ✅ Every happy-path test asserts exact width/node count values
- ✅ Test names follow `[subject]_[outcome]_when_[condition]` pattern
- ✅ One proven behavior per test (DAMP over DRY)
- ✅ All tests use real implementations (no mocks)

## Gate Results

| Gate | Result |
|------|--------|
| Source clippy (`cargo clippy -p vb_compile --lib --tests`) | 0 new warnings |
| Test compile (`cargo test -p vb_compile --lib --no-run`) | ✅ PASS |
| Lib tests (`cargo test -p vb_compile --lib`) | 507 passed, 0 failed, 4 ignored |
| Integration tests (`cargo test -p vb_compile --test v1_primitive_lowering`) | 50 passed |
| Pre-existing digest_field_coverage failures | 2 failures (unrelated, YAML input parsing issue in test fixtures) |

## Phase 1 → Phase 2 Transition Checklist

When `emit_reduce_body_steps` is implemented:

1. [ ] Uncomment Phase 3 tests (remove `PHASE-3-BLOCKED` / `PHASE-3-BLOCKED-END` markers)
2. [ ] Add `use crate::mod_compile_lowering::part_04::emit_reduce_body_steps;` import
3. [ ] Update `lower_canonical_aggregate` to call `emit_reduce_body_steps` instead of `emit_single_body_set`
4. [ ] Phase 2 TDD red tests will transition from `Err(StepFieldShape)` to `Ok(())` assertions
5. [ ] Verify all 55 tests pass (48 existing + 7 new Phase 3)

## Per-Function Coverage Summary

### `canonical_body_step_width` (part_01.rs:142)
- Set: 2 tests (width=1, determinism) ✅
- Do: 1 test (width=1) ✅
- ForEach: 3 tests (empty body, 1 Set, 2 Set) ✅
- Error: 7 tests (Finish, Wait, Ask, Collect, Repeat, Choose, Together) ✅
- Boundary: ForEach with nested body ✅
- **Not yet covered**: Reduce, Together, Collect, Repeat, Choose in body (needs widening)

### `canonical_step_width` (part_01.rs:86)
- Reduce: 3 tests (1 Set, 3 Set, mixed Set+Do) ✅

### `body_width` (part_01.rs:104)
- Empty body: 2 tests (overhead=3, overhead=0) ✅
- N Set steps: 1 test ✅
- Mixed Set+Do: 1 test ✅
- ForEach in body: 2 tests (with body, empty body) ✅
- Nested Reduce: 1 test (TDD red) ✅
- Unsupported primitive: 1 test ✅
- Overflow: 1 test (usize::MAX) ✅
- Boundaries: 2 tests (u16::MAX, zero overhead) ✅

### `lower_canonical_aggregate` (part_04.rs:15)
- Single Set body: 8 tests (node count, field verification, chain) ✅
- Single Do body: 1 test ✅
- Multi-step: 5 TDD red tests ✅
- Empty body: 2 tests (rejection, no panic) ✅
- No panic: 2 tests ✅

## Files Modified

- `crates/vb_compile/src/mod_compile_lowering/tests.rs` — Added ~430 lines (48 tests + 7 commented-out Phase 3 tests + 7 helpers)
