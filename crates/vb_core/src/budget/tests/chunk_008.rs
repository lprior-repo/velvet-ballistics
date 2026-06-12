#![allow(unused_imports, dead_code)]
//! Test chunk 008 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 1858–2108 of the original. Semantic content is
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
fn repeat_start_body_accounting() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 7,
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
    // Cold-AST conservative iter count is 1, so body_count * 1 = 1 (header + 1 body + 1 finish).
    ensure_equal(budget.max_total_steps, 1 + 1 + 1)?;
    // `max_repeat_attempts` is the user-declared max_attempts and is tracked
    // separately from the step-count budget.
    ensure_equal(budget.max_repeat_attempts, 7)
}

// -------------------------------------------------------------------------
// 5. Max step budget boundary (exactly at limit, one step over)
// -------------------------------------------------------------------------

#[test]
fn policy_allows_budget_at_exact_total_steps_limit() -> Result<(), String> {
    let budget = test_budget(1_000_000, 0, 0, 0);
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        ..BoundednessPolicy::DEFAULT
    };
    ensure_equal(policy.validate(&budget), Ok(()))
}

#[test]
fn policy_rejects_budget_one_over_total_steps_limit() -> Result<(), String> {
    let budget = test_budget(1_000_001, 0, 0, 0);
    let policy = BoundednessPolicy {
        max_total_steps: 1_000_000,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::TotalStepsExceeded { actual, limit }) => {
            ensure_equal(actual, 1_000_001)?;
            ensure_equal(limit, 1_000_000)
        }
        other => Err(format!("expected TotalStepsExceeded, got {other:?}")),
    }
}

#[test]
fn policy_boundary_exact_fanout() -> Result<(), String> {
    let budget = test_budget(1, 0, 64, 0);
    let policy = BoundednessPolicy {
        max_fanout: 64,
        ..BoundednessPolicy::DEFAULT
    };
    ensure_equal(policy.validate(&budget), Ok(()))
}

#[test]
fn policy_boundary_fanout_one_over() -> Result<(), String> {
    let budget = test_budget(1, 0, 65, 0);
    let policy = BoundednessPolicy {
        max_fanout: 64,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::FanoutExceeded { actual, limit }) => {
            ensure_equal(actual, 65)?;
            ensure_equal(limit, 64)
        }
        other => Err(format!("expected FanoutExceeded, got {other:?}")),
    }
}

#[test]
fn policy_boundary_exact_nesting_depth() -> Result<(), String> {
    let budget = test_budget(1, 0, 0, 8);
    let policy = BoundednessPolicy {
        max_nesting_depth: 8,
        ..BoundednessPolicy::DEFAULT
    };
    ensure_equal(policy.validate(&budget), Ok(()))
}

#[test]
fn policy_boundary_nesting_depth_one_over() -> Result<(), String> {
    let budget = test_budget(1, 0, 0, 9);
    let policy = BoundednessPolicy {
        max_nesting_depth: 8,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::NestingDepthExceeded { actual, limit }) => {
            ensure_equal(actual, 9)?;
            ensure_equal(limit, 8)
        }
        other => Err(format!("expected NestingDepthExceeded, got {other:?}")),
    }
}

// -------------------------------------------------------------------------
// 6. Budget reset/reinitialization
// -------------------------------------------------------------------------

#[test]
fn step_budget_recreated_after_exhaustion() -> Result<(), String> {
    let mut b = StepBudget::new(2);
    b.try_take().map_err(|e| e.to_string())?;
    b.try_take().map_err(|e| e.to_string())?;
    ensure_equal(b.remaining(), 0)?;

    let mut b2 = StepBudget::new(2);
    ensure_equal(b2.remaining(), 2)?;
    ensure_equal(b2.try_take().map_err(|e| e.to_string())?, true)?;
    ensure_equal(b2.remaining(), 1)
}

#[test]
fn whole_workflow_budget_recompute_produces_same_result() -> Result<(), String> {
    let nodes = single_node_workflow();
    let contract = test_contract(1, 1);

    let budget1 = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    let budget2 = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;

    ensure_equal(budget1, budget2)
}

// -------------------------------------------------------------------------
// 7. Nested loop budget computation
// -------------------------------------------------------------------------

#[test]
fn nested_for_each_triple_depth() -> Result<(), String> {
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
                limit: 2,
                body: StepIdx::new(1),
                done: StepIdx::new(6),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(2),
                item_slot: SlotIdx::new(3),
                limit: 3,
                body: StepIdx::new(2),
                done: StepIdx::new(5),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(4),
                item_slot: SlotIdx::new(5),
                limit: 4,
                body: StepIdx::new(3),
                done: StepIdx::new(4),
            },
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
            output: Some(SlotIdx::new(6)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(6),
            },
        },
        CompiledNode {
            id: StepIdx::new(5),
            output: Some(SlotIdx::new(7)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(7),
            },
        },
        CompiledNode {
            id: StepIdx::new(6),
            output: Some(SlotIdx::new(8)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(8),
            },
        },
    ];
    let contract = test_contract(7, 9);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;

    ensure_equal(budget.max_nesting_depth, 3)?;
    ensure_equal(budget.max_total_steps > 0, true)
}
