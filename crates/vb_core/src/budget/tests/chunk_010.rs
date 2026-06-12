//! Test chunk 010 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 2388–2662 of the original. Semantic content is
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


fn policy_rejects_run_time_exceeded() -> Result<(), String> {
    let mut budget = test_budget(1, 0, 0, 0);
    budget.max_run_time_seconds = 5_000_000;
    let policy = BoundednessPolicy {
        absolute_max_run_time_seconds: 2_592_000,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::RunTimeExceeded { actual, limit }) => {
            ensure_equal(actual, 5_000_000)?;
            ensure_equal(limit, 2_592_000)
        }
        other => Err(format!("expected RunTimeExceeded, got {other:?}")),
    }
}

#[test]
fn policy_rejects_steps_executable_exceeded() -> Result<(), String> {
    let mut budget = test_budget(1, 0, 0, 0);
    budget.max_steps_executable = 2_000_000;
    let policy = BoundednessPolicy {
        absolute_max_steps_executable: 1_000_000,
        ..BoundednessPolicy::DEFAULT
    };
    match policy.validate(&budget) {
        Err(BudgetError::StepsExecutableExceeded { actual, limit }) => {
            ensure_equal(actual, 2_000_000)?;
            ensure_equal(limit, 1_000_000)
        }
        other => Err(format!("expected StepsExecutableExceeded, got {other:?}")),
    }
}

// -------------------------------------------------------------------------
// Additional coverage: Do node action ticket counting
// -------------------------------------------------------------------------

#[test]
fn do_node_increments_action_tickets() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(1),
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
    let contract = test_contract(3, 2);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_action_tickets, 2)?;
    ensure_equal(budget.max_total_steps, 3)
}

// -------------------------------------------------------------------------
// Additional coverage: ForEach iteration accumulation
// -------------------------------------------------------------------------

#[test]
fn multiple_for_each_accumulates_iterations() -> Result<(), String> {
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
                limit: 5,
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
            output: Some(SlotIdx::new(2)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(3),
                item_slot: SlotIdx::new(4),
                limit: 10,
                body: StepIdx::new(4),
                done: StepIdx::new(5),
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
            output: Some(SlotIdx::new(5)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(5),
            },
        },
    ];
    let mut nodes = nodes;
    nodes[2].next = Some(StepIdx::new(3));

    let contract = test_contract(6, 6);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_for_each_iterations, 15)?;
    ensure_equal(budget.max_total_steps, 19)
}

// -------------------------------------------------------------------------
// Additional coverage: Jump node handling
// -------------------------------------------------------------------------

#[test]
fn jump_chain_counts_all_nodes() -> Result<(), String> {
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
                target: StepIdx::new(2),
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

#[test]
fn jump_self_cycle_detected() -> Result<(), String> {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Jump {
            target: StepIdx::new(0),
        },
    }];
    let contract = test_contract(1, 1);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::JumpCycle { step, target }) => {
            ensure_equal(step, StepIdx::new(0))?;
            ensure_equal(target, StepIdx::new(0))
        }
        other => Err(format!("expected JumpCycle, got {other:?}")),
    }
}

// -------------------------------------------------------------------------
// Additional coverage: BoundednessPolicy::DEFAULT sanity checks
// -------------------------------------------------------------------------

#[test]
fn default_policy_total_steps_is_one_thousand() -> Result<(), String> {
    ensure_equal(BoundednessPolicy::DEFAULT.max_total_steps, 1_000)
}

#[test]
fn default_policy_max_fanout_is_64() -> Result<(), String> {
    ensure_equal(BoundednessPolicy::DEFAULT.max_fanout, 64)
}

#[test]
fn default_policy_nesting_depth_is_8() -> Result<(), String> {
    ensure_equal(BoundednessPolicy::DEFAULT.max_nesting_depth, 8)
}

// -------------------------------------------------------------------------
// Additional coverage: BudgetError Display and Error trait
// -------------------------------------------------------------------------

#[test]
fn budget_error_implements_std_error() -> Result<(), String> {
    let err = BudgetError::TotalStepsExceeded {
        actual: 10,
        limit: 5,
    };
    let _: &dyn std::error::Error = &err;
    Ok(())
}

#[test]
fn budget_error_total_slots_display() -> Result<(), String> {
    let err = BudgetError::TotalSlotsExceeded {
        actual: 100,
        limit: 50,
    };
    ensure_equal(
        format!("{err}"),
        "total slots exceeded: 100 > 50".to_string(),
    )
}

#[test]
