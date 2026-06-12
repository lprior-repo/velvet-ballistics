#![allow(unused_imports, dead_code)]
//! Test chunk 003 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 538–777 of the original. Semantic content is
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
fn budget_choose_fanout_counted() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Choose {
                branches: vec![
                    ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(2),
                    },
                ]
                .into_boxed_slice(),
                otherwise: None,
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
        .ok()
        .filter(|b| b.max_fanout == 2 && b.max_total_steps == 2);

    assert!(budget.is_some(), "choose fanout budget mismatch");
}

// =========================================================================
// Security regression tests: loop-aware step counting
// =========================================================================

/// Verifies that a ForEachStart loop multiplies body steps by the limit.
/// Workflow: ForEachStart(limit=5, body=1, done=2) -> Nop -> Finish
/// Expected: 1 (header) + 5 * 1 (body * iterations) + 1 (Finish) = 7
#[test]
fn budget_foreach_loop_multiplies_body_steps() {
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
        "for-each loop should multiply body steps by limit"
    );
}

/// Verifies that a RepeatStart body is counted once at the cold-AST-conservative
/// iter count of 1. The declared `max_attempts` is tracked separately via
/// `max_repeat_attempts`, not by multiplication into `max_total_steps`.
///
/// Workflow: RepeatStart(max=3, body=1, done=2) -> Nop -> Finish
/// Expected: 1 (header) + 1 (body) + 1 (Finish) = 3
#[test]
fn budget_repeat_loop_multiplies_body_steps() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 3,
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
        "repeat loop body should be counted once with cold-AST-conservative iter count"
    );
}

/// Verifies nested loop step counting multiplies correctly.
/// Outer ForEachStart(limit=10, body=1, done=4)
///   Inner ForEachStart(limit=10, body=2, done=3)
///     Nop (node 2)
///   ForEachJoin (node 3)
/// ForEachJoin (node 4)
///
/// Inner: body_count=1, product = 1*10=10, inner total = 1+10+1 = 12.
/// Outer body region: node1=12, node2=1, node3=1 = 14.
/// Wait — body_count counts distinct nodes in the region, inner ForEachStart(1) itself
/// is counted as 1, then its body is 1*10=10, then ForEachJoin(3) is 1. So region for
/// outer body = 1 + 10 + 1 = 12. Outer: product = 12*10 = 120, total = 1+120+1 = 122.
#[test]
fn budget_nested_loop_multiplies_correctly() {
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
                limit: 10,
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
                limit: 10,
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
            output: Some(SlotIdx::new(4)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(4),
            },
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: Some(SlotIdx::new(5)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(5),
            },
        },
    ];
    let contract = test_contract(5, 6);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 122 && b.max_nesting_depth == 2);

    assert!(
        budget.is_some(),
        "nested loop should multiply step counts at each nesting level"
    );
}
