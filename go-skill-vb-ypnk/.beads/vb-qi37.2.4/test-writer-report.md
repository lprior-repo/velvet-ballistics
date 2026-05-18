# Test Writer Report: vb-qi37.2.4 State 8 - Attempt 3 (Repair)

## Summary

Executed State 8 test repair after State 9 rejection attempt 2.

**Key Fixes in Attempt 3:**
- **ActionTicketsExceeded**: Fixed to use `Do` nodes instead of linear Nop chain. Now properly triggers the error variant.
- **RunTimeExceeded**: Changed from false-pass (`Ok(())`) to explicit failing-first test with GAP-2 evidence. Test now PANICS with clear GAP documentation when `Ok(())` is returned.
- **Density**: Expanded from 17 to 46 passing integration tests, achieving 5.1x density (46/9 > 5x requirement).

**Status**: GREEN PHASE - 46 passing integration tests. 1 intentional failure (RunTimeExceeded - GAP-2).

---

## Changes from Attempt 2

### Fixed: ActionTicketsExceeded Test
**Problem**: Test used `build_linear_workflow()` which creates Nop/Finish nodes only. Linear workflows have `max_action_tickets = 0`, so ActionTicketsExceeded could never trigger. Test passed via `Ok(())` branch - FALSE PASS.

**Solution**: Added `build_workflow_with_do_nodes()` helper and modified test to create 100 Do nodes, properly incrementing `max_action_tickets` to trigger the error.

### Fixed: RunTimeExceeded Test
**Problem**: `budget.max_run_time_seconds` is always 0 in implementation (budget.rs:113). Test passed via `Ok(())` branch - FALSE PASS.

**Solution**: Changed from accepting `Ok(())` to explicit `panic!()` with GAP-2 documentation when `Ok(())` is returned. Test now correctly FAILS with clear evidence that RunTimeExceeded cannot be triggered through public API.

### Added: 29 New Passing Tests
Added tests to reach >=45 passing tests for 5x density requirement:
- `integration_budget_compute_single_finish_node`
- `integration_budget_compute_single_do_node`
- `integration_budget_compute_nop_chain`
- `integration_budget_compute_collect_small_limit`
- `integration_budget_compute_repeat_small_attempts`
- `integration_budget_compute_together_small_branches`
- `integration_policy_accepts_workflow_within_limits`
- `integration_policy_rejects_at_exact_total_steps_boundary`
- `integration_policy_rejects_at_exact_fanout_boundary`
- `integration_policy_rejects_at_nesting_depth_boundary`
- `integration_budget_error_total_steps_exceeded_display`
- `integration_budget_error_fanout_exceeded_display`
- `integration_budget_error_parallel_exceeded_display`
- `integration_policy_checks_total_steps_before_fanout`
- `integration_policy_checks_fanout_before_parallel`
- `integration_collect_with_minimum_limit`
- `integration_collect_large_limit_tracks_gather_items`
- `integration_repeat_with_minimum_attempts`
- `integration_repeat_large_attempts_tracks_repeat_attempts`
- `integration_together_with_minimum_branches`
- `integration_together_large_branches_tracks_fanout`
- `integration_default_policy_accepts_moderate_workflow`
- `integration_default_policy_accepts_large_workflow`
- `integration_policy_validates_total_slots`
- `integration_policy_validates_nesting_depth`
- `integration_policy_validates_steps_executable`
- `integration_policy_validates_result_bytes`
- `integration_policy_validate_order_is_deterministic`
- `integration_budget_compute_empty_workflow_error`
- `integration_budget_compute_single_nop`

---

## Integration Test Results

### Final Test Count
- **Total tests**: 47
- **Passing**: 46
- **Failing (intentional)**: 1 (`integration_policy_returns_runtime_exceeded_when_runtime_crosses_limit` - GAP-2 evidence)

### Density Achievement
- **Public functions**: 9
- **Passing tests**: 46
- **Density ratio**: 5.1x (exceeds 5x requirement)

---

## Test Commands and Results

### Compile check (integration tests)
```bash
cd /home/lewis/src/vb-femdation/vb-qi37-2-4
cargo build --package velvet-ballastics-workspace-tests --tests
```
**Exit status**: 0 (compiles successfully)

### Run integration tests
```bash
cd /home/lewis/src/vb-femdation/vb-qi37-2-4
cargo nextest run --package velvet-ballastics-workspace-tests --test vb_qi37_2_4_integration_budget_errors
```
**Result**: 47 tests, 46 passed, 1 failed (intentional GAP-2)

### Run unit tests (preserved from attempt 1)
```bash
cd /home/lewis/src/vb-femdation/vb-qi37-2-4
cargo nextest run --package vb_core -- budget
```
**Result**: 227 passed, 3 failed (intentional red-phase failures for GAPs)

---

## GAP Documentation

### GAP-2 (BLOCK_LOCAL): RunTimeExceeded Cannot Be Triggered
**Location**: `integration_policy_returns_runtime_exceeded_when_runtime_crosses_limit`

**Evidence**:
```
GAP-2: RunTimeExceeded cannot be triggered. budget.max_run_time_seconds is always 0 in implementation.
To fix: compute max_run_time_seconds from max_step_budget_per_tick * max_steps or similar.
```

**Root Cause**: `WholeWorkflowBudget::compute` sets `max_run_time_seconds: 0` unconditionally (budget.rs:113). The value is never computed from workflow characteristics.

**Fix Required**: Implementation must compute `max_run_time_seconds` from `max_step_budget_per_tick * max_steps` or similar formula.

### GAP-1: Collect/Repeat Body Multiplication (Preserved from Attempt 1)
**Location**: `prop_collect_body_multiplies_with_finite_limit`, `prop_repeat_body_multiplies_with_max_attempts`

**Evidence**: Proptest fails with assertion that actual steps != expected steps.

**Root Cause**: Budget computation does not multiply body by limit for CollectStart/RepeatStart.

### GAP-3: Nested Loop Depth Tracking (Preserved from Attempt 1)
**Location**: `prop_nested_loops_multiply_correctly`

**Evidence**: Proptest fails with minimal input `outer_limit=2, inner_limit=2, inner_body_count=1`.

**Root Cause**: Implementation bug in nested loop traversal.

### GAP-4: BudgetError Lacks Diagnostic Fields (Preserved from Attempt 1)
**Location**: PROP-DIAG-001 tests

**Evidence**: Tests expect `primitive`, `node_index`, `structural_path` fields in BudgetError.

**Root Cause**: BudgetError only has `actual` and `limit` fields.

---

## Files Modified

```
crates/workspace_tests/tests/vb_qi37_2_4_integration_budget_errors.rs  (modified - 2117 lines)
crates/vb_core/src/budget.rs                                           (modified - added test module include from attempt 1)
crates/vb_core/src/budget/vb_qi37_2_4_state8_tests.rs               (preserved from attempt 1)
```

---

## Next Steps (State 9)

1. **Execute State 9 (Test Review)**: Full Tier 0-3 review of all tests
2. **Fix GAPs in State 10**: GAP-1 (body multiplication), GAP-2 (runtime tracking), GAP-3 (nested loops), GAP-4 (diagnostic fields)
3. **Preserve Kani proofs**: 7 Kani harnesses preserved for when `cargo kani` is available
4. **Re-run tests**: After implementation fixes, re-run proptest invariants to verify green phase

---

## Artifacts Preserved

The following artifacts are preserved under `.beads/vb-qi37.2.4/`:
- `test-plan.md` (input)
- `test-repair-guide.md` (repair guidance)
- `proof-review.md` (approved proof artifact)
- `contract-verification-review.md` (approved contract artifact)
- `test-writer-report.md` (this report - output)

---

*Generated: State 8 Attempt 3 (Test Repair) for vb-qi37.2.4*
*Green Phase: 46 integration tests passing*
*Red Phase: 1 intentional GAP-2 failure + 3 proptest failures (expected)*
*Density: 5.1x (46 passing / 9 public functions)*
