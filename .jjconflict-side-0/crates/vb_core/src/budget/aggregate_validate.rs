#![forbid(unsafe_code)]
//! Aggregate budget construction and policy validation: whole-workflow
//! conversion, tenant and global quota checks, and boundedness checks.

use super::{
    AggregateBudgetError, AggregateResourceBudget, BoundednessPolicy, WholeWorkflowBudget,
    check_capacity, check_policy, validate_step_ceilings,
};
use crate::workflow::{CompiledWorkflow, ResourceContract, WorkflowError};

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
            max_open_gates: budget.max_open_gates,
            max_signal_subscriptions: budget.max_signal_subscriptions,
            max_incident_emissions: budget.max_incident_emissions,
            max_transitions: budget.max_transitions,
            max_journal_bytes_total: budget.max_journal_bytes_total,
            max_value_store_entries: budget.max_value_store_entries,
            max_cost_units: budget.max_cost_units,
        })
    }
}

#[cfg(not(kani))]
fn map_workflow_budget_error(error: WorkflowError) -> AggregateBudgetError {
    AggregateBudgetError::WorkflowBudget(error)
}

#[cfg(kani)]
fn map_workflow_budget_error(_error: WorkflowError) -> AggregateBudgetError {
    AggregateBudgetError::WorkflowBudget
}

/// Checks a tenant-scoped cost quota: the tenant's already-admitted cost
/// plus the requested run's cost must not exceed the tenant quota.
/// This is the explicit-sum path: callers that hold a [`ShardUsageSnapshot`]
/// should sum from it with [`ShardUsageSnapshot::tenant_cost_usage`]
/// instead of passing a sum computed elsewhere.
pub fn check_tenant_quota(
    tenant_cost_usage: u64,
    requested_cost: u64,
    tenant_quota: u64,
) -> Result<(), AggregateBudgetError> {
    let total =
        tenant_cost_usage
            .checked_add(requested_cost)
            .ok_or(AggregateBudgetError::Overflow {
                resource: "tenant_quota_cost_units",
            })?;
    check_capacity("tenant_quota_cost_units", total, tenant_quota)
}

/// Checks a shard-wide cost quota: the shard's already-admitted cost
/// plus the requested run's cost must not exceed the global quota.
/// The shard usage snapshot carries no summed global cost, so
/// global-scoped admission passes the shard's summed cost explicitly.
pub fn check_global_quota(
    global_cost_usage: u64,
    requested_cost: u64,
    global_quota: u64,
) -> Result<(), AggregateBudgetError> {
    let total =
        global_cost_usage
            .checked_add(requested_cost)
            .ok_or(AggregateBudgetError::Overflow {
                resource: "global_quota_cost_units",
            })?;
    check_capacity("global_quota_cost_units", total, global_quota)
}

pub fn validate_aggregate_budget(
    budget: &AggregateResourceBudget,
    policy: &BoundednessPolicy,
) -> Result<(), AggregateBudgetError> {
    check_policy(
        "max_steps_executable",
        u64::from(budget.max_steps_executable),
        u64::from(policy.absolute_max_steps_executable),
    )?;
    check_policy(
        "max_action_tickets",
        u64::from(budget.max_action_tickets),
        u64::from(policy.absolute_max_action_tickets),
    )?;
    check_policy(
        "max_parallel_in_flight",
        u64::from(budget.max_parallel_in_flight),
        u64::from(policy.absolute_max_parallel),
    )?;
    check_policy(
        "max_retries_per_action",
        u64::from(budget.max_retries_per_action),
        u64::from(u16::MAX),
    )?;
    check_policy(
        "max_gather_pages",
        u64::from(budget.max_gather_pages),
        u64::from(u32::MAX),
    )?;
    check_policy(
        "max_gather_items",
        u64::from(budget.max_gather_items),
        u64::from(u32::MAX),
    )?;
    check_policy(
        "max_for_each_iterations",
        u64::from(budget.max_for_each_iterations),
        u64::from(u32::MAX),
    )?;
    check_policy(
        "max_together_branches",
        u64::from(budget.max_together_branches),
        u64::from(policy.max_fanout),
    )?;
    check_policy(
        "max_repeat_attempts",
        u64::from(budget.max_repeat_attempts),
        u64::from(u16::MAX),
    )?;
    check_policy(
        "max_run_time_seconds",
        budget.max_run_time_seconds,
        policy.absolute_max_run_time_seconds,
    )?;
    check_policy(
        "max_result_bytes",
        u64::from(budget.max_result_bytes),
        u64::from(policy.absolute_max_result_bytes),
    )?;
    check_policy(
        "max_total_slots_written",
        u64::from(budget.max_total_slots_written),
        policy.max_total_slots,
    )?;
    check_policy(
        "max_timer_entries",
        u64::from(budget.max_timer_entries),
        u64::from(policy.absolute_max_timer_entries),
    )?;
    check_policy(
        "max_trace_events",
        budget.max_trace_events,
        policy.absolute_max_trace_events,
    )?;
    check_policy(
        "max_queue_depth",
        u64::from(budget.max_queue_depth),
        u64::from(policy.absolute_max_queue_depth),
    )?;
    check_policy(
        "max_journal_batch_bytes",
        u64::from(budget.max_journal_batch_bytes),
        u64::from(policy.absolute_max_journal_batch_bytes),
    )?;
    check_policy(
        "max_ipc_payload_bytes",
        u64::from(budget.max_ipc_payload_bytes),
        u64::from(policy.absolute_max_ipc_payload_bytes),
    )?;
    check_policy(
        "max_blob_bytes",
        budget.max_blob_bytes,
        policy.absolute_max_blob_bytes,
    )?;
    check_policy(
        "max_input_bytes",
        u64::from(budget.max_input_bytes),
        u64::from(policy.absolute_max_input_bytes),
    )?;
    check_policy(
        "max_open_gates",
        u64::from(budget.max_open_gates),
        u64::from(policy.absolute_max_open_gates),
    )?;
    check_policy(
        "max_signal_subscriptions",
        u64::from(budget.max_signal_subscriptions),
        u64::from(policy.absolute_max_signal_subscriptions),
    )?;
    check_policy(
        "max_incident_emissions",
        budget.max_incident_emissions,
        policy.absolute_max_incident_emissions,
    )?;
    check_policy(
        "max_transitions",
        budget.max_transitions,
        policy.absolute_max_transitions,
    )?;
    check_policy(
        "max_journal_bytes_total",
        budget.max_journal_bytes_total,
        policy.absolute_max_journal_bytes_total,
    )?;
    check_policy(
        "max_value_store_entries",
        budget.max_value_store_entries,
        policy.absolute_max_value_store_entries,
    )?;
    check_policy(
        "max_cost_units",
        budget.max_cost_units,
        policy.absolute_max_cost_units,
    )
}
