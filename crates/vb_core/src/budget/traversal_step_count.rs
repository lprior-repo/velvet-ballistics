#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind, ExprBranch, SlotBranch};

use super::budget_error::BudgetError;
use super::traversal::BudgetTraversalError;
use super::traversal_loop::count_and_push_loop_body;
use super::traversal_path::count_path_steps;
use super::traversal_successors::{find_node_position, node_at_position, push_successor_targets};
use super::traversal_tracking::{
    bounded_tracking_vec, insert_tracked_jump_edge, insert_tracked_step, remove_tracked_step,
    tracked_steps_contain,
};

/// Counts the worst-case total number of runtime steps by performing a DFS walk
/// from the entry node. Unlike a naive unique-node count, this function accounts
/// for loop iteration limits: when a loop header (ForEachStart, CollectStart,
/// RepeatStart, ReduceStart) is encountered, the body subgraph step count is
/// multiplied by the iteration limit and added once for the header itself.
///
/// The algorithm works in two phases:
/// 1. **Body counting phase**: A DFS walk counts unique nodes in each loop body
///    region (nodes reachable from `body` but not from `done`). This avoids
///    infinite recursion from back-edges.
/// 2. **Worst-case multiplication**: Loop body counts are multiplied by the
///    declared iteration limits and summed with non-loop node counts.
pub(super) fn count_total_steps(
    nodes: &[CompiledNode],
    entry: StepIdx,
    node_count: usize,
) -> Result<u64, BudgetTraversalError> {
    let mut visited: Vec<bool> = vec![false; node_count];
    let mut jump_edges: Vec<(u16, u16)> = bounded_tracking_vec(node_count);
    let mut in_path: Vec<u16> = bounded_tracking_vec(node_count);
    let mut total: u64 = 0;

    let mut stack: Vec<StepIdx> = Vec::with_capacity(node_count);
    stack.push(entry);

    while let Some(current) = stack.pop() {
        let current_u16 = current.get();
        remove_tracked_step(&mut in_path, current_u16);
        total = visit_node_for_total_steps(
            nodes,
            current,
            node_count,
            &mut visited,
            &mut jump_edges,
            &mut in_path,
            total,
            &mut stack,
        )?;
    }
    Ok(total)
}

/// Visits a single node during step counting and updates the total and stack.
#[allow(clippy::too_many_arguments)]
fn visit_node_for_total_steps(
    nodes: &[CompiledNode],
    current: StepIdx,
    node_count: usize,
    visited: &mut [bool],
    jump_edges: &mut Vec<(u16, u16)>,
    in_path: &mut Vec<u16>,
    mut total: u64,
    stack: &mut Vec<StepIdx>,
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
            .map_err(|e| {
                let actual = match e {
                    BudgetError::TotalStepsExceeded { actual, .. } => actual,
                    _ => u64::MAX,
                };
                BudgetTraversalError::StepCountOverflow { actual }
            })?;
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
            .map_err(|e| {
                let actual = match e {
                    BudgetError::TotalStepsExceeded { actual, .. } => actual,
                    _ => u64::MAX,
                };
                BudgetTraversalError::StepCountOverflow { actual }
            })?;
        }
        CompiledNodeKind::ReduceStart { body, done, .. } => {
            // Cold-AST invariant (master §45) drops body, so the declared input
            // length is unknown to the budget traversal. Use the conservative
            // default of 1 iteration for the worst-case step count.
            total =
                count_and_push_loop_body(nodes, *body, *done, 1, visited, node_count, total, stack)
                    .map_err(|e| {
                        let actual = match e {
                            BudgetError::TotalStepsExceeded { actual, .. } => actual,
                            _ => u64::MAX,
                        };
                        BudgetTraversalError::StepCountOverflow { actual }
                    })?;
        }
        CompiledNodeKind::RepeatStart {
            max_attempts: _,
            body,
            done,
        } => {
            // Cold-AST invariant (master §45) drops body, so the runtime
            // attempt count cannot be bounded from the compiled IR alone.
            // Use the conservative default of 1 iteration for the
            // worst-case step count. The declared `max_attempts` is still
            // tracked separately in `WholeWorkflowBudget.max_repeat_attempts`.
            total =
                count_and_push_loop_body(nodes, *body, *done, 1, visited, node_count, total, stack)
                    .map_err(|e| {
                        let actual = match e {
                            BudgetError::TotalStepsExceeded { actual, .. } => actual,
                            _ => u64::MAX,
                        };
                        BudgetTraversalError::StepCountOverflow { actual }
                    })?;
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
            total = add_conditional_max_steps(nodes, branches, *otherwise, node_count, total)?;
        }
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            total = add_conditional_slot_max_steps(nodes, branches, *otherwise, node_count, total)?;
        }
        _ => {
            push_successor_targets(&node.kind, stack);
            if let Some(next) = node.next {
                stack.push(next);
            }
        }
    }
    Ok(total)
}

fn add_conditional_max_steps(
    nodes: &[CompiledNode],
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
    node_count: usize,
    total: u64,
) -> Result<u64, BudgetTraversalError> {
    let mut max_branch = match otherwise {
        Some(target) => count_path_steps(nodes, target, node_count)?,
        None => 0,
    };
    for branch in branches {
        let branch_steps = count_path_steps(nodes, branch.target, node_count)?;
        max_branch = max_branch.max(branch_steps);
    }
    checked_step_add(total, max_branch)
}

fn add_conditional_slot_max_steps(
    nodes: &[CompiledNode],
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
    node_count: usize,
    total: u64,
) -> Result<u64, BudgetTraversalError> {
    let mut max_branch = match otherwise {
        Some(target) => count_path_steps(nodes, target, node_count)?,
        None => 0,
    };
    for branch in branches {
        let branch_steps = count_path_steps(nodes, branch.target, node_count)?;
        max_branch = max_branch.max(branch_steps);
    }
    checked_step_add(total, max_branch)
}

pub(super) fn checked_step_add(left: u64, right: u64) -> Result<u64, BudgetTraversalError> {
    match left.checked_add(right) {
        Some(value) => Ok(value),
        None => Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX }),
    }
}
