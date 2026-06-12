// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

use super::*;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::ResourceContract;

// Helper: build minimal WorkflowParts with just nodes and slot_count.
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

// ===== Gate 7 tests =====

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

// ===== Gate 8 tests =====

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

// ===== Gate 9 tests =====

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

// ===== Gate 11 tests =====

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

// ===== Gate 13 tests =====

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
                value: vb_core::ids::ConstIdx::new(0),
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
    assert_eq!(
        validate_gate_13_no_slot_cycles(&parts),
        Err(ValidationError::SlotDependencyCycle {
            slot: 1,
            chain: "slot 1 -> slot 0".into(),
        })
    );
}

#[test]
fn gate_13_accepts_self_copy_not_cycle() {
    // A self-copy is a no-op dependency, not a cross-slot cycle.
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
fn gate13_accepts_direct_self_dependency() {
    // Given an expression node that writes slot 0.
    let mut parts = make_parts(vec![finish_node(1, 0)], 1);
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
        max_stack: 1,
    }]);
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(0),
        },
    };
    parts.nodes = Box::new([node, finish_node(1, 0)]);

    // When the expression also reads slot 0.
    // Then Gate 13 treats the self edge as an in-place update, not a cycle.
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
                value: vb_core::ids::ConstIdx::new(0),
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

// ===== Compute stack depth tests =====

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

// ===== Adversarial tests: Gate 13 EvalExpr cycle detection =====

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

// ===== Adversarial tests: Gate 7 edge cases =====

#[test]
fn gate_07_rejects_underflow_binary_op_on_empty_stack() {
    let mut parts = make_parts(vec![finish_node(0, 0)], 1);
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::Eq]), // pops 2 from empty stack => underflow
        max_stack: 0,
    }]);
    assert!(
        matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackExceeded { .. })
        ),
        "gate 7 must reject binary op on empty stack (stack underflow)"
    );
}

#[test]
fn gate_07_accepts_single_node_workflow_with_no_expressions() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
}

// ===== Adversarial tests: Gate 8 edge cases =====

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
    assert!(
        matches!(
            validate_gate_08_accessor_path_segments(&parts),
            Err(ValidationError::AccessorPathInvalid { .. })
        ),
        "gate 8 must reject u32::MAX index segment"
    );
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

// ===== Adversarial tests: Gate 9 edge cases =====

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
    assert!(
        matches!(
            validate_gate_09_slot_references(&parts),
            Err(ValidationError::SlotReferenceOutOfRange { .. })
        ),
        "gate 9 must reject BuildObject with out-of-range slot"
    );
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
    assert!(
        matches!(
            validate_gate_09_slot_references(&parts),
            Err(ValidationError::SlotReferenceOutOfRange { .. })
        ),
        "gate 9 must reject BuildList with out-of-range slot"
    );
}

// ===== Adversarial tests: Gate 11 edge cases =====

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
    assert!(
        matches!(
            validate_gate_11_loop_body_graph(&parts),
            Err(ValidationError::LoopBodyStepOutOfRange { .. })
        ),
        "gate 11 must reject done step before body step"
    );
}

#[test]
fn gate_11_accepts_single_node_workflow() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
}

// =========================================================================
// BLACKHAT security regression tests
// =========================================================================

/// BLACKHAT: stack_effect must not use `as` casts (engineering rule).
///
/// SEVERITY: MEDIUM (engineering rule violation -- `as` casts can silently
/// truncate or wrap, violating the "no `as` casts" rule)
/// DESCRIPTION: The `stack_effect` helper previously used `push as i8` and
/// `pop as i8` with `#[allow(clippy::as_conversions)]`. This was replaced
/// with safe `i16::from()` widening conversion and `i8::try_from()`.
/// This test verifies the function produces correct values for all op
/// categories: load (net +1), unary (net 0), binary (net -1), ternary
/// (net -2).
#[test]
fn blackhat_stack_effect_no_as_casts_correct_values() {
    // LoadSlot: pop 0, push 1 => net +1
    assert_eq!(stack_effect(&ExprOp::LoadSlot(SlotIdx::new(0))), 1);
    // LoadConst: pop 0, push 1 => net +1
    assert_eq!(stack_effect(&ExprOp::LoadConst(ConstIdx::new(0))), 1);
    // LoadAccessor: pop 0, push 1 => net +1
    assert_eq!(stack_effect(&ExprOp::LoadAccessor(AccessorIdx::new(0))), 1);
    // Not: pop 1, push 1 => net 0
    assert_eq!(stack_effect(&ExprOp::Not), 0);
    // Exists: pop 1, push 1 => net 0
    assert_eq!(stack_effect(&ExprOp::Exists), 0);
    // Length: pop 1, push 1 => net 0
    assert_eq!(stack_effect(&ExprOp::Length), 0);
    // Eq: pop 2, push 1 => net -1
    assert_eq!(stack_effect(&ExprOp::Eq), -1);
    // Add: pop 2, push 1 => net -1
    assert_eq!(stack_effect(&ExprOp::Add), -1);
    // AppendIf: pop 3, push 1 => net -2
    assert_eq!(stack_effect(&ExprOp::AppendIf), -2);
}

/// BLACKHAT: compute_stack_depth correctly detects stack underflow.
///
/// SEVERITY: HIGH (could allow malformed expression programs to pass
/// validation, leading to runtime stack corruption)
/// DESCRIPTION: A binary op on an empty stack should cause underflow
/// detection. This verifies that `checked_sub` correctly catches the
/// underflow and returns an error instead of wrapping.
#[test]
fn blackhat_compute_stack_depth_rejects_underflow_from_binary_op() {
    let ops = vec![ExprOp::Eq];
    let result = compute_stack_depth(&ops);
    assert!(
        matches!(result, Err(ValidationError::ExpressionStackExceeded { .. })),
        "blackhat: binary op on empty stack must cause stack underflow error"
    );
}

/// BLACKHAT: compute_stack_depth rejects ternary op (AppendIf) with
/// insufficient stack depth.
///
/// SEVERITY: HIGH
/// DESCRIPTION: AppendIf pops 3 values; with only 1 on the stack, it should
/// fail with underflow.
#[test]
fn blackhat_compute_stack_depth_rejects_append_if_underflow() {
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::AppendIf];
    let result = compute_stack_depth(&ops);
    assert!(
        matches!(result, Err(ValidationError::ExpressionStackExceeded { .. })),
        "blackhat: AppendIf with only 1 value on stack must cause underflow"
    );
}

/// BLACKHAT: compute_stack_depth accepts valid expression with max depth.
///
/// SEVERITY: INFO (correctness verification)
#[test]
fn blackhat_compute_stack_depth_accepts_valid_expression() {
    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::LoadSlot(SlotIdx::new(2)),
        ExprOp::Eq,
        ExprOp::Not,
    ];
    // Stack: 1, 2, 3 -> Eq pops 2 pushes 1 => 2 -> Not pops 1 pushes 1 => 2
    // Max depth = 3
    let result = compute_stack_depth(&ops);
    assert_eq!(result, Ok(3));
}

/// BLACKHAT: Gate 10 rejects Do node with sentinel action_id.
///
/// SEVERITY: HIGH (sentinel action_id could bypass action contract
/// validation)
/// DESCRIPTION: A Do node with action_id set to u16::MAX (sentinel) must
/// be rejected by gate 10.
#[test]
fn blackhat_gate_10_rejects_sentinel_action_id() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(u16::MAX),
            input: SlotIdx::new(0),
        },
    };
    let mut parts = make_parts(vec![node], 1);
    parts.constants = Box::new([vb_core::value::ConstValue::Null]);
    let result = validate_gate_10_node_kind_specific(&parts);
    assert!(
        matches!(
            result,
            Err(ValidationError::NodeKindConstraintViolation { .. })
        ),
        "blackhat: sentinel action_id must be rejected"
    );
}

/// BLACKHAT: Gate 14 detects slot type inconsistency (I64 vs Bool).
///
/// SEVERITY: MEDIUM (type inconsistency in slots could cause runtime
/// type errors or memory safety issues)
/// DESCRIPTION: When two SetConst nodes write incompatible types (I64 vs
/// Bool) to the same slot, gate 14 must detect the inconsistency.
#[test]
fn blackhat_gate_14_rejects_incompatible_const_types() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0), // I64
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(1), // Bool
            },
        },
    ];
    let mut parts = make_parts(nodes, 1);
    parts.constants = Box::new([
        vb_core::value::ConstValue::I64(42),
        vb_core::value::ConstValue::Bool(true),
    ]);
    let result = validate_gate_14_slot_type_consistency(&parts);
    assert!(
        matches!(
            result,
            Err(ValidationError::SlotTypeInconsistency { slot: 0 })
        ),
        "blackhat: I64 and Bool writers to same slot must be rejected"
    );
}

/// BLACKHAT: Gate 15 rejects consecutive non-deterministic nodes.
///
/// SEVERITY: HIGH (consecutive non-deterministic nodes could violate
/// journal replay determinism)
/// DESCRIPTION: Two Do nodes chained together via `next` must be rejected
/// by the determinism proof gate.
#[test]
fn blackhat_gate_15_rejects_consecutive_do_nodes() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(2),
                input: SlotIdx::new(1),
            },
        },
        finish_node(2, 1),
    ];
    let parts = make_parts(nodes, 2);
    let result = validate_gate_15_determinism_proof(&parts);
    assert!(
        matches!(
            result,
            Err(ValidationError::NonDeterministicPath {
                from_node: 0,
                to_node: 1
            })
        ),
        "blackhat: consecutive Do nodes must be rejected as non-deterministic path"
    );
}

/// BLACKHAT: Gate 12 rejects orphan action contracts (contract with no Do node).
///
/// SEVERITY: MEDIUM (orphan contracts indicate compilation errors or
/// potential dead code that could mask security issues)
#[test]
fn blackhat_gate_12_rejects_orphan_contract() {
    let nodes = vec![finish_node(0, 0)];
    let parts = make_parts(nodes, 1);
    let contracts = vec![vb_core::action::ActionContract {
        id: ActionId::new(99),
        name: vb_core::action::ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: vb_core::action::Idempotency::DeterministicPure,
        side_effect: vb_core::action::SideEffect::Pure,
        retry_safety: vb_core::action::RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    }];
    let result = validate_gate_12_action_contract_completeness(&parts, &contracts);
    assert!(
        matches!(
            result,
            Err(ValidationError::ActionContractOrphan { action_id: 99 })
        ),
        "blackhat: orphan contract with no Do node must be rejected"
    );
}

// =========================================================================
// vb-u09ai: 4-variant RetrySafety gate test (Tier 1).
// =========================================================================

/// Tier 1: `vb_core::action::is_idempotent(RetrySafety::Idempotent) == true`
/// per the master §65 contract (C6). The `is_idempotent(RetrySafety)` const
/// fn is a TDD target State 11 will add — on 3-variant code this test
/// fails to compile (preserves the failing-first signal).
#[test]
fn blackhat_gate_12_idempotent_retry_safety_recognized() {
    use vb_core::action::{is_idempotent, RetrySafety};
    assert!(
        is_idempotent(RetrySafety::Idempotent),
        "Idempotent must be considered idempotent (C6)"
    );
}
