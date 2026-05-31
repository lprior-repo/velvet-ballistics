    use super::*;
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ExprProgram, ResourceContract};

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

    // -- Pass cases --

    #[test]
    fn accepts_empty_expressions() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_single_load_expression() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
            max_stack: 1,
        }]);
        assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_load_and_binary_op() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 2);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Eq,
            ]),
            max_stack: 2,
        }]);
        assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
    }

    #[test]
    fn accepts_multiple_expressions() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 2);
        parts.expressions = Box::new([
            ExprProgram {
                ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
                max_stack: 1,
            },
            ExprProgram {
                ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(1))]),
                max_stack: 1,
            },
        ]);
        assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
    }

    #[test]
    fn accepts_unary_op_after_load() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Not]),
            max_stack: 1,
        }]);
        assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
    }

    // -- Fail cases --

    #[test]
    fn rejects_stack_mismatch_declared_too_high() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
            max_stack: 2,
        }]);
        assert!(matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackMismatch { .. })
        ));
    }

    #[test]
    fn rejects_stack_mismatch_declared_too_low() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
            max_stack: 0,
        }]);
        assert!(matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackMismatch { .. })
        ));
    }

    #[test]
    fn rejects_expression_exceeding_contract() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.resource_contract = ResourceContract {
            max_expr_stack: 1,
            ..ResourceContract::DEFAULT
        };
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(0)),
            ]),
            max_stack: 2,
        }]);
        assert!(matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackExceeded { .. })
        ));
    }

    #[test]
    fn rejects_contract_exceeding_protocol_limit() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.resource_contract = ResourceContract {
            max_expr_stack: 128,
            ..ResourceContract::DEFAULT
        };
        assert!(matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackExceeded { .. })
        ));
    }

    #[test]
    fn rejects_underflow_binary_op_on_empty_stack() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::Eq]),
            max_stack: 0,
        }]);
        assert!(matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackExceeded { .. })
        ));
    }

    #[test]
    fn rejects_underflow_unary_op_on_empty_stack() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::Not]),
            max_stack: 0,
        }]);
        assert!(matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackExceeded { .. })
        ));
    }

    // -- compute_stack_depth unit tests --

    #[test]
    fn compute_depth_empty() {
        assert_eq!(compute_stack_depth(&[]), Ok(0));
    }

    #[test]
    fn compute_depth_single_load() {
        assert_eq!(
            compute_stack_depth(&[ExprOp::LoadSlot(SlotIdx::new(0))]),
            Ok(1)
        );
    }

    #[test]
    fn compute_depth_load_and_binary() {
        assert_eq!(
            compute_stack_depth(&[
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Eq,
            ]),
            Ok(2)
        );
    }

    #[test]
    fn compute_depth_three_loads_then_two_binary() {
        // [load, load, load] -> depth 3; [add] -> depth 2; [eq] -> depth 1
        assert_eq!(
            compute_stack_depth(&[
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::LoadSlot(SlotIdx::new(2)),
                ExprOp::Add,
                ExprOp::Eq,
            ]),
            Ok(3)
        );
    }
