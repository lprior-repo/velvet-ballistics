#![allow(unused_imports, dead_code)]
//! Test chunk 012 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 2921–3172 of the original. Semantic content is
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
fn foreach_multi_step_body() -> Result<(), String> {
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
                limit: 3,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(5, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 11)
}

// -------------------------------------------------------------------------
// Additional coverage: RepeatStart with max_attempts=1
// -------------------------------------------------------------------------

#[test]
fn repeat_start_one_attempt() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 1,
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
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 3)?;
    ensure_equal(budget.max_repeat_attempts, 1)
}

// -------------------------------------------------------------------------
// Additional coverage: Policy validates first violation only
// -------------------------------------------------------------------------

#[test]
fn policy_reports_first_violation_steps_over_slots_over() -> Result<(), String> {
    let mut budget = test_budget(2_000_000, 200_000, 100, 20);
    budget.max_action_tickets = 500_000;
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
    match policy.validate(&budget) {
        Err(BudgetError::TotalStepsExceeded { actual, limit }) => {
            ensure_equal(actual, 2_000_000)?;
            ensure_equal(limit, 1_000_000)
        }
        other => Err(format!("expected TotalStepsExceeded, got {other:?}")),
    }
}

// -------------------------------------------------------------------------
// Additional coverage: WholeWorkflowBudget max_steps_executable derivation
// -------------------------------------------------------------------------

#[test]
fn max_steps_executable_equals_total_steps_when_under_u32_max() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(2, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    let expected_executable = u32::try_from(budget.max_total_steps).unwrap_or(u32::MAX);
    ensure_equal(budget.max_steps_executable, expected_executable)?;
    ensure_equal(budget.max_steps_executable, 2)
}

// -------------------------------------------------------------------------
// Additional coverage: max_total_slots_written equals contract max_slots
// -------------------------------------------------------------------------

#[test]
fn max_total_slots_written_equals_contract_max_slots() -> Result<(), String> {
    let nodes = single_node_workflow();
    let contract = test_contract(1, 42);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_slots_written, 42)
}

// -------------------------------------------------------------------------
// Additional coverage: ErrorHandler node step counting
// -------------------------------------------------------------------------

#[test]
fn error_handler_counts_body_and_handler() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: None,
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
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
    let contract = test_contract(3, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 3)
}

// -------------------------------------------------------------------------
// Additional coverage: BoundednessPolicy validate returns checks in order
// -------------------------------------------------------------------------

#[test]
fn policy_check_order_total_steps_before_slots() -> Result<(), String> {
    let budget = test_budget(2_000_000, 200_000, 0, 0);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    match result {
        Err(BudgetError::TotalStepsExceeded { .. }) => Ok(()),
        other => Err(format!(
            "expected TotalStepsExceeded (first check), got {other:?}"
        )),
    }
}

#[test]
fn policy_check_order_slots_before_fanout() -> Result<(), String> {
    let budget = test_budget(100, 200_000, 100, 0);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    match result {
        Err(BudgetError::TotalSlotsExceeded { .. }) => Ok(()),
        other => Err(format!(
            "expected TotalSlotsExceeded (second check), got {other:?}"
        )),
    }
}
