//! Test chunk 022 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 5415–5691 of the original. Semantic content is
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

#[test]
fn validate_step_ceilings_rejects_transitions_over_hard_limit() -> Result<(), String> {
    let budget = AggregateResourceBudget {
        max_step_budget_per_tick: 5000,
        max_transitions_per_tick: 2_000_000,
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
        Err(AggregateBudgetError::PerTickCeilingExceeded {
            requested: 2_000_000,
            ..
        }) => Ok(()),
        other => Err(format!("expected PerTickCeilingExceeded, got {:?}", other)),
    }
}

// -------------------------------------------------------------------------
// AggregateCapacity and AggregateReservation
// -------------------------------------------------------------------------

#[test]
fn aggregate_resource_capacity_is_copy() -> Result<(), String> {
    let cap = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 100,
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
    let copy = cap;
    ensure_equal(cap.max_steps_executable, copy.max_steps_executable)
}

#[test]
fn aggregate_resource_usage_is_copy() -> Result<(), String> {
    let usage = AggregateResourceUsage::default();
    let copy = usage;
    ensure_equal(usage.max_steps_executable, copy.max_steps_executable)
}

#[test]
fn aggregate_reservation_debug_format() -> Result<(), String> {
    let reservation = crate::budget::AggregateReservation {
        run: crate::ids::RunId::new(42),
        requested: AggregateResourceBudget {
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
        },
    };
    let debug = format!("{:?}", reservation);
    ensure_equal(debug.contains("AggregateReservation"), true)
}

// -------------------------------------------------------------------------
// AggregateBudgetError Debug
// -------------------------------------------------------------------------

#[test]
fn aggregate_budget_error_workflow_debug() -> Result<(), String> {
    let err = AggregateBudgetError::WorkflowBudget(WorkflowError::EntryOutOfBounds {
        entry: StepIdx::new(5),
    });
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_policy_exceeded_debug() -> Result<(), String> {
    let err = AggregateBudgetError::PolicyExceeded {
        resource: "max_steps",
        actual: 100,
        limit: 50,
    };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_capacity_exceeded_debug() -> Result<(), String> {
    let err = AggregateBudgetError::CapacityExceeded {
        resource: "max_steps",
        requested: 100,
        available: 50,
    };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_overflow_debug() -> Result<(), String> {
    let err = AggregateBudgetError::Overflow { resource: "cpu" };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_underflow_debug() -> Result<(), String> {
    let err = AggregateBudgetError::Underflow { resource: "cpu" };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_invalid_capacity_debug() -> Result<(), String> {
    let err = AggregateBudgetError::InvalidCapacity { resource: "cpu" };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_reservation_not_found_debug() -> Result<(), String> {
    let err = AggregateBudgetError::ReservationNotFound {
        run: crate::ids::RunId::new(42),
    };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_step_ceiling_exceeded_debug() -> Result<(), String> {
    let err = AggregateBudgetError::StepCeilingExceeded {
        requested: 100,
        limit: 50,
    };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

#[test]
fn aggregate_budget_error_per_tick_ceiling_exceeded_debug() -> Result<(), String> {
    let err = AggregateBudgetError::PerTickCeilingExceeded {
        requested: 100,
        limit: 50,
    };
    let debug = format!("{:?}", err);
    ensure_equal(debug.is_empty(), false)
}

// -------------------------------------------------------------------------
// BudgetError from WorkflowError
// -------------------------------------------------------------------------

#[test]
fn budget_error_from_step_out_of_bounds() -> Result<(), String> {
    let wf_err = WorkflowError::StepOutOfBounds {
        step: StepIdx::new(10),
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
