# RE-001: `compute_max_parallel_in_flight` does not account for nested `Together`

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_runtime/src/engine/drive.rs:18-37`
- **Confidence**: likely

## Description

`compute_max_parallel_in_flight` walks every node once and returns the maximum branch count across all `TogetherStart` nodes. It does not sum branches across nesting, so a branch whose body contains a second `TogetherStart` will see its outer branches already counted toward `parallel_in_flight` when the inner `together_start` runs the same arithmetic against the same `max`. The inner start then rejects with `ParallelLimitExceeded` even though the runtime has not actually exceeded any real concurrency limit (the runtime is deterministic-synchronous).

## Evidence

`crates/vb_runtime/src/engine/drive.rs:18-37`:

```rust
pub(crate) fn compute_max_parallel_in_flight(plan: &CompiledWorkflow) -> RuntimeEngineResult<u16> {
    let mut max_branches: u16 = 0;
    for i in 0..plan.node_count() {
        let step = StepIdx::new(i);
        if let Some(node) = plan.node(step)
            && let CompiledNodeKind::TogetherStart { branches, .. } = &node.kind
        {
            let branch_count = u16::try_from(branches.len()).map_err(...)?;
            if branch_count > max_branches {
                max_branches = branch_count;
            }
        }
    }
    Ok(max_branches)
}
```

Trace for a workflow with outer `together(3 branches)` and one of those branches containing inner `together(4 branches)`:

1. `compute_max_parallel_in_flight` returns `max(3, 4) = 4`.
2. `initialize_drive` sets `max_parallel_in_flight = 4`.
3. Outer `together_start`: `current = 0`, `count = 3`, `0 + 3 = 3 ≤ 4` → ok, `parallel_in_flight = 3`.
4. Outer branches run sequentially. Branch 2's body contains inner `together_start`.
5. Inner `together_start`: `current = 3`, `count = 4`, `3 + 4 = 7 > 4` → `EngineError::ParallelLimitExceeded`.

The runtime rejects a legal nested composition. The `parallel_in_flight` counter is never actually decremented between outer branches (only at `together_join`), so any inner `together_start` collides with the outer's outstanding count.

## Adversarial Check

1. *"The workflow validator rejects nested Togethers."* — I could not find such a check in `vb_core::workflow::validation`. The `budget` validator only sums the declared branch counts of each `TogetherStart` independently (`budget/tests/chunk_009.rs` covers a single-level budget test). Without a positive rejection, nesting is structurally expressible.
2. *"Together is single-level by convention."* — The primitive doc (together.rs:13-17) does not state this. The type signature accepts `branches: &[StepIdx]` where each `StepIdx` may be any node, including another `TogetherStart`.
3. *"Nested parallelism would be a real concurrency bug, so rejecting it is correct."* — The runtime is explicitly deterministic-synchronous (together.rs:14: "branches execute sequentially in declaration order"). There is no actual concurrency. The limit check exists for resource accounting, not for safety. Rejecting legal nests is the wrong layer to enforce single-level semantics.

Severity Medium: this is an outright correctness bug for nested workflows, but only manifests if the workflow compiler emits nested `together` nodes.

## Suggested Fix

Either:

(a) Compute the nesting-aware limit by walking branch entry nodes transitively, summing branch counts where a branch contains a `TogetherStart`. Output the actual peak parallel-in-flight, not the per-start max.

(b) Reject nested `TogetherStart` in `vb_core::workflow::validation` with an explicit `NestedTogether` error so the failure is surfaced at compile time with a diagnostic, not at runtime with `ParallelLimitExceeded`.

(c) Track per-branch in-flight accounting in `RunFrame` so the limit is enforced against the actual call-stack depth rather than a single global high-water mark.

Option (b) is the cheapest and gives the best operator experience.
