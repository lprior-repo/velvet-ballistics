//! Test chunk 004 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 786–1053 of the original. Semantic content is
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
fn budget_large_loop_counted_realistically() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 1000,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 1002);

    assert!(budget.is_some(), "large loop should count 1002 steps not 3");

    // The default policy (1000 steps) should REJECT this (1002 > 1000).
    let budget_val = budget.as_ref().unwrap();
    assert!(
        BoundednessPolicy::DEFAULT.validate(budget_val).is_err(),
        "1002 steps should exceed the default policy (1000-step cap)"
    );
}

/// Verifies that a CollectStart loop multiplies body steps by the limit.
#[test]
fn budget_collect_loop_multiplies_body_steps() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 5,
                page_size: 2,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 7);

    assert!(
        budget.is_some(),
        "collect loop should multiply body steps by limit"
    );
}

/// Verifies that StepBudget clamps values above MAX_STEP_BUDGET.
#[test]
fn step_budget_clamps_above_max() {
    let budget = StepBudget::new(crate::limits::MAX_STEP_BUDGET + 100);
    assert_eq!(
        budget.remaining(),
        crate::limits::MAX_STEP_BUDGET,
        "budget should be clamped to MAX_STEP_BUDGET"
    );
}

/// Verifies that StepBudget::MAX equals MAX_STEP_BUDGET.
#[test]
fn step_budget_max_equals_limit() {
    assert_eq!(
        StepBudget::MAX.remaining(),
        crate::limits::MAX_STEP_BUDGET,
        "StepBudget::MAX should equal MAX_STEP_BUDGET"
    );
}

/// Verifies that StepBudget zero budget exhausts immediately.
#[test]
fn step_budget_zero_exhausts_immediately() {
    let mut budget = StepBudget::new(0);
    let result = budget.try_take();
    assert!(
        result.is_ok() && result.as_ref().map_err(|_| "").unwrap() == &false,
        "zero budget should return Ok(false) immediately"
    );
}

// =========================================================================
// BLACKHAT adversarial tests -- budget overflow, bypass, and edge cases
// =========================================================================

// --- FINDING BH-BUD-01: max_steps_executable silent saturation bypass ---

#[test]
fn blackhat_steps_executable_saturates_on_large_total() {
    let budget = WholeWorkflowBudget {
        max_total_steps: u64::from(u32::MAX) + 1,
        max_total_slots: 0,
        max_fanout: 0,
        max_nesting_depth: 0,
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
        max_timer_entries: 7,
        max_trace_events: 8,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 9,
        max_blob_bytes: 10,
        max_input_bytes: 11,
        max_queue_depth: 0,
    };

    let saturated = u32::try_from(budget.max_total_steps).unwrap_or(u32::MAX);
    assert_eq!(
        saturated,
        u32::MAX,
        "BLACKHAT BH-BUD-01: u32 saturation hides overflow"
    );
    assert!(
        budget.max_total_steps > u64::from(saturated),
        "BLACKHAT BH-BUD-01: true count exceeds reported executable steps"
    );
}

// --- FINDING BH-BUD-02: max_run_time_seconds hardcoded to 0 ---

#[test]
fn blackhat_run_time_seconds_always_zero_in_computed_budget() {
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
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_run_time_seconds == 0);

    assert!(
        budget.is_none(),
        "BLACKHAT BH-BUD-02: max_run_time_seconds must not be hardcoded to 0"
    );
}

// --- FINDING BH-BUD-03: From<WorkflowError> for BudgetError loses information ---

#[test]
fn blackhat_workflow_error_to_budget_error_produces_equal_actual_and_limit() {
    let workflow_err = WorkflowError::EntryOutOfBounds {
        entry: StepIdx::new(5),
    };
    let budget_err: BudgetError = workflow_err.into();

    match budget_err {
        BudgetError::TotalStepsExceeded { actual, limit } => {
            assert_eq!(actual, u64::MAX);
            assert_eq!(limit, u64::MAX);
        }
        other => panic!("BLACKHAT BH-BUD-03: unexpected variant: {other:?}"),
    }
}

// --- FINDING BH-BUD-04: ForEachStart limit=0 counts as 1 iteration ---

#[test]
fn blackhat_foreach_limit_zero_still_counts_as_one_iteration() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 0,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(3, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 3);

    assert!(
        budget.is_some(),
        "BLACKHAT BH-BUD-04: limit=0 counts as 1 iteration"
    );
}
