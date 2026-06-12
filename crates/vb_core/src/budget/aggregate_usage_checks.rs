#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

use super::aggregate_budget::{AggregateBudgetError, AggregateResourceCapacity};
use super::aggregate_usage::AggregateResourceUsage;
use super::policy::BoundednessPolicy;
use super::validation::{check_capacity, check_policy};

impl AggregateResourceUsage {
    pub fn fits_within(
        &self,
        capacity: &AggregateResourceCapacity,
    ) -> Result<(), AggregateBudgetError> {
        check_capacity(
            "max_steps_executable",
            self.max_steps_executable,
            capacity.max_steps_executable,
        )?;
        check_capacity(
            "max_action_tickets",
            self.max_action_tickets,
            capacity.max_action_tickets,
        )?;
        check_capacity(
            "max_parallel_in_flight",
            self.max_parallel_in_flight,
            u64::from(capacity.max_parallel_in_flight),
        )?;
        check_capacity(
            "max_gather_pages",
            self.max_gather_pages,
            capacity.max_gather_pages,
        )?;
        check_capacity(
            "max_gather_items",
            self.max_gather_items,
            capacity.max_gather_items,
        )?;
        check_capacity(
            "max_result_bytes",
            self.max_result_bytes,
            capacity.max_result_bytes,
        )?;
        check_capacity(
            "max_total_slots_written",
            self.max_total_slots_written,
            capacity.max_total_slots_written,
        )?;
        check_capacity(
            "max_timer_entries",
            self.max_timer_entries,
            capacity.max_timer_entries,
        )?;
        check_capacity(
            "max_trace_events",
            self.max_trace_events,
            capacity.max_trace_events,
        )?;
        check_capacity(
            "max_active_runs",
            self.max_active_runs,
            capacity.max_active_runs,
        )?;
        check_capacity(
            "max_queue_depth",
            self.max_queue_depth,
            capacity.max_queue_depth,
        )?;
        check_capacity(
            "max_journal_batch_bytes",
            self.max_journal_batch_bytes,
            capacity.max_journal_batch_bytes,
        )?;
        check_capacity(
            "max_ipc_payload_bytes",
            self.max_ipc_payload_bytes,
            capacity.max_ipc_payload_bytes,
        )?;
        check_capacity(
            "max_blob_bytes",
            self.max_blob_bytes,
            capacity.max_blob_bytes,
        )?;
        check_capacity(
            "max_input_bytes",
            self.max_input_bytes,
            capacity.max_input_bytes,
        )?;
        check_capacity(
            "max_step_budget_per_tick",
            self.max_step_budget_per_tick,
            capacity.max_step_budget_per_tick,
        )?;
        check_capacity(
            "max_transitions_per_tick",
            self.max_transitions_per_tick,
            capacity.max_transitions_per_tick,
        )
    }

    /// Checks if this usage satisfies a boundedness policy.
    /// Returns `Ok(())` if all usage dimensions are within policy limits,
    /// or `Err(AggregateBudgetError::PolicyExceeded)` if any dimension exceeds.
    pub fn check_policy(&self, policy: &BoundednessPolicy) -> Result<(), AggregateBudgetError> {
        check_policy(
            "max_steps_executable",
            self.max_steps_executable,
            u64::from(policy.absolute_max_steps_executable),
        )?;
        check_policy(
            "max_action_tickets",
            self.max_action_tickets,
            u64::from(policy.absolute_max_action_tickets),
        )?;
        check_policy(
            "max_parallel_in_flight",
            self.max_parallel_in_flight,
            u64::from(policy.absolute_max_parallel),
        )?;
        check_policy(
            "max_result_bytes",
            self.max_result_bytes,
            u64::from(policy.absolute_max_result_bytes),
        )?;
        check_policy(
            "max_timer_entries",
            self.max_timer_entries,
            u64::from(policy.absolute_max_timer_entries),
        )?;
        check_policy(
            "max_trace_events",
            self.max_trace_events,
            policy.absolute_max_trace_events,
        )?;
        check_policy(
            "max_journal_batch_bytes",
            self.max_journal_batch_bytes,
            u64::from(policy.absolute_max_journal_batch_bytes),
        )?;
        check_policy(
            "max_queue_depth",
            self.max_queue_depth,
            u64::from(policy.absolute_max_queue_depth),
        )?;
        check_policy(
            "max_ipc_payload_bytes",
            self.max_ipc_payload_bytes,
            u64::from(policy.absolute_max_ipc_payload_bytes),
        )?;
        check_policy(
            "max_blob_bytes",
            self.max_blob_bytes,
            policy.absolute_max_blob_bytes,
        )?;
        check_policy(
            "max_input_bytes",
            self.max_input_bytes,
            u64::from(policy.absolute_max_input_bytes),
        )
    }
}
