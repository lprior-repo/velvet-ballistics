# CB-007: `push_done_continuation` assumes linear IR layout

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/budget/traversal_loop.rs:42`
- **Confidence**: confirmed

## Description

When a loop's `done` node has no explicit `next`, `push_done_continuation`
infers the continuation as `nodes[done_idx + 1].id` — i.e. the IR is
assumed to be in linear fall-through order. For any non-canonical IR (dead
code after `done`, reordered basic blocks, or `done` being the last node),
this either pushes an unrelated step or silently pushes nothing, producing
an inaccurate step count.

## Evidence

```rust
fn push_done_continuation(
    nodes: &[CompiledNode],
    done: StepIdx,
    node_count: usize,
    stack: &mut Vec<StepIdx>,
) -> Result<(), BudgetError> {
    let done_idx = find_node_position(nodes, done, node_count)?;
    if let Some(node) = nodes.get(done_idx)
        && node.next.is_none()
        && let Some(next_idx) = done_idx.checked_add(1)
        && next_idx < nodes.len()
        && let Some(next_node) = nodes.get(next_idx)
    {
        stack.push(next_node.id);
    }
    stack.push(done);
    Ok(())
}
```

(`crates/vb_core/src/budget/traversal_loop.rs:42-59`)

If `done` is the *last* node in the slice and has `next: None`, the
function only pushes `done` itself — there is no continuation, but the
caller may expect to advance past the loop. If a dead node follows `done`
in the slice, that dead node becomes the continuation, polluting the count.

## Adversarial Check

A defender might say "the IR invariant guarantees that loop exit falls
through to the next node." That invariant is not documented or asserted
anywhere I can find in scope. The same file's `find_node_position` (see
CB-003) is already permissive of non-canonical IR, which strongly suggests
the IR is *not* assumed canonical elsewhere. Treating IR layout as linear
here while scanning it as a graph everywhere else is inconsistent.

## Suggested Fix

Either (a) document and assert the linear-fall-through invariant at
`CompiledWorkflow` construction, or (b) require the IR to provide an
explicit continuation for every loop-exit node, and emit
`InvalidCompiledWorkflow` here when `done.next` is None and no explicit
continuation exists.
