# RA-009: `preflight_step_budget` and `preflight_step_gate` implement the same step-ceiling check twice

- **Severity**: Info
- **Category**: simplification (DRY)
- **Location**: `crates/vb_runtime/src/admission/admission.rs:265-286` and `crates/vb_runtime/src/runtime/admission/admission_check.rs:70-89`
- **Confidence**: confirmed

## Description

The crate-level standalone function `preflight_step_budget` and the runtime facade method `Runtime::preflight_step_gate` both compute `max(declared, computed).max_steps_executable` and compare it against `per_workflow_step_ceiling()`, with identical policy gating. The two implementations are line-for-line equivalent.

## Evidence

`admission/admission.rs:265-286`:

```rust
pub fn preflight_step_budget(
    workflow: &vb_core::workflow::CompiledWorkflow,
    policy: vb_core::policy::RuntimePolicy,
) -> Result<(), AdmissionError> {
    if !matches!(policy, RuntimePolicy::Strict | RuntimePolicy::Journaled) {
        return Ok(());
    }
    let limit = per_workflow_step_ceiling();
    let declared = u32::from(workflow.resource_contract().max_steps);
    let requested = AggregateResourceBudget::from_workflow(workflow)
        .map_err(super::budget_error_map::map_budget_error)?;
    let observed = declared.max(requested.max_steps_executable);
    if observed > limit {
        return Err(AdmissionError::BudgetExceeded { actual: observed, limit });
    }
    Ok(())
}
```

`runtime/admission/admission_check.rs:70-89`:

```rust
fn preflight_step_gate(
    workflow: &CompiledWorkflow,
    budget_request: &AdmissionBudgetRequest,
    policy: RuntimePolicy,
) -> RuntimeResult<()> {
    if !super::admission_policy::requires_admission(policy) {
        return Ok(());
    }
    let limit = crate::admission::per_workflow_step_ceiling();
    let declared = u32::from(workflow.resource_contract().max_steps);
    let actual = budget_request.requested.max_steps_executable;
    let observed = declared.max(actual);
    if observed > limit {
        return Err(RuntimeError::AdmissionBudgetExceeded { actual: observed, limit });
    }
    Ok(())
}
```

The only differences are: (a) error type (`AdmissionError::BudgetExceeded` vs `RuntimeError::AdmissionBudgetExceeded`, which are already 1:1 via `map_admission_error`), (b) the runtime version receives the budget request pre-built (saving one `from_workflow` call), and (c) the runtime version uses `requires_admission(policy)` while the crate version uses `matches!(policy, Strict | Journaled)` — but those are textually identical predicates.

`preflight_step_budget` is only referenced by `step_budget_tests/mod.rs` (tests); production code uses `preflight_step_gate`.

## Adversarial Check

One could argue the crate-level fn is a public API intended for external callers. But its docstring (`admission.rs:242-264`) describes it as "the production-extension of `admit_run_with_budget_policy`" used by "the runtime", which directly contradicts the actual production path (`preflight_step_gate` via `preflight_direct_admission`). The standalone function is effectively dead in production and exists only as a test entry point — keeping it as a public API invites divergence, which has already happened (the runtime version takes the pre-built request, the crate version rebuilds it).

## Suggested Fix

Delete the public `preflight_step_budget` from `admission.rs` and rewrite the test caller to use the runtime facade or to test the underlying `per_workflow_step_ceiling` + `AggregateResourceBudget::from_workflow` building blocks directly. If the standalone function must remain as a low-level API, have `preflight_step_gate` delegate to it after building the request, so there is a single source of truth for the limit check.
