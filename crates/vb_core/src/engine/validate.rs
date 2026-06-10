#![forbid(unsafe_code)]
//! Workflow validation functions.

use crate::ids::StepIdx;
use crate::workflow::{
    CompiledNodeKind, CompiledWorkflow, ExprBranch, SlotBranch, WorkflowError, WorkflowParts,
};

/// Validates compiled workflow IR integrity.
pub fn validate_compiled_workflow(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    CompiledWorkflow::try_from_parts(parts.clone())?;
    Ok(())
}

/// Validates that all node indices are within the node array bounds.
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

/// Validates that all step transition targets reference valid node indices.
pub fn validate_transition_target(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    for node in &parts.nodes {
        match &node.kind {
            CompiledNodeKind::Jump { target } if target.as_usize() >= node_count => {
                return Err(WorkflowError::StepOutOfBounds { step: *target });
            }
            CompiledNodeKind::Choose {
                branches,
                otherwise,
            } => {
                validate_branch_targets(branches, *otherwise, node_count)?;
            }
            CompiledNodeKind::ChooseSlot {
                branches,
                otherwise,
            } => {
                validate_slot_branch_targets(branches, *otherwise, node_count)?;
            }
            CompiledNodeKind::ForEachStart { body, done, .. }
            | CompiledNodeKind::ForEachNext { body, done, .. }
            | CompiledNodeKind::CollectStart { body, done, .. }
            | CompiledNodeKind::CollectPage { body, done, .. }
            | CompiledNodeKind::CollectNext { body, done, .. }
            | CompiledNodeKind::ReduceStart { body, done, .. }
            | CompiledNodeKind::ReduceNext { body, done, .. }
            | CompiledNodeKind::RepeatStart { body, done, .. }
            | CompiledNodeKind::RepeatAttempt { body, done, .. } => {
                validate_two_step_targets(*body, *done, node_count)?;
            }
            CompiledNodeKind::TogetherStart { branches, join } => {
                for branch in branches {
                    if branch.as_usize() >= node_count {
                        return Err(WorkflowError::StepOutOfBounds { step: *branch });
                    }
                }
                if join.as_usize() >= node_count {
                    return Err(WorkflowError::StepOutOfBounds { step: *join });
                }
            }
            CompiledNodeKind::TogetherBranch { entry, join, .. } => {
                validate_two_step_targets(*entry, *join, node_count)?;
            }
            CompiledNodeKind::RepeatCheck { done, .. } if done.as_usize() >= node_count => {
                return Err(WorkflowError::StepOutOfBounds { step: *done });
            }
            CompiledNodeKind::RetryCheck {
                body, exhausted, ..
            } => {
                validate_two_step_targets(*body, *exhausted, node_count)?;
            }
            CompiledNodeKind::ErrorHandler { body, handler, .. } => {
                validate_two_step_targets(*body, *handler, node_count)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_branch_targets(
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
    node_count: usize,
) -> Result<(), WorkflowError> {
    for branch in branches {
        if branch.target.as_usize() >= node_count {
            return Err(WorkflowError::StepOutOfBounds {
                step: branch.target,
            });
        }
    }
    if let Some(target) = otherwise
        && target.as_usize() >= node_count
    {
        return Err(WorkflowError::StepOutOfBounds { step: target });
    }
    Ok(())
}

fn validate_slot_branch_targets(
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
    node_count: usize,
) -> Result<(), WorkflowError> {
    for branch in branches {
        if branch.target.as_usize() >= node_count {
            return Err(WorkflowError::StepOutOfBounds {
                step: branch.target,
            });
        }
    }
    if let Some(target) = otherwise
        && target.as_usize() >= node_count
    {
        return Err(WorkflowError::StepOutOfBounds { step: target });
    }
    Ok(())
}

fn validate_two_step_targets(
    first: StepIdx,
    second: StepIdx,
    node_count: usize,
) -> Result<(), WorkflowError> {
    if first.as_usize() >= node_count {
        return Err(WorkflowError::StepOutOfBounds { step: first });
    }
    if second.as_usize() >= node_count {
        return Err(WorkflowError::StepOutOfBounds { step: second });
    }
    Ok(())
}

#[cfg(test)]
#[path = "validate/tests/red_phase_behavior_tests.rs"]
mod red_phase_behavior_tests;
#[cfg(test)]
#[path = "validate/tests/red_phase_tests.rs"]
mod red_phase_tests;
