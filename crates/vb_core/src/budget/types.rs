#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, ResourceContract, WorkflowError};

use super::small_linear::compute_small_linear_budget;
use super::traversal::BudgetTraversalError;

/// Computed budget for an entire workflow, derived by walking the IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WholeWorkflowBudget {
    /// Sum of all step budgets across all branches.
    pub max_total_steps: u64,
    /// Maximum slot count across all paths.
    pub max_total_slots: u64,
    /// Maximum concurrent branches (fanout).
    pub max_fanout: u16,
    /// Maximum loop nesting depth.
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
}

impl WholeWorkflowBudget {
    /// Walks the compiled IR starting from `entry` and computes all
    /// budget dimensions.
    pub fn compute(
        nodes: &[CompiledNode],
        entry: StepIdx,
        contract: &ResourceContract,
    ) -> Result<Self, WorkflowError> {
        Self::compute_budget_local(nodes, entry, contract).map_err(WorkflowError::from)
    }

    /// Internal budget traversal path with a narrow error type. This keeps Kani
    /// from exploring unrelated `WorkflowError::Expression(CoreError)` drops.
    #[cfg_attr(kani, allow(unreachable_code))]
    pub(crate) fn compute_budget_local(
        nodes: &[CompiledNode],
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
            use super::traversal_driver::compute_fanout_and_depth;
            use super::traversal_step_count::count_total_steps;
            use super::traversal_tracking::bounded_tracking_vec;

            let mut visited: Vec<bool> = vec![false; node_count];
            let mut in_path: Vec<u16> = bounded_tracking_vec(node_count);
            let max_total_steps = count_total_steps(nodes, entry, node_count)?;

            let mut max_fanout: u16 = 0;
            let mut max_nesting_depth: u16 = 0;
            let mut max_action_tickets: u32 = 0;
            let mut max_parallel_in_flight: u16 = 0;
            let mut max_gather_pages: u32 = 0;
            let mut max_gather_items: u32 = 0;
            let mut max_for_each_iterations: u32 = 0;
            let mut max_together_branches: u16 = 0;
            let mut max_repeat_attempts: u16 = 0;
            let mut max_timer_entries: u32 = 0;
            compute_fanout_and_depth(
                nodes,
                entry,
                &mut visited,
                &mut in_path,
                node_count,
                0,
                &mut max_fanout,
                &mut max_nesting_depth,
                &mut max_action_tickets,
                &mut max_parallel_in_flight,
                &mut max_gather_pages,
                &mut max_gather_items,
                &mut max_for_each_iterations,
                &mut max_together_branches,
                &mut max_repeat_attempts,
                &mut max_timer_entries,
            )?;

            let max_total_slots = u64::from(contract.max_slots);

            // Phase 0 executes at most one step per runtime tick, so steps bound time.
            let max_run_time_seconds = max_total_steps;

            Ok(Self {
                max_total_steps,
                max_total_slots,
                max_fanout,
                max_nesting_depth,
                max_steps_executable: match u32::try_from(max_total_steps) {
                    Ok(value) => value,
                    Err(_) => {
                        return Err(BudgetTraversalError::StepCountOverflow {
                            actual: max_total_steps,
                        });
                    }
                },
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
            })
        }
    }
}
