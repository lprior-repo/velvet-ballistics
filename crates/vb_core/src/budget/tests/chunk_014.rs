//! Test chunk 014 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 3449–3705 of the original. Semantic content is
//! preserved exactly; only the file structure changed.
//! Budget module integration tests.

use crate::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceUsage, BoundednessPolicy,
    BudgetError, WholeWorkflowBudget,
};
use crate::engine::StepBudget;
use crate::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, ExprBranch, ResourceContract, SlotBranch, WorkflowError,
};

use super::prelude::*;

#[test]
fn step_budget_new_one_consumes_to_zero() -> Result<(), String> {
    let mut b = StepBudget::new(1);
    ensure_equal(b.remaining(), 1)?;
    let taken = b.try_take().map_err(|e| e.to_string())?;
    ensure_equal(taken, true)?;
    ensure_equal(b.remaining(), 0)?;
    let taken2 = b.try_take().map_err(|e| e.to_string())?;
    ensure_equal(taken2, false)
}

#[test]
fn step_budget_try_take_never_panics() -> Result<(), String> {
    let mut b = StepBudget::new(0);
    for _ in 0..100 {
        let result = b.try_take();
        match result {
            Ok(false) => {}
            other => return Err(format!("expected Ok(false), got {other:?}")),
        }
    }
    Ok(())
}

#[test]
fn boundedness_policy_custom_zero_limits_accept_zero_budget() -> Result<(), String> {
    let policy = BoundednessPolicy {
        max_total_steps: 0,
        max_total_slots: 0,
        max_fanout: 0,
        max_nesting_depth: 0,
        absolute_max_action_tickets: 0,
        absolute_max_parallel: 0,
        absolute_max_run_time_seconds: 0,
        absolute_max_result_bytes: 0,
        absolute_max_steps_executable: 0,
        ..BoundednessPolicy::DEFAULT
    };
    let budget = test_budget(0, 0, 0, 0);
    ensure_equal(policy.validate(&budget), Ok(()))
}

#[test]
fn boundedness_policy_custom_zero_limits_reject_nonzero() -> Result<(), String> {
    let policy = BoundednessPolicy {
        max_total_steps: 0,
        max_total_slots: 0,
        max_fanout: 0,
        max_nesting_depth: 0,
        absolute_max_action_tickets: 0,
        absolute_max_parallel: 0,
        absolute_max_run_time_seconds: 0,
        absolute_max_result_bytes: 0,
        absolute_max_steps_executable: 0,
        ..BoundednessPolicy::DEFAULT
    };
    let budget = test_budget(1, 0, 0, 0);
    match policy.validate(&budget) {
        Err(BudgetError::TotalStepsExceeded {
            actual: 1,
            limit: 0,
        }) => Ok(()),
        other => Err(format!("expected TotalStepsExceeded, got {other:?}")),
    }
}

// =========================================================================
// VB-CORE-BUDGET overflow/underflow tests (BudgetArithmetic.tla)
// =========================================================================

/// UT-BUDGET-001: AggregateResourceUsage::try_add_budget returns Err on overflow.
/// Does NOT panic; overflow propagates as AggregateBudgetError::Overflow.
#[test]
fn ut_budget_add_never_overflows() {
    // Test overflow on a u64 field: u64::MAX - 1 + 2 = u64::MAX + 1 -> overflow
    let usage_near_max = AggregateResourceUsage {
        max_steps_executable: u64::MAX - 1,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_timer_entries: 12,
        max_trace_events: 13,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 14,
        max_blob_bytes: 15,
        max_input_bytes: 16,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let budget_adds_2 = AggregateResourceBudget {
        max_steps_executable: 2,
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
        max_timer_entries: 7,
        max_trace_events: 8,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 9,
        max_blob_bytes: 10,
        max_input_bytes: 11,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage_near_max.try_add_budget(&budget_adds_2);
    assert!(
        result.is_err(),
        "u64::MAX - 1 + 2 should overflow (return Err), got Ok"
    );

    match result.unwrap_err() {
        AggregateBudgetError::Overflow { resource } => {
            assert_eq!(
                resource, "max_steps_executable",
                "overflow should be in max_steps_executable"
            );
        }
        other => panic!("expected AggregateBudgetError::Overflow, got {other:?}"),
    }

    // Verify adding small budget to zero usage does NOT overflow for non-u64 fields
    let zero_usage = AggregateResourceUsage::default();
    let small_budget = AggregateResourceBudget {
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
        max_timer_entries: 3,
        max_trace_events: 4,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 5,
        max_blob_bytes: 6,
        max_input_bytes: 7,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    let result = zero_usage.try_add_budget(&small_budget);
    assert!(
        result.is_ok(),
        "adding small budget to zero usage should not overflow, got Err"
    );
}

/// UT-BUDGET-002: AggregateResourceUsage::try_subtract_budget returns Err on underflow.
/// Subtraction of a larger budget from a smaller one returns Underflow error.
#[test]
fn ut_budget_sub_never_underflows() {
    // Zero usage minus any budget should underflow
    let zero_usage = AggregateResourceUsage::default();
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
        max_timer_entries: 7,
        max_trace_events: 8,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 9,
        max_blob_bytes: 10,
        max_input_bytes: 11,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = zero_usage.try_subtract_budget(&budget);
    assert!(
        result.is_err(),
        "subtracting from zero usage should underflow (return Err), got Ok"
    );

    match result.unwrap_err() {
        AggregateBudgetError::Underflow { resource: _ } => {}
        other => panic!("expected AggregateBudgetError::Underflow, got {other:?}"),
    }

    // Non-zero usage subtract that results in zero is fine (no underflow)
    let usage = AggregateResourceUsage {
        max_steps_executable: 5,
        max_action_tickets: 5,
        max_parallel_in_flight: 5,
        max_gather_pages: 5,
        max_gather_items: 5,
        max_result_bytes: 5,
        max_total_slots_written: 5,
        max_timer_entries: 14,
        max_trace_events: 16,
        max_active_runs: 5,
        max_queue_depth: 5,
        max_journal_batch_bytes: 5,
        max_ipc_payload_bytes: 18,
        max_blob_bytes: 20,
        max_input_bytes: 22,
        max_step_budget_per_tick: 5,
        max_transitions_per_tick: 5,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 3,
        max_action_tickets: 3,
        max_parallel_in_flight: 3,
        max_retries_per_action: 0, // 0 subtract 0 = 0, no underflow
        max_gather_pages: 3,
        max_gather_items: 3,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 3,
        max_total_slots_written: 3,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 3,
        max_journal_batch_bytes: 3,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 3,
        max_transitions_per_tick: 3,
    };

    let result = usage.try_subtract_budget(&budget);
    assert!(
        result.is_ok(),
        "subtract resulting in non-negative should be Ok, got {result:?}"
    );
}
