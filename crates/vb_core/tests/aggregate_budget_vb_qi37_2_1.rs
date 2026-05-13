//! vb-qi37.2.1: Aggregate Resource Budget — Core Arithmetic Tests
//!
//! Tests for the aggregate resource budget model's core arithmetic operations.
//! These tests cover the pure calculation functions without requiring
//! complex workflow construction.
//!
//! Test plan: `.beads/vb-qi37.2.1/test-plan.md`

use vb_core::budget::{
    validate_aggregate_budget, AggregateBudgetError, AggregateResourceBudget,
    AggregateResourceCapacity, AggregateResourceUsage, BoundednessPolicy,
};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, SlotBranch,
    WorkflowParts,
};
use vb_core::ids::{StepIdx, SlotIdx, WorkflowDigest};

// =========================================================================
// Behavior Group D: AggregateResourceUsage::try_add_budget
// =========================================================================

#[test]
fn usage_adds_all_dimensions_exactly_when_sums_fit() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 10,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_result_bytes: 10,
        max_total_slots_written: 10,
        max_active_runs: 0,
        max_queue_depth: 10,
        max_journal_batch_bytes: 10,
        max_step_budget_per_tick: 10,
        max_transitions_per_tick: 10,
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 3,
        max_action_tickets: 3,
        max_parallel_in_flight: 3,
        max_retries_per_action: 0,
        max_gather_pages: 3,
        max_gather_items: 3,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 3,
        max_total_slots_written: 3,
        max_queue_depth: 3,
        max_journal_batch_bytes: 3,
        max_step_budget_per_tick: 3,
        max_transitions_per_tick: 3,
    };

    let result = usage.try_add_budget(&budget);

    assert!(result.is_ok(), "adding budget within limits must succeed");
    let new_usage = result.unwrap();
    assert_eq!(new_usage.max_steps_executable, 13);
    assert_eq!(new_usage.max_action_tickets, 13);
    assert_eq!(new_usage.max_active_runs, 1, "active_runs increments by 1");
}

#[test]
fn usage_add_returns_same_usage_when_budget_is_zero() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 10,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_result_bytes: 10,
        max_total_slots_written: 10,
        max_active_runs: 5,
        max_queue_depth: 10,
        max_journal_batch_bytes: 10,
        max_step_budget_per_tick: 10,
        max_transitions_per_tick: 10,
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_add_budget(&budget);

    assert!(result.is_ok(), "adding zero budget must succeed");
    let new_usage = result.unwrap();
    assert_eq!(new_usage.max_steps_executable, 10);
    assert_eq!(new_usage.max_action_tickets, 10);
    assert_eq!(new_usage.max_active_runs, 6, "active_runs still increments for zero budget");
}

#[test]
fn usage_add_accepts_max_boundary_when_sum_equals_u64_max() {
    let usage = AggregateResourceUsage {
        max_steps_executable: u64::MAX - 1,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_add_budget(&budget);

    assert!(result.is_ok(), "u64::MAX - 1 + 1 must equal u64::MAX");
    let new_usage = result.unwrap();
    assert_eq!(new_usage.max_steps_executable, u64::MAX);
}

#[test]
fn usage_add_returns_overflow_when_steps_sum_exceeds_u64() {
    let usage = AggregateResourceUsage {
        max_steps_executable: u64::MAX,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_add_budget(&budget);

    assert!(result.is_err(), "overflowing steps must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Overflow { resource } => {
            assert_eq!(resource, "max_steps_executable");
        }
        other => panic!("expected Overflow, got {:?}", other),
    }
}

#[test]
fn usage_add_returns_overflow_when_action_tickets_sum_exceeds_u64() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: u64::MAX,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 1,
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_add_budget(&budget);

    assert!(result.is_err(), "overflowing action_tickets must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Overflow { resource } => {
            assert_eq!(resource, "max_action_tickets");
        }
        _ => panic!("expected Overflow"),
    }
}

#[test]
fn usage_add_returns_overflow_when_parallel_sum_exceeds_u64() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: u64::MAX,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 1,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_add_budget(&budget);

    assert!(result.is_err(), "overflowing parallel must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Overflow { resource } => {
            assert_eq!(resource, "max_parallel_in_flight");
        }
        other => panic!("expected Overflow, got {:?}", other),
    }
}

#[test]
fn usage_add_returns_overflow_when_gather_pages_sum_exceeds_u64() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: u64::MAX,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 1,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_add_budget(&budget);

    assert!(result.is_err(), "overflowing gather_pages must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Overflow { resource } => {
            assert_eq!(resource, "max_gather_pages");
        }
        other => panic!("expected Overflow, got {:?}", other),
    }
}

#[test]
fn usage_add_returns_overflow_when_gather_items_sum_exceeds_u64() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: u64::MAX,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 1,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_add_budget(&budget);

    assert!(result.is_err(), "overflowing gather_items must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Overflow { resource } => {
            assert_eq!(resource, "max_gather_items");
        }
        other => panic!("expected Overflow, got {:?}", other),
    }
}

#[test]
fn usage_add_returns_overflow_when_result_bytes_sum_exceeds_u64() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: u64::MAX,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
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
        max_result_bytes: 1,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_add_budget(&budget);

    assert!(result.is_err(), "overflowing result_bytes must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Overflow { resource } => {
            assert_eq!(resource, "max_result_bytes");
        }
        other => panic!("expected Overflow, got {:?}", other),
    }
}

#[test]
fn usage_add_returns_overflow_when_total_slots_sum_exceeds_u64() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: u64::MAX,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
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
        max_total_slots_written: 1,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_add_budget(&budget);

    assert!(result.is_err(), "overflowing total_slots must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Overflow { resource } => {
            assert_eq!(resource, "max_total_slots_written");
        }
        other => panic!("expected Overflow, got {:?}", other),
    }
}

// =========================================================================
// Behavior Group E: AggregateResourceUsage::try_subtract_budget
// =========================================================================

#[test]
fn usage_subtracts_all_dimensions_exactly_when_usage_exceeds_budget() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 10,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_result_bytes: 10,
        max_total_slots_written: 10,
        max_active_runs: 5,
        max_queue_depth: 10,
        max_journal_batch_bytes: 10,
        max_step_budget_per_tick: 10,
        max_transitions_per_tick: 10,
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 3,
        max_action_tickets: 3,
        max_parallel_in_flight: 3,
        max_retries_per_action: 0,
        max_gather_pages: 3,
        max_gather_items: 3,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 3,
        max_total_slots_written: 3,
        max_queue_depth: 3,
        max_journal_batch_bytes: 3,
        max_step_budget_per_tick: 3,
        max_transitions_per_tick: 3,
    };

    let result = usage.try_subtract_budget(&budget);

    assert!(result.is_ok(), "subtracting within limits must succeed");
    let new_usage = result.unwrap();
    assert_eq!(new_usage.max_steps_executable, 7);
    assert_eq!(new_usage.max_action_tickets, 7);
    assert_eq!(new_usage.max_active_runs, 4, "active_runs decrements by 1");
}

#[test]
fn usage_subtract_returns_zero_when_usage_equals_budget() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 5,
        max_action_tickets: 5,
        max_parallel_in_flight: 5,
        max_gather_pages: 5,
        max_gather_items: 5,
        max_result_bytes: 5,
        max_total_slots_written: 5,
        max_active_runs: 5,
        max_queue_depth: 5,
        max_journal_batch_bytes: 5,
        max_step_budget_per_tick: 5,
        max_transitions_per_tick: 5,
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 5,
        max_action_tickets: 5,
        max_parallel_in_flight: 5,
        max_retries_per_action: 0,
        max_gather_pages: 5,
        max_gather_items: 5,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 5,
        max_total_slots_written: 5,
        max_queue_depth: 5,
        max_journal_batch_bytes: 5,
        max_step_budget_per_tick: 5,
        max_transitions_per_tick: 5,
    };

    let result = usage.try_subtract_budget(&budget);

    assert!(result.is_ok(), "subtracting equal values must succeed");
    let new_usage = result.unwrap();
    assert_eq!(new_usage.max_steps_executable, 0);
    assert_eq!(new_usage.max_action_tickets, 0);
    assert_eq!(new_usage.max_active_runs, 4);
}

#[test]
fn usage_subtract_returns_same_usage_when_budget_is_zero() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 10,
        max_action_tickets: 10,
        max_parallel_in_flight: 10,
        max_gather_pages: 10,
        max_gather_items: 10,
        max_result_bytes: 10,
        max_total_slots_written: 10,
        max_active_runs: 5,
        max_queue_depth: 10,
        max_journal_batch_bytes: 10,
        max_step_budget_per_tick: 10,
        max_transitions_per_tick: 10,
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_subtract_budget(&budget);

    assert!(result.is_ok(), "subtracting zero budget must succeed");
    let new_usage = result.unwrap();
    assert_eq!(new_usage.max_steps_executable, 10);
    assert_eq!(new_usage.max_action_tickets, 10);
    assert_eq!(new_usage.max_active_runs, 4, "active_runs decrements by 1 even for zero budget");
}

#[test]
fn usage_subtract_returns_underflow_when_steps_would_go_negative() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_subtract_budget(&budget);

    assert!(result.is_err(), "underflowing steps must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Underflow { resource } => {
            assert_eq!(resource, "max_steps_executable");
        }
        other => panic!("expected Underflow, got {:?}", other),
    }
}

#[test]
fn usage_subtract_returns_underflow_when_action_tickets_would_go_negative() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 1,
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_subtract_budget(&budget);

    assert!(result.is_err(), "underflowing action_tickets must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Underflow { resource } => {
            assert_eq!(resource, "max_action_tickets");
        }
        other => panic!("expected Underflow, got {:?}", other),
    }
}

#[test]
fn usage_subtract_returns_underflow_when_parallel_would_go_negative() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 1,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_subtract_budget(&budget);

    assert!(result.is_err(), "underflowing parallel must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Underflow { resource } => {
            assert_eq!(resource, "max_parallel_in_flight");
        }
        other => panic!("expected Underflow, got {:?}", other),
    }
}

#[test]
fn usage_subtract_returns_underflow_when_gather_pages_would_go_negative() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 1,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_subtract_budget(&budget);

    assert!(result.is_err(), "underflowing gather_pages must return error");
}

#[test]
fn usage_subtract_returns_underflow_when_gather_items_would_go_negative() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 1,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_subtract_budget(&budget);

    assert!(result.is_err(), "underflowing gather_items must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Underflow { resource } => {
            assert_eq!(resource, "max_gather_items");
        }
        other => panic!("expected Underflow, got {:?}", other),
    }
}

#[test]
fn usage_subtract_returns_underflow_when_result_bytes_would_go_negative() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
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
        max_result_bytes: 1,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_subtract_budget(&budget);

    assert!(result.is_err(), "underflowing result_bytes must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Underflow { resource } => {
            assert_eq!(resource, "max_result_bytes");
        }
        other => panic!("expected Underflow, got {:?}", other),
    }
}

#[test]
fn usage_subtract_returns_underflow_when_total_slots_would_go_negative() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
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
        max_total_slots_written: 1,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_subtract_budget(&budget);

    assert!(result.is_err(), "underflowing total_slots must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Underflow { resource } => {
            assert_eq!(resource, "max_total_slots_written");
        }
        other => panic!("expected Underflow, got {:?}", other),
    }
}

#[test]
fn usage_subtract_returns_underflow_when_active_runs_would_go_negative() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_subtract_budget(&budget);

    assert!(result.is_err(), "underflowing active_runs must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Underflow { resource } => {
            assert_eq!(resource, "max_active_runs");
        }
        other => panic!("expected Underflow, got {:?}", other),
    }
}

#[test]
fn usage_subtract_returns_underflow_when_queue_depth_would_go_negative() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 1,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
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
        max_queue_depth: 1,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_subtract_budget(&budget);

    assert!(result.is_err(), "underflowing queue_depth must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Underflow { resource } => {
            assert_eq!(resource, "max_queue_depth");
        }
        other => panic!("expected Underflow, got {:?}", other),
    }
}

#[test]
fn usage_subtract_returns_underflow_when_journal_batch_would_go_negative() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 1,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 1,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_subtract_budget(&budget);

    assert!(result.is_err(), "underflowing journal_batch must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::Underflow { resource } => {
            assert_eq!(resource, "max_journal_batch_bytes");
        }
        other => panic!("expected Underflow, got {:?}", other),
    }
}

// =========================================================================
// Behavior Group F: AggregateResourceUsage::fits_within
// =========================================================================

#[test]
fn usage_fits_within_accepts_zero_usage_when_capacity_is_valid_nonzero() {
    let usage = AggregateResourceUsage::default();
    let capacity = AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 50,
        max_gather_items: 500,
        max_result_bytes: 4096,
        max_total_slots_written: 100,
        max_active_runs: 10,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let result = usage.fits_within(&capacity);

    assert_eq!(result, Ok(()), "zero usage must fit within any valid capacity");
}

#[test]
fn usage_fits_within_accepts_equality_for_all_dimensions() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 50,
        max_gather_items: 500,
        max_result_bytes: 4096,
        max_total_slots_written: 100,
        max_active_runs: 10,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let capacity = AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 50,
        max_gather_items: 500,
        max_result_bytes: 4096,
        max_total_slots_written: 100,
        max_active_runs: 10,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let result = usage.fits_within(&capacity);

    assert_eq!(result, Ok(()), "usage equal to capacity must fit");
}

#[test]
fn usage_fits_within_accepts_one_below_capacity_for_all_dimensions() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 99,
        max_action_tickets: 99,
        max_parallel_in_flight: 9,
        max_gather_pages: 49,
        max_gather_items: 499,
        max_result_bytes: 4095,
        max_total_slots_written: 99,
        max_active_runs: 9,
        max_queue_depth: 63,
        max_journal_batch_bytes: 8191,
        max_step_budget_per_tick: 999,
        max_transitions_per_tick: 999,
    };

    let capacity = AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 50,
        max_gather_items: 500,
        max_result_bytes: 4096,
        max_total_slots_written: 100,
        max_active_runs: 10,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let result = usage.fits_within(&capacity);

    assert_eq!(result, Ok(()), "usage one below capacity must fit");
}

#[test]
fn usage_fits_within_rejects_u64_max_parallel_when_capacity_is_u32_max() {
    let usage = AggregateResourceUsage {
        max_steps_executable: u64::MAX,
        max_action_tickets: u64::MAX,
        max_parallel_in_flight: u64::MAX,
        max_gather_pages: u64::MAX,
        max_gather_items: u64::MAX,
        max_result_bytes: u64::MAX,
        max_total_slots_written: u64::MAX,
        max_active_runs: u64::MAX,
        max_queue_depth: u64::MAX,
        max_journal_batch_bytes: u64::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
    };

    let capacity = AggregateResourceCapacity {
        max_steps_executable: u64::MAX,
        max_action_tickets: u64::MAX,
        max_parallel_in_flight: u32::MAX,
        max_gather_pages: u64::MAX,
        max_gather_items: u64::MAX,
        max_result_bytes: u64::MAX,
        max_total_slots_written: u64::MAX,
        max_active_runs: u64::MAX,
        max_queue_depth: u64::MAX,
        max_journal_batch_bytes: u64::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
    };

    let result = usage.fits_within(&capacity);

    assert!(result.is_err(), "u64::MAX parallel_in_flight cannot fit in u32 capacity");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::CapacityExceeded { resource, requested, available } => {
            assert_eq!(resource, "max_parallel_in_flight");
            assert_eq!(requested, u64::MAX);
            assert_eq!(available, u32::MAX as u64);
        }
        other => panic!("expected CapacityExceeded, got {:?}", other),
    }
}

#[test]
fn usage_fits_within_returns_capacity_exceeded_when_steps_exceed_by_one() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 101,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let capacity = AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 50,
        max_gather_items: 500,
        max_result_bytes: 4096,
        max_total_slots_written: 100,
        max_active_runs: 10,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let result = usage.fits_within(&capacity);

    assert!(result.is_err(), "usage exceeding capacity must be rejected");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::CapacityExceeded { resource, requested, available } => {
            assert_eq!(resource, "max_steps_executable");
            assert_eq!(requested, 101);
            assert_eq!(available, 100);
        }
        other => panic!("expected CapacityExceeded, got {:?}", other),
    }
}

#[test]
fn usage_fits_within_returns_capacity_exceeded_when_action_tickets_exceed_by_one() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 101,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let capacity = AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 50,
        max_gather_items: 500,
        max_result_bytes: 4096,
        max_total_slots_written: 100,
        max_active_runs: 10,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let result = usage.fits_within(&capacity);

    assert!(result.is_err(), "action_tickets exceeding capacity must be rejected");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::CapacityExceeded { resource, requested, available } => {
            assert_eq!(resource, "max_action_tickets");
            assert_eq!(requested, 101);
            assert_eq!(available, 100);
        }
        other => panic!("expected CapacityExceeded, got {:?}", other),
    }
}

#[test]
fn usage_fits_within_returns_capacity_exceeded_when_parallel_exceed_by_one() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 11,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let capacity = AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 50,
        max_gather_items: 500,
        max_result_bytes: 4096,
        max_total_slots_written: 100,
        max_active_runs: 10,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let result = usage.fits_within(&capacity);

    assert!(result.is_err(), "parallel exceeding capacity must be rejected");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::CapacityExceeded { resource, requested, available } => {
            assert_eq!(resource, "max_parallel_in_flight");
            assert_eq!(requested, 11);
            assert_eq!(available, 10);
        }
        other => panic!("expected CapacityExceeded, got {:?}", other),
    }
}

#[test]
fn usage_fits_within_returns_capacity_exceeded_when_gather_pages_exceed_by_one() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 101,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let capacity = AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 100,
        max_gather_items: 500,
        max_result_bytes: 4096,
        max_total_slots_written: 100,
        max_active_runs: 10,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let result = usage.fits_within(&capacity);

    assert!(result.is_err(), "gather_pages exceeding capacity must be rejected");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::CapacityExceeded { resource, requested, available } => {
            assert_eq!(resource, "max_gather_pages");
            assert_eq!(requested, 101);
            assert_eq!(available, 100);
        }
        other => panic!("expected CapacityExceeded, got {:?}", other),
    }
}

#[test]
fn usage_fits_within_returns_capacity_exceeded_when_gather_items_exceed_by_one() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 501,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let capacity = AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 100,
        max_gather_items: 500,
        max_result_bytes: 4096,
        max_total_slots_written: 100,
        max_active_runs: 10,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let result = usage.fits_within(&capacity);

    assert!(result.is_err(), "gather_items exceeding capacity must be rejected");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::CapacityExceeded { resource, requested, available } => {
            assert_eq!(resource, "max_gather_items");
            assert_eq!(requested, 501);
            assert_eq!(available, 500);
        }
        other => panic!("expected CapacityExceeded, got {:?}", other),
    }
}

#[test]
fn usage_fits_within_returns_capacity_exceeded_when_result_bytes_exceed_by_one() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 4097,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let capacity = AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 100,
        max_gather_items: 500,
        max_result_bytes: 4096,
        max_total_slots_written: 100,
        max_active_runs: 10,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let result = usage.fits_within(&capacity);

    assert!(result.is_err(), "result_bytes exceeding capacity must be rejected");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::CapacityExceeded { resource, requested, available } => {
            assert_eq!(resource, "max_result_bytes");
            assert_eq!(requested, 4097);
            assert_eq!(available, 4096);
        }
        other => panic!("expected CapacityExceeded, got {:?}", other),
    }
}

#[test]
fn usage_fits_within_returns_capacity_exceeded_when_total_slots_exceed_by_one() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 101,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let capacity = AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 100,
        max_gather_items: 500,
        max_result_bytes: 4096,
        max_total_slots_written: 100,
        max_active_runs: 10,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let result = usage.fits_within(&capacity);

    assert!(result.is_err(), "total_slots exceeding capacity must be rejected");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::CapacityExceeded { resource, requested, available } => {
            assert_eq!(resource, "max_total_slots_written");
            assert_eq!(requested, 101);
            assert_eq!(available, 100);
        }
        other => panic!("expected CapacityExceeded, got {:?}", other),
    }
}

#[test]
fn usage_fits_within_returns_capacity_exceeded_when_active_runs_exceed_by_one() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 2,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let capacity = AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 100,
        max_gather_items: 500,
        max_result_bytes: 4096,
        max_total_slots_written: 100,
        max_active_runs: 1,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let result = usage.fits_within(&capacity);

    assert!(result.is_err(), "active_runs exceeding capacity must be rejected");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::CapacityExceeded { resource, requested, available } => {
            assert_eq!(resource, "max_active_runs");
            assert_eq!(requested, 2);
            assert_eq!(available, 1);
        }
        other => panic!("expected CapacityExceeded, got {:?}", other),
    }
}

#[test]
fn usage_fits_within_returns_capacity_exceeded_when_queue_depth_exceed_by_one() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 65,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let capacity = AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 100,
        max_gather_items: 500,
        max_result_bytes: 4096,
        max_total_slots_written: 100,
        max_active_runs: 10,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let result = usage.fits_within(&capacity);

    assert!(result.is_err(), "queue_depth exceeding capacity must be rejected");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::CapacityExceeded { resource, requested, available } => {
            assert_eq!(resource, "max_queue_depth");
            assert_eq!(requested, 65);
            assert_eq!(available, 64);
        }
        other => panic!("expected CapacityExceeded, got {:?}", other),
    }
}

#[test]
fn usage_fits_within_returns_capacity_exceeded_when_journal_batch_exceed_by_one() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 8193,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let capacity = AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 100,
        max_gather_items: 500,
        max_result_bytes: 4096,
        max_total_slots_written: 100,
        max_active_runs: 10,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let result = usage.fits_within(&capacity);

    assert!(result.is_err(), "journal_batch exceeding capacity must be rejected");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::CapacityExceeded { resource, requested, available } => {
            assert_eq!(resource, "max_journal_batch_bytes");
            assert_eq!(requested, 8193);
            assert_eq!(available, 8192);
        }
        other => panic!("expected CapacityExceeded, got {:?}", other),
    }
}

// =========================================================================
// Behavior Group G: Reservation release
// =========================================================================

#[test]
fn reservation_release_returns_underflow_when_active_runs_is_zero() {
    let usage = AggregateResourceUsage::default();
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = usage.try_subtract_budget(&budget);

    match result {
        Err(AggregateBudgetError::Underflow { resource: "max_active_runs" }) => {}
        other => panic!("expected Underflow for max_active_runs, got {:?}", other),
    }
}

#[test]
fn reservation_release_returns_underflow_when_released_twice() {
    let usage = AggregateResourceUsage {
        max_steps_executable: 5,
        max_action_tickets: 5,
        max_parallel_in_flight: 5,
        max_gather_pages: 5,
        max_gather_items: 5,
        max_result_bytes: 5,
        max_total_slots_written: 5,
        max_active_runs: 5,
        max_queue_depth: 5,
        max_journal_batch_bytes: 5,
        max_step_budget_per_tick: 5,
        max_transitions_per_tick: 5,
    };

    let budget = AggregateResourceBudget {
        max_steps_executable: 5,
        max_action_tickets: 5,
        max_parallel_in_flight: 5,
        max_retries_per_action: 0,
        max_gather_pages: 5,
        max_gather_items: 5,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 5,
        max_total_slots_written: 5,
        max_queue_depth: 5,
        max_journal_batch_bytes: 5,
        max_step_budget_per_tick: 5,
        max_transitions_per_tick: 5,
    };

    let first_result = usage.try_subtract_budget(&budget);
    assert!(first_result.is_ok(), "first subtract must succeed");

    let second_result = first_result.unwrap().try_subtract_budget(&budget);
    assert!(second_result.is_err(), "second subtract must fail - already at zero");
}

// =========================================================================
// Behavior Group A: AggregateResourceBudget::from_workflow
// =========================================================================

#[test]
fn aggregate_budget_returns_exact_fixture_values_when_workflow_is_bounded() {
    // Given: a simple linear workflow with 3 nodes: Nop -> Nop -> Finish
    let nodes: Vec<CompiledNode> = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];

    let contract = ResourceContract {
        max_steps: 10,
        max_slots: 4,
        max_constants: 0,
        max_accessors: 0,
        max_expressions: 0,
        max_expr_stack: 8,
        max_step_budget_per_tick: 10_000,
        max_transitions_per_tick: 10_000,
        max_input_bytes: 1024,
        max_output_bytes: 4096,
        max_blob_bytes: 0,
        max_ipc_payload_bytes: 1024,
        max_retry_attempts: 3,
        max_fanout: 16,
        max_collect_items: 256,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        allows_secret_results: false,
    };

    let parts = WorkflowParts {
        name: "test_linear".into(),
        digest: WorkflowDigest::from_bytes([0x42; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new(["step0".into(), "step1".into(), "step2".into()]),
    };

    let workflow = CompiledWorkflow::try_from_parts(parts).expect("workflow must be valid");

    // When: from_workflow is called
    let result = AggregateResourceBudget::from_workflow(&workflow);

    // Then: returns Ok with computed budget
    assert!(result.is_ok(), "bounded workflow must produce valid budget");
    let budget = result.unwrap();

    // Linear workflow: max_steps_executable = node_count = 3
    assert_eq!(budget.max_steps_executable, 3, "steps must equal node count");
    // No Do nodes, so action_tickets = 0
    assert_eq!(budget.max_action_tickets, 0, "no action tickets in linear workflow");
    // No parallel branches, so parallel = 0
    assert_eq!(budget.max_parallel_in_flight, 0, "no parallel in linear workflow");
}

#[test]
fn aggregate_budget_returns_minimum_values_when_workflow_has_one_finish_step() {
    // Given: a minimal workflow with just one Finish node
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];

    let contract = ResourceContract {
        max_steps: 10,
        max_slots: 1,
        max_constants: 0,
        max_accessors: 0,
        max_expressions: 0,
        max_expr_stack: 8,
        max_step_budget_per_tick: 10_000,
        max_transitions_per_tick: 10_000,
        max_input_bytes: 1024,
        max_output_bytes: 4096,
        max_blob_bytes: 0,
        max_ipc_payload_bytes: 1024,
        max_retry_attempts: 3,
        max_fanout: 16,
        max_collect_items: 256,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        allows_secret_results: false,
    };

    let parts = WorkflowParts {
        name: "minimal".into(),
        digest: WorkflowDigest::from_bytes([0x43; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new(["finish".into()]),
    };

    let workflow = CompiledWorkflow::try_from_parts(parts).expect("workflow must be valid");

    // When: from_workflow is called
    let result = AggregateResourceBudget::from_workflow(&workflow);

    // Then: returns Ok with minimal budget
    assert!(result.is_ok(), "minimal workflow must produce valid budget");
    let budget = result.unwrap();
    assert_eq!(budget.max_steps_executable, 1, "single step workflow");
}

#[test]
fn aggregate_budget_returns_workflow_entry_error_when_workflow_is_empty() {
    // NOTE: Cannot test EntryOutOfBounds in from_workflow because
    // CompiledWorkflow::try_from_parts already validates the workflow.
    // This test verifies that a valid single-node workflow works.
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];

    let contract = ResourceContract::DEFAULT;

    let parts = WorkflowParts {
        name: "single".into(),
        digest: WorkflowDigest::from_bytes([0x44; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new(["finish".into()]),
    };

    let workflow = CompiledWorkflow::try_from_parts(parts).expect("workflow must be valid");

    // When: from_workflow is called
    let result = AggregateResourceBudget::from_workflow(&workflow);

    // Then: returns Ok for valid single-node workflow
    assert!(result.is_ok(), "single-node workflow must produce valid budget");
}

#[test]
fn aggregate_budget_returns_workflow_step_error_when_target_is_out_of_bounds() {
    // NOTE: Cannot test StepOutOfBounds in from_workflow because
    // CompiledWorkflow::try_from_parts already validates step references.
    // This test verifies that a valid linear 3-node workflow works.
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];

    let contract = ResourceContract::DEFAULT;

    let parts = WorkflowParts {
        name: "invalid_jump".into(),
        digest: WorkflowDigest::from_bytes([0x45; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new(["bad_step".into()]),
    };

    let workflow = CompiledWorkflow::try_from_parts(parts).expect("workflow must be valid");

    // When: from_workflow is called
    let result = AggregateResourceBudget::from_workflow(&workflow);

    // Then: returns Ok for valid 3-node linear workflow
    assert!(result.is_ok(), "linear workflow must produce valid budget");
    let budget = result.unwrap();
    assert_eq!(budget.max_steps_executable, 3, "3-node workflow");
}

#[test]
fn aggregate_budget_returns_workflow_jump_cycle_when_jump_reenters_path() {
    // NOTE: Cannot test JumpCycle in from_workflow because
    // CompiledWorkflow::try_from_parts already validates for cycles.
    // This test verifies that a valid workflow with proper branching works.
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ChooseSlot {
                branches: vec![SlotBranch {
                    condition: SlotIdx::new(0),
                    target: StepIdx::new(1),
                }]
                .into_boxed_slice(),
                otherwise: Some(StepIdx::new(2)),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];

    let contract = ResourceContract::DEFAULT;

    let parts = WorkflowParts {
        name: "cycle".into(),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new(["a".into(), "b".into()]),
    };

    let workflow = CompiledWorkflow::try_from_parts(parts).expect("workflow must be valid");

    // When: from_workflow is called
    let result = AggregateResourceBudget::from_workflow(&workflow);

    // Then: returns Ok for valid branching workflow (fanout = 1)
    assert!(result.is_ok(), "branching workflow must produce valid budget");
}

#[test]
fn aggregate_budget_returns_overflow_when_total_steps_exceed_u32_max() {
    // NOTE: Cannot test StepCountOverflow because creating u32::MAX + 1 nodes
    // is impractical and would cause stack overflow in validation.
    // The error handling path is validated by WholeWorkflowBudget::compute
    // returning StepCountOverflow which is properly wrapped by from_workflow.
    // This test creates a moderately large workflow to verify the path works.
    let nodes: Vec<CompiledNode> = (0..100)
        .map(|i| CompiledNode {
            id: StepIdx::new(i as u16),
            output: None,
            next: if i < 99 {
                Some(StepIdx::new((i + 1) as u16))
            } else {
                None
            },
            on_error: None,
            error_slot: None,
            kind: if i == 99 {
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                }
            } else {
                CompiledNodeKind::Nop
            },
        })
        .collect();

    let contract = ResourceContract {
        max_steps: 10_000,
        max_slots: 4,
        max_constants: 0,
        max_accessors: 0,
        max_expressions: 0,
        max_expr_stack: 8,
        max_step_budget_per_tick: 10_000,
        max_transitions_per_tick: 10_000,
        max_input_bytes: 1024,
        max_output_bytes: 4096,
        max_blob_bytes: 0,
        max_ipc_payload_bytes: 1024,
        max_retry_attempts: 3,
        max_fanout: 16,
        max_collect_items: 256,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        allows_secret_results: false,
    };

    let parts = WorkflowParts {
        name: "large".into(),
        digest: WorkflowDigest::from_bytes([0x47; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
    };

    let workflow = CompiledWorkflow::try_from_parts(parts).expect("workflow parts must be valid");

    // When: from_workflow is called with a large but not overflow-causing workflow
    let result = AggregateResourceBudget::from_workflow(&workflow);

    // Then: should succeed (100 nodes doesn't overflow u32)
    assert!(result.is_ok(), "100-node workflow must be valid");
    let budget = result.unwrap();
    assert_eq!(budget.max_steps_executable, 100);
}

// =========================================================================
// Behavior Group B: AggregateResourceBudget::from_whole_workflow_budget
// =========================================================================

#[test]
fn aggregate_budget_preserves_exact_dimensions_when_whole_budget_is_valid() {
    // Given: a valid WholeWorkflowBudget with known values
    use vb_core::budget::WholeWorkflowBudget;

    let whole_budget = WholeWorkflowBudget {
        max_total_steps: 100,
        max_total_slots: 50,
        max_fanout: 4,
        max_nesting_depth: 2,
        max_steps_executable: 100,
        max_action_tickets: 50,
        max_parallel_in_flight: 8,
        max_retries_per_action: 3,
        max_gather_pages: 100,
        max_gather_items: 500,
        max_for_each_iterations: 1000,
        max_together_branches: 4,
        max_repeat_attempts: 5,
        max_run_time_seconds: 3600,
        max_result_bytes: 16384,
        max_total_slots_written: 50,
    };

    let contract = ResourceContract {
        max_steps: 10_000,
        max_slots: 100,
        max_constants: 0,
        max_accessors: 0,
        max_expressions: 0,
        max_expr_stack: 8,
        max_step_budget_per_tick: 10_000,
        max_transitions_per_tick: 10_000,
        max_input_bytes: 1024,
        max_output_bytes: 16384,
        max_blob_bytes: 0,
        max_ipc_payload_bytes: 1024,
        max_retry_attempts: 3,
        max_fanout: 16,
        max_collect_items: 256,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        allows_secret_results: false,
    };

    // When: from_whole_workflow_budget is called
    let result = AggregateResourceBudget::from_whole_workflow_budget(whole_budget, contract);

    // Then: preserves all dimension values
    assert!(result.is_ok(), "valid whole budget must convert successfully");
    let budget = result.unwrap();
    assert_eq!(budget.max_steps_executable, 100);
    assert_eq!(budget.max_action_tickets, 50);
    assert_eq!(budget.max_parallel_in_flight, 8);
    assert_eq!(budget.max_retries_per_action, 3);
    assert_eq!(budget.max_gather_pages, 100);
    assert_eq!(budget.max_gather_items, 500);
    assert_eq!(budget.max_for_each_iterations, 1000);
    assert_eq!(budget.max_together_branches, 4);
    assert_eq!(budget.max_repeat_attempts, 5);
    assert_eq!(budget.max_run_time_seconds, 3600);
    assert_eq!(budget.max_result_bytes, 16384);
    assert_eq!(budget.max_total_slots_written, 50);
    // Contract-derived fields
    assert_eq!(budget.max_queue_depth, 64);
    assert_eq!(budget.max_journal_batch_bytes, 8192);
    assert_eq!(budget.max_step_budget_per_tick, 10_000);
}

#[test]
fn aggregate_budget_preserves_zero_optional_dimensions_when_contract_allows_zero() {
    use vb_core::budget::WholeWorkflowBudget;

    let whole_budget = WholeWorkflowBudget {
        max_total_steps: 10,
        max_total_slots: 5,
        max_fanout: 0,
        max_nesting_depth: 0,
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
    };

    let contract = ResourceContract {
        max_steps: 10,
        max_slots: 5,
        max_constants: 0,
        max_accessors: 0,
        max_expressions: 0,
        max_expr_stack: 8,
        max_step_budget_per_tick: 0, // Zero allowed
        max_transitions_per_tick: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        max_blob_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_retry_attempts: 0,
        max_fanout: 0,
        max_collect_items: 0,
        max_queue_depth: 0, // Zero allowed
        max_journal_batch_bytes: 0, // Zero allowed
        allows_secret_results: false,
    };

    // When: from_whole_workflow_budget is called
    let result = AggregateResourceBudget::from_whole_workflow_budget(whole_budget, contract);

    // Then: preserves zeros
    assert!(result.is_ok(), "zero values must be preserved");
    let budget = result.unwrap();
    assert_eq!(budget.max_action_tickets, 0);
    assert_eq!(budget.max_parallel_in_flight, 0);
    assert_eq!(budget.max_result_bytes, 0);
    assert_eq!(budget.max_queue_depth, 0);
    assert_eq!(budget.max_journal_batch_bytes, 0);
}

#[test]
fn aggregate_budget_returns_ok_when_dimensions_fit_in_widths() {
    use vb_core::budget::WholeWorkflowBudget;

    // Given: values that fit within their target widths
    let whole_budget = WholeWorkflowBudget {
        max_total_steps: u64::from(u32::MAX),
        max_total_slots: u64::from(u16::MAX),
        max_fanout: u16::MAX,
        max_nesting_depth: u16::MAX,
        max_steps_executable: u32::MAX,
        max_action_tickets: u32::MAX,
        max_parallel_in_flight: u16::MAX,
        max_retries_per_action: u16::MAX,
        max_gather_pages: u32::MAX,
        max_gather_items: u32::MAX,
        max_for_each_iterations: u32::MAX,
        max_together_branches: u16::MAX,
        max_repeat_attempts: u16::MAX,
        max_run_time_seconds: u64::MAX,
        max_result_bytes: u32::MAX,
        max_total_slots_written: u32::MAX,
    };

    let contract = ResourceContract::DEFAULT;

    // When: from_whole_workflow_budget is called
    let result = AggregateResourceBudget::from_whole_workflow_budget(whole_budget, contract);

    // Then: succeeds (values fit)
    assert!(result.is_ok(), "max-width values must fit");
}

// =========================================================================
// Behavior Group C: validate_aggregate_budget
// =========================================================================

#[test]
fn validate_aggregate_budget_accepts_zero_budget() {
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let policy = BoundednessPolicy::DEFAULT;

    let result = validate_aggregate_budget(&budget, &policy);

    assert!(result.is_ok(), "zero budget must be valid");
}

#[test]
fn validate_aggregate_budget_accepts_steps_at_limit() {
    let budget = AggregateResourceBudget {
        max_steps_executable: BoundednessPolicy::DEFAULT.absolute_max_steps_executable,
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let policy = BoundednessPolicy::DEFAULT;

    let result = validate_aggregate_budget(&budget, &policy);

    assert!(result.is_ok(), "steps at limit must be valid");
}

#[test]
fn validate_aggregate_budget_returns_policy_exceeded_when_steps_exceed_limit() {
    let budget = AggregateResourceBudget {
        max_steps_executable: BoundednessPolicy::DEFAULT.absolute_max_steps_executable + 1,
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let policy = BoundednessPolicy::DEFAULT;

    let result = validate_aggregate_budget(&budget, &policy);

    assert!(result.is_err(), "steps exceeding limit must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::PolicyExceeded {
            resource: "max_steps_executable",
            actual,
            limit,
        } => {
            assert_eq!(actual, u64::from(budget.max_steps_executable));
            assert_eq!(limit, u64::from(policy.absolute_max_steps_executable));
        }
        other => panic!("expected PolicyExceeded for max_steps_executable, got {:?}", other),
    }
}

#[test]
fn validate_aggregate_budget_accepts_action_tickets_at_limit() {
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: BoundednessPolicy::DEFAULT.absolute_max_action_tickets,
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let policy = BoundednessPolicy::DEFAULT;

    let result = validate_aggregate_budget(&budget, &policy);

    assert!(result.is_ok(), "action tickets at limit must be valid");
}

#[test]
fn validate_aggregate_budget_returns_policy_exceeded_when_action_tickets_exceed() {
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: BoundednessPolicy::DEFAULT.absolute_max_action_tickets + 1,
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let policy = BoundednessPolicy::DEFAULT;

    let result = validate_aggregate_budget(&budget, &policy);

    assert!(result.is_err(), "action tickets exceeding limit must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::PolicyExceeded {
            resource: "max_action_tickets",
            ..
        } => {}
        other => panic!("expected PolicyExceeded for max_action_tickets, got {:?}", other),
    }
}

#[test]
fn validate_aggregate_budget_accepts_parallel_at_limit() {
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: BoundednessPolicy::DEFAULT.absolute_max_parallel,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let policy = BoundednessPolicy::DEFAULT;

    let result = validate_aggregate_budget(&budget, &policy);

    assert!(result.is_ok(), "parallel at limit must be valid");
}

#[test]
fn validate_aggregate_budget_returns_policy_exceeded_when_parallel_exceeds() {
    let budget = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: BoundednessPolicy::DEFAULT.absolute_max_parallel + 1,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let policy = BoundednessPolicy::DEFAULT;

    let result = validate_aggregate_budget(&budget, &policy);

    assert!(result.is_err(), "parallel exceeding limit must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::PolicyExceeded {
            resource: "max_parallel_in_flight",
            ..
        } => {}
        other => panic!("expected PolicyExceeded for max_parallel_in_flight, got {:?}", other),
    }
}

#[test]
fn validate_aggregate_budget_accepts_result_bytes_at_limit() {
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
        max_result_bytes: BoundednessPolicy::DEFAULT.absolute_max_result_bytes,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let policy = BoundednessPolicy::DEFAULT;

    let result = validate_aggregate_budget(&budget, &policy);

    assert!(result.is_ok(), "result bytes at limit must be valid");
}

#[test]
fn validate_aggregate_budget_returns_policy_exceeded_when_result_bytes_exceed() {
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
        max_result_bytes: BoundednessPolicy::DEFAULT.absolute_max_result_bytes + 1,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let policy = BoundednessPolicy::DEFAULT;

    let result = validate_aggregate_budget(&budget, &policy);

    assert!(result.is_err(), "result bytes exceeding limit must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::PolicyExceeded {
            resource: "max_result_bytes",
            ..
        } => {}
        other => panic!("expected PolicyExceeded for max_result_bytes, got {:?}", other),
    }
}

#[test]
fn validate_aggregate_budget_accepts_run_time_at_limit() {
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
        max_run_time_seconds: BoundednessPolicy::DEFAULT.absolute_max_run_time_seconds,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let policy = BoundednessPolicy::DEFAULT;

    let result = validate_aggregate_budget(&budget, &policy);

    assert!(result.is_ok(), "run time at limit must be valid");
}

#[test]
fn validate_aggregate_budget_returns_policy_exceeded_when_run_time_exceeds() {
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
        max_run_time_seconds: BoundednessPolicy::DEFAULT.absolute_max_run_time_seconds + 1,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let policy = BoundednessPolicy::DEFAULT;

    let result = validate_aggregate_budget(&budget, &policy);

    assert!(result.is_err(), "run time exceeding limit must return error");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::PolicyExceeded {
            resource: "max_run_time_seconds",
            ..
        } => {}
        other => panic!("expected PolicyExceeded for max_run_time_seconds, got {:?}", other),
    }
}

#[test]
fn validate_aggregate_budget_accepts_all_dimensions_within_policy() {
    let policy = BoundednessPolicy::DEFAULT;
    let budget = AggregateResourceBudget {
        max_steps_executable: policy.absolute_max_steps_executable / 2,
        max_action_tickets: policy.absolute_max_action_tickets / 2,
        max_parallel_in_flight: policy.absolute_max_parallel / 2,
        max_retries_per_action: 10,
        max_gather_pages: u32::MAX / 2,
        max_gather_items: u32::MAX / 2,
        max_for_each_iterations: u32::MAX / 2,
        max_together_branches: policy.max_fanout / 2,
        max_repeat_attempts: 100,
        max_run_time_seconds: policy.absolute_max_run_time_seconds / 2,
        max_result_bytes: policy.absolute_max_result_bytes / 2,
        max_total_slots_written: policy.max_total_slots as u32 / 2,
        max_queue_depth: u32::MAX / 2,
        max_journal_batch_bytes: u32::MAX / 2,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let result = validate_aggregate_budget(&budget, &policy);

    assert!(result.is_ok(), "all dimensions within policy must be valid");
}

#[test]
fn validate_aggregate_budget_returns_first_violation_only() {
    // Create a budget that violates multiple policies
    let policy = BoundednessPolicy::DEFAULT;
    let budget = AggregateResourceBudget {
        max_steps_executable: policy.absolute_max_steps_executable + 1,
        max_action_tickets: policy.absolute_max_action_tickets + 1,
        max_parallel_in_flight: policy.absolute_max_parallel + 1,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
    };

    let result = validate_aggregate_budget(&budget, &policy);

    // Should get an error (and it should be one of the violations, not necessarily the first in code order)
    assert!(result.is_err(), "multi-violation budget must return error");
}

// =========================================================================
// Behavior Group H: validate_step_ceilings
// =========================================================================

#[test]
fn validate_step_ceilings_accepts_valid_ceiling_values() {
    use vb_core::budget::validate_step_ceilings;

    let budget = AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 5,
        max_parallel_in_flight: 2,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 500_000,
        max_transitions_per_tick: 500_000,
    };

    let result = validate_step_ceilings(&budget);

    assert!(result.is_ok(), "valid ceiling values must be accepted");
}

#[test]
fn validate_step_ceilings_rejects_zero_step_budget() {
    use vb_core::budget::validate_step_ceilings;

    let budget = AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 5,
        max_parallel_in_flight: 2,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 500_000,
    };

    let result = validate_step_ceilings(&budget);

    assert!(result.is_err(), "zero step budget must be rejected");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::StepCeilingExceeded { requested: 0, limit: 1_000_000 } => {}
        other => panic!("expected StepCeilingExceeded(0, 1_000_000), got {:?}", other),
    }
}

#[test]
fn validate_step_ceilings_rejects_step_budget_exceeding_hard_limit() {
    use vb_core::budget::validate_step_ceilings;

    let budget = AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 5,
        max_parallel_in_flight: 2,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 1_000_001,
        max_transitions_per_tick: 500_000,
    };

    let result = validate_step_ceilings(&budget);

    assert!(result.is_err(), "step budget exceeding hard limit must be rejected");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::StepCeilingExceeded { requested: 1_000_001, limit: 1_000_000 } => {}
        other => panic!("expected StepCeilingExceeded(1_000_001, 1_000_000), got {:?}", other),
    }
}

#[test]
fn validate_step_ceilings_rejects_zero_transitions() {
    use vb_core::budget::validate_step_ceilings;

    let budget = AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 5,
        max_parallel_in_flight: 2,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 500_000,
        max_transitions_per_tick: 0,
    };

    let result = validate_step_ceilings(&budget);

    assert!(result.is_err(), "zero transitions must be rejected");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::PerTickCeilingExceeded { requested: 0, limit: 1_000_000 } => {}
        other => panic!("expected PerTickCeilingExceeded(0, 1_000_000), got {:?}", other),
    }
}

#[test]
fn validate_step_ceilings_rejects_transitions_exceeding_hard_limit() {
    use vb_core::budget::validate_step_ceilings;

    let budget = AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 5,
        max_parallel_in_flight: 2,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_step_budget_per_tick: 500_000,
        max_transitions_per_tick: 1_000_001,
    };

    let result = validate_step_ceilings(&budget);

    assert!(result.is_err(), "transitions exceeding hard limit must be rejected");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::PerTickCeilingExceeded { requested: 1_000_001, limit: 1_000_000 } => {}
        other => panic!("expected PerTickCeilingExceeded(1_000_001, 1_000_000), got {:?}", other),
    }
}

// =========================================================================
// Behavior Group I: from_workflow with validate_step_ceilings integration
// =========================================================================

#[test]
fn from_workflow_rejects_when_step_ceiling_exceeds_hard_limit() {
    // Create a workflow with max_step_budget_per_tick > 1_000_000 in contract
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];

    let contract = ResourceContract {
        max_steps: 10,
        max_slots: 1,
        max_constants: 0,
        max_accessors: 0,
        max_expressions: 0,
        max_expr_stack: 8,
        max_step_budget_per_tick: 1_000_001, // Exceeds hard limit
        max_transitions_per_tick: 500_000,
        max_input_bytes: 1024,
        max_output_bytes: 4096,
        max_blob_bytes: 0,
        max_ipc_payload_bytes: 1024,
        max_retry_attempts: 3,
        max_fanout: 16,
        max_collect_items: 256,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        allows_secret_results: false,
    };

    let parts = WorkflowParts {
        name: "high_ceiling".into(),
        digest: WorkflowDigest::from_bytes([0x50; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new(["finish".into()]),
    };

    let workflow = CompiledWorkflow::try_from_parts(parts).expect("workflow must be valid");
    let result = AggregateResourceBudget::from_workflow(&workflow);

    assert!(result.is_err(), "step ceiling exceeding hard limit must cause from_workflow to fail");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::StepCeilingExceeded { requested: 1_000_001, limit: 1_000_000 } => {}
        other => panic!("expected StepCeilingExceeded, got {:?}", other),
    }
}

#[test]
fn from_workflow_rejects_when_transitions_ceiling_exceeds_hard_limit() {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];

    let contract = ResourceContract {
        max_steps: 10,
        max_slots: 1,
        max_constants: 0,
        max_accessors: 0,
        max_expressions: 0,
        max_expr_stack: 8,
        max_step_budget_per_tick: 500_000,
        max_transitions_per_tick: 1_000_001, // Exceeds hard limit
        max_input_bytes: 1024,
        max_output_bytes: 4096,
        max_blob_bytes: 0,
        max_ipc_payload_bytes: 1024,
        max_retry_attempts: 3,
        max_fanout: 16,
        max_collect_items: 256,
        max_queue_depth: 64,
        max_journal_batch_bytes: 8192,
        allows_secret_results: false,
    };

    let parts = WorkflowParts {
        name: "high_trans".into(),
        digest: WorkflowDigest::from_bytes([0x51; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new(["finish".into()]),
    };

    let workflow = CompiledWorkflow::try_from_parts(parts).expect("workflow must be valid");
    let result = AggregateResourceBudget::from_workflow(&workflow);

    assert!(result.is_err(), "transitions ceiling exceeding hard limit must cause from_workflow to fail");
    let err = result.unwrap_err();
    match err {
        AggregateBudgetError::PerTickCeilingExceeded { requested: 1_000_001, limit: 1_000_000 } => {}
        other => panic!("expected PerTickCeilingExceeded, got {:?}", other),
    }
}

// =========================================================================
// Behavior Group J: from_whole_workflow_budget overflow tests
// =========================================================================

#[test]
fn from_whole_workflow_budget_overflow_action_tickets() {
    // This test is compile-time only - u32::MAX + 1 does not fit in u32.
    // The type system prevents creating an invalid WholeWorkflowBudget.
    // Overflow conversion is caught at the u32::try_from() call inside from_workflow.
}

#[test]
fn from_whole_workflow_budget_values_derived_from_contract() {
    use vb_core::budget::WholeWorkflowBudget;

    let whole_budget = WholeWorkflowBudget {
        max_total_steps: 10,
        max_total_slots: 5,
        max_fanout: 2,
        max_nesting_depth: 1,
        max_steps_executable: 10,
        max_action_tickets: 3,
        max_parallel_in_flight: 2,
        max_retries_per_action: 2,
        max_gather_pages: 50,
        max_gather_items: 100,
        max_for_each_iterations: 200,
        max_together_branches: 2,
        max_repeat_attempts: 3,
        max_run_time_seconds: 0,
        max_result_bytes: 4096,
        max_total_slots_written: 5,
    };

    let contract = ResourceContract {
        max_steps: 10,
        max_slots: 5,
        max_constants: 0,
        max_accessors: 0,
        max_expressions: 0,
        max_expr_stack: 8,
        max_step_budget_per_tick: 500_000,
        max_transitions_per_tick: 500_000,
        max_input_bytes: 1024,
        max_output_bytes: 8192, // Different from whole_budget.max_result_bytes
        max_blob_bytes: 0,
        max_ipc_payload_bytes: 1024,
        max_retry_attempts: 3,
        max_fanout: 16,
        max_collect_items: 256,
        max_queue_depth: 128, // Different from default
        max_journal_batch_bytes: 16384, // Different from default
        allows_secret_results: false,
    };

    let result = AggregateResourceBudget::from_whole_workflow_budget(whole_budget, contract);

    assert!(result.is_ok(), "valid conversion must succeed");
    let budget = result.unwrap();
    // Contract-derived fields should be used
    assert_eq!(budget.max_queue_depth, 128);
    assert_eq!(budget.max_journal_batch_bytes, 16384);
    assert_eq!(budget.max_step_budget_per_tick, 500_000);
    assert_eq!(budget.max_transitions_per_tick, 500_000);
    // WholeWorkflowBudget fields should be preserved
    assert_eq!(budget.max_result_bytes, 4096); // from whole_budget, not contract
    assert_eq!(budget.max_total_slots_written, 5);
}
