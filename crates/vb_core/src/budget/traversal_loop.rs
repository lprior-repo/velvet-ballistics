#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind};

use super::budget_error::BudgetError;
use super::traversal::BudgetTraversalError;
use super::traversal_successors::{find_node_position, node_at_position, push_successor_targets};

/// Counts body region steps for a loop header and adds multiplied iterations to total.
#[inline]
#[allow(clippy::too_many_arguments)]
pub(super) fn count_and_push_loop_body(
    nodes: &[CompiledNode],
    body: StepIdx,
    done: StepIdx,
    iter_count: u64,
    visited: &mut [bool],
    node_count: usize,
    mut total: u64,
    stack: &mut Vec<StepIdx>,
) -> Result<u64, BudgetError> {
    let body_count = count_body_region_nodes(nodes, body, done, visited, node_count)?;
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
    nodes: &[CompiledNode],
    done: StepIdx,
    node_count: usize,
    stack: &mut Vec<StepIdx>,
) -> Result<(), BudgetError> {
    let done_idx = find_node_position(nodes, done, node_count)?;
    if let Some(node) = nodes.get(done_idx)
        && node.next.is_none()
        && let Some(next_idx) = done_idx.checked_add(1)
        && next_idx < nodes.len()
        && let Some(next_node) = nodes.get(next_idx)
    {
        stack.push(next_node.id);
    }
    stack.push(done);
    Ok(())
}

/// Counts the worst-case total steps in a loop body region: all nodes reachable
/// from `body` that are not at or past `done` (the loop exit). Nested loop
/// headers within the body are recursively multiplied by their iteration limits.
fn count_body_region_nodes(
    nodes: &[CompiledNode],
    body: StepIdx,
    done: StepIdx,
    global_visited: &mut [bool],
    node_count: usize,
) -> Result<u64, BudgetError> {
    let mut region_visited: Vec<bool> = vec![false; node_count];
    let mut stack: Vec<StepIdx> = Vec::new();
    stack.push(body);

    let mut count: u64 = 0;
    while let Some(current) = stack.pop() {
        count = visit_body_region_node(
            nodes,
            current,
            done,
            node_count,
            global_visited,
            &mut region_visited,
            &mut stack,
            count,
        )?;
    }
    let body_span = done.get().saturating_sub(body.get()).saturating_sub(1);
    Ok(count.max(u64::from(body_span)))
}

/// Visits a single node in a body region during step counting.
#[allow(clippy::too_many_arguments)]
fn visit_body_region_node(
    nodes: &[CompiledNode],
    current: StepIdx,
    done: StepIdx,
    node_count: usize,
    global_visited: &mut [bool],
    region_visited: &mut [bool],
    stack: &mut Vec<StepIdx>,
    mut count: u64,
) -> Result<u64, BudgetError> {
    if current == done {
        return Ok(count);
    }
    let idx = find_node_position(nodes, current, node_count)?;
    if region_visited.get(idx).copied() == Some(true) {
        return Ok(count);
    }
    let Some(flag) = region_visited.get_mut(idx) else {
        return Err(BudgetTraversalError::StepOutOfBounds { step: current }.into());
    };
    *flag = true;

    count = count
        .checked_add(1)
        .ok_or(BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        })?;

    let node = node_at_position(nodes, idx, current)?;

    match &node.kind {
        CompiledNodeKind::ForEachStart {
            limit, body, done, ..
        } => {
            if *body != current {
                count = count_nested_for_region(
                    nodes,
                    *body,
                    *done,
                    u64::from(*limit).max(1),
                    global_visited,
                    node_count,
                    count,
                    stack,
                )?;
            }
        }
        CompiledNodeKind::CollectStart {
            limit, body, done, ..
        } => {
            if *body != current {
                count = count_nested_for_region(
                    nodes,
                    *body,
                    *done,
                    u64::from(*limit).max(1),
                    global_visited,
                    node_count,
                    count,
                    stack,
                )?;
            }
        }
        CompiledNodeKind::ReduceStart { body, done, .. } => {
            // Cold-AST invariant (master §45) drops body, so the declared input
            // length is unknown to the budget traversal. Use the conservative
            // default of 1 iteration for the worst-case step count.
            if *body != current {
                count = count_nested_for_region(
                    nodes,
                    *body,
                    *done,
                    1,
                    global_visited,
                    node_count,
                    count,
                    stack,
                )?;
            }
        }
        CompiledNodeKind::RepeatStart {
            max_attempts: _,
            body,
            done,
            ..
        } => {
            // Cold-AST invariant (master §45) drops body, so the runtime
            // attempt count cannot be bounded from the compiled IR alone.
            // Use the conservative default of 1 iteration for the
            // worst-case step count. The declared `max_attempts` is still
            // tracked separately in `WholeWorkflowBudget.max_repeat_attempts`.
            if *body != current {
                count = count_nested_for_region(
                    nodes,
                    *body,
                    *done,
                    1,
                    global_visited,
                    node_count,
                    count,
                    stack,
                )?;
            }
        }
        _ => {
            push_successor_targets(&node.kind, stack);
            if let Some(next) = node.next {
                stack.push(next);
            }
        }
    }
    Ok(count)
}

/// Counts a nested loop body within a region and adds multiplied iterations.
#[inline]
#[allow(clippy::too_many_arguments)]
fn count_nested_for_region(
    nodes: &[CompiledNode],
    body: StepIdx,
    done: StepIdx,
    iter_count: u64,
    global_visited: &mut [bool],
    node_count: usize,
    count: u64,
    stack: &mut Vec<StepIdx>,
) -> Result<u64, BudgetError> {
    let body_count = count_body_region_nodes(nodes, body, done, global_visited, node_count)?;
    stack.push(done);
    let product = body_count
        .checked_mul(iter_count)
        .ok_or(BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        })?;
    count
        .checked_add(product)
        .ok_or(BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        })
}
