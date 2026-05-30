# Architectural Drift Report: vb_qi37_2_4_integration_budget_errors.rs

## File Summary

| Metric | Value |
|--------|-------|
| **Total Lines** | 2311 |
| **Test Count** | 47 |
| **Location** | `crates/workspace_tests/tests/vb_qi37_2_4_integration_budget_errors.rs` |
| **Location Category** | `workspace_tests` (integration test) |
| **Size Status** | **VIOLATION** (>300 line limit) |

## Violations Detected

### 1. File Size Violation (CRITICAL)
- **Rule**: All `.rs` files must not exceed 300 lines
- **Actual**: 2311 lines
- **Excess**: 2011 lines over limit (670% of allowed size)

### 2. Test Density Achievement
- The file comment at line 2051-2053 explicitly states intent to "Achieve 5x Density (>=45 tests)"
- 47 tests achieved in 2311 lines = ~49 lines per test average
- This indicates test compression rather than architectural decomposition

## Structural Analysis

### DDD Cohesion Assessment
- **Cohesion**: Low - Test file mixes multiple concerns (BudgetError variants, policy validation order, edge cases, display formatting)
- **Single Responsibility**: Violated - File handles too many distinct test scenarios
- **Workflow Test Fixtures**: 5 helper functions (test_contract, tight_policy, build_linear_workflow, build_collect_workflow, build_repeat_workflow, build_together_workflow, build_workflow_with_do_nodes) - all properly scoped

### Test Organization
| Category | Count | Lines | Lines/Category |
|----------|-------|-------|----------------|
| BudgetError Variant Coverage (I1-I15) | 15 | ~600 | 40 |
| E2E Diagnostic Tests | 2 | ~90 | 45 |
| WholeWorkflowBudget::compute Edge Cases | 5 | ~120 | 24 |
| BoundednessPolicy::validate Edge Cases | 4 | ~200 | 50 |
| Error Display Tests | 3 | ~150 | 50 |
| Validation Order Tests | 2 | ~80 | 40 |
| Collect/Repeat/Together Variants | 6 | ~200 | 33 |
| Default Policy Tests | 2 | ~70 | 35 |
| Additional Validation Tests | 8 | ~300 | 37 |

## Recommendations

### Immediate Action Required
**SPLIT THE FILE** into the following modules:

1. **`vb_qi37_2_4_integration_budget_errors_variants.rs`** (~550 lines)
   - Tests I1-I15 (BudgetError variant coverage)
   - Helper functions: test_contract, tight_policy, build_linear_workflow, build_collect_workflow, build_repeat_workflow, build_together_workflow, build_workflow_with_do_nodes

2. **`vb_qi37_2_4_integration_budget_errors_policy.rs`** (~500 lines)
   - BoundednessPolicy::validate edge cases
   - Validation order tests
   - Boundary condition tests

3. **`vb_qi37_2_4_integration_budget_errors_compute.rs`** (~400 lines)
   - WholeWorkflowBudget::compute edge cases
   - Collect/Repeat/Together variants with various limits

4. **`vb_qi37_2_4_integration_budget_errors_e2e.rs`** (~300 lines)
   - E2E diagnostic field exposure tests
   - Error display formatting tests
   - Default policy acceptance tests

5. **`vb_qi37_2_4_integration_budget_errors_smoke.rs`** (~300 lines)
   - Smoke tests for quick validation
   - Determinism tests
   - Empty/minimal workflow tests

### Update Required
After splitting, update `crates/workspace_tests/tests/mod.rs` to reference the new modules.

## GAP Documentation (Non-Blocking)

The file correctly documents two BLOCK_LOCAL gaps:
- **GAP-1**: BudgetError lacks `primitive`, `node_index`, `structural_path` fields (documented at lines 15-16, 1265-1320)
- **GAP-2**: RunTimeExceeded cannot be triggered because `max_run_time_seconds` is always 0 (documented at lines 673-682, 734-749)

These are properly marked as BLOCK_LOCAL and do not constitute architectural drift.

## Verdict

**STATUS: REFACTOR REQUIRED**

The file violates the 300-line maximum by 2011 lines. It must be decomposed into 5 smaller test modules before this integration test suite can be considered architecturally compliant.
