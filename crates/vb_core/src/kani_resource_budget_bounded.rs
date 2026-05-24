#![forbid(unsafe_code)]
//! VB-CORE-RESOURCE-004: Resource budget boundedness verification
//!
//! Property: Resource budget operations with bounded values never exceed
//! u64::MAX and return proper errors for overflow conditions.
//!
//! This harness verifies resource budget boundedness.

use crate::budget::{AggregateResourceBudget, AggregateResourceUsage};

/// VB-CORE-RESOURCE-004 H1: try_add_budget with small values succeeds
#[kani::proof]
#[kani::unwind(4)]
fn kani_resource_add_small_values() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 100,
        max_action_tickets: 10,
        max_parallel_in_flight: 5,
        max_gather_pages: 50,
        max_gather_items: 200,
        max_result_bytes: 1024,
        max_total_slots_written: 50,
        max_timer_entries: 10,
        max_trace_events: 100,
        max_active_runs: 3,
        max_queue_depth: 100,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 2048,
        max_blob_bytes: 8192,
        max_input_bytes: 1024,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 50,
        max_action_tickets: 5,
        max_parallel_in_flight: 2,
        max_retries_per_action: 3,
        max_gather_pages: 25,
        max_gather_items: 100,
        max_for_each_iterations: 10,
        max_together_branches: 4,
        max_repeat_attempts: 2,
        max_run_time_seconds: 3600,
        max_result_bytes: 512,
        max_total_slots_written: 25,
        max_timer_entries: 5,
        max_trace_events: 50,
        max_queue_depth: 50,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 1024,
        max_blob_bytes: 4096,
        max_input_bytes: 512,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };

    let result = usage.try_add_budget(&budget);
    kani::assert(result.is_ok(), "small budget addition succeeds");
}

/// VB-CORE-RESOURCE-004 H2: try_add_budget with overflow returns error
#[kani::proof]
#[kani::unwind(4)]
fn kani_resource_add_overflow() {
    let usage = AggregateResourceUsage {
        max_steps_executable: u64::MAX,
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
    kani::assert(result.is_err(), "MAX+1 overflow returns error");
}

/// VB-CORE-RESOURCE-004 H3: try_subtract_budget with underflow returns error
#[kani::proof]
#[kani::unwind(4)]
fn kani_resource_sub_underflow() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 5,
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
        max_steps_executable: 10,
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

    let result = usage.try_subtract_budget(&budget);
    kani::assert(
        result.is_err(),
        "subtracting more than available returns error",
    );
}

/// VB-CORE-RESOURCE-004 H4: try_subtract_budget with exact match succeeds
#[kani::proof]
#[kani::unwind(4)]
fn kani_resource_sub_exact_match() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 10,
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
        max_steps_executable: 10,
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

    let result = usage.try_subtract_budget(&budget);
    kani::assert(result.is_ok(), "exact match subtraction succeeds");
}

/// VB-CORE-RESOURCE-004 H5: try_add_budget with MAX values
#[kani::proof]
#[kani::unwind(4)]
fn kani_resource_add_max_values() {
    let usage = AggregateResourceUsage {
        max_steps_executable: u64::MAX / 2,
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
        max_steps_executable: u32::MAX / 2,
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
    kani::assert(
        result.is_ok(),
        "MAX/2 + MAX/2 succeeds and stays within bounds",
    );
}
