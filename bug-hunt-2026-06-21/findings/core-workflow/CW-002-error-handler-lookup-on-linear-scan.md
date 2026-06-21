# CW-002: `error_handler_for_body` performs O(n) linear scan over all nodes

- **Severity**: Medium
- **Category**: perf
- **Location**: `crates/vb_core/src/workflow/workflow.rs:158-162`, called from `crates/vb_core/src/engine/error_routing.rs:125-128`
- **Confidence**: confirmed

## Description

Looking up an `ErrorHandler` node by its `body` step uses `nodes.iter().find(...)`, scanning every node in the workflow. This is called inside `route_error_handler`, which fires on every step failure that has an `on_error` target.

## Evidence

```rust
// workflow.rs:158
pub(crate) fn error_handler_for_body(&self, body_step: StepIdx) -> Option<&CompiledNode> {
    self.nodes.iter().find(|node| {
        matches!(node.kind, super::node::CompiledNodeKind::ErrorHandler { body, .. } if body == body_step)
    })
}
```

```rust
// error_routing.rs:125
let error_slot = plan
    .error_handler_for_body(failed_step)
    .and_then(|eh| eh.error_slot)
    .or(node.error_slot);
```

A workflow with N nodes (up to `MAX_STEPS_PER_WORKFLOW = 1_000`) and F failing steps pays O(N·F) just for this lookup. In the worst case of a flaky workflow that fails on every step inside an error-handling boundary, this is 1,000,000 iterator comparisons per tick.

## Adversarial Check

This is not in the innermost deterministic step loop (only fires on failure), but failure paths in distributed systems are hot during incident recovery: every retry, every timeout, every conflict routes through here. A quadratic lookup on the failure path is the kind of cost that converts a recoverable incident into a cascading slowdown. The fix is also a functional-rust simplification.

## Suggested Fix

Pre-compute a `Box<[Option<SlotIdx>; MAX_STEPS_PER_WORKFLOW]>` (or a `HashMap<StepIdx, SlotIdx>`) mapping `body_step -> error_slot` at workflow construction time (`try_from_parts`) and store it on `CompiledWorkflow`. Then `error_handler_for_body` becomes O(1).
