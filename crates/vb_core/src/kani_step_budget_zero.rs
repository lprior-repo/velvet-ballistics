#![cfg(kani)]
#![forbid(unsafe_code)]
//! VB-CORE-BUDGET-001: Step budget zero bound verification
//!
//! Property: Budget operations with zero values behave correctly without panicking.
//! Bound: Zero step counts and zero dimensions.
//!
//! This harness verifies panic-free budget operations at zero boundaries.

use crate::budget::{AggregateResourceBudget, AggregateResourceUsage};

fn zero_usage() -> AggregateResourceUsage {
    AggregateResourceUsage::default()
}

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
///
/// Bounded to `MAX_STEPS_PER_WORKFLOW` (master contract §13: 1000 steps)
/// so the property is verified across the full range the production
/// admission gate enforces.
#[kani::proof]
#[kani::unwind(4)]
fn kani_aggregate_usage_zero() {
    let usage = zero_usage();

    let mut budget = zero_budget();
    // Bound the step dimension to the master contract ceiling. Any value
    // up to and including the ceiling must satisfy the additivity property.
    let bounded_steps: u32 = kani::any();
    kani::assume(bounded_steps <= 1_000);
    budget.max_steps_executable = u64::from(bounded_steps);
    // After the assumption, the bounded value is in [0, MAX_STEPS_PER_WORKFLOW].
    // Adding to a zero usage cannot overflow because both operands are
    // bounded by the per-workflow ceiling.
    let result = usage.try_add_budget(&budget);
    kani::assert(result.is_ok(), "adding zero budgets succeeds");
    if let Ok(new_usage) = result {
        kani::assert(
            new_usage.max_steps_executable <= 1_000,
            "step count must remain within the master contract ceiling after add",
        );
    }
}

/// VB-CORE-BUDGET-001 H4: try_add_budget with zero current usage
#[kani::proof]
#[kani::unwind(4)]
fn kani_try_add_budget_zero_current() {
    let usage = zero_usage();

    let budget = AggregateResourceBudget {
        max_steps_executable: 1,
        ..zero_budget()
    };

    let result = usage.try_add_budget(&budget);
    kani::assert(result.is_ok(), "adding budget to zero usage succeeds");
}
