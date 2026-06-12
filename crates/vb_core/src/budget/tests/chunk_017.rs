//! Test chunk 017 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 4231–4500 of the original. Semantic content is
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
fn aggregate_resource_usage_try_add_budget_overflow() -> Result<(), String> {
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
    match usage.try_add_budget(&budget) {
        Err(AggregateBudgetError::Overflow { resource }) => {
            ensure_equal(resource, "max_steps_executable")
        }
        other => Err(format!("expected Overflow, got {:?}", other)),
    }
}

#[test]
fn aggregate_resource_usage_try_subtract_budget_underflow() -> Result<(), String> {
    let usage = AggregateResourceUsage::default();
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
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource: _ }) => Ok(()),
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

#[test]
fn aggregate_resource_usage_fits_within_capacity() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 100,
        max_action_tickets: 50,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 1000,
        max_total_slots_written: 500,
        max_timer_entries: 7,
        max_trace_events: 8,
        max_active_runs: 5,
        max_queue_depth: 20,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 9,
        max_blob_bytes: 10,
        max_input_bytes: 11,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 200,
        max_action_tickets: 100,
        max_parallel_in_flight: 20,
        max_gather_pages: 10,
        max_gather_items: 200,
        max_result_bytes: 2000,
        max_total_slots_written: 1000,
        max_timer_entries: 14,
        max_trace_events: 16,
        max_active_runs: 10,
        max_queue_depth: 40,
        max_journal_batch_bytes: 8192,
        max_ipc_payload_bytes: 18,
        max_blob_bytes: 20,
        max_input_bytes: 22,
        max_step_budget_per_tick: 2000,
        max_transitions_per_tick: 1000,
    };
    ensure_equal(usage.fits_within(&capacity), Ok(()))
}

#[test]
fn aggregate_resource_usage_fits_within_rejects_insufficient() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 300,
        max_action_tickets: 50,
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
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 200,
        max_action_tickets: 100,
        max_parallel_in_flight: 20,
        max_gather_pages: 10,
        max_gather_items: 200,
        max_result_bytes: 2000,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 10,
        max_queue_depth: 40,
        max_journal_batch_bytes: 8192,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 2000,
        max_transitions_per_tick: 1000,
    };
    match usage.fits_within(&capacity) {
        Err(AggregateBudgetError::CapacityExceeded { resource, .. }) => {
            ensure_equal(resource, "max_steps_executable")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_action_tickets() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 150,
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
            ensure_equal(resource, "max_action_tickets")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_parallel_in_flight() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 15,
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
            ensure_equal(resource, "max_parallel_in_flight")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}
