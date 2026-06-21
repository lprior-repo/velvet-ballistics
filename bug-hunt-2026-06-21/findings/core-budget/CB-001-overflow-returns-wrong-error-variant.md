# CB-001: Step-count overflow reported as `StepOutOfBounds` in `visit_node_for_total_steps`

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/budget/traversal_step_count.rs:82`
- **Confidence**: confirmed

## Description

When `total.checked_add(1)` fails while visiting a node, the code returns
`BudgetTraversalError::StepOutOfBounds` instead of `StepCountOverflow`. The
`StepOutOfBounds` variant is meant to signal an out-of-bounds *step index*,
not an arithmetic overflow of the running step count. Any caller that
pattern-matches on the variant to decide recovery / diagnostics will
misclassify the failure.

## Evidence

```rust
total = match total.checked_add(1) {
    Some(v) => v,
    None => return Err(BudgetTraversalError::StepOutOfBounds { step: current }),
};
```

(`crates/vb_core/src/budget/traversal_step_count.rs:82`)

The sibling helper `checked_step_add` (same file, line 240) and the four
`count_and_push_loop_body` map_err blocks (lines 100-163) all use the
correct variant `StepCountOverflow { actual: u64::MAX }`. The body of
`visit_node_for_total_steps` is the only site that diverges.

## Adversarial Check

A reader might argue the variant is unimportant because `WorkflowError::from`
eventually collapses both into `TotalStepsExceeded`. But that itself is the
separate error-swallowing bug (CB-002), and in the meantime
`BudgetTraversalError` is `pub(crate)` and consumed directly by
`compute_budget_local` which returns it verbatim before the collapse — so the
wrong variant is observable by any intra-crate caller and by every Kani
harness that asserts on the discriminant.

## Suggested Fix

Return `StepCountOverflow { actual: total }` (the value before the failed
add) and ideally reuse `checked_step_add(total, 1)?` for parity with the rest
of the file.
