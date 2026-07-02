#![forbid(unsafe_code)]
//! Control-flow validation for workflow documents.
//!
//! Validates CFG construction, rejects backward branches (cycles), ensures
//! forward-only `then` targets, and checks that all steps are reachable
//! from the workflow entry.

use crate::{ValidationError, ValidationResult};

/// Validates control flow for a workflow's step graph.
///
/// Checks that branch targets are valid indices, that all then/branch
/// targets point forward, and that every step is reachable from step 0.
pub fn validate_control_flow(flow: &WorkflowFlow) -> ValidationResult<()> {
    validate_forward_targets(flow)?;
    validate_reachability(flow)?;
    Ok(())
}

/// Validates that all branch targets point to valid forward-only indices.
pub fn validate_forward_only_then(flow: &WorkflowFlow) -> ValidationResult<()> {
    for (step_index, step) in flow.steps.iter().enumerate() {
        for &target in &step.branch_targets {
            validate_forward_target(step_index, target, flow.steps.len())?;
        }
        if let Some(then_target) = step.then_target {
            validate_forward_target(step_index, then_target, flow.steps.len())?;
        }
    }
    Ok(())
}

/// Validates that all steps are reachable from step 0.
pub fn validate_reachability(flow: &WorkflowFlow) -> ValidationResult<()> {
    if flow.steps.is_empty() {
        return Err(ValidationError::UnreachableStep {
            step: "workflow has no steps".to_owned(),
        });
    }
    let mut reachable = vec![false; flow.steps.len()];
    mark_reachable(flow, &mut reachable)?;
    reject_unreachable(flow, &reachable)
}

fn validate_forward_targets(flow: &WorkflowFlow) -> ValidationResult<()> {
    for (step_index, step) in flow.steps.iter().enumerate() {
        for &target in &step.branch_targets {
            validate_target_index(target, flow.steps.len())?;
            if target <= step_index {
                return Err(ValidationError::ControlFlowCycle);
            }
        }
    }
    Ok(())
}

fn validate_target_index(target: usize, step_count: usize) -> ValidationResult<()> {
    if target >= step_count {
        return Err(ValidationError::InvalidThenTarget);
    }
    Ok(())
}

fn validate_forward_target(
    step_index: usize,
    target: usize,
    step_count: usize,
) -> ValidationResult<()> {
    validate_target_index(target, step_count)?;
    if target <= step_index {
        return Err(ValidationError::ControlFlowCycle);
    }
    Ok(())
}

fn mark_reachable(flow: &WorkflowFlow, reachable: &mut [bool]) -> ValidationResult<()> {
    let mut stack = Vec::with_capacity(flow.steps.len());
    stack.push(0_usize);
    while let Some(index) = stack.pop() {
        if *reachable
            .get(index)
            .ok_or(ValidationError::InvalidThenTarget)?
        {
            continue;
        }
        *reachable
            .get_mut(index)
            .ok_or(ValidationError::InvalidThenTarget)? = true;
        push_successors(flow, index, &mut stack);
    }
    Ok(())
}

fn push_successors(flow: &WorkflowFlow, index: usize, stack: &mut Vec<usize>) {
    let Some(step) = flow.steps.get(index) else {
        return;
    };
    for &target in &step.branch_targets {
        if target < flow.steps.len() {
            stack.push(target);
        }
    }
    if let Some(then_target) = step.then_target {
        if then_target < flow.steps.len() {
            stack.push(then_target);
        }
    } else if let Some(next) = index.checked_add(1).filter(|&n| n < flow.steps.len()) {
        stack.push(next);
    }
}

fn reject_unreachable(flow: &WorkflowFlow, reachable: &[bool]) -> ValidationResult<()> {
    for (index, is_reachable) in reachable.iter().enumerate() {
        if !is_reachable {
            let id = match flow.steps.get(index).and_then(|step| step.id.clone()) {
                Some(step_id) => step_id,
                None => format!("step_{index}"),
            };
            return Err(ValidationError::UnreachableStep { step: id });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Control-flow model
// ---------------------------------------------------------------------------

/// Workflow step graph for control-flow validation.
#[derive(Debug, Clone, Default)]
pub struct WorkflowFlow {
    /// Steps in declaration order.
    pub steps: Vec<StepFlow>,
}

/// Single step's control-flow edges.
#[derive(Debug, Clone, Default)]
pub struct StepFlow {
    /// Step ID (for diagnostics).
    pub id: Option<String>,
    /// Branch targets (e.g., choose on_true, on_false).
    pub branch_targets: Vec<usize>,
    /// Explicit then target, if any. If None, falls through to next step.
    pub then_target: Option<usize>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "control_flow/tests.rs"]
mod tests;
