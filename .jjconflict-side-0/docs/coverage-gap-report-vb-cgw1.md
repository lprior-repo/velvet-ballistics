# vb-cgw1: Coverage Gap Analysis Report

## Executive Summary

**Status**: BLOCKED - vb_core fails to compile under coverage instrumentation, preventing any coverage analysis across all kernel crates.

Coverage cannot be measured because:
1. `vb_core/src/workflow.rs:734-738` - phantom match arm referencing non-existent `BudgetError::BudgetComputationOverflow`
2. `vb_core/src/budget.rs:888-897` - `update_workflow_metrics()` returns unused `Result`

---

## Critical: Compilation Errors Blocking Coverage

### Error 1: Phantom Match Arm (workflow.rs:734-738)

**File**: `crates/vb_core/src/workflow.rs`
**Location**: `validate_budget()` function, line 701 match statement

```rust
match BoundednessPolicy::DEFAULT.validate(&budget) {
    Ok(()) => Ok(()),
    // ... other arms ...
    Err(BudgetError::BudgetComputationOverflow { .. }) => {  // <-- THIS VARIANT DOES NOT EXIST
        Err(WorkflowError::BudgetPolicyExceeded {
            detail: "budget_computation_overflow",
        })
    }
}
```

**Problem**: `BudgetError::BudgetComputationOverflow` does not exist in the `BudgetError` enum (defined at `budget.rs:211-275`). The enum has only 9 variants:
- `TotalStepsExceeded`
- `TotalSlotsExceeded`
- `FanoutExceeded`
- `NestingDepthExceeded`
- `ParallelExceeded`
- `ActionTicketsExceeded`
- `RunTimeExceeded`
- `ResultBytesExceeded`
- `StepsExecutableExceeded`

The phantom match arm creates a compiler error: the match is considered non-exhaustive because the arm references a variant that doesn't exist in the enum.

**Fix Required**: Remove lines 734-738 or add `BudgetComputationOverflow` to `BudgetError` if it was accidentally removed.

### Error 2: Unused Result (budget.rs:888-897)

**File**: `crates/vb_core/src/budget.rs`
**Location**: Inside a function at line 888

```rust
update_workflow_metrics(
    &node.kind,
    max_action_tickets,
    max_parallel_in_flight,
    max_gather_pages,
    max_gather_items,
    max_for_each_iterations,
    max_together_branches,
    max_repeat_attempts,
);  // <-- Returns Result, but result is discarded
```

**Problem**: `update_workflow_metrics()` returns `Result` but the call site discards it. The `-D unused-must-use` lint flag in the coverage build treats this as an error.

**Fix Required**: Change to `let _ = update_workflow_metrics(...);`

---

## Known Zero-Coverage Modules (Code Inspection)

Since compilation fails, llvm-cov cannot run. The following modules are identified by code inspection as having no test coverage:

### vb_core/replay/ - ENTIRE MODULE (0%)

| File | Status |
|------|--------|
| `src/replay/choose.rs` | No test file |
| `src/replay/mod.rs` | No test file |
| `src/replay/step.rs` | No test file |
| `src/replay/ops.rs` | No test file |
| `src/replay/tests.rs` | EXISTS - only test file |

### vb_storage/recovery/replay/ - ENTIRE MODULE (0%)

| File | Status |
|------|--------|
| `src/recovery/replay/core.rs` | No test file |
| `src/recovery/replay/summary.rs` | No test file |
| `src/recovery/replay/mod.rs` | No test file |

### vb_storage - LIKELY 0% Coverage

| File | Concern |
|------|---------|
| `src/process_lock.rs` | No `*_tests.rs` or `#[test]` found |
| `src/artifacts.rs` | No `*_tests.rs` or `#[test]` found |
| `src/blobs.rs` | No `*_tests.rs` or `#[test]` found |
| `src/batch.rs` | No `*_tests.rs` or `#[test]` found |
| `src/snapshots.rs` | No `*_tests.rs` or `#[test]` found |
| `src/events.rs` | No `*_tests.rs` or `#[test]` found |
| `src/headers.rs` | No `*_tests.rs` or `#[test]` found |
| `src/codec.rs` | No `*_tests.rs` or `#[test]` found |
| `src/keys.rs` | No `*_tests.rs` or `#[test]` found |
| `src/records.rs` | No `*_tests.rs` or `#[test]` found |
| `src/indexes.rs` | No `*_tests.rs` or `#[test]` found |
| `src/constants.rs` | Likely only constants |

### vb_storage - HAS TESTS

| File | Note |
|------|------|
| `src/tests.rs` | Main test module |
| `src/recovery/tests.rs` | Recovery tests |
| `src/test_helpers.rs` | Helper functions |
| `src/proptests.rs` | Proptest definitions |
| `src/security_tests.rs` | Security tests |

---

## vb_core - Module-by-Module Coverage Status

### WITH Tests (partial coverage expected)

- `src/engine/tests.rs`
- `src/engine/expr_eval/tests.rs`
- `src/workflow/tests.rs`
- `src/replay/tests.rs`

### WITHOUT Tests (0% expected)

- `src/workflow/validation/` - entire subdirectory
- `src/engine/expr_eval/` - ops files, accessors, stack, core
- `src/engine/node_helpers.rs`
- `src/engine/signals.rs`
- `src/engine/step.rs`
- `src/engine/object_list.rs`
- `src/engine/expr_eval/ops_text_list.rs`
- `src/value_store.rs`
- `src/diagnostic.rs`
- `src/policy.rs`
- `src/limits.rs`
- `src/compiled_workflow.rs`
- `src/value.rs`
- `src/action.rs`
- `src/expressions.rs`
- `src/accessors.rs`
- `src/span.rs`
- `src/error.rs`
- `src/nodes.rs`
- `src/ids.rs`
- `src/capability.rs`
- `src/validation/` - entire subdirectory

---

## vb_runtime - Module-by-Module Coverage Status

### WITH Tests

| File | Note |
|------|------|
| `src/shard/tests.rs` | Shard tests |
| `src/journal_tests.rs` | Journal tests |
| `src/runtime_tests.rs` | Runtime tests |
| `src/engine/tests.rs` | Engine tests |
| `src/admission_tests.rs` | Admission tests |
| `src/recovery_tests.rs` | Recovery tests |
| `src/counters_tests.rs` | Counters tests |
| `src/trace_tests.rs` | Trace tests |
| `src/frame_pool_tests.rs` | Frame pool tests |
| `src/action_tests.rs` | Action tests |
| `src/primitives/for_each/tests.rs` | ForEach tests |

### WITHOUT Tests (0% expected)

- `src/shard/lifecycle.rs`
- `src/shard/timer_wheel.rs`
- `src/shard/transitions.rs`
- `src/shard/types.rs`
- `src/shard/impl_.rs`
- `src/shard/helpers.rs`
- `src/engine/execute.rs`
- `src/engine/signals.rs`
- `src/engine/signal.rs`
- `src/engine/iteration_engine.rs`
- `src/engine/step_engine.rs`
- `src/engine/drive.rs`
- `src/engine/transition.rs`
- `src/engine/action_engine.rs`
- `src/engine/run_engine.rs`
- `src/engine/helpers.rs`
- `src/primitives/together.rs`
- `src/primitives/retry.rs`
- `src/primitives/collect.rs`
- `src/primitives/reduce.rs`
- `src/primitives/wait_ask.rs`
- `src/primitives/for_each.rs` (impl only, tests exist)
- `src/primitives/repeat.rs`
- `src/primitives/helpers.rs`
- `src/frame_pool.rs` (impl only, tests exist)
- `src/journal_storage.rs`
- `src/idempotency.rs`
- `src/counters.rs` (impl only, tests exist)
- `src/trace.rs` (impl only, tests exist)

---

## Recommendations

1. **FIX FIRST**: Resolve the 2 compilation errors in vb_core before any coverage analysis can proceed
2. **Priority Modules**: Focus test writing on vb_storage recovery/replay and vb_core/replay modules as they have 0% coverage
3. **Coverage Gate**: Add llvm-cov to moon CI to prevent regression

---

## Reproduction

```bash
cargo llvm-cov --no-report
# Error: cannot compile vb_core due to 2 errors

# Workaround to see file list without coverage:
cargo build -p vb_core 2>&1 | head -50
```
