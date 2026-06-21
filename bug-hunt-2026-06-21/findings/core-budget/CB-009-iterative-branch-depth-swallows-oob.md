# CB-009: `iterative_branch_depth` silently skips out-of-bounds branch targets

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/budget/traversal_path.rs:69` (also 79, 85)
- **Confidence**: confirmed

## Description

`iterative_branch_depth` swallows `find_node_position` errors and
out-of-bounds `visited` accesses via `let Ok(...) = ... else { continue; }`
and `if let Some(...)`. A malformed `Choose` branch whose target is not in
the slice is silently treated as depth-0 instead of producing an
out-of-bounds error, which can cause `push_longest_expr_branch` to pick a
different (or no) branch and silently under-count.

## Evidence

```rust
while let Some(current) = stack.pop() {
    ...
    let Ok(idx) = find_node_position(nodes, current, nodes.len()) else {
        continue;            // <-- OOB step swallowed
    };
    if visited.get(idx).copied() == Some(true) {
        continue;
    }
    if let Some(flag) = visited.get_mut(idx) {
        *flag = true;
    }
    ...
    let Ok(node) = node_at_position(nodes, idx, current) else {
        continue;            // <-- OOB position swallowed
    };
    ...
}
```

(`crates/vb_core/src/budget/traversal_path.rs:75-95`)

The caller `push_longest_expr_branch` then compares depths via
`depth > selected_depth`; an OOB branch reports `depth = 0`, so it can never
be the longest, and a workflow that is *only* reachable via an OOB target
yields `selected = None`, so nothing gets pushed onto the stack.

## Adversarial Check

One might argue this is graceful degradation — keep computing when the IR is
malformed. But the rest of the budget pipeline *does* surface OOB as
`StepOutOfBounds` (e.g. `count_path_steps` line 21). Silently degrading here
creates an asymmetry: a malformed IR errors in one code path and under-counts
in another, so the operator cannot trust either result.

## Suggested Fix

Propagate `Result<u64, BudgetTraversalError>` out of `iterative_branch_depth`
and surface OOB as `StepOutOfBounds`, matching the rest of the traversal
pipeline.
