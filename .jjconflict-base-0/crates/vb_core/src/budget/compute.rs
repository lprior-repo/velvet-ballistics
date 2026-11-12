#![forbid(unsafe_code)]
//! Whole-workflow budget computation: the `WholeWorkflowBudget` type and
//! the traversal that derives every budget dimension from the IR.

use crate::ids::StepIdx;
use crate::workflow::{ResourceContract, WorkflowError};

use super::BudgetTraversalError;
use super::depth::compute_structural_depth;
use super::loop_body::count_total_steps;
use super::reachable::compute_reachable_metrics;
use super::small_linear::compute_small_linear_budget;
use super::successors::bounded_tracking_vec;

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
pub(super) struct DerivedBudgetDimensions {
    pub(super) max_transitions: u64,
    pub(super) max_journal_bytes_total: u64,
    pub(super) max_value_store_entries: u64,
    pub(super) max_incident_emissions: u64,
    pub(super) max_cost_units: u64,
}

pub(super) fn derive_budget_dimensions(
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
