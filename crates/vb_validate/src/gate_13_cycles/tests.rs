//! Tests for gate 13: No circular references in slot dependency graph.

use super::*;
use vb_core::ids::{ConstIdx, ExprIdx, StepIdx};

use crate::ValidationError;

use super::super::test_helpers::{copy_node, finish_node, make_parts};

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
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let parts = make_parts(vec![node], 1);
    assert_eq!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Ok(())
    );
}

#[test]
fn accepts_linear_chain() {
    let nodes = vec![
        vb_core::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            Some(StepIdx::new(1)),
        ),
        copy_node(1, 0, 1),
        vb_core::workflow::node(
            StepIdx::new(2),
            CompiledNodeKind::Copy {
                source: SlotIdx::new(1),
            },
            Some(SlotIdx::new(2)),
            None,
        ),
    ];
    let parts = make_parts(nodes, 3);
    assert_eq!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Ok(())
    );
}

#[test]
fn accepts_self_copy_not_cycle() {
    let nodes = vec![vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Copy {
            source: SlotIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        None,
    )];
    let parts = make_parts(nodes, 1);
    assert_eq!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Ok(())
    );
}

#[test]
fn accepts_diamond_dependency() {
    let nodes = vec![
        vb_core::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            Some(StepIdx::new(1)),
        ),
        copy_node(1, 0, 1),
        vb_core::workflow::node(
            StepIdx::new(2),
            CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
            Some(SlotIdx::new(2)),
            Some(StepIdx::new(3)),
        ),
        vb_core::workflow::node(
            StepIdx::new(3),
            CompiledNodeKind::BuildList {
                items: Box::new([SlotIdx::new(1), SlotIdx::new(2)]),
            },
            Some(SlotIdx::new(3)),
            None,
        ),
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
            vb_core::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            vb_core::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(SlotIdx::new(1)),
                None,
            ),
        ],
        2,
    );
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
        max_stack: 1,
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
        vb_core::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Copy {
                source: SlotIdx::new(1),
            },
            Some(SlotIdx::new(0)),
            Some(StepIdx::new(1)),
        ),
        vb_core::workflow::node(
            StepIdx::new(1),
            CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
            Some(SlotIdx::new(1)),
            None,
        ),
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
        vb_core::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Copy {
                source: SlotIdx::new(1),
            },
            Some(SlotIdx::new(0)),
            Some(StepIdx::new(1)),
        ),
        vb_core::workflow::node(
            StepIdx::new(1),
            CompiledNodeKind::Copy {
                source: SlotIdx::new(2),
            },
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(2)),
        ),
        vb_core::workflow::node(
            StepIdx::new(2),
            CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
            Some(SlotIdx::new(2)),
            None,
        ),
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
            vb_core::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            vb_core::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
                Some(SlotIdx::new(1)),
                None,
            ),
        ],
        2,
    );
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(1))]),
        max_stack: 1,
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
            vb_core::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            vb_core::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            vb_core::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(1),
                },
                Some(SlotIdx::new(2)),
                None,
            ),
        ],
        3,
    );
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(2))]),
        max_stack: 1,
    }]);
    assert!(matches!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Err(ValidationError::SlotDependencyCycle { .. })
    ));
}

#[test]
fn rejects_cycle_through_build_object() {
    let nodes = vec![
        vb_core::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::BuildObject {
                fields: Box::new([(vb_core::ids::SymbolId::new(1), SlotIdx::new(1))]),
            },
            Some(SlotIdx::new(0)),
            Some(StepIdx::new(1)),
        ),
        vb_core::workflow::node(
            StepIdx::new(1),
            CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
            Some(SlotIdx::new(1)),
            None,
        ),
    ];
    let parts = make_parts(nodes, 2);
    assert!(matches!(
        crate::gate_13_cycles::validate_gate_13_no_slot_cycles(&parts),
        Err(ValidationError::SlotDependencyCycle { .. })
    ));
}
