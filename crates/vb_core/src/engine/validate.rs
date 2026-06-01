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

/// Validates resource contract bounds against hard limits.
pub fn validate_resource_contract(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let contract = parts.resource_contract;
    if usize::from(contract.max_steps) > crate::limits::MAX_STEPS_PER_WORKFLOW {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_steps",
        });
    }
    if usize::from(contract.max_slots) > crate::limits::MAX_SLOTS_PER_WORKFLOW {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_slots",
        });
    }
    if usize::from(contract.max_constants) > crate::limits::MAX_CONSTANTS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_constants",
        });
    }
    if usize::from(contract.max_accessors) > crate::limits::MAX_ACCESSORS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_accessors",
        });
    }
    if usize::from(contract.max_expressions) > crate::limits::MAX_EXPRESSIONS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_expressions",
        });
    }
    if contract.max_expr_stack > crate::limits::MAX_EXPRESSION_STACK {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_expr_stack",
        });
    }
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
#[path = "tests/mod.rs"]
mod tests;
