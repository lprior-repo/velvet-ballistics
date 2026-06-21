# CB-006: `traversal_loop.rs` `global_visited` parameter is plumbed but never used

- **Severity**: Low
- **Category**: bug
- **Location**: `crates/vb_core/src/budget/traversal_loop.rs:64` (also 99, 212)
- **Confidence**: confirmed

## Description

`count_body_region_nodes` accepts a `global_visited: &mut [bool]` parameter
and forwards it to `visit_body_region_node` and `count_nested_for_region`,
but neither function ever reads from or writes to it. Each invocation
allocates a fresh `region_visited: Vec<bool>` (line 71) and uses that for
cycle detection. The `global_visited` machinery is dead state.

## Evidence

```rust
fn count_body_region_nodes(
    nodes: &[CompiledNode],
    body: StepIdx,
    done: StepIdx,
    global_visited: &mut [bool],          // <-- accepted
    node_count: usize,
) -> Result<u64, BudgetError> {
    let mut region_visited: Vec<bool> = vec![false; node_count];   // <-- fresh local
    ...
    while let Some(current) = stack.pop() {
        count = visit_body_region_node(
            ...,
            global_visited,              // <-- forwarded
            &mut region_visited,
            ...,
        )?;
    }
    ...
}
```

(`crates/vb_core/src/budget/traversal_loop.rs:64-90`)

Inside `visit_body_region_node` (lines 92-207) the parameter
`global_visited: &mut [bool]` is declared at line 99 but never referenced
in the function body.

## Adversarial Check

A charitable reader might claim "the parameter is reserved for future
cross-region deduplication." But unused `&mut` state in a hot budget path is
a maintenance trap: a future change could silently start mutating it and
introduce cross-region interference, and there is no test that catches the
regression because no current code reads it. Worse, the caller
(`count_and_push_loop_body` in `traversal_step_count.rs:91-100`) passes the
*shared* outer `visited` slice as `global_visited`; if someone wires it up
later, the outer DFS will be polluted by inner body counts.

The cleaner interpretation is that this was a refactoring artifact when
region-scoped visited tracking was introduced.

## Suggested Fix

Delete the `global_visited` parameter from `count_body_region_nodes`,
`visit_body_region_node`, and `count_nested_for_region`. If cross-region
deduplication is genuinely needed later, add it back with tests.
