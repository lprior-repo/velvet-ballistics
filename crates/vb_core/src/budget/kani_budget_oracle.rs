#![forbid(unsafe_code)]

use crate::budget::{AggregateResourceBudget, AggregateResourceUsage};

fn expected_budget_sum(
    current: u64,
    requested: u64,
    resource: &'static str,
) -> Result<u64, &'static str> {
    current.checked_add(requested).ok_or(resource)
}

pub(super) fn expected_usage_after_budget(
    usage: &AggregateResourceUsage,
    budget: &AggregateResourceBudget,
) -> Result<AggregateResourceUsage, &'static str> {
    Ok(AggregateResourceUsage {
        max_steps_executable: expected_budget_sum(
            usage.max_steps_executable,
            u64::from(budget.max_steps_executable),
            "max_steps_executable",
        )?,
        max_action_tickets: expected_budget_sum(
            usage.max_action_tickets,
            u64::from(budget.max_action_tickets),
            "max_action_tickets",
        )?,
        max_parallel_in_flight: expected_budget_sum(
            usage.max_parallel_in_flight,
            u64::from(budget.max_parallel_in_flight),
            "max_parallel_in_flight",
        )?,
        max_gather_pages: expected_budget_sum(
            usage.max_gather_pages,
            u64::from(budget.max_gather_pages),
            "max_gather_pages",
        )?,
        max_gather_items: expected_budget_sum(
            usage.max_gather_items,
            u64::from(budget.max_gather_items),
            "max_gather_items",
        )?,
        max_result_bytes: expected_budget_sum(
            usage.max_result_bytes,
            u64::from(budget.max_result_bytes),
            "max_result_bytes",
        )?,
        max_total_slots_written: expected_budget_sum(
            usage.max_total_slots_written,
            u64::from(budget.max_total_slots_written),
            "max_total_slots_written",
        )?,
        max_timer_entries: expected_budget_sum(
            usage.max_timer_entries,
            u64::from(budget.max_timer_entries),
            "max_timer_entries",
        )?,
        max_trace_events: expected_budget_sum(
            usage.max_trace_events,
            budget.max_trace_events,
            "max_trace_events",
        )?,
        max_active_runs: expected_budget_sum(usage.max_active_runs, 1, "max_active_runs")?,
        max_queue_depth: expected_budget_sum(
            usage.max_queue_depth,
            u64::from(budget.max_queue_depth),
            "max_queue_depth",
        )?,
        max_journal_batch_bytes: expected_budget_sum(
            usage.max_journal_batch_bytes,
            u64::from(budget.max_journal_batch_bytes),
            "max_journal_batch_bytes",
        )?,
        max_ipc_payload_bytes: expected_budget_sum(
            usage.max_ipc_payload_bytes,
            u64::from(budget.max_ipc_payload_bytes),
            "max_ipc_payload_bytes",
        )?,
        max_blob_bytes: expected_budget_sum(
            usage.max_blob_bytes,
            budget.max_blob_bytes,
            "max_blob_bytes",
        )?,
        max_input_bytes: expected_budget_sum(
            usage.max_input_bytes,
            u64::from(budget.max_input_bytes),
            "max_input_bytes",
        )?,
        max_step_budget_per_tick: expected_budget_sum(
            usage.max_step_budget_per_tick,
            budget.max_step_budget_per_tick,
            "max_step_budget_per_tick",
        )?,
        max_transitions_per_tick: expected_budget_sum(
            usage.max_transitions_per_tick,
            budget.max_transitions_per_tick,
            "max_transitions_per_tick",
        )?,
        max_open_gates: expected_budget_sum(
            usage.max_open_gates,
            u64::from(budget.max_open_gates),
            "max_open_gates",
        )?,
        max_signal_subscriptions: expected_budget_sum(
            usage.max_signal_subscriptions,
            u64::from(budget.max_signal_subscriptions),
            "max_signal_subscriptions",
        )?,
        max_incident_emissions: expected_budget_sum(
            usage.max_incident_emissions,
            budget.max_incident_emissions,
            "max_incident_emissions",
        )?,
        max_transitions: expected_budget_sum(
            usage.max_transitions,
            budget.max_transitions,
            "max_transitions",
        )?,
        max_journal_bytes_total: expected_budget_sum(
            usage.max_journal_bytes_total,
            budget.max_journal_bytes_total,
            "max_journal_bytes_total",
        )?,
        max_value_store_entries: expected_budget_sum(
            usage.max_value_store_entries,
            budget.max_value_store_entries,
            "max_value_store_entries",
        )?,
        max_cost_units: expected_budget_sum(
            usage.max_cost_units,
            budget.max_cost_units,
            "max_cost_units",
        )?,
    })
}
