# CB-004: `compute_child_depth` reports u16 depth overflow as `StepCountOverflow`

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_core/src/budget/traversal_depth.rs:38`
- **Confidence**: confirmed

## Description

When u16 nesting-depth overflow is detected, the function returns
`BudgetTraversalError::StepCountOverflow { actual: u64::MAX }`. The overflow
is a *nesting-depth* overflow, not a step-count overflow, and `u64::MAX` is
a sentinel that destroys the actual depth value (`u16::MAX + 1`).

## Evidence

```rust
let new_depth = current_depth
    .checked_add(1)
    .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
```

(`crates/vb_core/src/budget/traversal_depth.rs:38-40`)

The file docstring (line 8) even calls this guard "u16 overflow," yet the
returned variant is `StepCountOverflow`. The same misclassification appears
in `traversal_metrics.rs` lines 36, 50, 53, 58, 70 for `max_action_tickets`,
`max_gather_pages`, `max_gather_items`, `max_for_each_iterations`, and
`max_timer_entries` overflows, and in `branch_count_to_u16`
(`traversal_successors.rs:174-181`) for branch-count overflow.

## Adversarial Check

One might argue `StepCountOverflow` is intended as a generic "u-something
overflowed" variant. But the enum also has a dedicated `JumpCycle` and the
pub-crate `BudgetTraversalError` is consumed by Kani harnesses that assert
on discriminants; from a verification standpoint, "step count" and "nesting
depth" are different obligations. The `u64::MAX` sentinel also makes the
actual offending magnitude invisible to operators.

## Suggested Fix

Add `DepthOverflow { depth: u16 }` (or rename `StepCountOverflow` to a
generic `Overflow { kind, actual }`), and thread the real value through.
