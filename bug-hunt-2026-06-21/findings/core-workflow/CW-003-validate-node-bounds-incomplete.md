# CW-003: `validate_node_bounds` only validates `next`, missing `on_error` and kind-specific targets

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/engine/validate.rs:16-29`
- **Confidence**: confirmed

## Description

The publicly-exported `validate_node_bounds` advertises that it "validates that all node indices are within the node array bounds" but only inspects `node.id` and `node.next`. It silently ignores `on_error` and every kind-specific target (Choose branches, CollectStart body/done, ForEachStart body/done, ReduceStart fields, RepeatStart fields, RetryCheck fields, ErrorHandler body/handler, TogetherStart branches/join, TogetherBranch entry/join, Jump target).

## Evidence

```rust
// engine/validate.rs:16
pub fn validate_node_bounds(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    for node in &parts.nodes {
        if node.id.as_usize() >= node_count {
            return Err(WorkflowError::StepOutOfBounds { step: node.id });
        }
        if let Some(next) = node.next
            && next.as_usize() >= node_count
        {
            return Err(WorkflowError::StepOutOfBounds { step: next });
        }
    }
    Ok(())
}
```

A caller that uses `validate_node_bounds` as their only bounds check (e.g. an external tool that admits a workflow without going through `CompiledWorkflow::try_from_parts`) will accept workflows with out-of-bounds `on_error`, `Jump{target}`, `Choose{branches}`, loop `body`/`done`, etc.

## Adversarial Check

The canonical admission path is `CompiledWorkflow::try_from_parts` (mod.rs:46), which delegates to `validation::validate_parts` and DOES check these via `nodes::kinds::validate_node_kind` and `nodes::common`. So internally-compiled workflows are safe. The bug is that `validate_node_bounds` is exported from `engine.rs:39` as a public API (re-exported in lib.rs:125) and its name/doc promises more than it delivers. A downstream crate that constructs a `WorkflowParts` and runs only the engine-level validators (instead of the full `validate_parts`) will accept invalid workflows.

## Suggested Fix

Either rename to `validate_node_id_and_next_bounds` to accurately reflect behavior, or delete and route callers through `validation::validate_parts`. If kept, expand the loop to also bounds-check `on_error` and dispatch to `validate_transition_target` for kind-specific edges.
