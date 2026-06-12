//! Test chunk 019 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 4744–4970 of the original. Semantic content is
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
fn fits_within_capacity_exceeded_queue_depth() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 3,
        max_queue_depth: 50,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_queue_depth")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_journal_batch_bytes() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 8192,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_journal_batch_bytes")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_step_budget_per_tick() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 2000,
        max_transitions_per_tick: 250,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_step_budget_per_tick")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_transitions_per_tick() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 3,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 1000,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_transitions_per_tick")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

// -------------------------------------------------------------------------
// validate_aggregate_budget tests
// -------------------------------------------------------------------------

#[test]
fn validate_aggregate_budget_accepts_valid_budget() -> Result<(), String> {
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
    ensure_equal(
        crate::budget::validate_aggregate_budget(&budget, &policy),
        Ok(()),
    )
}
