#![forbid(unsafe_code)]
//! Budget computation over one- and two-node simple control-flow graphs.

use crate::ids::StepIdx;
use crate::workflow::{CompiledNodeKind, ResourceContract};

use super::BudgetTraversalError;
use super::WholeWorkflowBudget;
use super::compute::derive_budget_dimensions;
use super::{find_node_position, node_at_position};

pub(super) struct SmallLinearGraph<'a> {
    nodes: &'a [crate::workflow::CompiledNode],
}

impl<'a> SmallLinearGraph<'a> {
    pub(super) fn new(nodes: &'a [crate::workflow::CompiledNode]) -> Option<Self> {
        if small_linear_domain(nodes) {
            Some(Self { nodes })
        } else {
            None
        }
    }

    pub(super) fn compute(
        self,
        entry: StepIdx,
        contract: &ResourceContract,
    ) -> Result<WholeWorkflowBudget, BudgetTraversalError> {
        let metrics = small_linear_metrics(self.nodes, entry)?;
        let derived = derive_budget_dimensions(metrics.steps, metrics.actions, contract)?;
        Ok(WholeWorkflowBudget {
            max_total_steps: metrics.steps,
            max_total_slots: u64::from(contract.max_slots),
            max_fanout: 0,
            max_nesting_depth: 0,
            max_steps_executable: WholeWorkflowBudget::MAX_STEPS_EXECUTABLE,
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
            max_open_gates: metrics.gates,
            max_signal_subscriptions: metrics.signals,
            max_incident_emissions: derived.max_incident_emissions,
            max_transitions: derived.max_transitions,
            max_journal_bytes_total: derived.max_journal_bytes_total,
            max_value_store_entries: derived.max_value_store_entries,
            max_cost_units: derived.max_cost_units,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SmallLinearMetrics {
    steps: u64,
    actions: u32,
    timers: u32,
    gates: u32,
    signals: u32,
}

fn small_linear_domain(nodes: &[crate::workflow::CompiledNode]) -> bool {
    match nodes {
        [first] => small_linear_node(first),
        [first, second] => small_linear_node(first) && small_linear_node(second),
        _ => false,
    }
}

fn small_linear_node(node: &crate::workflow::CompiledNode) -> bool {
    matches!(
        node.kind,
        CompiledNodeKind::Nop
            | CompiledNodeKind::Do { .. }
            | CompiledNodeKind::WaitUntil { .. }
            | CompiledNodeKind::WaitEvent { .. }
            | CompiledNodeKind::Ask { .. }
            | CompiledNodeKind::Finish { .. }
    )
}

fn small_linear_metrics(
    nodes: &[crate::workflow::CompiledNode],
    entry: StepIdx,
) -> Result<SmallLinearMetrics, BudgetTraversalError> {
    // Two first visits push at most two routes each, plus the initial entry.
    let mut pending = [Some(entry), None, None, None, None];
    let mut pending_len = 1_usize;
    let mut visited = [false; 2];
    let mut metrics = SmallLinearMetrics {
        steps: 0,
        actions: 0,
        timers: 0,
        gates: 0,
        signals: 0,
    };
    while let Some(index) = pending_len.checked_sub(1) {
        pending_len = index;
        let current = pending
            .get_mut(index)
            .and_then(Option::take)
            .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        let position = find_node_position(nodes, current, nodes.len())?;
        let flag = visited
            .get_mut(position)
            .ok_or(BudgetTraversalError::StepOutOfBounds { step: current })?;
        if *flag {
            continue;
        }
        *flag = true;
        let node = node_at_position(nodes, position, current)?;
        metrics = metrics.add(small_linear_node_metrics(node));
        push_small_target(&mut pending, &mut pending_len, node.routing.next.step())?;
        push_small_target(&mut pending, &mut pending_len, node.routing.error.handler())?;
    }
    Ok(metrics)
}

fn push_small_target(
    pending: &mut [Option<StepIdx>; 5],
    len: &mut usize,
    target: Option<StepIdx>,
) -> Result<(), BudgetTraversalError> {
    if let Some(target) = target {
        let slot = pending
            .get_mut(*len)
            .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        *slot = Some(target);
        *len = len
            .checked_add(1)
            .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
    }
    Ok(())
}

fn small_linear_node_metrics(node: &crate::workflow::CompiledNode) -> SmallLinearMetrics {
    let mut metrics = match node.kind {
        CompiledNodeKind::Do { .. } => SmallLinearMetrics {
            steps: 1,
            actions: 1,
            timers: 0,
            gates: 0,
            signals: 0,
        },
        CompiledNodeKind::WaitEvent { .. } => SmallLinearMetrics {
            steps: 1,
            actions: 0,
            timers: 1,
            gates: 0,
            signals: 1,
        },
        CompiledNodeKind::Ask { .. } => SmallLinearMetrics {
            steps: 1,
            actions: 0,
            timers: 1,
            gates: 1,
            signals: 0,
        },
        CompiledNodeKind::WaitUntil { .. } => SmallLinearMetrics {
            steps: 1,
            actions: 0,
            timers: 1,
            gates: 0,
            signals: 0,
        },
        _ => SmallLinearMetrics {
            steps: 1,
            actions: 0,
            timers: 0,
            gates: 0,
            signals: 0,
        },
    };
    metrics.timers = metrics.timers.saturating_add(u32::from(
        node.retry.is_some_and(|policy| policy.delay_ms > 0),
    ));
    metrics
}

impl SmallLinearMetrics {
    const fn add(self, other: Self) -> Self {
        Self {
            steps: self.steps.saturating_add(other.steps),
            actions: self.actions.saturating_add(other.actions),
            timers: self.timers.saturating_add(other.timers),
            gates: self.gates.saturating_add(other.gates),
            signals: self.signals.saturating_add(other.signals),
        }
    }
}

#[cfg(test)]
#[path = "small_linear_route_tests.rs"]
mod route_tests;
