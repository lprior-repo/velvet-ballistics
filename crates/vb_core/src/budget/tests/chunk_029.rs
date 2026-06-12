//! Test chunk 029 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 7257–7339 of the original. Semantic content is
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


fn sub_dim_returns_underflow_when_current_is_zero() {
    // B-BUDGET-003: sub_dim returns Underflow when current == 0 && requested > 0
    let current = 0u64;
    let requested = 1u64;
    let result = crate::budget::sub_dim(current, requested, "test_resource");
    assert!(result.is_err(), "sub_dim must return error for underflow");
    match result {
        Err(AggregateBudgetError::Underflow { resource }) => {
            assert_eq!(resource, "test_resource");
        }
        other => panic!("expected Underflow, got {:?}", other),
    }
}

#[test]
fn fits_within_returns_capacity_exceeded_when_requested_greater_than_available() {
    // B-BUDGET-005: fits_within returns CapacityExceeded when requested > available
    let usage = AggregateResourceUsage {
        max_steps_executable: 100,
        ..Default::default()
    };
    let capacity = crate::budget::AggregateResourceCapacity {
        max_steps_executable: 50,
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
    let result = usage.fits_within(&capacity);
    assert!(result.is_err(), "fits_within must reject over-capacity");
    match result {
        Err(AggregateBudgetError::CapacityExceeded {
            resource,
            requested,
            available,
        }) => {
            assert_eq!(resource, "max_steps_executable");
            assert_eq!(requested, 100);
            assert_eq!(available, 50);
        }
        other => panic!("expected CapacityExceeded, got {:?}", other),
    }
}

#[test]
fn check_policy_returns_policy_exceeded_when_actual_greater_than_limit() {
    // B-BUDGET-006: check_policy returns PolicyExceeded when actual > limit
    // Uses BoundednessPolicy::DEFAULT which has absolute_max_steps_executable = 1_000_000
    let usage = AggregateResourceUsage {
        max_steps_executable: 2_000_000, // exceeds DEFAULT limit of 1_000_000
        ..Default::default()
    };
    let policy = BoundednessPolicy::DEFAULT;
    let result = usage.check_policy(&policy);
    assert!(
        result.is_err(),
        "check_policy must reject policy-exceeding usage"
    );
    match result {
        Err(AggregateBudgetError::PolicyExceeded {
            resource,
            actual,
            limit,
        }) => {
            assert_eq!(resource, "max_steps_executable");
            assert_eq!(actual, 2_000_000);
            assert_eq!(limit, 1_000_000);
        }
        other => panic!("expected PolicyExceeded, got {:?}", other),
    }
}
