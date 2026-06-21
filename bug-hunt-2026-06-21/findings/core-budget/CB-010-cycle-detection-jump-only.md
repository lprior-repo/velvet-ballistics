# CB-010: Cycle detection is asymmetric — only `Jump` cycles are errors

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/budget/traversal_driver.rs:60`
- **Confidence**: confirmed

## Description

`compute_fanout_and_depth` only treats a `Jump { target }` whose target is
in the current DFS path as a `JumpCycle` error. Cycles formed via any other
edge kind (`next` pointer, branch targets of `Choose`/`TogetherStart`,
`ErrorHandler` body, etc.) are silently terminated by the global `visited`
bit instead of being reported. The budget computation therefore completes
without error on workflows containing non-`Jump` back-edges, even though
those workflows would loop forever at runtime.

## Evidence

```rust
if let CompiledNodeKind::Jump { target } = &node.kind {
    let target_u16 = target.get();
    if tracked_steps_contain(in_path, target_u16) {
        remove_tracked_step(in_path, current_u16);
        return Err(BudgetTraversalError::JumpCycle { step: current, target: *target });
    }
}
```

(`crates/vb_core/src/budget/traversal_driver.rs:60-69`)

No other edge kind is checked against `in_path`. `count_total_steps` in
`traversal_step_count.rs:164-178` has the same asymmetry.

## Adversarial Check

One might argue that the global `visited` bit prevents infinite recursion,
so the budget computation is sound. That is true for *termination* but not
for *correctness*: a workflow whose `next` field points back to an ancestor
is invalid and should fail admission with a clear diagnostic, the same way
a `Jump` cycle does. Treating only `Jump` cycles as errors is also
inconsistent with `is_valid_step_state_transition` discipline elsewhere in
the crate, where structural invariants are enforced eagerly.

## Suggested Fix

Generalize the cycle check: after pushing any successor (via
`push_successor_targets` or `node.next`), if the target is in `in_path`,
return `JumpCycle { step: current, target }` (or a renamed `BackEdge`).
This requires inserting the check in the recursion at line 93-114 instead
of relying on `visited` to silently swallow the back-edge.
