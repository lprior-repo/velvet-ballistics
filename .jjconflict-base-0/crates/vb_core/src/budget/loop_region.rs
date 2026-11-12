#![forbid(unsafe_code)]
//! Loop-body region step counting: worst-case node counts within a loop body
//! region, with nested loop bodies multiplied by their iteration limits.

use crate::ids::StepIdx;
use crate::workflow::CompiledNodeKind;

use super::successors::push_successor_targets;
use super::{BudgetError, BudgetTraversalError, find_node_position, node_at_position};

/// Counts the worst-case total steps in a loop body region: all nodes reachable
/// from `body` that are not at or past `done` (the loop exit). Nested loop
/// headers within the body are recursively multiplied by their iteration limits.
pub(super) fn count_body_region_nodes(
    nodes: &[crate::workflow::CompiledNode],
    body: StepIdx,
    done: StepIdx,
    node_count: usize,
) -> Result<u64, BudgetError> {
    super::region_counter::count(nodes, body, done, node_count)
}

/// Visits a single node in a body region during step counting.
#[allow(clippy::too_many_arguments)]
pub(super) fn visit_body_region_node(
    nodes: &[crate::workflow::CompiledNode],
    current: StepIdx,
    done: StepIdx,
    node_count: usize,
    region_visited: &mut [bool],
    stack: &mut Vec<StepIdx>,
    mut count: u64,
    cache: &std::collections::HashMap<(StepIdx, StepIdx), u64>,
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
                    *body,
                    *done,
                    u64::from(limit.get()).max(1),
                    count,
                    stack,
                    cache,
                )?;
            }
        }
        CompiledNodeKind::CollectStart {
            limit, body, done, ..
        } => {
            if *body != current {
                count = count_nested_for_region(
                    *body,
                    *done,
                    u64::from(limit.get()).max(1),
                    count,
                    stack,
                    cache,
                )?;
            }
        }
        CompiledNodeKind::ReduceStart { body, done, .. } => {
            let iter = match u64::try_from(crate::limits::MAX_LIST_ITEMS_PER_VALUE) {
                Ok(value) => value,
                Err(_) => {
                    return Err(BudgetError::TotalStepsExceeded {
                        actual: u64::MAX,
                        limit: u64::MAX,
                    });
                }
            };
            if *body != current {
                count = count_nested_for_region(*body, *done, iter, count, stack, cache)?;
            }
        }
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
            ..
        } => {
            if *body != current {
                count = count_nested_for_region(
                    *body,
                    *done,
                    u64::from(max_attempts.get()).max(1),
                    count,
                    stack,
                    cache,
                )?;
            }
        }
        _ => {
            push_successor_targets(&node.kind, stack);
            if let Some(next) = node.routing.next.step() {
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
    body: StepIdx,
    done: StepIdx,
    iter_count: u64,
    count: u64,
    stack: &mut Vec<StepIdx>,
    cache: &std::collections::HashMap<(StepIdx, StepIdx), u64>,
) -> Result<u64, BudgetError> {
    let body_count = cache
        .get(&(body, done))
        .copied()
        .ok_or(BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        })?;
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
