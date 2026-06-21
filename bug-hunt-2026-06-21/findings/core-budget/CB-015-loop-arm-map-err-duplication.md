# CB-015: Massive `map_err` duplication across the four loop-header cases in `visit_node_for_total_steps`

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_core/src/budget/traversal_step_count.rs:101` (also 122, 136, 156)
- **Confidence**: confirmed

## Description

The `ForEachStart`, `CollectStart`, `ReduceStart`, and `RepeatStart` arms
of `visit_node_for_total_steps` each repeat the identical 7-line `.map_err`
block that converts `BudgetError → BudgetTraversalError`. The bodies are
textually identical; only the iteration count differs (lines 95, 116,
135, 155).

## Evidence

```rust
.map_err(|e| {
    let actual = match e {
        BudgetError::TotalStepsExceeded { actual, .. } => actual,
        _ => u64::MAX,
    };
    BudgetTraversalError::StepCountOverflow { actual }
})?;
```

(`crates/vb_core/src/budget/traversal_step_count.rs:101-107, 122-128, 136-142, 156-162`)

Four identical copies.

## Adversarial Check

One might claim that keeping the conversion inline aids readability.
Counter: the conversion is opaque (it discards every `BudgetError` variant
other than `TotalStepsExceeded`), so inlining actually obscures the
behavior. A single `fn map_budget_err(e: BudgetError) -> BudgetTraversalError`
would make the information loss explicit and shrink the call site to one
line, which is the holzman-rust "data, calc, actions" layering pattern.

## Suggested Fix

```rust
fn budget_err_to_traversal(e: BudgetError) -> BudgetTraversalError {
    let actual = match e {
        BudgetError::TotalStepsExceeded { actual, .. } => actual,
        _ => u64::MAX,
    };
    BudgetTraversalError::StepCountOverflow { actual }
}
```

Then each arm becomes
`.map_err(budget_err_to_traversal)?`. Combined with a `(body, done, iter_count)`
tuple, the four arms collapse into one helper.
