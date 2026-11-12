---
section: 20
title: "Boundedness and Resource Analysis"
parent: velvet-ballistics-MASTER.md
---

## 20. Boundedness and Resource Analysis

No accepted workflow has unknown bounds.

The compiler computes:

```rust
pub struct WholeWorkflowBudget {
    pub max_steps_executable: u32,
    pub max_action_tickets: u32,
    pub max_parallel_in_flight: u16,
    pub max_retries_per_action: u16,
    pub max_collect_pages: u32,
    pub max_collect_items: u32,
    pub max_for_each_iterations: u32,
    pub max_together_branches: u16,
    pub max_repeat_attempts: u16,
    pub max_run_time_seconds: u64,
    pub max_result_bytes: u32,
    pub max_total_slots_written: u32,
    pub max_arena_cells: u32,
    pub max_arena_bytes: u64,
}
```

Reject conditions:

```text
for_each without bound
collect without pages/items/time limit
repeat without times/time limit
retry without max_attempts
wait without timeout
ask without timeout
together branch count exceeds policy
nested fanout exceeds policy
result size cannot be bounded
unbounded string/list/object/blob field
unknown action output size under strict policy
```

Runtime budget object:

```rust
pub struct RuntimeBudget {
    pub transitions: Counter,
    pub slot_writes: Counter,
    pub arena_cells: Counter,
    pub arena_bytes: Counter,
    pub outbox_items: Counter,
    pub trace_items: Counter,
}
```

All runtime growth goes through budget methods. Raw hot-path `Vec::push` without capacity/budget proof is forbidden.

---

