#![allow(unused_imports, dead_code)]
//! Test chunk 016 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 3966–4229 of the original. Semantic content is
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
fn whole_workflow_budget_jump_cycle_detected_in_compute() -> Result<(), String> {
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
                target: StepIdx::new(0),
            },
        },
    ];
    let contract = test_contract(2, 1);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::JumpCycle { step, target }) => {
            ensure_equal(step, StepIdx::new(1))?;
            ensure_equal(target, StepIdx::new(0))
        }
        other => Err(format!("expected JumpCycle, got {:?}", other)),
    }
}

#[test]
fn whole_workflow_budget_step_out_of_bounds_in_visit() -> Result<(), String> {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: Some(StepIdx::new(99)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }];
    let contract = test_contract(1, 0);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::StepOutOfBounds { step }) => ensure_equal(step, StepIdx::new(99)),
        other => Err(format!("expected StepOutOfBounds, got {:?}", other)),
    }
}

// -------------------------------------------------------------------------
// AggregateResourceBudget and AggregateResourceUsage
// -------------------------------------------------------------------------

#[test]
fn aggregate_resource_budget_from_whole_workflow_budget() -> Result<(), String> {
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
    let wfb = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|e| format!("{:?}", e))?;
    let arb = crate::budget::AggregateResourceBudget::from_whole_workflow_budget(wfb, contract)
        .map_err(|e| format!("{:?}", e))?;
    ensure_equal(arb.max_steps_executable, wfb.max_steps_executable)?;
    ensure_equal(arb.max_action_tickets, wfb.max_action_tickets)?;
    ensure_equal(arb.max_parallel_in_flight, wfb.max_parallel_in_flight)?;
    ensure_equal(arb.max_timer_entries, wfb.max_timer_entries)?;
    ensure_equal(arb.max_trace_events, wfb.max_trace_events)?;
    ensure_equal(arb.max_result_bytes, 1)?;
    ensure_equal(arb.max_queue_depth, 1)?;
    ensure_equal(arb.max_journal_batch_bytes, 1)?;
    ensure_equal(arb.max_ipc_payload_bytes, 1)?;
    ensure_equal(arb.max_blob_bytes, 1)?;
    ensure_equal(arb.max_input_bytes, 1)
}
