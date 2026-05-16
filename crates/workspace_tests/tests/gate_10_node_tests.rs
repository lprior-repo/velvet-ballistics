#![forbid(unsafe_code)]
//! Unit tests for Gate 10 - Node-kind-specific constraints (bead vb-qi37.8).
//!
//! Tests cover:
//! - Finish node validation
//! - Choose/ChooseSlot branch validation
//! - SetConst const index bounds
//! - EvalExpr expression index bounds
//! - Do action_id and input slot validation
//! - ForEachStart/TogetherStart structural validation
//! - BuildObject/BuildList field/item slot validation

use vb_core::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, ExprBranch, ResourceContract, WorkflowParts,
};
use vb_validate::ValidationError;
use vb_validate::gates::validate_gate_10_node_kind_specific;

// ---------------------------------------------------------------------------
// Helper constructors
// ---------------------------------------------------------------------------

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

// ===========================================================================
// Finish node validation
// ===========================================================================

#[test]
fn gate_10_accepts_valid_finish() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn gate_10_rejects_finish_result_out_of_range() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(99), // slot_count = 1
        },
    };
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

// ===========================================================================
// Choose node validation
// ===========================================================================

#[test]
fn gate_10_accepts_valid_choose() {
    let mut parts = make_parts(
        vec![finish_node(0, 0), finish_node(1, 0), finish_node(2, 0)],
        1,
    );
    parts.expressions = Box::new([vb_core::workflow::ExprProgram {
        ops: Box::new([vb_core::workflow::ExprOp::LoadSlot(SlotIdx::new(0))]),
        max_stack: 1,
    }]);
    parts.nodes = Box::new([
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Choose {
                branches: Box::new([ExprBranch {
                    condition: ExprIdx::new(0),
                    target: StepIdx::new(1),
                }]),
                otherwise: Some(StepIdx::new(2)),
            },
        },
        finish_node(1, 0),
        finish_node(2, 0),
    ]);
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn gate_10_rejects_choose_expr_out_of_range() {
    let mut parts = make_parts(vec![finish_node(0, 0), finish_node(1, 0)], 1);
    parts.expressions = Box::new([]); // No expressions, but Choose references expr 0
    parts.nodes = Box::new([CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Choose {
            branches: Box::new([ExprBranch {
                condition: ExprIdx::new(99), // Out of range
                target: StepIdx::new(1),
            }]),
            otherwise: None,
        },
    }]);
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

// ===========================================================================
// ChooseSlot node validation
// ===========================================================================

#[test]
fn gate_10_accepts_valid_choose_slot() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ChooseSlot {
                branches: Box::new([vb_core::workflow::SlotBranch {
                    condition: SlotIdx::new(0),
                    target: StepIdx::new(1),
                }]),
                otherwise: Some(StepIdx::new(2)),
            },
        },
        finish_node(1, 0),
        finish_node(2, 0),
    ];
    let parts = make_parts(nodes, 1);
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn gate_10_rejects_choose_slot_condition_out_of_range() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ChooseSlot {
            branches: Box::new([vb_core::workflow::SlotBranch {
                condition: SlotIdx::new(99), // slot_count = 1
                target: StepIdx::new(1),
            }]),
            otherwise: None,
        },
    };
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

// ===========================================================================
// SetConst validation
// ===========================================================================

#[test]
fn gate_10_accepts_valid_set_const() {
    let mut parts = make_parts(vec![finish_node(1, 0)], 1);
    parts.constants = Box::new([ConstValue::I64(42)]);
    parts.nodes = Box::new([CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0), // Valid: index 0 < const_count 1
        },
    }]);
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn gate_10_rejects_set_const_index_out_of_range() {
    let mut parts = make_parts(vec![finish_node(1, 0)], 1);
    parts.constants = Box::new([ConstValue::I64(42)]); // const_count = 1
    parts.nodes = Box::new([CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(99), // Out of range
        },
    }]);
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

// ===========================================================================
// EvalExpr validation
// ===========================================================================

#[test]
fn gate_10_accepts_valid_eval_expr() {
    let mut parts = make_parts(vec![finish_node(1, 0)], 1);
    parts.expressions = Box::new([vb_core::workflow::ExprProgram {
        ops: Box::new([vb_core::workflow::ExprOp::LoadSlot(SlotIdx::new(0))]),
        max_stack: 1,
    }]);
    parts.nodes = Box::new([CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(0), // Valid
        },
    }]);
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn gate_10_rejects_eval_expr_index_out_of_range() {
    let mut parts = make_parts(vec![finish_node(1, 0)], 1);
    parts.expressions = Box::new([]); // No expressions
    parts.nodes = Box::new([CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(99), // Out of range
        },
    }]);
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

// ===========================================================================
// Do node validation
// ===========================================================================

#[test]
fn gate_10_accepts_valid_do() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(1),
                input: SlotIdx::new(0),
            },
        },
        finish_node(1, 0),
    ];
    let parts = make_parts(nodes, 1);
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn gate_10_rejects_do_input_out_of_range() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: vb_core::ids::ActionId::new(1),
            input: SlotIdx::new(99), // slot_count = 1
        },
    };
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn gate_10_rejects_do_sentinel_action_id() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: vb_core::ids::ActionId::new(u16::MAX), // Sentinel value
            input: SlotIdx::new(0),
        },
    };
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

// ===========================================================================
// ForEachStart validation
// ===========================================================================

#[test]
fn gate_10_accepts_valid_foreach_start() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        },
        finish_node(1, 0),
        finish_node(2, 0),
        finish_node(3, 0),
    ];
    let parts = make_parts(nodes, 2);
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn gate_10_rejects_foreach_start_input_out_of_range() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(99), // slot_count = 1
            item_slot: SlotIdx::new(1),
            limit: 10,
            body: StepIdx::new(1),
            done: StepIdx::new(3),
        },
    };
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

// ===========================================================================
// TogetherStart validation
// ===========================================================================

#[test]
fn gate_10_accepts_valid_together_start() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1)]),
                join: StepIdx::new(3),
            },
        },
        finish_node(1, 0),
        finish_node(2, 0),
        finish_node(3, 0),
    ];
    let parts = make_parts(nodes, 1);
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn gate_10_rejects_together_start_branch_out_of_range() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::TogetherStart {
            branches: Box::new([StepIdx::new(99)]), // node_count = 1
            join: StepIdx::new(1),
        },
    };
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

// ===========================================================================
// BuildObject validation
// ===========================================================================

#[test]
fn gate_10_accepts_valid_build_object() {
    let mut parts = make_parts(vec![finish_node(1, 0)], 1);
    parts.symbols_count = 2;
    parts.nodes = Box::new([CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::BuildObject {
            fields: Box::new([
                (SymbolId::new(0), SlotIdx::new(0)),
                (SymbolId::new(1), SlotIdx::new(0)),
            ]),
        },
    }]);
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn gate_10_rejects_build_object_slot_out_of_range() {
    let mut parts = make_parts(vec![finish_node(1, 0)], 1);
    parts.symbols_count = 2;
    parts.nodes = Box::new([CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::BuildObject {
            fields: Box::new([(SymbolId::new(0), SlotIdx::new(99))]), // slot_count = 1
        },
    }]);
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

// ===========================================================================
// BuildList validation
// ===========================================================================

#[test]
fn gate_10_accepts_valid_build_list() {
    let parts = make_parts(
        vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildList {
                items: Box::new([SlotIdx::new(0), SlotIdx::new(0)]),
            },
        }],
        1,
    );
    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn gate_10_rejects_build_list_item_out_of_range() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::BuildList {
            items: Box::new([SlotIdx::new(99)]), // slot_count = 1
        },
    };
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}
