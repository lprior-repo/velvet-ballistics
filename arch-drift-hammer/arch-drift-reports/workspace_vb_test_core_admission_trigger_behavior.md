# Architectural Drift Report: vb_test_core_admission_trigger_behavior.rs

## File Summary

| Metric | Value |
|--------|-------|
| **File** | `crates/workspace_tests/tests/vb_test_core_admission_trigger_behavior.rs` |
| **Total Lines** | 1389 |
| **Size Limit** | 300 lines |
| **Drift** | **CRITICAL** — 463% over limit |

## Test Count

| Module | Test Count |
|--------|------------|
| `admission_policy_enforcement` | 6 |
| `trigger_condition_evaluation` | 7 |
| `fail_closed_vs_fail_open` | 7 |
| `state_transitions` | 13 |
| `resource_contract_admission` | 2 |
| `capability_admission` | 6 |
| `step_budget_trigger` | 5 |
| `signal_exhaustion_paths` | 3 |
| `action_resumption` | 2 |
| **TOTAL** | **51 tests** |

## Drift Analysis

### Violation: File Size (CRITICAL)

The file is **1389 lines**, exceeding the **300-line maximum** by **1089 lines** (463% of limit).

### DDD Cohesion Review

| Concern | Status | Notes |
|---------|--------|-------|
| Primitive Obsession | ⚠️ Minor | Helper functions (`digest`, `make_simple_workflow`, `make_frame`, etc.) are well-structured NewType wrappers around `WorkflowDigest`, `CompiledWorkflow`, `RunFrame`. No raw `String` or `i32` ID abuse observed. |
| Workflow State Modeling | ✅ Pass | `StepState` machine transitions are explicitly tested as state transitions. Terminal states (Succeeded, Failed, Cancelled, Skipped) are verified as unrecoverable. |
| Parse Don't Validate | ✅ Pass | Constructors return `Result<String>` with sharp error paths. No validation-after-construction patterns observed. |

### Structural Observations

1. **Test Organization**: Excellent modularity — 9 behavior modules with clear boundaries covering admission policy, trigger conditions, fail-closed/fail-open paths, state transitions, resource contracts, capabilities, step budgets, signals, and action resumption.

2. **Helper Duplication**: Helpers are shared across modules correctly via `use super::*` imports. No significant duplication within the file.

3. **Sharp Assertions**: The file demonstrates excellent test discipline — sharp assertions on exact error variants, state transitions, and signal emissions.

4. **Test Count Density**: 51 tests in 1389 lines = ~27 lines per test on average, which is reasonable.

## Recommendation

**REFACTOR REQUIRED** — File exceeds 300-line limit by 1089 lines.

Split into 9 separate test files organized by behavior module:

```
tests/
  vb_test_core_admission_trigger_behavior.rs (REMOVE)
  vb_test_core_admission_policy_enforcement.rs (~200 lines)
  vb_test_core_trigger_condition_evaluation.rs (~250 lines)
  vb_test_core_fail_closed_vs_fail_open.rs (~350 lines)
  vb_test_core_state_transitions.rs (~400 lines)
  vb_test_core_resource_contract_admission.rs (~80 lines)
  vb_test_core_capability_admission.rs (~200 lines)
  vb_test_core_step_budget_trigger.rs (~180 lines)
  vb_test_core_signal_exhaustion_paths.rs (~100 lines)
  vb_test_core_action_resumption.rs (~130 lines)
```

Each new file should retain the shared test helpers (or extract to a `tests/helpers/` module if duplication becomes an issue).

## Verdict

**STATUS: DRIFT DETECTED** — File MUST be split before merge.
