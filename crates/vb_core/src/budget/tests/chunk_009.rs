//! Test chunk 009 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 2114–2385 of the original. Semantic content is
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
fn together_start_tracks_max_parallel_in_flight() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(1), StepIdx::new(2)].into_boxed_slice(),
                join: StepIdx::new(3),
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
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(4, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;

    ensure_equal(budget.max_fanout, 2)?;
    ensure_equal(budget.max_parallel_in_flight, 2)?;
    ensure_equal(budget.max_together_branches, 2)?;
    ensure_equal(budget.max_total_steps, 4)
}

#[test]
fn larger_together_start_dominates_fanout() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(1), StepIdx::new(2)].into_boxed_slice(),
                join: StepIdx::new(3),
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
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: vec![
                    StepIdx::new(4),
                    StepIdx::new(5),
                    StepIdx::new(6),
                    StepIdx::new(7),
                    StepIdx::new(8),
                ]
                .into_boxed_slice(),
                join: StepIdx::new(9),
            },
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(6),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(7),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(8),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(9),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(10, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;

    ensure_equal(budget.max_fanout, 5)?;
    ensure_equal(budget.max_parallel_in_flight, 5)?;
    ensure_equal(budget.max_together_branches, 5)
}

// -------------------------------------------------------------------------
// 9. Zero-budget edge cases
// -------------------------------------------------------------------------

#[test]
fn step_budget_zero_never_allows_consumption() -> Result<(), String> {
    let mut b = StepBudget::new(0);
    for _ in 0..10 {
        let taken = b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(taken, false)?;
        ensure_equal(b.remaining(), 0)?;
    }
    Ok(())
}

#[test]
fn whole_workflow_budget_zero_slots_contract() -> Result<(), String> {
    let nodes = single_node_workflow();
    let contract = test_contract(1, 0);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_slots, 0)
}

#[test]
fn policy_validate_accepts_zero_budget() -> Result<(), String> {
    let budget = test_budget(0, 0, 0, 0);
    ensure_equal(BoundednessPolicy::DEFAULT.validate(&budget), Ok(()))
}

// -------------------------------------------------------------------------
// 10. Budget arithmetic overflow protection
// -------------------------------------------------------------------------

#[test]
fn whole_workflow_budget_max_total_slots_derives_from_contract() -> Result<(), String> {
    let nodes = single_node_workflow();
    let contract = test_contract(1, 500);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_slots, 500)
}

#[test]
fn whole_workflow_budget_result_bytes_derive_from_contract() -> Result<(), String> {
    let nodes = single_node_workflow();
    let mut contract = test_contract(1, 1);
    contract.max_output_bytes = 9999;
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_result_bytes, 9999)
}

#[test]
fn whole_workflow_budget_max_retries_from_contract() -> Result<(), String> {
    let nodes = single_node_workflow();
    let mut contract = test_contract(1, 1);
    contract.max_retry_attempts = 7;
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_retries_per_action, 7)
}

#[test]
fn policy_rejects_action_tickets_exceeded() -> Result<(), String> {
    let mut budget = test_budget(1, 0, 0, 0);
    budget.max_action_tickets = 200_000;
    let policy = BoundednessPolicy {
        absolute_max_action_tickets: 100_000,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::ActionTicketsExceeded { actual, limit }) => {
            ensure_equal(actual, 200_000)?;
            ensure_equal(limit, 100_000)
        }
        other => Err(format!("expected ActionTicketsExceeded, got {other:?}")),
    }
}

#[test]
fn policy_rejects_parallel_exceeded() -> Result<(), String> {
    let mut budget = test_budget(1, 0, 0, 0);
    budget.max_parallel_in_flight = 512;
    let policy = BoundednessPolicy {
        absolute_max_parallel: 256,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::ParallelExceeded { actual, limit }) => {
            ensure_equal(actual, 512)?;
            ensure_equal(limit, 256)
        }
        other => Err(format!("expected ParallelExceeded, got {other:?}")),
    }
}

#[test]
fn policy_rejects_result_bytes_exceeded() -> Result<(), String> {
    let mut budget = test_budget(1, 0, 0, 0);
    budget.max_result_bytes = 1_000_000;
    let policy = BoundednessPolicy {
        absolute_max_result_bytes: 262_144,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::ResultBytesExceeded { actual, limit }) => {
            ensure_equal(actual, 1_000_000)?;
            ensure_equal(limit, 262_144)
        }
        other => Err(format!("expected ResultBytesExceeded, got {other:?}")),
    }
}
