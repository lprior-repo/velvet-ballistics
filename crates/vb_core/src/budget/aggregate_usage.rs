#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

use super::aggregate_budget::{AggregateBudgetError, AggregateResourceBudget};
use super::validation::{add_dim, sub_dim};

/// Active shard aggregate usage snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AggregateResourceUsage {
    pub max_steps_executable: u64,
    pub max_action_tickets: u64,
    pub max_parallel_in_flight: u64,
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
    /// Current step budget per tick usage.
    pub max_step_budget_per_tick: u64,
    /// Current transitions per tick usage.
    pub max_transitions_per_tick: u64,
}

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
        })
    }
}
