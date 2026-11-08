#![forbid(unsafe_code)]
//! Aggregate resource usage arithmetic: checked add and subtract,
//! capacity fit, and boundedness-policy check.

use super::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceCapacity,
    AggregateResourceUsage, BoundednessPolicy, add_dim, check_capacity, check_policy, sub_dim,
};

impl AggregateResourceUsage {
    pub fn try_add_budget(
        &self,
        budget: &AggregateResourceBudget,
    ) -> Result<Self, AggregateBudgetError> {
        Ok(Self {
            max_steps_executable: add_dim(
                self.max_steps_executable,
                u64::from(budget.max_steps_executable),
                "max_steps_executable",
            )?,
            max_action_tickets: add_dim(
                self.max_action_tickets,
                u64::from(budget.max_action_tickets),
                "max_action_tickets",
            )?,
            max_parallel_in_flight: add_dim(
                self.max_parallel_in_flight,
                u64::from(budget.max_parallel_in_flight),
                "max_parallel_in_flight",
            )?,
            max_gather_pages: add_dim(
                self.max_gather_pages,
                u64::from(budget.max_gather_pages),
                "max_gather_pages",
            )?,
            max_gather_items: add_dim(
                self.max_gather_items,
                u64::from(budget.max_gather_items),
                "max_gather_items",
            )?,
            max_result_bytes: add_dim(
                self.max_result_bytes,
                u64::from(budget.max_result_bytes),
                "max_result_bytes",
            )?,
            max_total_slots_written: add_dim(
                self.max_total_slots_written,
                u64::from(budget.max_total_slots_written),
                "max_total_slots_written",
            )?,
            max_timer_entries: add_dim(
                self.max_timer_entries,
                u64::from(budget.max_timer_entries),
                "max_timer_entries",
            )?,
            max_trace_events: add_dim(
                self.max_trace_events,
                budget.max_trace_events,
                "max_trace_events",
            )?,
            max_active_runs: add_dim(self.max_active_runs, 1, "max_active_runs")?,
            max_queue_depth: add_dim(
                self.max_queue_depth,
                u64::from(budget.max_queue_depth),
                "max_queue_depth",
            )?,
            max_journal_batch_bytes: add_dim(
                self.max_journal_batch_bytes,
                u64::from(budget.max_journal_batch_bytes),
                "max_journal_batch_bytes",
            )?,
            max_ipc_payload_bytes: add_dim(
                self.max_ipc_payload_bytes,
                u64::from(budget.max_ipc_payload_bytes),
                "max_ipc_payload_bytes",
            )?,
            max_blob_bytes: add_dim(self.max_blob_bytes, budget.max_blob_bytes, "max_blob_bytes")?,
            max_input_bytes: add_dim(
                self.max_input_bytes,
                u64::from(budget.max_input_bytes),
                "max_input_bytes",
            )?,
            max_step_budget_per_tick: add_dim(
                self.max_step_budget_per_tick,
                budget.max_step_budget_per_tick,
                "max_step_budget_per_tick",
            )?,
            max_transitions_per_tick: add_dim(
                self.max_transitions_per_tick,
                budget.max_transitions_per_tick,
                "max_transitions_per_tick",
            )?,
            max_open_gates: add_dim(
                self.max_open_gates,
                u64::from(budget.max_open_gates),
                "max_open_gates",
            )?,
            max_signal_subscriptions: add_dim(
                self.max_signal_subscriptions,
                u64::from(budget.max_signal_subscriptions),
                "max_signal_subscriptions",
            )?,
            max_incident_emissions: add_dim(
                self.max_incident_emissions,
                budget.max_incident_emissions,
                "max_incident_emissions",
            )?,
            max_transitions: add_dim(
                self.max_transitions,
                budget.max_transitions,
                "max_transitions",
            )?,
            max_journal_bytes_total: add_dim(
                self.max_journal_bytes_total,
                budget.max_journal_bytes_total,
                "max_journal_bytes_total",
            )?,
            max_value_store_entries: add_dim(
                self.max_value_store_entries,
                budget.max_value_store_entries,
                "max_value_store_entries",
            )?,
            max_cost_units: add_dim(self.max_cost_units, budget.max_cost_units, "max_cost_units")?,
        })
    }

    pub fn try_subtract_budget(
        &self,
        budget: &AggregateResourceBudget,
    ) -> Result<Self, AggregateBudgetError> {
        Ok(Self {
            max_steps_executable: sub_dim(
                self.max_steps_executable,
                u64::from(budget.max_steps_executable),
                "max_steps_executable",
            )?,
            max_action_tickets: sub_dim(
                self.max_action_tickets,
                u64::from(budget.max_action_tickets),
                "max_action_tickets",
            )?,
            max_parallel_in_flight: sub_dim(
                self.max_parallel_in_flight,
                u64::from(budget.max_parallel_in_flight),
                "max_parallel_in_flight",
            )?,
            max_gather_pages: sub_dim(
                self.max_gather_pages,
                u64::from(budget.max_gather_pages),
                "max_gather_pages",
            )?,
            max_gather_items: sub_dim(
                self.max_gather_items,
                u64::from(budget.max_gather_items),
                "max_gather_items",
            )?,
            max_result_bytes: sub_dim(
                self.max_result_bytes,
                u64::from(budget.max_result_bytes),
                "max_result_bytes",
            )?,
            max_total_slots_written: sub_dim(
                self.max_total_slots_written,
                u64::from(budget.max_total_slots_written),
                "max_total_slots_written",
            )?,
            max_timer_entries: sub_dim(
                self.max_timer_entries,
                u64::from(budget.max_timer_entries),
                "max_timer_entries",
            )?,
            max_trace_events: sub_dim(
                self.max_trace_events,
                budget.max_trace_events,
                "max_trace_events",
            )?,
            max_active_runs: sub_dim(self.max_active_runs, 1, "max_active_runs")?,
            max_queue_depth: sub_dim(
                self.max_queue_depth,
                u64::from(budget.max_queue_depth),
                "max_queue_depth",
            )?,
            max_journal_batch_bytes: sub_dim(
                self.max_journal_batch_bytes,
                u64::from(budget.max_journal_batch_bytes),
                "max_journal_batch_bytes",
            )?,
            max_ipc_payload_bytes: sub_dim(
                self.max_ipc_payload_bytes,
                u64::from(budget.max_ipc_payload_bytes),
                "max_ipc_payload_bytes",
            )?,
            max_blob_bytes: sub_dim(self.max_blob_bytes, budget.max_blob_bytes, "max_blob_bytes")?,
            max_input_bytes: sub_dim(
                self.max_input_bytes,
                u64::from(budget.max_input_bytes),
                "max_input_bytes",
            )?,
            max_step_budget_per_tick: sub_dim(
                self.max_step_budget_per_tick,
                budget.max_step_budget_per_tick,
                "max_step_budget_per_tick",
            )?,
            max_transitions_per_tick: sub_dim(
                self.max_transitions_per_tick,
                budget.max_transitions_per_tick,
                "max_transitions_per_tick",
            )?,
            max_open_gates: sub_dim(
                self.max_open_gates,
                u64::from(budget.max_open_gates),
                "max_open_gates",
            )?,
            max_signal_subscriptions: sub_dim(
                self.max_signal_subscriptions,
                u64::from(budget.max_signal_subscriptions),
                "max_signal_subscriptions",
            )?,
            max_incident_emissions: sub_dim(
                self.max_incident_emissions,
                budget.max_incident_emissions,
                "max_incident_emissions",
            )?,
            max_transitions: sub_dim(
                self.max_transitions,
                budget.max_transitions,
                "max_transitions",
            )?,
            max_journal_bytes_total: sub_dim(
                self.max_journal_bytes_total,
                budget.max_journal_bytes_total,
                "max_journal_bytes_total",
            )?,
            max_value_store_entries: sub_dim(
                self.max_value_store_entries,
                budget.max_value_store_entries,
                "max_value_store_entries",
            )?,
            max_cost_units: sub_dim(self.max_cost_units, budget.max_cost_units, "max_cost_units")?,
        })
    }

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
        )?;
        check_capacity(
            "max_open_gates",
            self.max_open_gates,
            capacity.max_open_gates,
        )?;
        check_capacity(
            "max_signal_subscriptions",
            self.max_signal_subscriptions,
            capacity.max_signal_subscriptions,
        )?;
        check_capacity(
            "max_incident_emissions",
            self.max_incident_emissions,
            capacity.max_incident_emissions,
        )?;
        check_capacity(
            "max_transitions",
            self.max_transitions,
            capacity.max_transitions,
        )?;
        check_capacity(
            "max_journal_bytes_total",
            self.max_journal_bytes_total,
            capacity.max_journal_bytes_total,
        )?;
        check_capacity(
            "max_value_store_entries",
            self.max_value_store_entries,
            capacity.max_value_store_entries,
        )?;
        check_capacity(
            "max_cost_units",
            self.max_cost_units,
            capacity.max_cost_units,
        )?;
        check_capacity(
            "global_quota_cost_units",
            self.max_cost_units,
            capacity.global_quota_cost_units,
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
        )?;
        check_policy(
            "max_open_gates",
            self.max_open_gates,
            u64::from(policy.absolute_max_open_gates),
        )?;
        check_policy(
            "max_signal_subscriptions",
            self.max_signal_subscriptions,
            u64::from(policy.absolute_max_signal_subscriptions),
        )?;
        check_policy(
            "max_incident_emissions",
            self.max_incident_emissions,
            policy.absolute_max_incident_emissions,
        )?;
        check_policy(
            "max_transitions",
            self.max_transitions,
            policy.absolute_max_transitions,
        )?;
        check_policy(
            "max_journal_bytes_total",
            self.max_journal_bytes_total,
            policy.absolute_max_journal_bytes_total,
        )?;
        check_policy(
            "max_value_store_entries",
            self.max_value_store_entries,
            policy.absolute_max_value_store_entries,
        )?;
        check_policy(
            "max_cost_units",
            self.max_cost_units,
            policy.absolute_max_cost_units,
        )
    }
}
