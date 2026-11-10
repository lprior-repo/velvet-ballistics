use crate::ids::StepIdx;
use crate::workflow::{CompiledNodeKind, NodeContinuation};

use super::conditional_counter;
use super::loop_region::count_body_region_nodes;
use super::successors::{
    insert_tracked_jump_edge, insert_tracked_step, push_successor_targets, tracked_steps_contain,
};
use super::{BudgetError, BudgetTraversalError, find_node_position, node_at_position};

pub(super) fn count_total_steps(
    nodes: &[crate::workflow::CompiledNode],
    entry: StepIdx,
    node_count: usize,
) -> Result<u64, BudgetTraversalError> {
    conditional_counter::count(nodes, entry, node_count)
}

/// Visits a single node during step counting and updates the total and stack.
#[allow(clippy::too_many_arguments)]
pub(super) fn visit_node_for_total_steps(
    nodes: &[crate::workflow::CompiledNode],
    current: StepIdx,
    node_count: usize,
    visited: &mut [bool],
    jump_edges: &mut Vec<(u16, u16)>,
    in_path: &mut Vec<u16>,
    mut total: u64,
    stack: &mut Vec<StepIdx>,
    cache: &std::collections::HashMap<StepIdx, u64>,
) -> Result<u64, BudgetTraversalError> {
    let idx = find_node_position(nodes, current, node_count)?;
    if visited.get(idx).copied() == Some(true) {
        return Ok(total);
    }
    let Some(flag) = visited.get_mut(idx) else {
        return Err(BudgetTraversalError::StepOutOfBounds { step: current });
    };
    *flag = true;

    let node = node_at_position(nodes, idx, current)?;

    total = match total.checked_add(1) {
        Some(v) => v,
        None => return Err(BudgetTraversalError::StepOutOfBounds { step: current }),
    };

    match &node.kind {
        CompiledNodeKind::ForEachStart {
            limit, body, done, ..
        } => {
            total = count_and_push_loop_body(
                nodes,
                *body,
                *done,
                u64::from(*limit),
                visited,
                node_count,
                total,
                stack,
            )
            .map_err(map_loop_body_budget_error)?;
        }
        CompiledNodeKind::CollectStart {
            limit, body, done, ..
        } => {
            total = count_and_push_loop_body(
                nodes,
                *body,
                *done,
                u64::from(*limit),
                visited,
                node_count,
                total,
                stack,
            )
            .map_err(map_loop_body_budget_error)?;
        }
        CompiledNodeKind::ReduceStart {
            limit, body, done, ..
        } => {
            total = count_and_push_loop_body(
                nodes,
                *body,
                *done,
                u64::from(*limit),
                visited,
                node_count,
                total,
                stack,
            )
            .map_err(map_loop_body_budget_error)?;
        }
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => {
            total = count_and_push_loop_body(
                nodes,
                *body,
                *done,
                u64::from(*max_attempts),
                visited,
                node_count,
                total,
                stack,
            )
            .map_err(map_loop_body_budget_error)?;
        }
        CompiledNodeKind::Jump { target } => {
            let from = current.get();
            let to = target.get();
            if tracked_steps_contain(in_path, to) {
                return Err(BudgetTraversalError::JumpCycle {
                    step: current,
                    target: *target,
                });
            }
            if !insert_tracked_jump_edge(jump_edges, (from, to), node_count)? {
                return Err(BudgetTraversalError::JumpCycle {
                    step: current,
                    target: *target,
                });
            }
            insert_tracked_step(in_path, to, node_count)?;
            stack.push(*target);
        }
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            total = add_conditional_max_steps(branches, *otherwise, total, cache)?;
        }
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            total = add_conditional_slot_max_steps(branches, *otherwise, total, cache)?;
        }
        _ => {
            push_successor_targets(&node.kind, stack);
            if let Some(next) = node.routing.next.step() {
                stack.push(next);
            }
        }
    }
    if let Some(handler) = node.routing.error.handler() {
        stack.push(handler);
    }
    Ok(total)
}

fn map_loop_body_budget_error(error: BudgetError) -> BudgetTraversalError {
    let actual = match error {
        BudgetError::TotalStepsExceeded { actual, .. } => actual,
        _ => u64::MAX,
    };
    BudgetTraversalError::StepCountOverflow { actual }
}

fn add_conditional_max_steps(
    branches: &[crate::workflow::ExprBranch],
    otherwise: Option<StepIdx>,
    total: u64,
    cache: &std::collections::HashMap<StepIdx, u64>,
) -> Result<u64, BudgetTraversalError> {
    let mut max_branch = match otherwise {
        Some(target) => cache
            .get(&target)
            .copied()
            .ok_or(BudgetTraversalError::StepOutOfBounds { step: target })?,
        None => 0,
    };
    for branch in branches {
        let branch_steps =
            cache
                .get(&branch.target)
                .copied()
                .ok_or(BudgetTraversalError::StepOutOfBounds {
                    step: branch.target,
                })?;
        max_branch = max_branch.max(branch_steps);
    }
    checked_step_add(total, max_branch)
}

fn add_conditional_slot_max_steps(
    branches: &[crate::workflow::SlotBranch],
    otherwise: Option<StepIdx>,
    total: u64,
    cache: &std::collections::HashMap<StepIdx, u64>,
) -> Result<u64, BudgetTraversalError> {
    let mut max_branch = match otherwise {
        Some(target) => cache
            .get(&target)
            .copied()
            .ok_or(BudgetTraversalError::StepOutOfBounds { step: target })?,
        None => 0,
    };
    for branch in branches {
        let branch_steps =
            cache
                .get(&branch.target)
                .copied()
                .ok_or(BudgetTraversalError::StepOutOfBounds {
                    step: branch.target,
                })?;
        max_branch = max_branch.max(branch_steps);
    }
    checked_step_add(total, max_branch)
}

fn checked_step_add(left: u64, right: u64) -> Result<u64, BudgetTraversalError> {
    match left.checked_add(right) {
        Some(value) => Ok(value),
        None => Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX }),
    }
}

/// Counts body region steps for a loop header and adds multiplied iterations to total.
#[inline]
#[allow(clippy::too_many_arguments, unused_variables)]
fn count_and_push_loop_body(
    nodes: &[crate::workflow::CompiledNode],
    body: StepIdx,
    done: StepIdx,
    iter_count: u64,
    visited: &mut [bool],
    node_count: usize,
    mut total: u64,
    stack: &mut Vec<StepIdx>,
) -> Result<u64, BudgetError> {
    let body_count = count_body_region_nodes(nodes, body, done, node_count)?;
    let iter_count = iter_count.max(1);
    let product = body_count
        .checked_mul(iter_count)
        .ok_or(BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        })?;
    total = total
        .checked_add(product)
        .ok_or(BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        })?;
    push_done_continuation(nodes, done, node_count, stack)?;
    Ok(total)
}

fn push_done_continuation(
    nodes: &[crate::workflow::CompiledNode],
    done: StepIdx,
    node_count: usize,
    stack: &mut Vec<StepIdx>,
) -> Result<(), BudgetError> {
    let done_idx = find_node_position(nodes, done, node_count)?;
    if let Some(node) = nodes.get(done_idx)
        && matches!(node.routing.next, NodeContinuation::End)
        && let Some(next_idx) = done_idx.checked_add(1)
        && next_idx < nodes.len()
        && let Some(next_node) = nodes.get(next_idx)
    {
        stack.push(next_node.id);
    }
    stack.push(done);
    Ok(())
}
