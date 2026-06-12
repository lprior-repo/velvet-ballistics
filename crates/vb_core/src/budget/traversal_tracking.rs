#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

use super::traversal::BudgetTraversalError;

pub(super) fn bounded_tracking_vec<T>(node_count: usize) -> Vec<T> {
    Vec::with_capacity(node_count)
}

pub(super) fn tracked_steps_contain(steps: &[u16], step: u16) -> bool {
    steps.iter().copied().any(|candidate| candidate == step)
}

pub(super) fn insert_tracked_step(
    steps: &mut Vec<u16>,
    step: u16,
    limit: usize,
) -> Result<bool, BudgetTraversalError> {
    if tracked_steps_contain(steps, step) {
        return Ok(false);
    }
    if steps.len() >= limit {
        return Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX });
    }
    steps.push(step);
    Ok(true)
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
    if edges.iter().copied().any(|candidate| candidate == edge) {
        return Ok(false);
    }
    if edges.len() >= limit {
        return Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX });
    }
    edges.push(edge);
    Ok(true)
}
