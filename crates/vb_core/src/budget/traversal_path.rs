#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind, ExprBranch, SlotBranch};

use super::traversal::BudgetTraversalError;
use super::traversal_step_count::checked_step_add;
use super::traversal_successors::{find_node_position, node_at_position, push_successor_targets};

pub(super) fn count_path_steps(
    nodes: &[CompiledNode],
    entry: StepIdx,
    node_count: usize,
) -> Result<u64, BudgetTraversalError> {
    let mut visited: Vec<bool> = vec![false; node_count];
    let mut stack: Vec<StepIdx> = Vec::new();
    stack.push(entry);
    let mut total: u64 = 0;
    while let Some(current) = stack.pop() {
        let idx = find_node_position(nodes, current, node_count)?;
        if visited.get(idx).copied() == Some(true) {
            continue;
        }
        let Some(flag) = visited.get_mut(idx) else {
            return Err(BudgetTraversalError::StepOutOfBounds { step: current });
        };
        *flag = true;
        total = checked_step_add(total, 1)?;
        let node = node_at_position(nodes, idx, current)?;
        push_path_successors(nodes, node, node_count, &mut stack)?;
    }
    Ok(total)
}

fn push_path_successors(
    nodes: &[CompiledNode],
    node: &CompiledNode,
    node_count: usize,
    stack: &mut Vec<StepIdx>,
) -> Result<(), BudgetTraversalError> {
    if matches!(node.kind, CompiledNodeKind::Finish { .. }) {
        return Ok(());
    }
    match &node.kind {
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            push_longest_expr_branch(nodes, branches, *otherwise, node_count, stack)?;
        }
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            push_longest_slot_branch(nodes, branches, *otherwise, node_count, stack)?;
        }
        _ => push_successor_targets(&node.kind, stack),
    }
    if let Some(next) = node.next {
        stack.push(next);
    }
    Ok(())
}

/// Iteratively counts nodes along a branch using an explicit stack (no recursion).
/// Bounded by `max_depth` to prevent DoS on adversarial graphs.
/// Returns 0 if start is None or node not found.
fn iterative_branch_depth(
    nodes: &[CompiledNode],
    start: StepIdx,
    max_depth: u64,
) -> u64 {
    let mut visited: Vec<bool> = vec![false; nodes.len()];
    let mut stack: Vec<StepIdx> = Vec::new();
    stack.push(start);
    let mut count: u64 = 0;

    while let Some(current) = stack.pop() {
        if count >= max_depth {
            return max_depth; // Hit limit — return max as sentinel
        }
        let Ok(idx) = find_node_position(nodes, current, nodes.len()) else {
            continue;
        };
        if visited.get(idx).copied() == Some(true) {
            continue;
        }
        if let Some(flag) = visited.get_mut(idx) {
            *flag = true;
        }
        count = match count.checked_add(1) {
            Some(next) => next,
            None => return max_depth,
        };
        // Push successors iteratively
        let Ok(node) = node_at_position(nodes, idx, current) else {
            continue;
        };
        match &node.kind {
            CompiledNodeKind::Finish { .. } => {}
            CompiledNodeKind::Choose {
                branches,
                otherwise,
            } => {
                // Push all branches; we'll count all their nodes
                for branch in branches {
                    stack.push(branch.target);
                }
                if let Some(t) = otherwise {
                    stack.push(*t);
                }
            }
            CompiledNodeKind::ChooseSlot {
                branches,
                otherwise,
            } => {
                for branch in branches {
                    stack.push(branch.target);
                }
                if let Some(t) = otherwise {
                    stack.push(*t);
                }
            }
            _ => {
                if let Some(next) = node.next {
                    stack.push(next);
                }
            }
        }
    }
    count
}

/// Iteratively counts nodes along a slot branch.
fn iterative_slot_branch_depth(
    nodes: &[CompiledNode],
    start: StepIdx,
    max_depth: u64,
) -> u64 {
    iterative_branch_depth(nodes, start, max_depth)
}

/// Iteratively finds the longest branch without recursive traversal.
/// This prevents stack overflow on deeply nested Choose graphs.
/// Returns the branch target with the most steps; pushes nothing on error.
fn push_longest_expr_branch(
    nodes: &[CompiledNode],
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
    _node_count: usize,
    stack: &mut Vec<StepIdx>,
) -> Result<(), BudgetTraversalError> {
    // Iteratively compare branch lengths using explicit stacks — no recursion.
    // This is O(branches * depth) but uses O(1) call stack instead of O(depth).
    let mut selected = otherwise;
    let mut selected_depth =
        otherwise.map_or(0, |target| iterative_branch_depth(nodes, target, 10_000));
    for branch in branches {
        // Depth-bounded traversal: stop at 10_000 nodes to prevent DoS.
        // This is a heuristic — we only need relative comparison.
        let depth = iterative_branch_depth(nodes, branch.target, 10_000);
        if depth > selected_depth {
            selected = Some(branch.target);
            selected_depth = depth;
        }
    }
    push_selected_branch(selected, stack);
    Ok(())
}

/// Iteratively finds the longest slot branch without recursive traversal.
/// This prevents stack overflow on deeply nested ChooseSlot graphs.
fn push_longest_slot_branch(
    nodes: &[CompiledNode],
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
    _node_count: usize,
    stack: &mut Vec<StepIdx>,
) -> Result<(), BudgetTraversalError> {
    let mut selected = otherwise;
    let mut selected_depth = otherwise.map_or(0, |target| {
        iterative_slot_branch_depth(nodes, target, 10_000)
    });
    for branch in branches {
        let depth = iterative_slot_branch_depth(nodes, branch.target, 10_000);
        if depth > selected_depth {
            selected = Some(branch.target);
            selected_depth = depth;
        }
    }
    push_selected_branch(selected, stack);
    Ok(())
}

fn push_selected_branch(selected: Option<StepIdx>, stack: &mut Vec<StepIdx>) {
    if let Some(target) = selected {
        stack.push(target);
    }
}
