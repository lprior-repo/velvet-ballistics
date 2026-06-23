#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};

use super::traversal::BudgetTraversalError;
use super::traversal_successors::node_at_position;
use super::types::WholeWorkflowBudget;

pub(super) fn compute_small_linear_budget(
    nodes: &[CompiledNode],
    entry: StepIdx,
    contract: &ResourceContract,
) -> Result<Option<WholeWorkflowBudget>, BudgetTraversalError> {
    // CB-016: `small_linear_domain` already handles the `len() > 2` case
    // through its `_ => false` arm. The duplicate `nodes.len() > 2` guard was
    // dead defensive code that forced a second site to be kept in sync if
    // the small-linear limit ever changed.
    if !small_linear_domain(nodes) {
        return Ok(None);
    }
    let metrics = small_linear_metrics(nodes, entry)?;
    Ok(Some(WholeWorkflowBudget {
        max_total_steps: metrics.steps,
        max_total_slots: u64::from(contract.max_slots),
        max_fanout: 0,
        max_nesting_depth: 0,
        max_steps_executable: match u32::try_from(metrics.steps) {
            Ok(value) => value,
            Err(_) => {
                return Err(BudgetTraversalError::StepCountOverflow {
                    actual: metrics.steps,
                });
            }
        },
        max_action_tickets: metrics.actions,
        max_parallel_in_flight: 0,
        max_retries_per_action: contract.max_retry_attempts,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: metrics.steps,
        max_result_bytes: contract.max_output_bytes,
        max_total_slots_written: u32::from(contract.max_slots),
        max_timer_entries: metrics.timers,
        max_trace_events: metrics.steps,
        max_journal_batch_bytes: contract.max_journal_batch_bytes,
        max_queue_depth: contract.max_queue_depth,
        max_ipc_payload_bytes: contract.max_ipc_payload_bytes,
        max_blob_bytes: contract.max_blob_bytes,
        max_input_bytes: contract.max_input_bytes,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SmallLinearMetrics {
    steps: u64,
    actions: u32,
    timers: u32,
}

fn small_linear_domain(nodes: &[CompiledNode]) -> bool {
    match nodes {
        [] => false,
        [first] => first.id == StepIdx::new(0) && small_linear_node(first, 1),
        [first, second] => {
            first.id == StepIdx::new(0)
                && second.id == StepIdx::new(1)
                && small_linear_node(first, 2)
                && small_linear_node(second, 2)
        }
        _ => false,
    }
}

fn small_linear_node(node: &CompiledNode, node_count: usize) -> bool {
    small_linear_next(node.next, node_count)
        && small_linear_next(node.on_error, node_count)
        && matches!(
            node.kind,
            CompiledNodeKind::Nop
                | CompiledNodeKind::Do { .. }
                | CompiledNodeKind::WaitUntil { .. }
                | CompiledNodeKind::WaitEvent { .. }
                | CompiledNodeKind::Ask { .. }
                | CompiledNodeKind::Finish { .. }
        )
}

fn small_linear_next(next: Option<StepIdx>, node_count: usize) -> bool {
    match next {
        Some(step) => step.as_usize() < node_count,
        None => true,
    }
}

fn small_linear_metrics(
    nodes: &[CompiledNode],
    entry: StepIdx,
) -> Result<SmallLinearMetrics, BudgetTraversalError> {
    let first_idx = entry.as_usize();
    let first = node_at_position(nodes, first_idx, entry)?;
    let first_metrics = small_linear_node_metrics(first);
    match first.next {
        Some(next) if next.as_usize() != first_idx => {
            let second = node_at_position(nodes, next.as_usize(), next)?;
            Ok(first_metrics.add(small_linear_node_metrics(second)))
        }
        _ => Ok(first_metrics),
    }
}

fn small_linear_node_metrics(node: &CompiledNode) -> SmallLinearMetrics {
    match node.kind {
        CompiledNodeKind::Do { .. } => SmallLinearMetrics {
            steps: 1,
            actions: 1,
            timers: 0,
        },
        CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. } => SmallLinearMetrics {
            steps: 1,
            actions: 0,
            timers: 1,
        },
        _ => SmallLinearMetrics {
            steps: 1,
            actions: 0,
            timers: 0,
        },
    }
}

impl SmallLinearMetrics {
    const fn add(self, other: Self) -> Self {
        Self {
            steps: self.steps.saturating_add(other.steps),
            actions: self.actions.saturating_add(other.actions),
            timers: self.timers.saturating_add(other.timers),
        }
    }
}
