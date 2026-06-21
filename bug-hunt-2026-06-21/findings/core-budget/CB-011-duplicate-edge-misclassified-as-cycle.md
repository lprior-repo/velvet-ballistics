# CB-011: `insert_tracked_jump_edge` returns false for any duplicate edge, misclassified as a cycle

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_core/src/budget/traversal_tracking.rs:35`
- **Confidence**: likely

## Description

`insert_tracked_jump_edge` returns `Ok(false)` whenever the edge
`(from, to)` is already present. The caller `visit_node_for_total_steps`
(traversal_step_count.rs:173-178) interprets `Ok(false)` as a cycle and
returns `Err(JumpCycle { step, target })`. A duplicate edge is not
necessarily a cycle; if it ever occurs (e.g. two distinct predecessor
Jumps land on the same target via the same source — admittedly structurally
impossible today but not enforced here), the diagnostic would be wrong.

## Evidence

```rust
pub(super) fn insert_tracked_jump_edge(
    edges: &mut Vec<(u16, u16)>,
    edge: (u16, u16),
    limit: usize,
) -> Result<bool, BudgetTraversalError> {
    if edges.iter().copied().any(|candidate| candidate == edge) {
        return Ok(false);
    }
    ...
}
```

(`crates/vb_core/src/budget/traversal_tracking.rs:35-47`)

Caller:

```rust
if !insert_tracked_jump_edge(jump_edges, (from, to), node_count)? {
    return Err(BudgetTraversalError::JumpCycle { step: current, target: *target });
}
```

(`crates/vb_core/src/budget/traversal_step_count.rs:173-178`)

## Adversarial Check

Because each `StepIdx` corresponds to exactly one node kind, the same
`(from, to)` pair can only arise if the same `Jump` node is visited twice,
which the outer `visited` bit already prevents. So today the false-return
path is dead. That does not make the misclassification correct: the
function's name says "insert_or_detect_duplicate" but the caller treats it
as "is_cycle". The naming and the call-site comment should at minimum
clarify the structural invariant that makes the two equivalent, or the
function should return an explicit `enum { Inserted, Duplicate, Full }`.

## Suggested Fix

Replace the `Result<bool, _>` contract with a three-state enum, or rename
the function and document why duplicate-edge and cycle are equivalent
*only under* the `visited`-bit invariant.
