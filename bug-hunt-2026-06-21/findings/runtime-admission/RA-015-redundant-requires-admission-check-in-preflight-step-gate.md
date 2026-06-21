# RA-015: `preflight_step_gate` re-checks `requires_admission(policy)` after caller already gated on it

- **Severity**: Info
- **Category**: simplification (dead branch)
- **Location**: `crates/vb_runtime/src/runtime/admission/admission_check.rs:33-49` and `crates/vb_runtime/src/runtime/admission/admission_check.rs:70-89`
- **Confidence**: confirmed

## Description

`preflight_direct_admission` early-returns `Ok(())` if `!requires_admission(shard.policy)` (line 39-41), then unconditionally calls `preflight_step_gate(workflow, &budget_request, shard.policy)`. `preflight_step_gate` repeats the same `requires_admission(policy)` early-return at line 75-77. The inner check can never fail when reached through the production caller.

## Evidence

```rust
pub(crate) fn preflight_direct_admission(
    shard: &Shard,
    run: RunId,
    workflow: &CompiledWorkflow,
    caps: CapabilitySet,
) -> RuntimeResult<()> {
    if !super::admission_policy::requires_admission(shard.policy) {
        return Ok(());
    }
    let digest = workflow.digest();
    Self::preflight_artifact_gate(shard, run, digest, &caps)?;
    let budget_request = build_admission_budget_request(workflow)
        .map_err(|error| super::admission_result::map_aggregate_budget_error(error, digest))?;
    Self::preflight_step_gate(workflow, &budget_request, shard.policy)?;
    ...
}

fn preflight_step_gate(
    workflow: &CompiledWorkflow,
    budget_request: &AdmissionBudgetRequest,
    policy: RuntimePolicy,
) -> RuntimeResult<()> {
    if !super::admission_policy::requires_admission(policy) {
        return Ok(());
    }
    ...
}
```

`shard.policy` is the same `RuntimePolicy` value in both calls; neither re-assigns it between the two checks.

## Adversarial Check

One could argue `preflight_step_gate` is a private method that should be defensive against future callers that do not pre-gate. But it is `fn` (private, not `pub(crate)`), and lives in the same module as its sole caller; inlining the assumption is safe. If a future caller is added that does not pre-gate, that caller should add its own gate rather than relying on a redundant one buried inside a helper.

## Suggested Fix

Remove the `if !requires_admission(policy) { return Ok(()); }` from `preflight_step_gate`, or make the gate an explicit `assert!(requires_admission(policy))`-shaped debug-only check (without using `assert!`, which is forbidden — use an `if !cfg!(debug_assertions) { ... } else { ... }` shape or a typed trace event).
