# test-repair-guide.md — vb-qi37.2.1

## Status: REJECTED — Return to State 8 (Test Writer)

## Rejected Findings Summary

- **L-1**: 16 weak assertions — `assert!(result.is_err())` without exact variant checks
- **L-2**: `from_workflow` has ZERO tests (behaviors 1-7 missing)
- **L-3**: `from_whole_workflow_budget` has ZERO tests (behaviors 8-13 missing)
- **L-4**: `validate_aggregate_budget` has ZERO tests (behaviors 14-31 missing)
- **L-5**: `admit_run_with_budget` has ZERO tests (behaviors 30-36 missing)
- **M-1**: 4 fits_within capacity tests don't assert requested/available fields
- **M-2**: budget.rs coverage 76%/72.5% — below 90% thresholds

**Total LETHAL: 5**
**Total MAJOR: 2**

---

## Repair Instructions

### Fix L-1: 16 weak assertions

For each of these 16 lines in `crates/vb_core/tests/aggregate_budget_vb_qi37_2_1.rs`, add the missing `unwrap_err()` + `match` + `assert_eq!` pattern.

**CORRECT PATTERN (copy from line 189-196):**
```rust
assert!(result.is_err(), "overflowing steps must return error");
let err = result.unwrap_err();
match err {
    AggregateBudgetError::Overflow { resource } => {
        assert_eq!(resource, "max_steps_executable");
    }
    other => panic!("expected Overflow, got {:?}", other),
}
```

**Lines needing fix:**

| Line | Test function | Missing assertion |
|------|--------------|-------------------|
| 490 | `usage_add_returns_overflow_when_active_runs_sum_exceeds_u64` | `Underflow { resource: "max_active_runs" }` |
| 531 | `usage_add_returns_overflow_when_queue_depth_sum_exceeds_u64` | `Overflow { resource: "max_queue_depth" }` |
| 572 | `usage_add_returns_overflow_when_journal_batch_sum_exceeds_u64` | `Overflow { resource: "max_journal_batch_bytes" }` |
| 800 | `usage_subtract_returns_underflow_when_action_tickets_would_go_negative` | `Underflow { resource: "max_action_tickets" }` |
| 841 | `usage_subtract_returns_underflow_when_parallel_would_go_negative` | `Underflow { resource: "max_parallel_in_flight" }` |
| 923 | `usage_subtract_returns_underflow_when_gather_items_would_go_negative` | `Underflow { resource: "max_gather_items" }` |
| 964 | `usage_subtract_returns_underflow_when_result_bytes_would_go_negative` | `Underflow { resource: "max_result_bytes" }` |
| 1005 | `usage_subtract_returns_underflow_when_total_slots_would_go_negative` | `Underflow { resource: "max_total_slots_written" }` |
| 1046 | `usage_subtract_returns_underflow_when_active_runs_would_go_negative` | `Underflow { resource: "max_active_runs" }` |
| 1087 | `usage_subtract_returns_underflow_when_queue_depth_would_go_negative` | `Underflow { resource: "max_queue_depth" }` |
| 1128 | `usage_subtract_returns_underflow_when_journal_batch_would_go_negative` | `Underflow { resource: "max_journal_batch_bytes" }` |
| 1266 | `usage_fits_within_rejects_u64_max_parallel_when_capacity_is_u32_max` | `CapacityExceeded { resource: "max_parallel_in_flight", requested: u64::MAX, available: u32::MAX as u64 }` |
| 1349 | `usage_fits_within_returns_capacity_exceeded_when_action_tickets_exceed_by_one` | `CapacityExceeded { resource: "max_action_tickets", requested: 101, available: 100 }` |
| 1386 | `usage_fits_within_returns_capacity_exceeded_when_parallel_exceed_by_one` | `CapacityExceeded { resource: "max_parallel_in_flight", requested: 11, available: 10 }` |
| 1423 | `usage_fits_within_returns_capacity_exceeded_when_gather_pages_exceed_by_one` | `CapacityExceeded { resource: "max_gather_pages", requested: 101, available: 100 }` |
| 1460 | `usage_fits_within_returns_capacity_exceeded_when_gather_items_exceed_by_one` | `CapacityExceeded { resource: "max_gather_items", requested: 501, available: 500 }` |

### Fix L-2: Add `from_workflow` tests

Write these 7 tests in `crates/vb_core/tests/aggregate_budget_vb_qi37_2_1.rs` (add new section):

```rust
// Behavior Group A: AggregateResourceBudget::from_workflow

#[test]
fn aggregate_budget_returns_exact_fixture_values_when_workflow_is_bounded() {
    // Given: CompiledWorkflow with 4 steps, 2 action tickets, etc.
    // When: from_workflow is called
    // Then: Ok(AggregateResourceBudget { exact values })
}

#[test]
fn aggregate_budget_returns_minimum_values_when_workflow_has_one_finish_step() {
    // Given: one finish step, no fanout
    // When: from_workflow is called
    // Then: Ok with exact minima
}

#[test]
fn aggregate_budget_returns_limit_values_when_workflow_is_at_policy_maximum() {
    // Given: workflow at policy limits
    // When: from_workflow is called
    // Then: Ok with all fields at limit
}

#[test]
fn aggregate_budget_returns_workflow_entry_error_when_workflow_is_empty() {
    // Given: empty workflow
    // When: from_workflow is called
    // Then: Err(AggregateBudgetError::WorkflowBudget(WorkflowError::EntryOutOfBounds { entry: StepIdx(0) }))
}

#[test]
fn aggregate_budget_returns_workflow_step_error_when_target_is_out_of_bounds() {
    // Given: workflow with target StepIdx(9) but node_count == 2
    // When: from_workflow is called
    // Then: Err(AggregateBudgetError::WorkflowBudget(WorkflowError::StepOutOfBounds { step: StepIdx(9) }))
}

#[test]
fn aggregate_budget_returns_workflow_jump_cycle_when_jump_reenters_path() {
    // Given: workflow with jump StepIdx(1) -> StepIdx(0)
    // When: from_workflow is called
    // Then: Err(AggregateBudgetError::WorkflowBudget(WorkflowError::JumpCycle { step: StepIdx(1), target: StepIdx(0) }))
}

#[test]
fn aggregate_budget_returns_overflow_when_total_steps_exceed_u32_max() {
    // Given: workflow requiring max_steps_executable = u32::MAX as u64 + 1
    // When: from_workflow is called
    // Then: Err(AggregateBudgetError::Overflow { resource: "max_steps_executable" })
}
```

### Fix L-3: Add `from_whole_workflow_budget` tests

Write these 6 tests in `crates/vb_core/tests/aggregate_budget_vb_qi37_2_1.rs`:

```rust
// Behavior Group B: AggregateResourceBudget::from_whole_workflow_budget

#[test]
fn aggregate_budget_preserves_exact_dimensions_when_whole_budget_is_valid() { ... }
#[test]
fn aggregate_budget_preserves_zero_optional_dimensions_when_contract_allows_zero() { ... }
#[test]
fn aggregate_budget_preserves_maximum_u32_dimensions_when_values_fit() { ... }
#[test]
fn aggregate_budget_returns_overflow_when_action_tickets_exceed_u32() { ... }
#[test]
fn aggregate_budget_returns_overflow_when_parallel_exceeds_u16() { ... }
#[test]
fn aggregate_budget_returns_overflow_when_journal_batch_exceeds_u32() { ... }
```

### Fix L-4: Add `validate_aggregate_budget` tests

Write 18+ tests covering each policy-governed dimension at equality/below/over and zero-limit rejection. These tests require `validate_aggregate_budget(&budget, &policy)` function access. If this is a free function, test it directly. If it's an associated function on a policy type, use the appropriate fixture.

Test names must follow this pattern exactly:
- `validate_aggregate_budget_accepts_steps_when_equal_to_limit`
- `validate_aggregate_budget_accepts_steps_when_one_below_limit`
- `validate_aggregate_budget_returns_policy_exceeded_when_steps_exceed_limit`
- ... repeat for each dimension ...

### Fix L-5: Add or waive `admit_run_with_budget` tests

Either:
- **Option A (preferred)**: Write runtime integration tests in `crates/vb_runtime/tests/vb_qi37_2_1_admission.rs` covering behaviors 30-36 using a real shard fixture.
- **Option B**: If the runtime admission path cannot be tested in isolation, produce a waiver document at `.beads/vb-qi37.2.1/admission-waiver.md` citing compensating evidence (e.g., manual QA smoke results in `manual-qa-smoke.md`).

---

## Re-run Requirements

After all fixes:
1. `cargo test -p vb_core --all-features --no-run` — must compile
2. `cargo nextest run -p vb_core --test-threads=4` — all tests pass
3. `cargo llvm-cov nextest -p vb_core` — budget.rs line coverage ≥90%, branch coverage ≥90%
4. Re-run test-reviewer in Mode 2 from Tier 0

---

## Non-Negotiable Rules

- Do NOT use `assert!(result.is_ok())` or `assert!(result.is_err())` as the sole assertion. Always add `unwrap_err()` + `match` + field assertions.
- Do NOT use `is_err()` as a boolean check — assert the exact variant and field values.
- Do NOT add tests that only check `is_ok()` for happy paths — assert exact output values.
- Do NOT skip any of the 5 LETHAL findings. All must be resolved before re-submission.
