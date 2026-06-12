//! Test chunk 021 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 5225–5480 of the original. Semantic content is
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
fn validate_aggregate_budget_rejects_exceeded_result_bytes() -> Result<(), String> {
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
        max_run_time_seconds: 3600,
        max_result_bytes: 300_000,
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
        Err(AggregateBudgetError::PolicyExceeded {
            resource: "max_result_bytes",
            actual: 300_000,
            limit: 262_144,
        }) => Ok(()),
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_total_slots() -> Result<(), String> {
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
        max_run_time_seconds: 3600,
        max_result_bytes: 65536,
        max_total_slots_written: 100_000,
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
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        max_total_slots: 65_535,
        max_fanout: 64,
        max_nesting_depth: 8,
        absolute_max_action_tickets: 100_000,
        absolute_max_parallel: 256,
        absolute_max_run_time_seconds: 2_592_000,
        absolute_max_result_bytes: 262_144,
        absolute_max_steps_executable: 1_000_000,
        ..BoundednessPolicy::DEFAULT
    };
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_total_slots_written")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

#[test]
fn validate_aggregate_budget_rejects_exceeded_together_branches() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_steps_executable: 1000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 3,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 100,
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
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        max_total_slots: 65_535,
        max_fanout: 64,
        max_nesting_depth: 8,
        absolute_max_action_tickets: 100_000,
        absolute_max_parallel: 256,
        absolute_max_run_time_seconds: 2_592_000,
        absolute_max_result_bytes: 262_144,
        absolute_max_steps_executable: 1_000_000,
        ..BoundednessPolicy::DEFAULT
    };
    match crate::budget::validate_aggregate_budget(&budget, &policy) {
        Err(AggregateBudgetError::PolicyExceeded { resource, .. }) => {
            ensure_equal(resource, "max_together_branches")
        }
        other => Err(format!("expected PolicyExceeded, got {:?}", other)),
    }
}

// -------------------------------------------------------------------------
// validate_step_ceilings tests
// -------------------------------------------------------------------------

#[test]
fn validate_step_ceilings_accepts_valid() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 5000,
        max_transitions_per_tick: 500,
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
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
    };
    ensure_equal(crate::budget::validate_step_ceilings(&budget), Ok(()))
}

#[test]
fn validate_step_ceilings_rejects_zero_step_budget() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 500,
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
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
    };
    match crate::budget::validate_step_ceilings(&budget) {
        Err(AggregateBudgetError::StepCeilingExceeded { requested: 0, .. }) => Ok(()),
        other => Err(format!("expected StepCeilingExceeded(0), got {:?}", other)),
    }
}

#[test]
fn validate_step_ceilings_rejects_zero_transitions() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 5000,
        max_transitions_per_tick: 0,
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
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
    };
    match crate::budget::validate_step_ceilings(&budget) {
        Err(AggregateBudgetError::PerTickCeilingExceeded { requested: 0, .. }) => Ok(()),
        other => Err(format!(
            "expected PerTickCeilingExceeded(0), got {:?}",
            other
        )),
    }
}

#[test]
fn validate_step_ceilings_rejects_step_over_hard_limit() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 2_000_000,
        max_transitions_per_tick: 500,
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
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
    };
    match crate::budget::validate_step_ceilings(&budget) {
        Err(AggregateBudgetError::StepCeilingExceeded {
            requested: 2_000_000,
            ..
        }) => Ok(()),
        other => Err(format!("expected StepCeilingExceeded, got {:?}", other)),
    }
}
