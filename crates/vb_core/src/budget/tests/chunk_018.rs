//! Test chunk 018 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 4502–4742 of the original. Semantic content is
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
fn fits_within_capacity_exceeded_gather_pages() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 10,
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
            ensure_equal(resource, "max_gather_pages")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_gather_items() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 200,
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
            ensure_equal(resource, "max_gather_items")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_result_bytes() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 2000,
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
        Err(AggregateBudgetError::CapacityExceeded {
            resource: "max_result_bytes",
            requested: 2000,
            available: 1000,
        }) => Ok(()),
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_total_slots_written() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 50,
        max_action_tickets: 50,
        max_parallel_in_flight: 5,
        max_gather_pages: 2,
        max_gather_items: 50,
        max_result_bytes: 500,
        max_total_slots_written: 1000,
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
            ensure_equal(resource, "max_total_slots_written")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}

#[test]
fn fits_within_capacity_exceeded_active_runs() -> Result<(), String> {
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
        max_active_runs: 10,
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
            ensure_equal(resource, "max_active_runs")
        }
        other => Err(format!("expected CapacityExceeded, got {:?}", other)),
    }
}
