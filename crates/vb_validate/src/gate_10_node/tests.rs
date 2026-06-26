//! Tests for gate_10_node.

#[cfg(test)]
use super::*;
use vb_core::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, ExprBranch, ExprOp, ExprProgram, SlotBranch,
};

use super::super::test_helpers::{make_parts, finish_node};

// -- Pass cases --

#[test]
fn accepts_finish_with_valid_slot() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn accepts_nop_node() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = make_parts(vec![node], 0);
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn accepts_choose_with_valid_branches() {
    let mut parts = make_parts(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: Box::new([ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]),
                    otherwise: Some(StepIdx::new(1)),
                },
            },
            finish_node(1, 0),
        ],
        1,
    );
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
        max_stack: 1,
    }]);
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn accepts_choose_slot_with_valid_branches() {
    let parts = make_parts(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: Box::new([SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(1),
                    }]),
                    otherwise: Some(StepIdx::new(1)),
                },
            },
            finish_node(1, 0),
        ],
        1,
    );
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn accepts_set_const_with_valid_index() {
    let mut parts = make_parts(
        vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        }],
        1,
    );
    parts.constants = Box::new([vb_core::value::ConstValue::I64(42)]);
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn accepts_eval_expr_with_valid_index() {
    let mut parts = make_parts(
        vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
        }],
        1,
    );
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
        max_stack: 1,
    }]);
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn accepts_do_with_valid_action() {
    let parts = make_parts(
        vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
        }],
        1,
    );
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

// -- Fail cases --

#[test]
fn rejects_finish_slot_out_of_range() {
    let parts = make_parts(vec![finish_node(0, 99)], 1);
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn rejects_choose_expr_index_out_of_range() {
    let parts = make_parts(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: Box::new([ExprBranch {
                        condition: ExprIdx::new(99),
                        target: StepIdx::new(1),
                    }]),
                    otherwise: None,
                },
            },
            finish_node(1, 0),
        ],
        1,
    );
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn rejects_choose_target_out_of_range() {
    let mut parts = make_parts(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: Box::new([ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(99),
                    }]),
                    otherwise: None,
                },
            },
            finish_node(1, 0),
        ],
        1,
    );
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
        max_stack: 1,
    }]);
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn rejects_choose_otherwise_out_of_range() {
    let mut parts = make_parts(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: Box::new([ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]),
                    otherwise: Some(StepIdx::new(99)),
                },
            },
            finish_node(1, 0),
        ],
        1,
    );
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
        max_stack: 1,
    }]);
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn rejects_choose_slot_condition_out_of_range() {
    let parts = make_parts(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: Box::new([SlotBranch {
                        condition: SlotIdx::new(99),
                        target: StepIdx::new(1),
                    }]),
                    otherwise: None,
                },
            },
            finish_node(1, 0),
        ],
        1,
    );
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn rejects_set_const_index_out_of_range() {
    let parts = make_parts(
        vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(99),
            },
        }],
        1,
    );
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn rejects_eval_expr_index_out_of_range() {
    let parts = make_parts(
        vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(99),
            },
        }],
        1,
    );
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn rejects_do_input_slot_out_of_range() {
    let parts = make_parts(
        vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(99),
            },
        }],
        1,
    );
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn rejects_do_sentinel_action_id() {
    let parts = make_parts(
        vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(u16::MAX),
                input: SlotIdx::new(0),
            },
        }],
        1,
    );
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn rejects_for_each_start_input_out_of_range() {
    let parts = make_parts(
        vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(99),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        }],
        2,
    );
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn rejects_together_start_branch_out_of_range() {
    let parts = make_parts(
        vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(99)]),
                join: StepIdx::new(1),
            },
        }],
        1,
    );
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn rejects_together_start_join_out_of_range() {
    let parts = make_parts(
        vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([]),
                join: StepIdx::new(99),
            },
        }],
        1,
    );
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}
