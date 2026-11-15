#![forbid(unsafe_code)]
//! Workflow validation functions.
#[path = "validate/targets.rs"]
mod targets;
use targets::{validate_branch_targets, validate_slot_branch_targets, validate_two_step_targets};
#[path = "validate/nested_together.rs"]
mod nested_together;
pub use nested_together::validate_no_nested_together;

use crate::workflow::{CompiledNodeKind, CompiledWorkflow, WorkflowError, WorkflowParts};

/// Validates compiled workflow IR integrity.
pub fn validate_compiled_workflow(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    CompiledWorkflow::try_from_parts(parts.clone())?;
    Ok(())
}

/// Validates resource contract bounds against hard limits.
///
/// Bounds-only pre-check: delegates to the canonical
/// `check_resource_contract_limits` implementation shared
/// with admission (`CompiledWorkflow::try_from_parts`). Actual-usage
/// (`Exceeded`) checks are admission-only and intentionally absent here.
pub fn validate_resource_contract(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    crate::workflow::check_resource_contract_limits(&parts.resource_contract)
}

/// Validates node IDs and ordinary/error routing targets against the node count.
pub fn validate_node_bounds(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    parts.nodes.iter().try_for_each(|node| {
        [
            Some(node.id),
            node.routing.next.step(),
            node.routing.error.handler(),
        ]
        .into_iter()
        .flatten()
        .try_for_each(|step| {
            if step.as_usize() < node_count {
                Ok(())
            } else {
                Err(WorkflowError::StepOutOfBounds { step })
            }
        })
    })
}

/// Validates that all step transition targets reference valid node indices.
pub fn validate_transition_target(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    for node in &parts.nodes {
        validate_node_transition_targets(node, node_count)?;
    }
    validate_no_nested_together(parts)?;
    Ok(())
}

fn validate_node_transition_targets(
    node: &crate::workflow::CompiledNode,
    node_count: usize,
) -> Result<(), WorkflowError> {
    validate_basic_transition_targets(&node.kind, node_count)?;
    validate_loop_transition_targets(&node.kind, node_count)?;
    validate_together_transition_targets(&node.kind, node_count)?;
    validate_routing_transition_targets(&node.kind, node_count)
}

fn validate_basic_transition_targets(
    kind: &CompiledNodeKind,
    node_count: usize,
) -> Result<(), WorkflowError> {
    match kind {
        CompiledNodeKind::Jump { target } if target.as_usize() >= node_count => {
            Err(WorkflowError::StepOutOfBounds { step: *target })
        }
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => validate_branch_targets(branches, otherwise.step(), node_count),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => validate_slot_branch_targets(branches, otherwise.step(), node_count),
        _ => Ok(()),
    }
}

fn validate_loop_transition_targets(
    kind: &CompiledNodeKind,
    node_count: usize,
) -> Result<(), WorkflowError> {
    match kind {
        CompiledNodeKind::ForEachStart { body, done, .. }
        | CompiledNodeKind::ForEachNext { body, done, .. }
        | CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. }
        | CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            validate_two_step_targets(*body, *done, node_count)
        }
        _ => Ok(()),
    }
}

fn validate_together_transition_targets(
    kind: &CompiledNodeKind,
    node_count: usize,
) -> Result<(), WorkflowError> {
    match kind {
        CompiledNodeKind::TogetherStart { branches, join } => {
            for branch in branches {
                if branch.as_usize() >= node_count {
                    return Err(WorkflowError::StepOutOfBounds { step: *branch });
                }
            }
            if join.as_usize() >= node_count {
                return Err(WorkflowError::StepOutOfBounds { step: *join });
            }
            Ok(())
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            validate_two_step_targets(*entry, *join, node_count)
        }
        _ => Ok(()),
    }
}

fn validate_routing_transition_targets(
    kind: &CompiledNodeKind,
    node_count: usize,
) -> Result<(), WorkflowError> {
    match kind {
        CompiledNodeKind::RepeatCheck { done, .. } if done.as_usize() >= node_count => {
            Err(WorkflowError::StepOutOfBounds { step: *done })
        }
        CompiledNodeKind::RetryCheck {
            body, exhausted, ..
        } => validate_two_step_targets(*body, *exhausted, node_count),
        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            validate_two_step_targets(*body, *handler, node_count)
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests;
