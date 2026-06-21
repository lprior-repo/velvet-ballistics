# CW-008: Together branch cardinality and branch ids are never cross-checked

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_core/src/workflow/validation/nodes/kinds.rs:61-79`
- **Confidence**: confirmed

## Description

The `Together*` IR carries branch cardinality in multiple places, but validation only checks step bounds and that `TogetherJoin.branch_count` is non-zero. It never verifies that the join count matches the start branch list or that each `TogetherBranch.branch` id is within the declared fanout.

## Evidence

```rust
// node.rs:84
TogetherStart {
    branches: Box<[StepIdx]>,
    join: StepIdx,
},
// node.rs:89
TogetherBranch {
    branch: u16,
    entry: StepIdx,
    join: StepIdx,
    accumulator: SlotIdx,
},
// node.rs:97
TogetherJoin {
    branch_count: u16,
    accumulator: SlotIdx,
},
```

```rust
// kinds.rs:61
CompiledNodeKind::TogetherStart { branches, join } => {
    validate_together(branches, *join, parts)
}
// kinds.rs:64
CompiledNodeKind::TogetherBranch {
    branch: _,
    entry,
    join,
    accumulator,
} => {
    validate_two_steps(*entry, *join, parts)?;
    validate_slot(*accumulator, parts.slot_count)
}
// kinds.rs:73
CompiledNodeKind::TogetherJoin {
    branch_count,
    accumulator,
} => {
    validate_nonzero_u16(*branch_count, "branch_count")?;
    validate_slot(*accumulator, parts.slot_count)
}
```

`validate_together` at `common.rs:100-109` validates only that the branch list is non-empty and that branch/join steps are in bounds.

## Adversarial Check

This cannot be dismissed as a compiler responsibility because `WorkflowParts` is documented as untrusted compiled input and `CompiledWorkflow::try_from_parts` is the validation boundary. A malformed workflow can declare two start branches, branch ids outside that range, and a join `branch_count` of one; no listed validator rejects that inconsistent graph.

## Suggested Fix

Add a cross-node `Together` validation pass. For each `TogetherStart`, validate `branches.len()` against `TogetherJoin.branch_count`, require every listed branch target to be a `TogetherBranch` for the same join, and require each `TogetherBranch.branch < branch_count` with no duplicates.
