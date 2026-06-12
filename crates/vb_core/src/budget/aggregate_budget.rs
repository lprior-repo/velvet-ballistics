#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

use thiserror::Error;

use crate::ids::RunId;
use crate::workflow::{CompiledWorkflow, ResourceContract, WorkflowError};

use super::types::WholeWorkflowBudget;
use super::validation::validate_step_ceilings;

/// Aggregate whole-run budget required for runtime admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AggregateResourceBudget {
    pub max_steps_executable: u32,
    pub max_action_tickets: u32,
    pub max_parallel_in_flight: u16,
    pub max_retries_per_action: u16,
    pub max_gather_pages: u32,
    pub max_gather_items: u32,
    pub max_for_each_iterations: u32,
    pub max_together_branches: u16,
    pub max_repeat_attempts: u16,
    pub max_run_time_seconds: u64,
    pub max_result_bytes: u32,
    pub max_total_slots_written: u32,
    pub max_timer_entries: u32,
    pub max_trace_events: u64,
    pub max_queue_depth: u32,
    pub max_journal_batch_bytes: u32,
    pub max_ipc_payload_bytes: u32,
    pub max_blob_bytes: u64,
    pub max_input_bytes: u32,
    /// Maximum step budget per runtime tick (from ResourceContract).
    pub max_step_budget_per_tick: u64,
    /// Maximum transitions per runtime tick.
    pub max_transitions_per_tick: u64,
}

/// Shard-local aggregate admission capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateResourceCapacity {
    pub max_steps_executable: u64,
    pub max_action_tickets: u64,
    pub max_parallel_in_flight: u32,
    pub max_gather_pages: u64,
    pub max_gather_items: u64,
    pub max_result_bytes: u64,
    pub max_total_slots_written: u64,
    pub max_timer_entries: u64,
    pub max_trace_events: u64,
    pub max_active_runs: u64,
    pub max_queue_depth: u64,
    pub max_journal_batch_bytes: u64,
    pub max_ipc_payload_bytes: u64,
    pub max_blob_bytes: u64,
    pub max_input_bytes: u64,
    /// Maximum step budget per tick capacity.
    pub max_step_budget_per_tick: u64,
    /// Maximum transitions per tick capacity.
    pub max_transitions_per_tick: u64,
}

/// Exact budget reservation associated with a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateReservation {
    pub run: RunId,
    pub requested: AggregateResourceBudget,
}

/// Aggregate resource-accounting failure.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AggregateBudgetError {
    /// Workflow budget validation failed.
    #[cfg(not(kani))]
    #[error("workflow budget error: {0}")]
    WorkflowBudget(#[source] WorkflowError),
    /// Workflow budget validation failed (Kani stub).
    #[cfg(kani)]
    #[error("workflow budget error")]
    WorkflowBudget,
    /// A policy-defined resource limit was exceeded.
    #[error("policy exceeded: {resource} {actual} > {limit}")]
    PolicyExceeded {
        /// Resource name.
        resource: &'static str,
        /// Actual value.
        actual: u64,
        /// Policy limit.
        limit: u64,
    },
    /// Requested capacity exceeds available.
    #[error("capacity exceeded: {resource} requested {requested}, available {available}")]
    CapacityExceeded {
        /// Resource name.
        resource: &'static str,
        /// Requested amount.
        requested: u64,
        /// Available amount.
        available: u64,
    },
    /// Arithmetic overflow.
    #[error("overflow: {resource}")]
    Overflow {
        /// Resource name.
        resource: &'static str,
    },
    /// Arithmetic underflow.
    #[error("underflow: {resource}")]
    Underflow {
        /// Resource name.
        resource: &'static str,
    },
    /// Invalid capacity configuration.
    #[error("invalid capacity: {resource}")]
    InvalidCapacity {
        /// Resource name.
        resource: &'static str,
    },
    /// Reservation not found.
    #[error("reservation not found: run {run:?}")]
    ReservationNotFound {
        /// Run identifier.
        run: RunId,
    },
    /// Step ceiling exceeded per tick.
    #[error("step ceiling exceeded: {requested} > {limit}")]
    StepCeilingExceeded {
        /// Requested steps.
        requested: u64,
        /// Tick limit.
        limit: u64,
    },
    /// Per-tick transition ceiling exceeded.
    #[error("per-tick ceiling exceeded: {requested} > {limit}")]
    PerTickCeilingExceeded {
        /// Requested transitions.
        requested: u64,
        /// Tick limit.
        limit: u64,
    },
}

#[cfg(kani)]
impl Drop for AggregateBudgetError {
    fn drop(&mut self) {}
}

impl AggregateResourceBudget {
    pub fn from_workflow(workflow: &CompiledWorkflow) -> Result<Self, AggregateBudgetError> {
        let parts = workflow.to_parts();
        let budget = WholeWorkflowBudget::compute(
            &parts.nodes,
            workflow.entry(),
            &workflow.resource_contract(),
        )
        .map_err(map_workflow_budget_error)?;
        let aggregate = Self::from_whole_workflow_budget(budget, workflow.resource_contract())?;
        validate_step_ceilings(&aggregate)?;
        Ok(aggregate)
    }

    pub fn from_whole_workflow_budget(
        budget: WholeWorkflowBudget,
        contract: ResourceContract,
    ) -> Result<Self, AggregateBudgetError> {
        Ok(Self {
            max_steps_executable: budget.max_steps_executable,
            max_action_tickets: budget.max_action_tickets,
            max_parallel_in_flight: budget.max_parallel_in_flight,
            max_retries_per_action: budget.max_retries_per_action,
            max_gather_pages: budget.max_gather_pages,
            max_gather_items: budget.max_gather_items,
            max_for_each_iterations: budget.max_for_each_iterations,
            max_together_branches: budget.max_together_branches,
            max_repeat_attempts: budget.max_repeat_attempts,
            max_run_time_seconds: budget.max_run_time_seconds,
            max_result_bytes: budget.max_result_bytes,
            max_total_slots_written: budget.max_total_slots_written,
            max_timer_entries: budget.max_timer_entries,
            max_trace_events: budget.max_trace_events,
            max_queue_depth: budget.max_queue_depth,
            max_journal_batch_bytes: budget.max_journal_batch_bytes,
            max_ipc_payload_bytes: budget.max_ipc_payload_bytes,
            max_blob_bytes: budget.max_blob_bytes,
            max_input_bytes: budget.max_input_bytes,
            max_step_budget_per_tick: contract.max_step_budget_per_tick,
            max_transitions_per_tick: contract.max_transitions_per_tick,
        })
    }
}

#[cfg(not(kani))]
pub(super) fn map_workflow_budget_error(error: WorkflowError) -> AggregateBudgetError {
    AggregateBudgetError::WorkflowBudget(error)
}

#[cfg(kani)]
pub(super) fn map_workflow_budget_error(_error: WorkflowError) -> AggregateBudgetError {
    AggregateBudgetError::WorkflowBudget
}
