    use super::*;
    use vb_core::ids::{ConstIdx, StepIdx, SymbolId};
    use vb_core::workflow::ResourceContract;

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

    // -- Pass cases --

    #[test]
    fn accepts_valid_finish_node() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_copy_node() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        };
        let parts = make_parts(vec![node, finish_node(1, 0)], 2);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_build_object() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(2)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildObject {
                fields: Box::new([
                    (SymbolId::new(1), SlotIdx::new(0)),
                    (SymbolId::new(2), SlotIdx::new(1)),
                ]),
            },
        };
        let parts = make_parts(vec![node], 3);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_build_list() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(2)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildList {
                items: Box::new([SlotIdx::new(0), SlotIdx::new(1)]),
            },
        };
        let parts = make_parts(vec![node], 3);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_set_const() {
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
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    #[test]
    fn accepts_expr_with_valid_slot_ref() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 2);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(1))]),
            max_stack: 1,
        }]);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    #[test]
    fn accepts_nop_node() {
        let parts = make_parts(vec![nop_node(0)], 0);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    // -- Fail cases --

    #[test]
    fn rejects_output_slot_out_of_range() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(99)),
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
    fn rejects_copy_source_out_of_range() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(50),
            },
        };
        let parts = make_parts(vec![node], 1);
        assert!(matches!(
            validate_gate_09_slot_references(&parts),
            Err(ValidationError::SlotReferenceOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_expr_load_slot_out_of_range() {
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
    fn rejects_build_object_slot_out_of_range() {
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
    fn rejects_build_list_slot_out_of_range() {
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

    #[test]
    fn rejects_do_input_out_of_range() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(1),
                input: SlotIdx::new(99),
            },
        };
        let parts = make_parts(vec![node], 1);
        assert!(matches!(
            validate_gate_09_slot_references(&parts),
            Err(ValidationError::SlotReferenceOutOfRange { .. })
        ));
    }
