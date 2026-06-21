# RA-008: `budget_error_map::map_budget_error` and `admission_result::aggregate_budget_to_admission_error` duplicate the same mapping

- **Severity**: Info
- **Category**: simplification (DRY)
- **Location**: `crates/vb_runtime/src/admission/budget_error_map.rs:8-78` and `crates/vb_runtime/src/runtime/admission/admission_result.rs:19-101`
- **Confidence**: confirmed

## Description

The crate-level `budget_error_map::map_budget_error` and the runtime-level `admission_result::aggregate_budget_to_admission_error` (plus its three helper fns) implement the exact same `AggregateBudgetError → AdmissionError` translation twice, in two files, with subtly different fallback shapes (`budget_policy_sentinel("workflow_budget")` vs `aggregate_budget_fallback_error()`).

## Evidence

`admission/budget_error_map.rs`:

```rust
pub(crate) fn map_budget_error(error: AggregateBudgetError) -> AdmissionError {
    match error {
        AggregateBudgetError::PolicyExceeded { .. } => AdmissionError::BudgetPolicyExceeded { .. },
        AggregateBudgetError::CapacityExceeded { .. } => AdmissionError::ResourceCapacityExceeded { .. },
        other => map_budget_resource_error(other),
    }
}
// ... 3 more cascading match fns, terminating in budget_policy_sentinel(...)
```

`runtime/admission/admission_result.rs`:

```rust
fn aggregate_budget_to_admission_error(error: AggregateBudgetError) -> crate::admission::AdmissionError {
    match error {
        AggregateBudgetError::PolicyExceeded { .. } => crate::admission::AdmissionError::BudgetPolicyExceeded { .. },
        AggregateBudgetError::CapacityExceeded { .. } => crate::admission::AdmissionError::ResourceCapacityExceeded { .. },
        other => aggregate_budget_resource_error(other),
    }
}
// ... 3 more cascading match fns, terminating in aggregate_budget_fallback_error()
```

The two cascades are structurally identical and differ only in: (1) visibility qualifier on `map_budget_error` (`pub(crate)` vs private), (2) the fallback resource string (`"workflow_budget"` / `"reservation_not_found"` / `"unknown_aggregate_budget_error"` vs the single string `"aggregate_budget"`), and (3) `actual` value (`u64::MAX` in both). The runtime version also drops the `WorkflowBudget(_)` payload discriminantly for `#[cfg(not(kani))]` vs `#[cfg(kani)]` — the crate-level version handles that too.

## Adversarial Check

One could argue the two copies serve different layers (crate admission vs runtime facade) and decoupling them is a feature. But the type translation is mechanical, lives in the same crate, and the runtime version calls `crate::admission::AdmissionError::*` anyway — so it is already coupled to the crate-level error type. The duplication is pure copy-paste and the two implementations have already diverged in their fallback resource strings, which is exactly the kind of bug the DRY principle predicts.

## Suggested Fix

Promote `map_budget_error` to `pub(super)` or `pub` in `budget_error_map.rs` and have `admission_result::map_aggregate_budget_error` call it directly:

```rust
pub(super) fn map_aggregate_budget_error(
    error: AggregateBudgetError,
    workflow_digest: WorkflowDigest,
) -> crate::RuntimeError {
    map_admission_error(crate::admission::budget_error_map::map_budget_error(error), workflow_digest)
}
```

Then delete the four private helper fns in `admission_result.rs`.
