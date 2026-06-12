#![allow(unused_imports, dead_code)]
//! Test chunk 007 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 1600–1855 of the original. Semantic content is
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
fn step_budget_multi_step_consumption_to_zero() -> Result<(), String> {
    let mut b = StepBudget::new(3);
    ensure_equal(b.try_take().map_err(|e| e.to_string())?, true)?;
    ensure_equal(b.remaining(), 2)?;
    ensure_equal(b.try_take().map_err(|e| e.to_string())?, true)?;
    ensure_equal(b.remaining(), 1)?;
    ensure_equal(b.try_take().map_err(|e| e.to_string())?, true)?;
    ensure_equal(b.remaining(), 0)
}

#[test]
fn step_budget_consumption_returns_true_each_time_until_exhausted() -> Result<(), String> {
    let mut b = StepBudget::new(4);
    for i in 0..4 {
        let taken = b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(taken, true)?;
        ensure_equal(b.remaining(), 3 - i)?;
    }
    let final_take = b.try_take().map_err(|e| e.to_string())?;
    ensure_equal(final_take, false)
}

// -------------------------------------------------------------------------
// 3. Budget exhaustion detection
// -------------------------------------------------------------------------

#[test]
fn step_budget_exhausted_returns_false() -> Result<(), String> {
    let mut b = StepBudget::new(1);
    ensure_equal(b.try_take().map_err(|e| e.to_string())?, true)?;
    ensure_equal(b.try_take().map_err(|e| e.to_string())?, false)?;
    ensure_equal(b.remaining(), 0)
}

#[test]
fn step_budget_exhaustion_stays_at_zero() -> Result<(), String> {
    let mut b = StepBudget::new(2);
    b.try_take().map_err(|e| e.to_string())?;
    b.try_take().map_err(|e| e.to_string())?;
    for _ in 0..5 {
        let taken = b.try_take().map_err(|e| e.to_string())?;
        ensure_equal(taken, false)?;
        ensure_equal(b.remaining(), 0)?;
    }
    Ok(())
}

// -------------------------------------------------------------------------
// 4. Sub-graph budget accounting
// -------------------------------------------------------------------------

// 4a. ForEach body cost multiplication with limit=1 (single iteration)
#[test]
fn foreach_limit_one_counts_body_once() -> Result<(), String> {
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
                limit: 1,
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
    ensure_equal(budget.max_total_steps, 3)
}

// 4b. Together branch budget counts all branches
#[test]
fn together_start_counts_parallel_branches() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: vec![
                    StepIdx::new(1),
                    StepIdx::new(2),
                    StepIdx::new(3),
                    StepIdx::new(4),
                ]
                .into_boxed_slice(),
                join: StepIdx::new(5),
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
            kind: CompiledNodeKind::Nop,
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
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(6, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_fanout, 4)?;
    ensure_equal(budget.max_parallel_in_flight, 4)?;
    ensure_equal(budget.max_together_branches, 4)?;
    ensure_equal(budget.max_total_steps, 6)
}

// 4c. Collect loop body cost
#[test]
fn collect_start_body_accounting() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 3,
                page_size: 1,
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
    ensure_equal(budget.max_total_steps, 5)?;
    ensure_equal(budget.max_gather_pages, 1)?;
    ensure_equal(budget.max_gather_items, 3)
}

// 4d. Reduce body cost
#[test]
fn reduce_start_body_accounting() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceStart {
                input: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                initial: ConstIdx::new(0),
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
    ensure_equal(budget.max_total_steps, 1 + 1 + 1)
}
