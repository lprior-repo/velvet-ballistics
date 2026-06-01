use crate::commands_workflow::{DotGraph, SimulationResult, SimulationStep};
use vb_core::{ActionId, CompiledNodeKind, ConstIdx, ExprBranch, ExprIdx, SlotIdx, StepIdx};

// ---------------------------------------------------------------------------
// Helpers tests (re-exported from helpers module via parent)
// ---------------------------------------------------------------------------

#[allow(unused_imports)]
use crate::commands_workflow::helpers::{node_kind_label, saturating_add};

#[test]
fn saturating_add_returns_sum_for_normal_values() {
    assert_eq!(saturating_add(3, 5), 8);
}

#[test]
fn saturating_add_returns_zero_for_zeroes() {
    assert_eq!(saturating_add(0, 0), 0);
}

#[test]
fn saturating_add_clamps_at_usize_max() {
    assert_eq!(saturating_add(usize::MAX, 1), usize::MAX);
}

#[test]
fn node_kind_label_returns_nop_for_nop() {
    assert_eq!(node_kind_label(&CompiledNodeKind::Nop), "nop");
}

#[test]
fn node_kind_label_returns_set_const() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::SetConst {
            value: ConstIdx::new(0)
        }),
        "set_const"
    );
}

#[test]
fn node_kind_label_returns_copy() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::Copy {
            source: SlotIdx::new(0)
        }),
        "copy"
    );
}

#[test]
fn node_kind_label_returns_do() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::Do {
            action: ActionId::new(1),
            input: SlotIdx::new(0)
        }),
        "do"
    );
}

#[test]
fn node_kind_label_returns_finish() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::Finish {
            result: SlotIdx::new(0)
        }),
        "finish"
    );
}

#[test]
fn node_kind_label_returns_ask() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None
        }),
        "ask"
    );
}

#[test]
fn node_kind_label_returns_wait_until() {
    assert_eq!(
        node_kind_label(&CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(0)
        }),
        "wait_until"
    );
}

// ---------------------------------------------------------------------------
// Simulate tests
// ---------------------------------------------------------------------------

#[allow(unused_imports)]
use crate::commands_workflow::simulate::describe_node_for_simulate;

#[test]
fn describe_node_for_simulate_returns_entry_for_nop() {
    let mut ac = 0usize;
    let mut bc = 0usize;
    let desc = describe_node_for_simulate(&CompiledNodeKind::Nop, &mut ac, &mut bc);
    assert_eq!(desc, "Entry");
}

#[test]
fn describe_node_for_simulate_increments_action_count_for_do() {
    let mut ac = 0usize;
    let mut bc = 0usize;
    let _ = describe_node_for_simulate(
        &CompiledNodeKind::Do {
            action: ActionId::new(1),
            input: SlotIdx::new(0),
        },
        &mut ac,
        &mut bc,
    );
    assert_eq!(ac, 1);
}

#[test]
fn describe_node_for_simulate_increments_branch_count_for_choose() {
    let mut ac = 0usize;
    let mut bc = 0usize;
    let branches: Box<[ExprBranch]> = Box::new([
        ExprBranch {
            condition: ExprIdx::new(0),
            target: StepIdx::new(2),
        },
        ExprBranch {
            condition: ExprIdx::new(0),
            target: StepIdx::new(3),
        },
    ]);
    let _ = describe_node_for_simulate(
        &CompiledNodeKind::Choose {
            branches,
            otherwise: None,
        },
        &mut ac,
        &mut bc,
    );
    assert_eq!(bc, 2);
}

// ---------------------------------------------------------------------------
// Struct access tests
// ---------------------------------------------------------------------------

#[test]
fn dot_graph_fields_are_accessible() {
    let graph = DotGraph {
        node_count: 5,
        edge_count: 3,
        dot: "digraph {}".into(),
    };
    assert_eq!(graph.node_count, 5);
    assert_eq!(graph.edge_count, 3);
    assert_eq!(graph.dot, "digraph {}");
}

#[test]
fn simulation_step_fields_are_accessible() {
    let step = SimulationStep {
        index: 0,
        kind_label: "nop".into(),
        description: "Entry".into(),
    };
    assert_eq!(step.index, 0);
    assert_eq!(step.kind_label, "nop");
    assert_eq!(step.description, "Entry");
}

#[test]
fn simulation_result_fields_are_accessible() {
    let result = SimulationResult {
        steps: vec![],
        total_steps: 0,
        action_count: 0,
        branch_count: 0,
    };
    assert_eq!(result.total_steps, 0);
    assert!(result.steps.is_empty());
}
