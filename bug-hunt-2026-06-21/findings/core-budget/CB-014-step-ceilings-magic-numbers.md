# CB-014: `validate_step_ceilings` uses hard-coded magic numbers

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_core/src/budget/validation.rs:119` (and 120)
- **Confidence**: confirmed

## Description

`validate_step_ceilings` enforces ceilings of `1_000_000` for both
`max_step_budget_per_tick` and `max_transitions_per_tick`. The limits are
inlined literals with a comment pointing at a `MAX_STEPS_PER_TICK` macro
"if defined" — i.e. they are placeholders. There is also a semantic
mismatch: the function rejects `max_step_budget_per_tick == 0` with the
same `StepCeilingExceeded` variant used for "too big", which makes the
error message (`"step ceiling exceeded: 0 > 1000000"`) nonsensical.

## Evidence

```rust
const HARD_MAX_STEP_BUDGET_PER_TICK: u64 = 1_000_000;
const HARD_MAX_TRANSITIONS_PER_TICK: u64 = 1_000_000;

if budget.max_step_budget_per_tick == 0 {
    return Err(AggregateBudgetError::StepCeilingExceeded {
        requested: 0,
        limit: HARD_MAX_STEP_BUDGET_PER_TICK,
    });
}
```

(`crates/vb_core/src/budget/validation.rs:119-127`)

The `requested: 0, limit: 1_000_000` pair would format as
`"step ceiling exceeded: 0 > 1000000"`.

## Adversarial Check

A defender might call this acceptable placeholder code. But the file ships
in production source and runs in `AggregateResourceBudget::from_workflow`
(line 160 of aggregate_budget.rs). An operator who configures
`max_step_budget_per_tick: 0` will see a misleading error message and have
no way to discover the real reason (zero is invalid) without reading the
source.

## Suggested Fix

Introduce a `ZeroStepCeiling` / `ZeroTransitionCeiling` error variant, or
reuse `InvalidCapacity`, and replace the inlined literals with constants
imported from `crate::limits` so they share a source of truth with the
rest of the policy.
