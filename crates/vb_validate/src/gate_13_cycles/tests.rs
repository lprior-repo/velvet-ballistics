//! Tests for gate 13: No circular references in slot dependency graph.

use super::*;
use vb_core::ids::{ConstIdx, ExprIdx, StepIdx};
use vb_core::workflow::{CompiledNode, ResourceContract};

use crate::{ValidationError, WorkflowParts};

fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn finish_node(index: u16, result_slot: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(result_slot),
        },
    }
}

fn copy_node(index: u16, source: u16, output: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: Some(SlotIdx::new(output)),
        next: Some(StepIdx::new(index.saturating_add(1))),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Copy {
            source: SlotIdx::new(source),
        },
    }
}

// -- Pass cases --

#[test]
fn accepts_empty_slots() {
    let parts = make_parts(vec![finish_node(0, 0)], 0);
    assert_eq!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Ok(())
    );
}

#[test]
fn accepts_single_slot_no_deps() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let parts = make_parts(vec![node], 1);
    assert_eq!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Ok(())
    );
}

#[test]
fn accepts_linear_chain() {
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
        copy_node(1, 0, 1),
        CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(2)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(1),
            },
        },
    ];
    let parts = make_parts(nodes, 3);
    assert_eq!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Ok(())
    );
}

#[test]
fn accepts_self_copy_not_cycle() {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Copy {
            source: SlotIdx::new(0),
        },
    }];
    let parts = make_parts(nodes, 1);
    assert_eq!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Ok(())
    );
}

#[test]
fn accepts_diamond_dependency() {
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
        copy_node(1, 0, 1),
        CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(2)),
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: Some(SlotIdx::new(3)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildList {
                items: Box::new([SlotIdx::new(1), SlotIdx::new(2)]),
            },
        },
    ];
    let parts = make_parts(nodes, 4);
    assert_eq!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Ok(())
    );
}

#[test]
fn accepts_linear_chain_through_eval_expr() {
    let mut parts = make_parts(
        vec![
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
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
            },
        ],
        2,
    );
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
        max_stack: 1,
        constants: Box::default(),
    }]);
    assert_eq!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Ok(())
    );
}

// -- Fail cases --

#[test]
fn rejects_direct_cycle() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        },
    ];
    let parts = make_parts(nodes, 2);
    assert!(matches!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Err(ValidationError::SlotDependencyCycle { .. })
    ));
}

#[test]
fn rejects_three_slot_cycle() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(2)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        },
    ];
    let parts = make_parts(nodes, 3);
    assert!(matches!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Err(ValidationError::SlotDependencyCycle { .. })
    ));
}

#[test]
fn rejects_cycle_through_eval_expr() {
    let mut parts = make_parts(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(1)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
            },
        ],
        2,
    );
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(1))]),
        max_stack: 1,
        constants: Box::default(),
    }]);
    assert!(matches!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Err(ValidationError::SlotDependencyCycle { .. })
    ));
}

#[test]
fn rejects_three_slot_cycle_through_eval_expr() {
    let mut parts = make_parts(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
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
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(1),
                },
            },
        ],
        3,
    );
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(2))]),
        max_stack: 1,
        constants: Box::default(),
    }]);
    assert!(matches!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Err(ValidationError::SlotDependencyCycle { .. })
    ));
}

#[test]
fn rejects_cycle_through_build_object() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildObject {
                fields: Box::new([(vb_core::ids::SymbolId::new(1), SlotIdx::new(1))]),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        },
    ];
    let parts = make_parts(nodes, 2);
    assert!(matches!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Err(ValidationError::SlotDependencyCycle { .. })
    ));
}
