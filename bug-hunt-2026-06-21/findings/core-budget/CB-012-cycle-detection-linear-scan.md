# CB-012: `tracked_steps_*` and `insert_tracked_jump_edge` use O(N) linear scan

- **Severity**: Low
- **Category**: perf
- **Location**: `crates/vb_core/src/budget/traversal_tracking.rs:10` (also 14, 29, 35)
- **Confidence**: confirmed

## Description

Every cycle-detection primitive in this module uses
`iter().any(|c| c == step)` / `iter().position(...)`. Each insertion and
removal is therefore O(N) in the depth of the DFS path, making the total
cycle-detection cost O(N²) in the worst case over an N-node workflow.

## Evidence

```rust
pub(super) fn tracked_steps_contain(steps: &[u16], step: u16) -> bool {
    steps.iter().copied().any(|candidate| candidate == step)
}

pub(super) fn remove_tracked_step(steps: &mut Vec<u16>, step: u16) {
    if let Some(position) = steps.iter().position(|candidate| *candidate == step) {
        steps.remove(position);
    }
}

pub(super) fn insert_tracked_jump_edge(
    edges: &mut Vec<(u16, u16)>,
    edge: (u16, u16),
    limit: usize,
) -> Result<bool, BudgetTraversalError> {
    if edges.iter().copied().any(|candidate| candidate == edge) { ... }
    ...
}
```

(`crates/vb_core/src/budget/traversal_tracking.rs:10-47`)

`remove_tracked_step` adds an additional O(N) shift because `Vec::remove`
shifts the tail. The functions are called once per node visit
(`compute_fanout_and_depth` line 57 and 116; `visit_node_for_total_steps`
line 44).

## Adversarial Check

For the master spec's 1000-step default policy, N² is only 10⁶, which is
trivial. But the same code path is also exercised by `absolute_max_steps_executable`
(1_000_000) and by stress workflows; for a 100k-node workflow the cost is
10¹⁰ operations, which is on the order of a minute of CPU on modern
hardware. Since budget computation is on the admission hot path, this is a
real if situational regression. The Holzman "no O(N²) when an O(N) or O(1)
alternative exists" rule applies.

## Suggested Fix

`bounded_tracking_vec` already pre-allocates `node_count` slots. Use a
`Box<[bool; N]>`-style bitmap keyed by step index for `in_path` membership,
and a `HashSet<(u16, u16)>` (or a 2D bitmap indexed by `(from, to)`) for
`jump_edges`. Both reduce the per-call cost to O(1) and the total to O(N).
