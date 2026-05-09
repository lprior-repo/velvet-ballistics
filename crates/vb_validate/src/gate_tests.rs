#![forbid(unsafe_code)]
//! Tests for plan verifier gates.

use crate::{ValidationError, ValidationResult};
use vb_core::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
use vb_core::workflow::{AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment, ResourceContract, WorkflowParts};

// ---------------------------------------------------------------------------
// Helper constructors
// ---------------------------------------------------------------------------

fn make_parts(
    nodes: Vec<CompiledNode>,
    slot_count: u16,
    symbols_count: u32,
) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
    }
}

fn nop_node(index: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: Some(StepIdx::new(index.saturating_add(1))),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
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

// ---------------------------------------------------------------------------
// Gate 7 tests
// ---------------------------------------------------------------------------

use crate::gate_07_stack::validate_gate_07_expression_stack_depth;
use crate::gate_07_stack::compute_stack_depth;

#[test]
fn gate_07_accepts_empty_expressions() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
}

#[test]
fn gate_07_accepts_valid_expression() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
        max_stack: 1,
    }]);
    assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
}

#[test]
fn gate_07_rejects_stack_mismatch() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
        max_stack: 2, // wrong: actual max is 1
    }]);
    assert!(matches!(
        validate_gate_07_expression_stack_depth(&parts),
        Err(ValidationError::ExpressionStackMismatch { .. })
    ));
}

#[test]
fn gate_07_rejects_stack_exceeding_contract() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.resource_contract = ResourceContract {
        max_expr_stack: 2,
        ..ResourceContract::DEFAULT
    };
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
        max_stack: 3, // exceeds contract of 2
    }]);
    assert!(matches!(
        validate_gate_07_expression_stack_depth(&parts),
        Err(ValidationError::ExpressionStackExceeded { .. })
    ));
}

#[test]
fn gate_07_rejects_contract_exceeding_protocol_limit() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.resource_contract = ResourceContract {
        max_expr_stack: 128, // exceeds protocol limit of 64
        ..ResourceContract::DEFAULT
    };
    assert!(matches!(
        validate_gate_07_expression_stack_depth(&parts),
        Err(ValidationError::ExpressionStackExceeded { .. })
    ));
}

#[test]
fn gate_07_rejects_underflow_binary_op_on_empty_stack() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::Eq]), // pops 2 from empty stack => underflow
        max_stack: 0,
    }]);
    assert!(matches!(
        validate_gate_07_expression_stack_depth(&parts),
        Err(ValidationError::ExpressionStackExceeded { .. })
    ));
}

#[test]
fn gate_07_accepts_single_node_workflow_with_no_expressions() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
}

#[test]
fn compute_stack_depth_single_load() {
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0))];
    assert_eq!(compute_stack_depth(&ops), Ok(1));
}

#[test]
fn compute_stack_depth_load_and_binary() {
    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Eq,
    ];
    // max depth = 2 (after two loads), then Eq reduces to 1
    assert_eq!(compute_stack_depth(&ops), Ok(2));
}

#[test]
fn compute_stack_depth_empty() {
    let ops: Vec<ExprOp> = vec![];
    assert_eq!(compute_stack_depth(&ops), Ok(0));
}

// ---------------------------------------------------------------------------
// Gate 8 tests
// ---------------------------------------------------------------------------

use crate::gate_08_accessor::validate_gate_08_accessor_path_segments;

#[test]
fn gate_08_accepts_empty_accessors() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
}

#[test]
fn gate_08_accepts_valid_accessor() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 2, 2);
    parts.accessors = Box::new([AccessorProgram {
        root: SlotIdx::new(0),
        path: Box::new([PathSegment::Field(SymbolId::new(1))]),
    }]);
    assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
}

#[test]
fn gate_08_rejects_accessor_root_out_of_range() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.accessors = Box::new([AccessorProgram {
        root: SlotIdx::new(5), // out of range for slot_count=1
        path: Box::new([]),
    }]);
    assert!(matches!(
        validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSlotOutOfRange { .. })
    ));
}

#[test]
fn gate_08_rejects_sentinel_index_segment() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.accessors = Box::new([AccessorProgram {
        root: SlotIdx::new(0),
        path: Box::new([PathSegment::Index(u32::MAX)]),
    }]);
    assert!(matches!(
        validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorPathInvalid { .. })
    ));
}

#[test]
fn gate_08_accepts_accessor_with_empty_path() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.accessors = Box::new([AccessorProgram {
        root: SlotIdx::new(0),
        path: Box::new([]),
    }]);
    assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
}

#[test]
fn gate_08_rejects_max_value_index_segment() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.accessors = Box::new([AccessorProgram {
        root: SlotIdx::new(0),
        path: Box::new([PathSegment::Index(u32::MAX)]),
    }]);
    assert!(matches!(
        validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorPathInvalid { .. })
    ));
}

#[test]
fn gate_08_accepts_zero_index_segment() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.accessors = Box::new([AccessorProgram {
        root: SlotIdx::new(0),
        path: Box::new([PathSegment::Index(0)]),
    }]);
    assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
}

// ---------------------------------------------------------------------------
// Gate 9 tests
// ---------------------------------------------------------------------------

use crate::gate_09_slots::validate_gate_09_slot_references;

#[test]
fn gate_09_accepts_valid_slot_references() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
}

#[test]
fn gate_09_rejects_output_slot_out_of_range() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(99)), // out of range for slot_count=1
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_09_slot_references(&parts),
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

#[test]
fn gate_09_rejects_copy_source_out_of_range() {
    let node = copy_node(0, 50, 0); // source=50 out of range for slot_count=1
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_09_slot_references(&parts),
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

#[test]
fn gate_09_rejects_expr_load_slot_out_of_range() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(99))]),
        max_stack: 1,
    }]);
    assert!(matches!(
        validate_gate_09_slot_references(&parts),
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

#[test]
fn gate_09_accepts_single_node_workflow() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
}

#[test]
fn gate_09_accepts_slot_at_boundary_slot_count_minus_one() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
}

#[test]
fn gate_09_rejects_build_object_slot_out_of_range() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::BuildObject {
            fields: Box::new([(SymbolId::new(1), SlotIdx::new(99))]),
        },
    };
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_09_slot_references(&parts),
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

#[test]
fn gate_09_rejects_build_list_slot_out_of_range() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::BuildList {
            items: Box::new([SlotIdx::new(50)]),
        },
    };
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_09_slot_references(&parts),
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

// ---------------------------------------------------------------------------
// Gate 11 tests
// ---------------------------------------------------------------------------

use crate::gate_11_loop::validate_gate_11_loop_body_graph;

#[test]
fn gate_11_accepts_nop_workflow() {
    let parts = make_parts(vec![nop_node(0), finish_node(1, 0)], 1);
    assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
}

#[test]
fn gate_11_accepts_valid_for_each() {
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
                done: StepIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        },
        finish_node(2, 0),
    ];
    let parts = make_parts(nodes, 2);
    assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
}

#[test]
fn gate_11_rejects_for_each_body_out_of_range() {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 10,
            body: StepIdx::new(99), // out of range
            done: StepIdx::new(2),
        },
    }];
    let parts = make_parts(nodes, 2);
    assert!(matches!(
        validate_gate_11_loop_body_graph(&parts),
        Err(ValidationError::LoopBodyStepOutOfRange { .. })
    ));
}

#[test]
fn gate_11_rejects_for_each_done_out_of_range() {
    let nodes = vec![CompiledNode {
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
            done: StepIdx::new(99), // out of range
        },
    }];
    let parts = make_parts(nodes, 2);
    assert!(matches!(
        validate_gate_11_loop_body_graph(&parts),
        Err(ValidationError::LoopBodyStepOutOfRange { .. })
    ));
}

#[test]
fn gate_11_accepts_valid_together() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(2)]),
                join: StepIdx::new(3),
            },
        },
        nop_node(1),
        nop_node(2),
        finish_node(3, 0),
    ];
    let parts = make_parts(nodes, 1);
    assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
}

#[test]
fn gate_11_rejects_together_branch_out_of_range() {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::TogetherStart {
            branches: Box::new([StepIdx::new(99)]),
            join: StepIdx::new(1),
        },
    }];
    let parts = make_parts(nodes, 1);
    assert!(matches!(
        validate_gate_11_loop_body_graph(&parts),
        Err(ValidationError::LoopBodyStepOutOfRange { .. })
    ));
}

#[test]
fn gate_11_rejects_loop_body_before_start() {
    // Body at index 0, start at index 0 => not forward
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 10,
            body: StepIdx::new(0), // same as start, not forward
            done: StepIdx::new(1),
        },
    }];
    let parts = make_parts(nodes, 2);
    assert!(matches!(
        validate_gate_11_loop_body_graph(&parts),
        Err(ValidationError::LoopBodyStepOutOfRange { .. })
    ));
}

#[test]
fn gate_11_accepts_valid_repeat() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 3,
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatAttempt {
                attempt_slot: SlotIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatCheck {
                attempt_slot: SlotIdx::new(0),
                done: StepIdx::new(3),
            },
        },
        finish_node(3, 0),
    ];
    let parts = make_parts(nodes, 1);
    assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
}

#[test]
fn gate_11_accepts_together_with_empty_branches() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([]),
                join: StepIdx::new(1),
            },
        },
        finish_node(1, 0),
    ];
    let parts = make_parts(nodes, 1);
    // Empty branches is structurally valid per gate 11 (no out-of-range steps)
    assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
}

#[test]
fn gate_11_rejects_for_each_done_before_body() {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 10,
            body: StepIdx::new(2),
            done: StepIdx::new(1), // done < body => invalid span
        },
    }];
    let parts = make_parts(nodes, 2);
    assert!(matches!(
        validate_gate_11_loop_body_graph(&parts),
        Err(ValidationError::LoopBodyStepOutOfRange { .. })
    ));
}

#[test]
fn gate_11_accepts_single_node_workflow() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
}

// ---------------------------------------------------------------------------
// Gate 13 tests
// ---------------------------------------------------------------------------

use crate::gate_13_cycles::validate_gate_13_no_slot_cycles;

#[test]
fn gate_13_accepts_empty_slots() {
    let parts = make_parts(vec![nop_node(0)], 0);
    assert_eq!(validate_gate_13_no_slot_cycles(&parts), Ok(()));
}

#[test]
fn gate_13_accepts_linear_slot_chain() {
    // slot 0 <- const, slot 1 <- slot 0, slot 2 <- slot 1
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
    assert_eq!(validate_gate_13_no_slot_cycles(&parts), Ok(()));
}

#[test]
fn gate_13_rejects_direct_slot_cycle() {
    // slot 0 writes from slot 1, slot 1 writes from slot 0 => cycle
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
        validate_gate_13_no_slot_cycles(&parts),
        Err(ValidationError::SlotDependencyCycle { .. })
    ));
}

#[test]
fn gate_13_accepts_self_copy_is_not_cycle() {
    // A node that reads and writes the same slot is not a cycle in our
    // model because we filter out self-edges in the adjacency list.
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
    assert_eq!(validate_gate_13_no_slot_cycles(&parts), Ok(()));
}

#[test]
fn gate_13_rejects_three_slot_cycle() {
    // slot 0 <- slot 1, slot 1 <- slot 2, slot 2 <- slot 0
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
        validate_gate_13_no_slot_cycles(&parts),
        Err(ValidationError::SlotDependencyCycle { .. })
    ));
}

#[test]
fn gate_13_accepts_diamond_dependency() {
    // slot 0 <- const, slot 1 <- slot 0, slot 2 <- slot 0, slot 3 <- slot 1 + slot 2
    // No cycle.
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
    assert_eq!(validate_gate_13_no_slot_cycles(&parts), Ok(()));
}

#[test]
fn gate_13_rejects_cycle_through_eval_expr() {
    // slot 0 writes from an expression that loads slot 1,
    // slot 1 writes from slot 0 => cycle through EvalExpr.
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
    }]);
    assert!(
        matches!(
            validate_gate_13_no_slot_cycles(&parts),
            Err(ValidationError::SlotDependencyCycle { .. })
        ),
        "gate 13 must detect cycle through EvalExpr LoadSlot"
    );
}

#[test]
fn gate_13_accepts_linear_chain_through_eval_expr() {
    // slot 0 <- const, slot 1 <- expr(slot 0) => no cycle
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
    }]);
    assert_eq!(validate_gate_13_no_slot_cycles(&parts), Ok(()));
}

#[test]
fn gate_13_rejects_three_slot_cycle_through_eval_expr() {
    // slot 0 <- expr(slot 2), slot 1 <- slot 0, slot 2 <- slot 1 => cycle
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
    }]);
    assert!(
        matches!(
            validate_gate_13_no_slot_cycles(&parts),
            Err(ValidationError::SlotDependencyCycle { .. })
        ),
        "gate 13 must detect 3-slot cycle through EvalExpr"
    );
}
