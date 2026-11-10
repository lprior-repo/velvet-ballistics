#![forbid(unsafe_code)]
//! Reachable-metrics computation over the workflow graph.

use crate::ids::StepIdx;
use crate::workflow::CompiledNodeKind;

use super::BudgetTraversalError;
use super::successors::{
    branch_count_to_u16, insert_tracked_step, push_successor_targets, remove_tracked_step,
    tracked_steps_contain,
};
use super::{find_node_position, node_at_position};

enum ReachableVisit {
    Enter(StepIdx),
    Exit(u16),
}

/// Computes once-only resource metrics and reachability via a DFS walk.
#[allow(clippy::too_many_arguments)]
pub(super) fn compute_reachable_metrics(
    nodes: &[crate::workflow::CompiledNode],
    current: StepIdx,
    visited: &mut [bool],
    in_path: &mut Vec<u16>,
    node_count: usize,
    max_fanout: &mut u16,
    max_action_tickets: &mut u32,
    max_gather_pages: &mut u32,
    max_gather_items: &mut u32,
    max_for_each_iterations: &mut u32,
    max_together_branches: &mut u16,
    max_repeat_attempts: &mut u16,
    max_timer_entries: &mut u32,
    max_open_gates: &mut u32,
    max_signal_subscriptions: &mut u32,
) -> Result<(), BudgetTraversalError> {
    // vb-8eocj.7.2.1 Explicit DFS event stack preserves the original recursive
    // in-path set semantics so jump cycle detection and backtracking remain
    // identical while the stack grows linearly with node count.
    let mut stack: Vec<ReachableVisit> = Vec::new();
    stack.push(ReachableVisit::Enter(current));

    while let Some(event) = stack.pop() {
        match event {
            ReachableVisit::Enter(step) => {
                let idx = find_node_position(nodes, step, node_count)?;
                if visited.get(idx).copied() == Some(true) {
                    continue;
                }
                let Some(flag) = visited.get_mut(idx) else {
                    return Err(BudgetTraversalError::StepOutOfBounds { step });
                };
                *flag = true;

                let node = node_at_position(nodes, idx, step)?;
                let step_u16 = step.get();
                insert_tracked_step(in_path, step_u16, node_count)?;

                if let CompiledNodeKind::Jump { target } = &node.kind {
                    let target_u16 = target.get();
                    if tracked_steps_contain(in_path, target_u16) {
                        remove_tracked_step(in_path, step_u16);
                        return Err(BudgetTraversalError::JumpCycle {
                            step,
                            target: *target,
                        });
                    }
                }

                if node.retry.is_some_and(|policy| policy.delay_ms > 0) {
                    *max_timer_entries = max_timer_entries
                        .checked_add(1)
                        .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
                }
                update_fanout(&node.kind, max_fanout)?;
                update_workflow_metrics(
                    &node.kind,
                    max_action_tickets,
                    max_gather_pages,
                    max_gather_items,
                    max_for_each_iterations,
                    max_together_branches,
                    max_repeat_attempts,
                    max_timer_entries,
                    max_open_gates,
                    max_signal_subscriptions,
                )?;

                let mut targets: Vec<StepIdx> = Vec::new();
                push_successor_targets(&node.kind, &mut targets);
                if let Some(next) = node.routing.next.step() {
                    targets.push(next);
                }

                if let Some(handler) = node.routing.error.handler() {
                    targets.push(handler);
                }

                stack.push(ReachableVisit::Exit(step_u16));
                for target in targets.iter().rev() {
                    if find_node_position(nodes, *target, node_count).is_ok() {
                        stack.push(ReachableVisit::Enter(*target));
                    }
                }
            }
            ReachableVisit::Exit(step_u16) => {
                remove_tracked_step(in_path, step_u16);
            }
        }
    }

    Ok(())
}

fn update_fanout(
    kind: &CompiledNodeKind,
    max_fanout: &mut u16,
) -> Result<(), BudgetTraversalError> {
    match kind {
        CompiledNodeKind::TogetherStart { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len())?;
            if branch_count > *max_fanout {
                *max_fanout = branch_count;
            }
        }
        CompiledNodeKind::ChooseSlot { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len())?;
            if branch_count > *max_fanout {
                *max_fanout = branch_count;
            }
        }
        CompiledNodeKind::Choose { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len())?;
            if branch_count > *max_fanout {
                *max_fanout = branch_count;
            }
        }
        _ => {}
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn update_workflow_metrics(
    kind: &CompiledNodeKind,
    max_action_tickets: &mut u32,
    max_gather_pages: &mut u32,
    max_gather_items: &mut u32,
    max_for_each_iterations: &mut u32,
    max_together_branches: &mut u16,
    max_repeat_attempts: &mut u16,
    max_timer_entries: &mut u32,
    max_open_gates: &mut u32,
    max_signal_subscriptions: &mut u32,
) -> Result<(), BudgetTraversalError> {
    match kind {
        CompiledNodeKind::Do { .. } => {
            *max_action_tickets = max_action_tickets
                .checked_add(1)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        CompiledNodeKind::TogetherStart { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len())?;
            if branch_count > *max_together_branches {
                *max_together_branches = branch_count;
            }
        }
        CompiledNodeKind::CollectStart {
            limit, page_size, ..
        } => {
            let per_page = u64::from(*page_size).max(1);
            let pages = u64::from(*limit).div_ceil(per_page);
            let pages = u32::try_from(pages)
                .map_err(|_| BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
            *max_gather_pages = max_gather_pages
                .checked_add(pages)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
            *max_gather_items = max_gather_items
                .checked_add(*limit)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        CompiledNodeKind::ForEachStart { limit, .. } => {
            *max_for_each_iterations = max_for_each_iterations
                .checked_add(*limit)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        CompiledNodeKind::RepeatStart { max_attempts, .. } => {
            *max_repeat_attempts = (*max_repeat_attempts).max(*max_attempts);
        }
        CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::RetryCheck { .. }
        | CompiledNodeKind::RepeatCheck { .. } => {
            *max_timer_entries = max_timer_entries
                .checked_add(1)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        _ => {}
    }
    match kind {
        CompiledNodeKind::Ask { .. } => {
            *max_open_gates = max_open_gates
                .checked_add(1)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        CompiledNodeKind::WaitEvent { .. } => {
            *max_signal_subscriptions = max_signal_subscriptions
                .checked_add(1)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        _ => {}
    }
    Ok(())
}
