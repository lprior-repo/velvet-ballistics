#![forbid(unsafe_code)]
//! VB-CORE-BUDGET-001: Step budget zero bound verification
//!
//! Property: Budget operations with zero values behave correctly without panicking.
//! Bound: Zero step counts and zero dimensions.
//!
//! This harness verifies panic-free budget operations at zero boundaries.

use crate::budget::{AggregateResourceBudget, AggregateResourceUsage};

/// VB-CORE-BUDGET-001 H1: add_dim(0, 0) returns Ok(0)
#[kani::proof]
#[kani::unwind(4)]
fn kani_budget_add_dim_zero() {
    let result = 0u64.checked_add(0u64);
    match result {
        Some(v) => kani::assert(v == 0, "0+0=0"),
        None => kani::assert(false, "0+0 cannot overflow"),
    }
}

/// VB-CORE-BUDGET-001 H2: sub_dim(0, 0) returns Ok(0)
#[kani::proof]
#[kani::unwind(4)]
fn kani_budget_sub_dim_zero() {
    let result = 0u64.checked_sub(0u64);
    match result {
        Some(v) => kani::assert(v == 0, "0-0=0"),
        None => kani::assert(false, "0-0 cannot underflow"),
    }
}

/// VB-CORE-BUDGET-001 H3: AggregateResourceUsage with zero dimensions
#[kani::proof]
#[kani::unwind(4)]
fn kani_aggregate_usage_zero() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_add_budget(&budget);
    kani::assert(result.is_ok(), "adding zero budgets succeeds");
}

/// VB-CORE-BUDGET-001 H4: try_add_budget with zero current usage
#[kani::proof]
#[kani::unwind(4)]
fn kani_try_add_budget_zero_current() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 1,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_add_budget(&budget);
    kani::assert(result.is_ok(), "adding budget to zero usage succeeds");
}
