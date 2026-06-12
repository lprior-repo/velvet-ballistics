//! Test chunk 027 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 6725–6979 of the original. Semantic content is
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


fn try_subtract_budget_underflow_journal_batch_bytes_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 2,
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
        max_journal_batch_bytes: 1,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource }) => {
            ensure_equal(resource, "max_journal_batch_bytes")
        }
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

#[test]
fn try_subtract_budget_underflow_step_budget_per_tick_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 2,
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
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 0,
    };
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource }) => {
            ensure_equal(resource, "max_step_budget_per_tick")
        }
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

#[test]
fn try_subtract_budget_underflow_transitions_per_tick_dimension() -> Result<(), String> {
    let usage = AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 2,
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
        max_transitions_per_tick: 1,
    };
    match usage.try_subtract_budget(&budget) {
        Err(AggregateBudgetError::Underflow { resource }) => {
            ensure_equal(resource, "max_transitions_per_tick")
        }
        other => Err(format!("expected Underflow, got {:?}", other)),
    }
}

// ============================================================================
// Mutation-killing tests for production code survivors
// These target mutations that survive when boundary-value tests are missing.
// ============================================================================

/// Kills: validate_step_ceilings > with >= at lines 740, 753
/// The mutation replaces `> HARD_MAX` with `>= HARD_MAX`, which would reject
/// values exactly at the hard limit. This test uses exact boundary values.
#[test]
fn validate_step_ceilings_accepts_exact_hard_limit() -> Result<(), String> {
    // HARD_MAX_STEP_BUDGET_PER_TICK = 1_000_000
    // HARD_MAX_TRANSITIONS_PER_TICK = 1_000_000
    // The production code uses `>` (strict), so value == 1_000_000 should pass.
    // The mutation `>` → `>=` would incorrectly reject 1_000_000.
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 1_000_000,
        max_transitions_per_tick: 1_000_000,
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

/// Kills: check_capacity > with >= at line 788 (via WholeWorkflowBudget path)
/// When current == limit, should NOT error. Mutation `>` → `>=` would fail.
/// This tests the boundary through the public add_budget API.
#[test]
fn whole_workflow_budget_add_at_exact_limit() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
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
    let mut usage = AggregateResourceUsage::default();
    // Set usage to exactly the limit
    usage.max_steps_executable = 1000;
    // Adding the budget should succeed (usage starts at 0, budget adds 1000 to steps = 2000)
    let expected = AggregateResourceUsage {
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
        max_steps_executable: 2000,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_gather_pages: 5,
        max_gather_items: 100,
        max_result_bytes: 65536,
        max_total_slots_written: 1000,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 50,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_active_runs: 1,
    };
    match usage.try_add_budget(&budget) {
        Ok(actual) => ensure_equal(actual, expected),
        Err(e) => Err(format!("unexpected error: {:?}", e)),
    }
}

/// Kills: check_policy > with >= at line 804 (via WholeWorkflowBudget path)
/// When usage == limit, policy check should pass.
#[test]
