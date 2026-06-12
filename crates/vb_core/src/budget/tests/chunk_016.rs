//! Test chunk 016 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 3889–4144 of the original. Semantic content is
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


fn count_and_push_loop_body_overflow_propagates_budget_error() {
    use crate::ids::StepIdx;
    use crate::workflow::{CompiledNode, CompiledNodeKind};

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
                limit: u32::MAX,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
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
                limit: u32::MAX,
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
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
    let contract = test_contract(5, 5);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Ok(budget) => {
            let _ = budget;
        }
        Err(WorkflowError::StepCountOverflow { actual: _ }) => {}
        Err(e) => panic!("expected StepCountOverflow, got {:?}", e),
    }
}

// -------------------------------------------------------------------------
// Additional coverage: WholeWorkflowBudget fields
// -------------------------------------------------------------------------

#[test]
fn whole_workflow_budget_max_parallel_in_flight_from_together() -> Result<(), String> {
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
                    StepIdx::new(5),
                ]
                .into_boxed_slice(),
                join: StepIdx::new(6),
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
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(6),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(7, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_parallel_in_flight, 5)
}

#[test]
fn whole_workflow_budget_max_action_tickets_from_do_nodes() -> Result<(), String> {
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
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(2),
                input: SlotIdx::new(2),
            },
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
    let contract = test_contract(4, 3);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| e.to_string())?;
    ensure_equal(budget.max_action_tickets, 3)
}

#[test]
fn whole_workflow_budget_max_gather_pages_and_items() -> Result<(), String> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 50,
                page_size: 10,
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
    ensure_equal(budget.max_gather_pages, 1)?;
    ensure_equal(budget.max_gather_items, 50)
}

// -------------------------------------------------------------------------
// WorkflowError variants from compute path
// -------------------------------------------------------------------------

#[test]
