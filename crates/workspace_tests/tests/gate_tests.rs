#![forbid(unsafe_code)]
//! Additional unit tests for Gates 7, 8, 9, 11, 13 (bead vb-qi37.8).
//!
//! These tests complement the existing gate_tests.rs in vb_validate.

use vb_core::ids::{AccessorIdx, ConstIdx, SlotIdx, StepIdx, SymbolId};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
    ResourceContract, WorkflowParts,
};
use vb_validate::ValidationError;
use vb_validate::gates::{
use vb_core::span::Span;
    validate_gate_07_expression_stack_depth, validate_gate_08_accessor_path_segments,
    validate_gate_09_slot_references, validate_gate_10_node_kind_specific,
    validate_gate_11_loop_body_graph, validate_gate_13_no_slot_cycles,
};

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

fn nop_node(index: u16, next: Option<u16>) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: next.map(StepIdx::new),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }
}

// ===========================================================================
// Gate 7: Expression stack depth
// ===========================================================================

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
        max_stack: 2, // Wrong: actual is 1
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
        max_stack: 3, // Exceeds contract of 2
    }]);
    assert!(matches!(
        validate_gate_07_expression_stack_depth(&parts),
        Err(ValidationError::ExpressionStackExceeded { .. })
    ));
}

#[test]
fn gate_07_rejects_underflow_binary_op() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::Eq]), // Binary op on empty stack
        max_stack: 0,
    }]);
    assert!(matches!(
        validate_gate_07_expression_stack_depth(&parts),
        Err(ValidationError::ExpressionStackExceeded { .. })
    ));
}

// ===========================================================================
// Gate 8: Accessor path segments
// ===========================================================================

#[test]
fn gate_08_accepts_empty_accessors() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_08_accessor_path_segments(&parts), Ok(()));
}

#[test]
fn gate_08_accepts_valid_accessor() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 2);
    parts.symbols_count = 2;
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
        root: SlotIdx::new(5), // Out of range for slot_count=1
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
fn gate_08_rejects_field_symbol_out_of_bounds_precisely() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.symbols_count = 2;
    parts.accessors = Box::new([AccessorProgram {
        root: SlotIdx::new(0),
        path: Box::new([PathSegment::Index(0), PathSegment::Field(SymbolId::new(9))]),
    }]);

    assert_eq!(
        validate_gate_08_accessor_path_segments(&parts),
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 1,
            symbol: 9,
            symbols_count: 2,
         span: Span::ZERO})
    );
}

// ===========================================================================
// Gate 9: Slot references
// ===========================================================================

#[test]
fn gate_09_accepts_valid_slot_references() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
}

#[test]
fn gate_09_rejects_output_slot_out_of_range() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(99)), // Out of range for slot_count=1
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
    let node = copy_node(0, 50, 0); // source=50 out of range
    let parts = make_parts(vec![node], 1);
    assert!(matches!(
        validate_gate_09_slot_references(&parts),
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
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

// ===========================================================================
// Gate 10: Non-slot reference integrity
// ===========================================================================

#[test]
fn gate_10_accepts_expression_const_and_accessor_references_in_range() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.constants = Box::new([ConstValue::I64(7)]);
    parts.accessors = Box::new([AccessorProgram {
        root: SlotIdx::new(0),
        path: Box::new([]),
    }]);
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadAccessor(AccessorIdx::new(0)),
        ]),
        max_stack: 2,
    }]);

    assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
}

#[test]
fn gate_10_rejects_expression_const_reference_out_of_range() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadConst(ConstIdx::new(0))]),
        max_stack: 1,
    }]);

    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { node_index: 0, detail , span: Span::ZERO})
            if detail == "Expression 0 LoadConst const index 0 out of range (const_count 0)"
    ));
}

#[test]
fn gate_10_rejects_expression_accessor_reference_out_of_range() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadAccessor(AccessorIdx::new(0))]),
        max_stack: 1,
    }]);

    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { node_index: 0, detail , span: Span::ZERO})
            if detail == "Expression 0 LoadAccessor accessor index 0 out of range (accessor_count 0)"
    ));
}

#[test]
fn gate_10_rejects_build_object_field_symbol_out_of_range() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::BuildObject {
            fields: Box::new([(SymbolId::new(7), SlotIdx::new(0))]),
        },
    };
    let mut parts = make_parts(vec![node], 1);
    parts.symbols_count = 2;

    assert!(matches!(
        validate_gate_10_node_kind_specific(&parts),
        Err(ValidationError::NodeKindConstraintViolation { node_index: 0, detail , span: Span::ZERO})
            if detail == "BuildObject field 0 symbol 7 out of range (symbols_count 2)"
    ));
}

// ===========================================================================
// Gate 11: Loop body graph
// ===========================================================================

#[test]
fn gate_11_accepts_nop_workflow() {
    let parts = make_parts(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            finish_node(1, 0),
        ],
        1,
    );
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
            body: StepIdx::new(99), // Out of range
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
        finish_node(3, 0),
    ];
    let parts = make_parts(nodes, 1);
    assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
}

#[test]
fn gate_11_rejects_loop_body_before_start() {
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
            body: StepIdx::new(0), // Same as start, not forward
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
fn gate_11_rejects_orphan_foreach_join() {
    let parts = make_parts(
        vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(0),
            },
        }],
        1,
    );
    assert!(matches!(
        validate_gate_11_loop_body_graph(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn gate_11_rejects_orphan_collect_next() {
    let parts = make_parts(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectNext {
                    collector_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            nop_node(1, Some(2)),
            finish_node(2, 0),
        ],
        1,
    );
    assert!(matches!(
        validate_gate_11_loop_body_graph(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn gate_11_rejects_collect_finish_without_start() {
    let parts = make_parts(
        vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(0),
            },
        }],
        1,
    );
    assert!(matches!(
        validate_gate_11_loop_body_graph(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn gate_11_rejects_reduce_next_with_mismatched_start() {
    let parts = make_parts(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ReduceStart {
                    input: SlotIdx::new(0),
                    accumulator: SlotIdx::new(1),
                    initial: ConstIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ReduceNext {
                    iterator_slot: SlotIdx::new(0),
                    accumulator: SlotIdx::new(1),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            finish_node(2, 0),
            finish_node(3, 0),
        ],
        2,
    );
    assert!(matches!(
        validate_gate_11_loop_body_graph(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn gate_11_rejects_together_branch_without_start() {
    let parts = make_parts(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherBranch {
                    branch: 0,
                    entry: StepIdx::new(1),
                    join: StepIdx::new(2),
                    accumulator: SlotIdx::new(0),
                },
            },
            nop_node(1, Some(2)),
            finish_node(2, 0),
        ],
        1,
    );
    assert!(matches!(
        validate_gate_11_loop_body_graph(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn gate_11_rejects_together_join_branch_count_mismatch() {
    let parts = make_parts(
        vec![
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
            nop_node(1, Some(3)),
            nop_node(2, Some(3)),
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherJoin {
                    branch_count: 1,
                    accumulator: SlotIdx::new(0),
                },
            },
        ],
        1,
    );
    assert!(matches!(
        validate_gate_11_loop_body_graph(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

#[test]
fn gate_11_rejects_repeat_check_without_start() {
    let parts = make_parts(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::RepeatCheck {
                    attempt_slot: SlotIdx::new(0),
                    done: StepIdx::new(1),
                },
            },
            finish_node(1, 0),
        ],
        1,
    );
    assert!(matches!(
        validate_gate_11_loop_body_graph(&parts),
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

// ===========================================================================
// Gate 13: Slot cycles
// ===========================================================================

#[test]
fn gate_13_accepts_empty_slots() {
    let parts = make_parts(
        vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }],
        0,
    );
    assert_eq!(validate_gate_13_no_slot_cycles(&parts), Ok(()));
}

#[test]
fn gate_13_accepts_linear_slot_chain() {
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
    let nodes = vec![
        copy_node(0, 1, 0), // slot 0 reads from slot 1
        copy_node(1, 0, 1), // slot 1 reads from slot 0 => cycle
    ];
    let parts = make_parts(nodes, 2);
    assert!(matches!(
        validate_gate_13_no_slot_cycles(&parts),
        Err(ValidationError::SlotDependencyCycle { .. })
    ));
}

#[test]
fn gate_13_accepts_self_copy_not_cycle() {
    // A node that reads and writes the same slot is an in-place update, not a cycle.
    let nodes = vec![copy_node(0, 0, 0)]; // slot 0 reads from slot 0
    let parts = make_parts(nodes, 1);
    assert_eq!(validate_gate_13_no_slot_cycles(&parts), Ok(()));
}

#[test]
fn gate_13_rejects_three_slot_cycle() {
    let nodes = vec![
        copy_node(0, 1, 0), // slot 0 reads slot 1
        copy_node(1, 2, 1), // slot 1 reads slot 2
        copy_node(2, 0, 2), // slot 2 reads slot 0 => cycle
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
        copy_node(2, 0, 2),
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
