# CW-010: `max_step_budget_per_tick` is not validated

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/workflow/validation/resource_contract.rs:17-22`
- **Confidence**: confirmed

## Description

`ResourceContract` carries two per-tick execution caps, but validation only checks `max_transitions_per_tick`. `max_step_budget_per_tick` can be zero or exceed the protocol step-budget limit and still pass resource-contract validation.

## Evidence

```rust
// resource_contract.rs:21
/// Maximum deterministic transitions per runtime tick.
pub max_step_budget_per_tick: u64,
/// Maximum transitions per runtime tick.
pub max_transitions_per_tick: u64,
```

```rust
// validation/resource_contract.rs:17
pub fn validate_resource_contract(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let contract = parts.resource_contract;
    validate_resource_counts(parts, contract)?;
    validate_expr_stack_contract(parts.expressions.as_ref(), contract.max_expr_stack)?;
    validate_transitions_per_tick(contract.max_transitions_per_tick)
}
```

The only per-tick cap validator is specific to `max_transitions_per_tick`:

```rust
// validation/resource_contract.rs:120
fn validate_transitions_per_tick(max_transitions_per_tick: u64) -> Result<(), WorkflowError> {
    use crate::limits::MAX_STEP_BUDGET;
    if max_transitions_per_tick == 0 { ... }
    if max_transitions_per_tick > MAX_STEP_BUDGET { ... }
    Ok(())
}
```

No analogous check exists for `contract.max_step_budget_per_tick`.

## Adversarial Check

This is not a harmless unused field in the type contract: the field is public, serialized, documented as a maximum deterministic transition budget, and has a non-zero default beside `max_transitions_per_tick`. If zero is accepted, a runtime that honors this contract cannot execute deterministic steps; if an oversized value is accepted, the serialized contract can exceed the same hard budget used to constrain transitions.

## Suggested Fix

Add a `validate_step_budget_per_tick` check beside `validate_transitions_per_tick`, rejecting zero and values above `MAX_STEP_BUDGET`. Consider also enforcing `max_step_budget_per_tick <= max_transitions_per_tick` if deterministic steps are a subset of all transitions.
