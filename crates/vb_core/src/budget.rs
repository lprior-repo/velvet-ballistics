#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

//! Whole-workflow budget computation and boundedness policy enforcement.

use crate::ids::StepIdx;
use crate::workflow::{CompiledNodeKind, ResourceContract, WorkflowError};
use depth::compute_structural_depth;
use loop_body::count_total_steps;
use loop_region::visit_body_region_node;
use reachable::compute_reachable_metrics;
use successors::bounded_tracking_vec;

/// Computed budget for an entire workflow, derived by walking the IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WholeWorkflowBudget {
    /// Sum of all step budgets across all branches.
    pub max_total_steps: u64,
    /// Maximum slot count across all paths.
    pub max_total_slots: u64,
    /// Maximum concurrent branches (fanout).
    pub max_fanout: u16,
    /// Conservative structural nesting bound over reachable owners.
    pub max_nesting_depth: u16,
    /// Maximum executable step count per workflow admission.
    pub max_steps_executable: u32,
    /// Maximum action tickets (Do nodes) in the workflow.
    pub max_action_tickets: u32,
    /// Maximum parallel in-flight actions.
    pub max_parallel_in_flight: u16,
    /// Maximum retries per action.
    pub max_retries_per_action: u16,
    /// Maximum gather pages across all CollectStart nodes.
    pub max_gather_pages: u32,
    /// Maximum gather items across all CollectStart nodes.
    pub max_gather_items: u32,
    /// Maximum for-each loop iterations.
    pub max_for_each_iterations: u32,
    /// Maximum together branches in any TogetherStart.
    pub max_together_branches: u16,
    /// Maximum repeat attempts in any RepeatStart.
    pub max_repeat_attempts: u16,
    /// Maximum run time in seconds.
    pub max_run_time_seconds: u64,
    /// Maximum result bytes.
    pub max_result_bytes: u32,
    /// Maximum total slots written.
    pub max_total_slots_written: u32,
    /// Maximum timer entries reserved for waits, asks, retries, and repeat checks.
    pub max_timer_entries: u32,
    /// Maximum trace events reserved for deterministic execution.
    pub max_trace_events: u64,
    /// Maximum journal batch bytes required by reachable journal-producing operations.
    pub max_journal_batch_bytes: u32,
    /// Maximum queue entries required by reachable suspension/admission operations.
    pub max_queue_depth: u32,
    /// Maximum IPC payload bytes required by reachable IPC operations.
    pub max_ipc_payload_bytes: u32,
    /// Maximum blob bytes required by reachable blob/resource operations.
    pub max_blob_bytes: u64,
    /// Maximum input bytes required by reachable input operations.
    pub max_input_bytes: u32,
    /// Maximum simultaneously open gates, counted as reachable Ask nodes.
    /// AskResume nodes are resumes, not new gates, and are not counted.
    pub max_open_gates: u32,
    /// Maximum signal subscriptions, counted as reachable WaitEvent nodes.
    /// WaitUntil nodes are timer waits, not signal subscriptions.
    pub max_signal_subscriptions: u32,
    /// Maximum incident emissions over the run. No IR node emits incidents,
    /// so this is bounded by total step executions: at most one incident
    /// per executed step.
    pub max_incident_emissions: u64,
    /// Maximum total engine transitions over the run. This is the run total
    /// that `max_transitions_per_tick` (per-tick, from the contract) is not:
    /// total steps times the retry factor, since a retry body re-executes.
    pub max_transitions: u64,
    /// Maximum cumulative journal bytes over the run: the per-batch cap
    /// times total steps. Batches hold at least one event each and journal
    /// events are bounded by trace events, which equal total steps.
    pub max_journal_bytes_total: u64,
    /// Maximum value-store entries over the run: at most one new top-level
    /// entry per executed step, plus input slots and the constant pool.
    /// Deeply nested action/input payloads are bounded at runtime by the
    /// byte caps and the remaining-budget counter, not by this bound.
    pub max_value_store_entries: u64,
    /// Maximum cost units over the run. Cost model: one unit per action
    /// ticket issued, and a retry re-issues the ticket, so this equals
    /// the action ticket bound times the retry factor, mirroring the
    /// transition bound: a verifier that counted distinct tickets only
    /// would under-approximate every retried action.
    pub max_cost_units: u64,
}

impl WholeWorkflowBudget {
    pub const MAX_STEPS_EXECUTABLE: u32 = 32_768;

    /// Walks the compiled IR starting from `entry` and computes all
    /// budget dimensions.
    pub fn compute(
        nodes: &[crate::workflow::CompiledNode],
        entry: StepIdx,
        contract: &ResourceContract,
    ) -> Result<Self, WorkflowError> {
        Self::compute_budget_local(nodes, entry, contract).map_err(WorkflowError::from)
    }

    /// Internal budget traversal path with a narrow error type. This keeps Kani
    /// from exploring unrelated `WorkflowError::Expression(CoreError)` drops.
    #[cfg_attr(kani, allow(unreachable_code))]
    pub(crate) fn compute_budget_local(
        nodes: &[crate::workflow::CompiledNode],
        entry: StepIdx,
        contract: &ResourceContract,
    ) -> Result<Self, BudgetTraversalError> {
        let node_count = nodes.len();
        if entry.as_usize() >= node_count {
            return Err(BudgetTraversalError::EntryOutOfBounds { entry });
        }

        if let Some(budget) = compute_small_linear_budget(nodes, entry, contract)? {
            return Ok(budget);
        }

        #[cfg(kani)]
        return Err(BudgetTraversalError::StepOutOfBounds { step: entry });

        #[cfg(not(kani))]
        {
            let mut visited: Vec<bool> = vec![false; node_count];
            let mut in_path: Vec<u16> = bounded_tracking_vec(node_count);
            let max_total_steps = count_total_steps(nodes, entry, node_count)?;
            check_total_steps_representable(max_total_steps)?;

            let mut max_fanout: u16 = 0;
            let mut max_action_tickets: u32 = 0;
            let mut max_parallel_in_flight: u16 = 0;
            let mut max_gather_pages: u32 = 0;
            let mut max_gather_items: u32 = 0;
            let mut max_for_each_iterations: u32 = 0;
            let mut max_together_branches: u16 = 0;
            let mut max_repeat_attempts: u16 = 0;
            let mut max_timer_entries: u32 = 0;
            let mut max_open_gates: u32 = 0;
            let mut max_signal_subscriptions: u32 = 0;
            compute_reachable_metrics(
                nodes,
                entry,
                &mut visited,
                &mut in_path,
                node_count,
                &mut max_fanout,
                &mut max_action_tickets,
                &mut max_parallel_in_flight,
                &mut max_gather_pages,
                &mut max_gather_items,
                &mut max_for_each_iterations,
                &mut max_together_branches,
                &mut max_repeat_attempts,
                &mut max_timer_entries,
                &mut max_open_gates,
                &mut max_signal_subscriptions,
            )?;

            let max_nesting_depth = compute_structural_depth(nodes, &visited)?;
            let max_total_slots = u64::from(contract.max_slots);

            // Phase 0 executes at most one step per runtime tick, so steps bound time.
            let max_run_time_seconds = max_total_steps;

            let derived = derive_budget_dimensions(max_total_steps, max_action_tickets, contract)?;

            Ok(Self {
                max_total_steps,
                max_total_slots,
                max_fanout,
                max_nesting_depth,
                max_steps_executable: Self::MAX_STEPS_EXECUTABLE,
                max_action_tickets,
                max_parallel_in_flight,
                max_retries_per_action: contract.max_retry_attempts,
                max_gather_pages,
                max_gather_items,
                max_for_each_iterations,
                max_together_branches,
                max_repeat_attempts,
                max_run_time_seconds,
                max_result_bytes: contract.max_output_bytes,
                max_total_slots_written: u32::from(contract.max_slots),
                max_timer_entries,
                max_trace_events: max_total_steps,
                max_journal_batch_bytes: contract.max_journal_batch_bytes,
                max_queue_depth: contract.max_queue_depth,
                max_ipc_payload_bytes: contract.max_ipc_payload_bytes,
                max_blob_bytes: contract.max_blob_bytes,
                max_input_bytes: contract.max_input_bytes,
                max_open_gates,
                max_signal_subscriptions,
                max_incident_emissions: derived.max_incident_emissions,
                max_transitions: derived.max_transitions,
                max_journal_bytes_total: derived.max_journal_bytes_total,
                max_value_store_entries: derived.max_value_store_entries,
                max_cost_units: derived.max_cost_units,
            })
        }
    }
}

fn check_total_steps_representable(total: u64) -> Result<(), BudgetTraversalError> {
    if total > u64::from(u32::MAX) {
        return Err(BudgetTraversalError::StepCountOverflow { actual: total });
    }
    Ok(())
}

/// Dimensions derived from the step and ticket bounds plus the contract.
/// Shared by the general and small-linear compute paths so both agree.
struct DerivedBudgetDimensions {
    max_transitions: u64,
    max_journal_bytes_total: u64,
    max_value_store_entries: u64,
    max_incident_emissions: u64,
    max_cost_units: u64,
}

fn derive_budget_dimensions(
    max_total_steps: u64,
    max_action_tickets: u32,
    contract: &ResourceContract,
) -> Result<DerivedBudgetDimensions, BudgetTraversalError> {
    let retry_factor = u64::from(contract.max_retry_attempts)
        .checked_add(1)
        .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
    let max_transitions = max_total_steps
        .checked_mul(retry_factor)
        .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
    let max_journal_bytes_total = u64::from(contract.max_journal_batch_bytes)
        .checked_mul(max_total_steps)
        .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
    let max_value_store_entries = max_total_steps
        .checked_add(u64::from(contract.max_slots))
        .and_then(|value| value.checked_add(u64::from(contract.max_constants)))
        .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
    let max_cost_units = u64::from(max_action_tickets)
        .checked_mul(retry_factor)
        .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
    Ok(DerivedBudgetDimensions {
        max_transitions,
        max_journal_bytes_total,
        max_value_store_entries,
        max_incident_emissions: max_total_steps,
        max_cost_units,
    })
}

/// Budget-local traversal failures. Deliberately excludes expression/core error
/// variants so proof harnesses do not pay for unrelated destructor graphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BudgetTraversalError {
    EntryOutOfBounds {
        entry: StepIdx,
    },
    StepOutOfBounds {
        step: StepIdx,
    },
    StepCountOverflow {
        actual: u64,
    },
    /// `u16` nesting depth overflowed at `compute_child_depth`; carries the
    /// actual pre-overflow depth so diagnostics can report the real value
    /// instead of the lossy `u64::MAX` sentinel previously produced here.
    DepthOverflow {
        depth: u16,
    },
    JumpCycle {
        step: StepIdx,
        target: StepIdx,
    },
    BackwardScopeExit {
        from: StepIdx,
        to: StepIdx,
    },
}

impl From<BudgetTraversalError> for WorkflowError {
    fn from(error: BudgetTraversalError) -> Self {
        match error {
            BudgetTraversalError::EntryOutOfBounds { entry } => Self::EntryOutOfBounds { entry },
            BudgetTraversalError::StepOutOfBounds { step } => Self::StepOutOfBounds { step },
            BudgetTraversalError::StepCountOverflow { actual } => {
                Self::StepCountOverflow { actual }
            }
            BudgetTraversalError::DepthOverflow { depth } => Self::DepthOverflow { depth },
            BudgetTraversalError::JumpCycle { step, target } => Self::JumpCycle { step, target },
            BudgetTraversalError::BackwardScopeExit { from, to } => Self::BackwardEdge { from, to },
        }
    }
}

fn compute_small_linear_budget(
    nodes: &[crate::workflow::CompiledNode],
    entry: StepIdx,
    contract: &ResourceContract,
) -> Result<Option<WholeWorkflowBudget>, BudgetTraversalError> {
    if nodes.len() > 2 || !small_linear_domain(nodes) {
        return Ok(None);
    }
    let metrics = small_linear_metrics(nodes, entry)?;
    let derived = derive_budget_dimensions(metrics.steps, metrics.actions, contract)?;
    Ok(Some(WholeWorkflowBudget {
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
    }))
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

fn small_linear_node(node: &crate::workflow::CompiledNode, node_count: usize) -> bool {
    small_linear_next(node.routing.next.step(), node_count)
        && small_linear_next(node.routing.error.handler(), node_count)
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
    nodes: &[crate::workflow::CompiledNode],
    entry: StepIdx,
) -> Result<SmallLinearMetrics, BudgetTraversalError> {
    let first_idx = entry.as_usize();
    let first = node_at_position(nodes, first_idx, entry)?;
    let first_metrics = small_linear_node_metrics(first);
    match first.routing.next.step() {
        Some(next) if next.as_usize() != first_idx => {
            let second = node_at_position(nodes, next.as_usize(), next)?;
            Ok(first_metrics.add(small_linear_node_metrics(second)))
        }
        _ => Ok(first_metrics),
    }
}

fn small_linear_node_metrics(node: &crate::workflow::CompiledNode) -> SmallLinearMetrics {
    match node.kind {
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
    }
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

pub use aggregate::{
    AggregateBudgetError, AggregateReservation, AggregateResourceBudget, AggregateResourceCapacity,
    AggregateResourceUsage, ShardUsageSnapshot,
};
pub use aggregate_validate::{check_global_quota, check_tenant_quota, validate_aggregate_budget};
pub use validate::BoundednessPolicy;
pub use validate_error::{BudgetDimension, BudgetError};

pub use remaining::{BudgetExhaustionOutcome, RemainingBudget, RemainingBudgetError};

/// Validates step ceiling dimensions (max_step_budget_per_tick and
/// max_transitions_per_tick) against hard limits.
pub fn validate_step_ceilings(
    budget: &AggregateResourceBudget,
) -> Result<(), AggregateBudgetError> {
    // Hard limit for step budget per tick - derived from MAX_STEPS_PER_TICK if defined,
    // otherwise use a conservative upper bound.
    const HARD_MAX_STEP_BUDGET_PER_TICK: u64 = 1_000_000;
    const HARD_MAX_TRANSITIONS_PER_TICK: u64 = 1_000_000;

    if budget.max_step_budget_per_tick == 0 {
        return Err(AggregateBudgetError::StepCeilingExceeded {
            requested: 0,
            limit: HARD_MAX_STEP_BUDGET_PER_TICK,
        });
    }
    if budget.max_step_budget_per_tick > HARD_MAX_STEP_BUDGET_PER_TICK {
        return Err(AggregateBudgetError::StepCeilingExceeded {
            requested: budget.max_step_budget_per_tick,
            limit: HARD_MAX_STEP_BUDGET_PER_TICK,
        });
    }

    if budget.max_transitions_per_tick == 0 {
        return Err(AggregateBudgetError::PerTickCeilingExceeded {
            requested: 0,
            limit: HARD_MAX_TRANSITIONS_PER_TICK,
        });
    }
    if budget.max_transitions_per_tick > HARD_MAX_TRANSITIONS_PER_TICK {
        return Err(AggregateBudgetError::PerTickCeilingExceeded {
            requested: budget.max_transitions_per_tick,
            limit: HARD_MAX_TRANSITIONS_PER_TICK,
        });
    }

    Ok(())
}

pub(super) fn add_dim(
    current: u64,
    requested: u64,
    resource: &'static str,
) -> Result<u64, AggregateBudgetError> {
    current
        .checked_add(requested)
        .ok_or(AggregateBudgetError::Overflow { resource })
}

pub(super) fn sub_dim(
    current: u64,
    requested: u64,
    resource: &'static str,
) -> Result<u64, AggregateBudgetError> {
    current
        .checked_sub(requested)
        .ok_or(AggregateBudgetError::Underflow { resource })
}

pub(super) fn check_capacity(
    resource: &'static str,
    requested: u64,
    available: u64,
) -> Result<(), AggregateBudgetError> {
    if requested > available {
        Err(AggregateBudgetError::CapacityExceeded {
            resource,
            requested,
            available,
        })
    } else {
        Ok(())
    }
}

pub(super) fn check_policy(
    resource: &'static str,
    actual: u64,
    limit: u64,
) -> Result<(), AggregateBudgetError> {
    if actual > limit {
        Err(AggregateBudgetError::PolicyExceeded {
            resource,
            actual,
            limit,
        })
    } else {
        Ok(())
    }
}

impl From<WorkflowError> for BudgetError {
    fn from(_err: WorkflowError) -> Self {
        BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        }
    }
}

impl From<BudgetTraversalError> for BudgetError {
    fn from(_err: BudgetTraversalError) -> Self {
        BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        }
    }
}

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
mod aggregate;
mod aggregate_validate;
mod conditional_counter;
mod validate;
mod validate_error;

fn find_node_position(
    nodes: &[crate::workflow::CompiledNode],
    step: StepIdx,
    node_count: usize,
) -> Result<usize, BudgetTraversalError> {
    let direct_idx = step.as_usize();
    if direct_idx < node_count
        && let Some(node) = nodes.get(direct_idx)
        && node.id == step
    {
        return Ok(direct_idx);
    }

    for (position, node) in nodes.iter().enumerate() {
        if node.id == step {
            return Ok(position);
        }
    }

    if direct_idx < node_count {
        return Ok(direct_idx);
    }

    Err(BudgetTraversalError::StepOutOfBounds { step })
}

fn node_at_position(
    nodes: &[crate::workflow::CompiledNode],
    position: usize,
    step: StepIdx,
) -> Result<&crate::workflow::CompiledNode, BudgetTraversalError> {
    match nodes.get(position) {
        Some(node) => Ok(node),
        None => Err(BudgetTraversalError::StepOutOfBounds { step }),
    }
}

mod loop_body;
mod loop_region;
mod region_counter;

mod depth;
mod reachable;
mod remaining;
mod successors;
mod usage;

mod tests_and_verification;

#[cfg(test)]
#[path = "budget/tests.rs"]
mod tests;

#[cfg(test)]
mod vb_qi37_2_4_state8_tests;

#[cfg(test)]
mod depth_overflow_tests {
    //! Regression tests for CB-004 / `vb-lk26g`:
    //! `compute_child_depth` must return `BudgetTraversalError::DepthOverflow`
    //! carrying the actual pre-overflow depth value, not the lossy
    //! `StepCountOverflow { actual: u64::MAX }` sentinel it used to produce.

    use super::depth::compute_child_depth;
    use super::{BudgetTraversalError, CompiledNodeKind};

    #[test]
    fn compute_child_depth_returns_depth_overflow_carrying_actual_depth() {
        let kind = CompiledNodeKind::ForEachStart {
            input: crate::ids::SlotIdx::new(0),
            item_slot: crate::ids::SlotIdx::new(1),
            limit: 1,
            body: crate::ids::StepIdx::new(0),
            done: crate::ids::StepIdx::new(0),
        };
        let mut max_nesting_depth: u16 = 0;
        let err = compute_child_depth(&kind, u16::MAX, &mut max_nesting_depth)
            .expect_err("u16::MAX + 1 must overflow");
        match err {
            BudgetTraversalError::DepthOverflow { depth } => {
                assert_eq!(
                    depth,
                    u16::MAX,
                    "DepthOverflow must carry the actual pre-overflow depth"
                );
            }
            other => {
                let msg = format!(
                    "expected BudgetTraversalError::DepthOverflow {{ depth: u16::MAX }}, got {other:?}"
                );
                debug_assert!(false, "{msg}");
            }
        }
    }

    #[test]
    fn compute_child_depth_does_not_emit_step_count_overflow_at_u16_max() {
        let kind = CompiledNodeKind::RepeatStart {
            max_attempts: 1,
            body: crate::ids::StepIdx::new(0),
            done: crate::ids::StepIdx::new(0),
        };
        let mut max_nesting_depth: u16 = 0;
        let err = compute_child_depth(&kind, u16::MAX, &mut max_nesting_depth)
            .expect_err("u16::MAX + 1 must overflow");
        assert!(
            !matches!(err, BudgetTraversalError::StepCountOverflow { .. }),
            "depth overflow must not be misclassified as StepCountOverflow (CB-004)"
        );
    }

    #[test]
    fn depth_overflow_converts_to_workflow_error_carrying_actual_depth() {
        let err: crate::workflow::WorkflowError =
            BudgetTraversalError::DepthOverflow { depth: 42 }.into();
        match err {
            crate::workflow::WorkflowError::DepthOverflow { depth } => {
                assert_eq!(depth, 42, "From conversion must preserve depth");
            }
            other => {
                let msg =
                    format!("expected WorkflowError::DepthOverflow {{ depth: 42 }}, got {other:?}");
                debug_assert!(false, "{msg}");
            }
        }
    }
}
