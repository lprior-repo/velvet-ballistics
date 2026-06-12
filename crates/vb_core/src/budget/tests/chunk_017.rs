//! Test chunk 017 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 4145–4406 of the original. Semantic content is
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


fn whole_workflow_budget_jump_cycle_detected_in_compute() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(0),
            },
        },
    ];
    let contract = test_contract(2, 1);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::JumpCycle { step, target }) => {
            ensure_equal(step, StepIdx::new(1))?;
            ensure_equal(target, StepIdx::new(0))
        }
        other => Err(format!("expected JumpCycle, got {:?}", other)),
    }
}

#[test]
fn whole_workflow_budget_step_out_of_bounds_in_visit() -> Result<(), String> {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: Some(StepIdx::new(99)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }];
    let contract = test_contract(1, 0);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::StepOutOfBounds { step }) => ensure_equal(step, StepIdx::new(99)),
        other => Err(format!("expected StepOutOfBounds, got {:?}", other)),
    }
}

// -------------------------------------------------------------------------
// AggregateResourceBudget and AggregateResourceUsage
// -------------------------------------------------------------------------

#[test]
fn aggregate_resource_budget_from_whole_workflow_budget() -> Result<(), String> {
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
    let contract = test_contract(1, 1);
    let wfb = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| format!("{:?}", e))?;
    let arb = crate::budget::AggregateResourceBudget::from_whole_workflow_budget(wfb, contract)
        .map_err(|e| format!("{:?}", e))?;
    ensure_equal(arb.max_steps_executable, wfb.max_steps_executable)?;
    ensure_equal(arb.max_action_tickets, wfb.max_action_tickets)?;
    ensure_equal(arb.max_parallel_in_flight, wfb.max_parallel_in_flight)?;
    ensure_equal(arb.max_timer_entries, wfb.max_timer_entries)?;
    ensure_equal(arb.max_trace_events, wfb.max_trace_events)?;
    ensure_equal(arb.max_result_bytes, 1)?;
    ensure_equal(arb.max_queue_depth, 1)?;
    ensure_equal(arb.max_journal_batch_bytes, 1)?;
    ensure_equal(arb.max_ipc_payload_bytes, 1)?;
    ensure_equal(arb.max_blob_bytes, 1)?;
    ensure_equal(arb.max_input_bytes, 1)
}

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
