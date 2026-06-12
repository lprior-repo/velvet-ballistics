//! Test chunk 020 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 4972–5223 of the original. Semantic content is
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
fn validate_aggregate_budget_reports_extended_payload_dimensions() -> Result<(), String> {
    let policy = BoundednessPolicy {
        absolute_max_timer_entries: 12,
        absolute_max_trace_events: 13,
        absolute_max_queue_depth: 14,
        absolute_max_journal_batch_bytes: 15,
        absolute_max_ipc_payload_bytes: 16,
        absolute_max_blob_bytes: 17,
        absolute_max_input_bytes: 18,
        ..BoundednessPolicy::DEFAULT
    };
    let base = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 12,
        max_trace_events: 13,
        max_queue_depth: 14,
        max_journal_batch_bytes: 15,
        max_ipc_payload_bytes: 16,
        max_blob_bytes: 17,
        max_input_bytes: 18,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };

    let mut over = base;
    over.max_timer_entries = 13;
    assert_policy_exceeded(
        crate::budget::validate_aggregate_budget(&over, &policy),
        "max_timer_entries",
        13,
        12,
    )?;
    over = base;
    over.max_trace_events = 14;
    assert_policy_exceeded(
        crate::budget::validate_aggregate_budget(&over, &policy),
        "max_trace_events",
        14,
        13,
    )?;
    over = base;
    over.max_journal_batch_bytes = 16;
    assert_policy_exceeded(
        crate::budget::validate_aggregate_budget(&over, &policy),
        "max_journal_batch_bytes",
        16,
        15,
    )?;
    over = base;
    over.max_queue_depth = 15;
    assert_policy_exceeded(
        crate::budget::validate_aggregate_budget(&over, &policy),
        "max_queue_depth",
        15,
        14,
    )?;
    over = base;
    over.max_ipc_payload_bytes = 17;
    assert_policy_exceeded(
        crate::budget::validate_aggregate_budget(&over, &policy),
        "max_ipc_payload_bytes",
        17,
        16,
    )?;
    over = base;
    over.max_blob_bytes = 18;
    assert_policy_exceeded(
        crate::budget::validate_aggregate_budget(&over, &policy),
        "max_blob_bytes",
        18,
        17,
    )?;
    over = base;
    over.max_input_bytes = 19;
    assert_policy_exceeded(
        crate::budget::validate_aggregate_budget(&over, &policy),
        "max_input_bytes",
        19,
        18,
    )
}

fn assert_policy_exceeded(
    actual: Result<(), AggregateBudgetError>,
    expected_resource: &'static str,
    expected_actual: u64,
    expected_limit: u64,
) -> Result<(), String> {
    match actual {
        Err(AggregateBudgetError::PolicyExceeded {
            resource,
            actual,
            limit,
        }) if resource == expected_resource
            && actual == expected_actual
            && limit == expected_limit =>
        {
            Ok(())
        }
        other => Err(format!(
            "expected PolicyExceeded {{ resource: {expected_resource}, actual: {expected_actual}, limit: {expected_limit} }}, got {other:?}"
        )),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_steps() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 2_000_000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let policy = BoundednessPolicy::DEFAULT;
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_steps_executable")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_action_tickets() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 200_000,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let policy = BoundednessPolicy::DEFAULT;
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_action_tickets")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_parallel_in_flight() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 512,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let policy = BoundednessPolicy::DEFAULT;
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_parallel_in_flight")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_run_time() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 5,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3_000_000,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let policy = BoundednessPolicy::DEFAULT;
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_run_time_seconds")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}
