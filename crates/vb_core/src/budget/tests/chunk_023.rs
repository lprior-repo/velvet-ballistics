//! Test chunk 023 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 5692–5944 of the original. Semantic content is
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


fn budget_error_from_jump_cycle() -> Result<(), String> {
    let wf_err = WorkflowError::JumpCycle {
        step: StepIdx::new(1),
        target: StepIdx::new(0),
    };
    let budget_err: BudgetError = wf_err.into();
    match budget_err {
        BudgetError::TotalStepsExceeded { actual, limit } => {
            ensure_equal(actual, u64::MAX)?;
            ensure_equal(limit, u64::MAX)
        }
        other => Err(format!(
            "expected TotalStepsExceeded sentinel, got {:?}",
            other
        )),
    }
}

#[test]
fn try_add_budget_exercises_multiple_dimensions() -> Result<(), String> {
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
    let budget = AggregateResourceBudget {
        max_steps_executable: 50,
        max_action_tickets: 25,
        max_parallel_in_flight: 5,
        max_retries_per_action: 2,
        max_gather_pages: 3,
        max_gather_items: 50,
        max_for_each_iterations: 10,
        max_together_branches: 2,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_timer_entries: 3,
        max_trace_events: 4,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 5,
        max_blob_bytes: 6,
        max_input_bytes: 7,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };
    let result = usage.try_add_budget(&budget);
    let added = result.map_err(|e| format!("{:?}", e))?;
    ensure_equal(added.max_steps_executable, 150)?;
    ensure_equal(added.max_action_tickets, 75)?;
    ensure_equal(added.max_parallel_in_flight, 15)?;
    ensure_equal(added.max_gather_pages, 8)?;
    ensure_equal(added.max_gather_items, 150)?;
    ensure_equal(added.max_result_bytes, 1500)?;
    ensure_equal(added.max_total_slots_written, 750)?;
    ensure_equal(added.max_timer_entries, 10)?;
    ensure_equal(added.max_trace_events, 12)?;
    ensure_equal(added.max_active_runs, 6)?;
    ensure_equal(added.max_queue_depth, 30)?;
    ensure_equal(added.max_journal_batch_bytes, 6144)?;
    ensure_equal(added.max_ipc_payload_bytes, 14)?;
    ensure_equal(added.max_blob_bytes, 16)?;
    ensure_equal(added.max_input_bytes, 18)?;
    ensure_equal(added.max_step_budget_per_tick, 1500)?;
    ensure_equal(added.max_transitions_per_tick, 750)
}

#[test]
fn try_subtract_budget_exercises_multiple_dimensions() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 150,
        max_action_tickets: 75,
        max_parallel_in_flight: 15,
        max_gather_pages: 8,
        max_gather_items: 150,
        max_result_bytes: 1500,
        max_total_slots_written: 750,
        max_timer_entries: 10,
        max_trace_events: 12,
        max_active_runs: 6,
        max_queue_depth: 30,
        max_journal_batch_bytes: 6144,
        max_ipc_payload_bytes: 14,
        max_blob_bytes: 16,
        max_input_bytes: 18,
        max_step_budget_per_tick: 1500,
        max_transitions_per_tick: 750,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 50,
        max_action_tickets: 25,
        max_parallel_in_flight: 5,
        max_retries_per_action: 2,
        max_gather_pages: 3,
        max_gather_items: 50,
        max_for_each_iterations: 10,
        max_together_branches: 2,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 500,
        max_total_slots_written: 250,
        max_timer_entries: 3,
        max_trace_events: 4,
        max_queue_depth: 10,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 5,
        max_blob_bytes: 6,
        max_input_bytes: 7,
        max_step_budget_per_tick: 500,
        max_transitions_per_tick: 250,
    };
    let result = usage.try_subtract_budget(&budget);
    let subtracted = result.map_err(|e| format!("{:?}", e))?;
    ensure_equal(subtracted.max_steps_executable, 100)?;
    ensure_equal(subtracted.max_action_tickets, 50)?;
    ensure_equal(subtracted.max_parallel_in_flight, 10)?;
    ensure_equal(subtracted.max_gather_pages, 5)?;
    ensure_equal(subtracted.max_gather_items, 100)?;
    ensure_equal(subtracted.max_result_bytes, 1000)?;
    ensure_equal(subtracted.max_total_slots_written, 500)?;
    ensure_equal(subtracted.max_timer_entries, 7)?;
    ensure_equal(subtracted.max_trace_events, 8)?;
    ensure_equal(subtracted.max_active_runs, 5)?;
    ensure_equal(subtracted.max_queue_depth, 20)?;
    ensure_equal(subtracted.max_journal_batch_bytes, 4096)?;
    ensure_equal(subtracted.max_ipc_payload_bytes, 9)?;
    ensure_equal(subtracted.max_blob_bytes, 10)?;
    ensure_equal(subtracted.max_input_bytes, 11)?;
    ensure_equal(subtracted.max_step_budget_per_tick, 1000)?;
    ensure_equal(subtracted.max_transitions_per_tick, 500)
}

#[test]
fn try_add_budget_overflow_action_tickets_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: u64::MAX - 1,
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
        max_steps_executable: 0,
        max_action_tickets: 2,
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
            ensure_equal(resource, "max_action_tickets")
        }
        other => Err(format!("expected Overflow, got {:?}", other)),
    }
}

#[test]
fn try_add_budget_overflow_parallel_in_flight_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: u64::MAX - 1,
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
        max_steps_executable: 0,
        max_action_tickets: 0,
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
            ensure_equal(resource, "max_parallel_in_flight")
        }
        other => Err(format!("expected Overflow, got {:?}", other)),
    }
}

#[test]
