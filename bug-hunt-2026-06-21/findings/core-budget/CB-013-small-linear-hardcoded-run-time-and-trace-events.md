# CB-013: `compute_small_linear_budget` hard-codes `max_run_time_seconds = metrics.steps`

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_core/src/budget/small_linear.rs:41`
- **Confidence**: confirmed

## Description

The small-linear fast path sets `max_run_time_seconds` to `metrics.steps`,
assuming one second per step. This contradicts the general path in
`types.rs:134` (which also hard-codes the same value, but documents it as
"Phase 0 executes at most one step per runtime tick"). The small-linear
path contains no such comment, so an operator reading the small-linear
budget would assume the wall-clock budget was actually derived from the
workflow's declared run-time, when in fact it is a structurally fixed
function of the step count.

## Evidence

```rust
Ok(Some(WholeWorkflowBudget {
    ...
    max_run_time_seconds: metrics.steps,
    ...
    max_trace_events: metrics.steps,
    ...
}))
```

(`crates/vb_core/src/budget/small_linear.rs:41, 45`)

Both `max_run_time_seconds` and `max_trace_events` are bound to
`metrics.steps`, with no comment justifying the equivalence.

## Adversarial Check

A defender can argue "1 step ≡ 1 second is the documented Phase-0 contract,
so this is correct." That argument is fine for `max_run_time_seconds`, but
the small-linear path skips the documentation that the general path
includes. The `max_trace_events = metrics.steps` mapping is even less
obvious: there is no a priori reason each step emits exactly one trace
event. If the runtime ever emits more than one trace event per step (e.g.
enter/leave events), this budget will be too low and silently reject
legitimate runs.

## Suggested Fix

At minimum, mirror the comment from `types.rs:133`. Better, derive both
from a named constant or method (`step_budget_to_time_seconds`,
`step_budget_to_trace_events`) so the mapping is centralized.
