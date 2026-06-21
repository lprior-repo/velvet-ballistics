# CW-007: `Jump` targets bypass the forward-edge validator

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/workflow/validation/forward_edges.rs:43-60`
- **Confidence**: confirmed

## Description

The forward-edge validator explicitly treats `Jump` as `Ok(())`, so a `Jump` can target the current node or any earlier node without producing `BackwardEdge`. That contradicts the module contract that all edges must point forward except recognized loop back-edges.

## Evidence

```rust
// forward_edges.rs:43
CompiledNodeKind::Nop
| CompiledNodeKind::SetConst { .. }
| CompiledNodeKind::Copy { .. }
| CompiledNodeKind::EvalExpr { .. }
| CompiledNodeKind::BuildObject { .. }
| CompiledNodeKind::BuildList { .. }
| CompiledNodeKind::Do { .. }
...
| CompiledNodeKind::Finish { .. }
| CompiledNodeKind::Jump { .. } => Ok(()),
```

Reachability still treats `Jump` as a graph edge:

```rust
// reachability.rs:145
crate::workflow::CompiledNodeKind::Jump { target } => {
    targets.push(*target);
}
```

Node-kind validation only checks that the jump target is in bounds at `validation/nodes/kinds.rs:157`; it does not check direction.

## Adversarial Check

Cycle detection elsewhere may reject some backward jumps that form obvious execution cycles, but this validator's stated invariant is stricter: backward edges are invalid unless they are recognized loop back-edges. A backward `Jump` can also be acyclic in the graph and still violate the topological IR contract, for example by jumping from one branch into an earlier shared block that does not lead back to the jump source.

## Suggested Fix

Validate `Jump { target }` with `validate_forward_target(*target, ci, cid)` unless the IR intentionally supports arbitrary gotos. If arbitrary `Jump` is intentional, document it as an explicit exception and rename the validator/error contract so callers do not rely on a false forward-only guarantee.
