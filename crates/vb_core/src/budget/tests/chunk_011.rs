//! Test chunk 011 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 2662–2915 of the original. Semantic content is
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
fn budget_error_parallel_display() -> Result<(), String> {
    let err = BudgetError::ParallelExceeded {
        actual: 300,
        limit: 256,
    };
    ensure_equal(format!("{err}"), "parallel exceeded: 300 > 256".to_string())
}

#[test]
fn budget_error_action_tickets_display() -> Result<(), String> {
    let err = BudgetError::ActionTicketsExceeded {
        actual: 150_000,
        limit: 100_000,
    };
    ensure_equal(
        format!("{err}"),
        "action tickets exceeded: 150000 > 100000".to_string(),
    )
}

#[test]
fn budget_error_run_time_display() -> Result<(), String> {
    let err = BudgetError::RunTimeExceeded {
        actual: 5_000_000,
        limit: 2_592_000,
    };
    ensure_equal(
        format!("{err}"),
        "run time exceeded: 5000000 > 2592000".to_string(),
    )
}

#[test]
fn budget_error_result_bytes_display() -> Result<(), String> {
    let err = BudgetError::ResultBytesExceeded {
        actual: 524_288,
        limit: 262_144,
    };
    ensure_equal(
        format!("{err}"),
        "result bytes exceeded: 524288 > 262144".to_string(),
    )
}

// -------------------------------------------------------------------------
// Additional coverage: WholeWorkflowBudget Copy and Clone
// -------------------------------------------------------------------------

#[test]
fn whole_workflow_budget_is_copy() -> Result<(), String> {
    let budget = test_budget(10, 100, 4, 2);
    let copy = budget;
    ensure_equal(budget, copy)
}

#[test]
fn boundedness_policy_is_copy() -> Result<(), String> {
    let policy = BoundednessPolicy::DEFAULT;
    let copy = policy;
    ensure_equal(policy, copy)
}

// -------------------------------------------------------------------------
// Additional coverage: ForEachStart limit=1 does not overcount
// -------------------------------------------------------------------------

#[test]
fn foreach_limit_one_exact_step_count() -> Result<(), String> {
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

// -------------------------------------------------------------------------
// Additional coverage: RepeatStart max_attempts=0 handled by max(1)
// -------------------------------------------------------------------------

#[test]
fn repeat_start_zero_attempts_counts_as_one() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 0,
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

// -------------------------------------------------------------------------
// Additional coverage: Linear chain with varied node types
// -------------------------------------------------------------------------

#[test]
fn linear_chain_set_const_copy_eval() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(2)),
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(2),
            },
        },
    ];
    let contract = test_contract(4, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_total_steps, 4)?;
    ensure_equal(budget.max_fanout, 0)?;
    ensure_equal(budget.max_nesting_depth, 0)
}

// -------------------------------------------------------------------------
// Additional coverage: CollectStart limit=0 handled by max(1)
// -------------------------------------------------------------------------

#[test]
fn collect_start_zero_limit_counts_as_one() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 0,
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
    ensure_equal(budget.max_total_steps, 3)
}
