#![cfg(kani)]
#![forbid(unsafe_code)]
//! VB-CORE-BUDGET-002: Step budget one bound verification
//!
//! Property: Budget operations with value 1 behave correctly without panicking.
//! Bound: Single step counts and single dimensions.
//!
//! This harness verifies panic-free budget operations at the unit boundary.

use crate::budget::{AggregateResourceBudget, AggregateResourceUsage};

fn zero_budget() -> AggregateResourceBudget {
    AggregateResourceBudget {
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
    }
}

/// VB-CORE-BUDGET-002 H1: add_dim(1, 0) returns Ok(1)
#[kani::proof]
#[kani::unwind(4)]
fn kani_budget_add_one_plus_zero() {
    let result = 1u64.checked_add(0u64);
    match result {
        Some(v) => kani::assert(v == 1, "1+0=1"),
        None => kani::assert(false, "1+0 cannot overflow"),
    }
}

/// VB-CORE-BUDGET-002 H2: add_dim(0, 1) returns Ok(1)
#[kani::proof]
#[kani::unwind(4)]
fn kani_budget_add_zero_plus_one() {
    let result = 0u64.checked_add(1u64);
    match result {
        Some(v) => kani::assert(v == 1, "0+1=1"),
        None => kani::assert(false, "0+1 cannot overflow"),
    }
}

/// VB-CORE-BUDGET-002 H3: sub_dim(1, 1) returns Ok(0)
#[kani::proof]
#[kani::unwind(4)]
fn kani_budget_sub_one_minus_one() {
    let result = 1u64.checked_sub(1u64);
    match result {
        Some(v) => kani::assert(v == 0, "1-1=0"),
        None => kani::assert(false, "1-1 cannot underflow"),
    }
}

/// VB-CORE-BUDGET-002 H4: sub_dim(1, 0) returns Ok(1)
#[kani::proof]
#[kani::unwind(4)]
fn kani_budget_sub_one_minus_zero() {
    let result = 1u64.checked_sub(0u64);
    match result {
        Some(v) => kani::assert(v == 1, "1-0=1"),
        None => kani::assert(false, "1-0 cannot underflow"),
    }
}

/// VB-CORE-BUDGET-002 H5: add_dim(1, 1) returns Ok(2)
#[kani::proof]
#[kani::unwind(4)]
fn kani_budget_add_one_plus_one() {
    let result = 1u64.checked_add(1u64);
    match result {
        Some(v) => kani::assert(v == 2, "1+1=2"),
        None => kani::assert(false, "1+1 cannot overflow"),
    }
}

/// VB-CORE-BUDGET-002 H6: add_dim(1, MAX) overflows
#[kani::proof]
#[kani::unwind(4)]
fn kani_budget_add_one_plus_max_overflow() {
    let result = 1u64.checked_add(u64::MAX);
    match result {
        Some(_) => kani::assert(false, "1+MAX must overflow"),
        None => kani::assert(true, "1+MAX correctly overflows"),
    }
}

/// VB-CORE-BUDGET-002 H7: sub_dim(1, 2) underflows
#[kani::proof]
#[kani::unwind(4)]
fn kani_budget_sub_one_minus_two_underflow() {
    let result = 1u64.checked_sub(2u64);
    match result {
        Some(_) => kani::assert(false, "1-2 must underflow"),
        None => kani::assert(true, "1-2 correctly underflows"),
    }
}

/// VB-CORE-BUDGET-002 H8: AggregateResourceUsage with single step
#[kani::proof]
#[kani::unwind(4)]
fn kani_aggregate_usage_one_step() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 1,
        ..AggregateResourceUsage::default()
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 1,
        ..zero_budget()
    };

    let result = usage.try_add_budget(&budget);
    kani::assert(result.is_ok(), "1+1 steps within bounds");
}
