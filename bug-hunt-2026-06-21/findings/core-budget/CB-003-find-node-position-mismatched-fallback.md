# CB-003: `find_node_position` returns a non-matching index for malformed IR

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_core/src/budget/traversal_successors.rs:9` (the whole function, fallback at line 28)
- **Confidence**: confirmed

## Description

`find_node_position` has three return paths: (1) fast path returning
`direct_idx` only when `nodes[direct_idx].id == step`, (2) linear scan
returning the position of the unique node whose `id == step`, and (3) a
silent fallback that returns `direct_idx` when no matching node was found
*but* `direct_idx < node_count`. Path (3) hands the caller a `position`
whose node has `id != step`, so the caller proceeds against the wrong node.

## Evidence

```rust
pub(super) fn find_node_position(
    nodes: &[CompiledNode],
    step: StepIdx,
    node_count: usize,
) -> Result<usize, BudgetTraversalError> {
    let direct_idx = step.as_usize();
    if direct_idx < node_count
        && let Some(node) = nodes.get(direct_idx)
        && node.id == step
    {
        return Ok(direct_idx);
    }

    for (position, node) in nodes.iter().enumerate() {
        if node.id == step {
            return Ok(position);
        }
    }

    if direct_idx < node_count {
        return Ok(direct_idx);   // <-- returns an index whose node.id != step
    }

    Err(BudgetTraversalError::StepOutOfBounds { step })
}
```

(`crates/vb_core/src/budget/traversal_successors.rs:9-33`)

Every caller (`compute_fanout_and_depth`, `visit_node_for_total_steps`,
`visit_body_region_node`, `count_path_steps`) then does
`node_at_position(nodes, idx, current)?` and walks that node's edges as if
it were the requested step. The walk is therefore off-target for any
malformed IR.

## Adversarial Check

A charitable reading is "this is the IR self-healing layer: if the IR is
shuffled we still want to fall back to direct indexing." But the linear scan
already returns Ok for any node whose `id == step`, so the only way to reach
the third branch is when **no** node in the slice has `id == step`. Falling
back to a node with a different id is not "self-healing" — it is "picking an
arbitrary node." The downstream budget math then produces wrong fanout,
nesting depth, and step counts without any error signal.

If the IR were guaranteed to be canonical (`nodes[i].id == i`) the third
branch would be unreachable; if it is not guaranteed, the third branch is
incorrect. Either way, returning a non-matching position is wrong.

## Suggested Fix

Drop the third branch and return `Err(StepOutOfBounds { step })` whenever the
linear scan fails. If the IR is ever expected to be non-canonical, the
correctness invariant should be asserted at `CompiledWorkflow` construction
rather than papered over here.
