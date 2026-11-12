---
section: 64
title: "Whole-Workflow Boundedness Analysis"
parent: velvet-ballistics-MASTER.md
---

## 64. Whole-Workflow Boundedness Analysis


### Principle

No accepted workflow has unknown bounds. The compiler must compute a conservative whole-workflow budget before accepting any artifact.

### Required Analysis

The boundedness analyzer performs static dataflow analysis on the compiled IR to compute:

```rust
pub struct WholeWorkflowBudget {
    pub max_steps_executable: u32,
    pub max_action_tickets: u32,
    pub max_parallel_in_flight: u16,
    pub max_retries_per_action: u16,
    pub max_gather_pages: u32,
    pub max_gather_items: u32,
    pub max_for_each_iterations: u32,
    pub max_together_branches: u16,
    pub max_repeat_attempts: u16,
    pub max_run_time_seconds: u64,
    pub max_result_bytes: u32,
    pub max_total_slots_written: u32,
}
```

### Relationship to `ResourceContract`

`ResourceContract` (Section 13) defines per-workflow static limits: `max_steps`, `max_slots`, `max_retry_attempts`, `max_fanout`, `max_output_bytes`. These are declared by the workflow author and validated at compile time.

`WholeWorkflowBudget` is a computed analysis result: the verifier derives it from `ResourceContract` plus the IR's actual loop bounds and nesting structure. The relationship:

| `ResourceContract` field | `WholeWorkflowBudget` field | Relationship |
|--------------------------|----------------------------|--------------|
| `max_steps: u16` | `max_steps_executable: u32` | Computed budget cannot exceed `max_steps` × nesting depth factor |
| `max_retry_attempts: u16` | `max_retries_per_action: u16` | Direct copy from contract |
| `max_fanout: u16` | `max_together_branches: u16` | Direct copy from contract |
| `max_output_bytes: u32` | `max_result_bytes: u32` | Computed budget cannot exceed `max_output_bytes` |

`BoundednessPolicy` (below) provides absolute upper limits that apply ACROSS all workflows. `ResourceContract` limits apply WITHIN a single workflow. Validation order: `ResourceContract` ≤ `BoundednessPolicy`. If a computed `WholeWorkflowBudget` exceeds either, the workflow is rejected.

### Boundedness Rules

Reject if any of these conditions is true:

1. `for_each` over a list with no declared `max` in schema or policy.
2. `collect` without `pages`, `items`, or `time` limit.
3. `repeat` without `times` or `time` limit.
4. `try_again` without `max_attempts`.
5. `wait` event without timeout.
6. `ask` without timeout.
7. `together` with branch count exceeding policy.
8. Nested fanout that exceeds policy (e.g., `for_each` containing `together`).
9. `finish` with result of unknown max size where policy requires proof.

### Dataflow Propagation

The analyzer propagates bounds through the IR:

1. **Leaf bounds**: Each primitive contributes its declared bound.
2. **Sequential composition**: `max_steps` and `max_tickets` are summed.
3. **Nested loops**: Bounds multiply (outer `for_each` limit × inner action count).
4. **Conditional branches**: Take the maximum across branches.
5. **Parallel branches**: `max_parallel_in_flight` is the `together` branch count.

The compiler must be able to state: "This workflow can create at most N action tickets under declared limits." Even if N is conservative, having a bound is the requirement.

### Budget Validation

The computed `WholeWorkflowBudget` is validated against policy limits:

```rust
pub struct BoundednessPolicy {
    pub absolute_max_action_tickets: u32,     // default: 100_000
    pub absolute_max_parallel: u16,           // default: 256
    pub absolute_max_run_time_seconds: u64,   // default: 30 days
    pub absolute_max_result_bytes: u32,       // default: 256 KiB
    pub absolute_max_steps_executable: u32,   // default: 1_000_000
}
```

If any computed budget exceeds policy, the workflow is rejected with a typed `UnboundedWorkflow` error identifying which limit was exceeded.

---
