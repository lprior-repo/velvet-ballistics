# CB-016: `compute_small_linear_budget` has a redundant length check before `small_linear_domain`

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_core/src/budget/small_linear.rs:16`
- **Confidence**: confirmed

## Description

The fast-exit guard `if nodes.len() > 2 || !small_linear_domain(nodes)`
duplicates the `len() > 2` check that `small_linear_domain` already handles
via its `_ => false` arm. The early-out is dead defensive code.

## Evidence

```rust
pub(super) fn compute_small_linear_budget(...) -> Result<Option<...>, ...> {
    if nodes.len() > 2 || !small_linear_domain(nodes) {
        return Ok(None);
    }
    ...
}

fn small_linear_domain(nodes: &[CompiledNode]) -> bool {
    match nodes {
        [] => false,
        [first] => ...,
        [first, second] => ...,
        _ => false,            // <-- already handles len() > 2
    }
}
```

(`crates/vb_core/src/budget/small_linear.rs:16` and `:61-73`)

## Adversarial Check

A defender might argue the early-out is a fast-path optimization. But
`small_linear_domain` immediately pattern-matches the slice, which is
O(1) — there is no measurable fast path. The redundancy just creates a
second site that has to be updated if the small-linear limit ever changes
from 2 to another value.

## Suggested Fix

Drop the `nodes.len() > 2` clause and rely on `small_linear_domain`.
