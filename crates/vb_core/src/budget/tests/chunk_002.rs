//! Test chunk 002 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 294–536 of the original. Semantic content is
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
fn budget_rejects_excessive_fanout() {
    let budget = test_budget(1, 10, 3, 0);
    let policy = test_policy(1_000_000, 65_535, 2, 8);

    match policy.validate(&budget) {
        Err(BudgetError::FanoutExceeded {
            actual: 3,
            limit: 2,
        }) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn budget_accepts_within_policy() {
    let budget = test_budget(10, 100, 4, 2);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    assert_eq!(result, Ok(()));
}

#[test]
fn budget_rejects_excessive_nesting_depth() {
    let budget = test_budget(1, 10, 1, 10);
    let policy = test_policy(1_000_000, 65_535, 64, 4);

    match policy.validate(&budget) {
        Err(BudgetError::NestingDepthExceeded {
            actual: 10,
            limit: 4,
        }) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn budget_rejects_excessive_total_slots() {
    let budget = test_budget(1, 200_000, 1, 0);
    let policy = test_policy(1_000_000, 65_535, 64, 8);

    match policy.validate(&budget) {
        Err(BudgetError::TotalSlotsExceeded {
            actual: 200_000,
            limit: 65_535,
        }) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn budget_default_policy_accepts_reasonable_budget() {
    let budget = test_budget(500, 10_000, 32, 4);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    assert_eq!(result, Ok(()));
}

#[test]
fn budget_compute_rejects_entry_out_of_bounds() {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }];
    let contract = test_contract(1, 0);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(5), &contract);

    match result {
        Err(WorkflowError::EntryOutOfBounds { entry }) if entry == StepIdx::new(5) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn budget_together_start_fanout() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(1), StepIdx::new(2), StepIdx::new(3)]
                    .into_boxed_slice(),
                join: StepIdx::new(4),
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
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(5, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_fanout == 3 && b.max_total_steps == 5);

    assert!(budget.is_some(), "together start fanout mismatch");
}

#[test]
fn budget_single_node_workflow() {
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
        .filter(|b| b.max_total_steps == 1 && b.max_fanout == 0 && b.max_nesting_depth == 0);

    assert!(budget.is_some(), "single-node workflow budget mismatch");
}

#[test]
fn budget_error_display_formatting() {
    let err = BudgetError::TotalStepsExceeded {
        actual: 5,
        limit: 3,
    };
    assert_eq!(format!("{err}"), "total steps exceeded: 5 > 3");

    let err = BudgetError::TotalSlotsExceeded {
        actual: 200,
        limit: 100,
    };
    assert_eq!(format!("{err}"), "total slots exceeded: 200 > 100");

    let err = BudgetError::FanoutExceeded {
        actual: 10,
        limit: 4,
    };
    assert_eq!(format!("{err}"), "fanout exceeded: 10 > 4");

    let err = BudgetError::NestingDepthExceeded {
        actual: 16,
        limit: 8,
    };
    assert_eq!(format!("{err}"), "nesting depth exceeded: 16 > 8");

    let err = BudgetError::ParallelExceeded {
        actual: 128,
        limit: 64,
    };
    assert_eq!(format!("{err}"), "parallel exceeded: 128 > 64");

    let err = BudgetError::ActionTicketsExceeded {
        actual: 200_000,
        limit: 100_000,
    };
    assert_eq!(format!("{err}"), "action tickets exceeded: 200000 > 100000");

    let err = BudgetError::RunTimeExceeded {
        actual: 3_000_000,
        limit: 2_592_000,
    };
    assert_eq!(format!("{err}"), "run time exceeded: 3000000 > 2592000");

    let err = BudgetError::ResultBytesExceeded {
        actual: 524_288,
        limit: 262_144,
    };
    assert_eq!(format!("{err}"), "result bytes exceeded: 524288 > 262144");

    let err = BudgetError::StepsExecutableExceeded {
        actual: 2_000_000,
        limit: 1_000_000,
    };
    assert_eq!(
        format!("{err}"),
        "steps executable exceeded: 2000000 > 1000000"
    );
}

#[test]
fn budget_step_count_overflow_detected() {
    // Construct a workflow where a node's next points out of bounds,
    // verifying error propagation through the count path.
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: Some(StepIdx::new(99)), // out of bounds
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }];
    let contract = test_contract(1, 0);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::StepOutOfBounds { .. }) => {}
        other => panic!("expected StepOutOfBounds, got {other:?}"),
    }
}

#[test]
fn budget_empty_nodes_rejected() {
    let nodes: Vec<CompiledNode> = vec![];
    let contract = test_contract(0, 0);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::EntryOutOfBounds { .. }) => {}
        other => panic!("expected EntryOutOfBounds for empty nodes, got {other:?}"),
    }
}
