#[cfg(test)]
mod proptests {
    use crate::{
        CodegenError, compare_generated_to_ir, emit_action_boundary, emit_resource_contract,
        emit_rust_workflow, validate_generated_subset,
    };
    use proptest::prelude::*;
    use vb_core::{
        ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue,
        ExprProgram, ResourceContract, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
    };

    fn arb_resource_contract() -> impl Strategy<Value = ResourceContract> {
        (
            1u16..100u16,
            1u16..100u16,
            1u16..100u16,
            1u16..100u16,
            1u16..100u16,
            1u8..64u8,
            1u32..10000u32,
            1u32..10000u32,
        )
            .prop_map(
                |(
                    steps,
                    slots,
                    constants,
                    accessors,
                    expressions,
                    expr_stack,
                    input_bytes,
                    output_bytes,
                )| {
                    ResourceContract {
                        max_steps: steps,
                        max_slots: slots,
                        max_constants: constants,
                        max_accessors: accessors,
                        max_expressions: expressions,
                        max_expr_stack: expr_stack,
                        max_input_bytes: input_bytes,
                        max_output_bytes: output_bytes,
                        max_step_budget_per_tick: 500,
                        max_blob_bytes: 1024,
                        max_ipc_payload_bytes: 2048,
                        max_retry_attempts: 3,
                        max_fanout: 8,
                        max_collect_items: 100,
                        max_queue_depth: 64,
                        max_journal_batch_bytes: 512,
                    }
                },
            )
    }

    proptest! {
        #[test]
        fn emit_resource_contract_output_contains_all_fields(contract in arb_resource_contract()) {
            let mut out = String::new();
            let result = emit_resource_contract(&mut out, contract);
            prop_assert!(result.is_ok(), "encoding valid resource contract must succeed");
            prop_assert!(!out.is_empty());
            prop_assert!(out.contains("CONTRACT_MAX_STEPS"));
            prop_assert!(out.contains("CONTRACT_MAX_SLOTS"));
            prop_assert!(out.contains("CONTRACT_MAX_CONSTANTS"));
            prop_assert!(out.contains("CONTRACT_MAX_ACCESSORS"));
            prop_assert!(out.contains("CONTRACT_MAX_EXPRESSIONS"));
            prop_assert!(out.contains("CONTRACT_MAX_EXPR_STACK"));
            prop_assert!(out.contains("CONTRACT_MAX_INPUT_BYTES"));
            prop_assert!(out.contains("CONTRACT_MAX_OUTPUT_BYTES"));
        }

        #[test]
        fn codegen_error_display_never_empty(error_idx in 0u8..6u8) {
            let error = match error_idx {
                0 => CodegenError::FormatBufferOverflow,
                1 => CodegenError::RustfmtFailed { detail: String::from("test") },
                2 => CodegenError::CompileCheckFailed { detail: String::from("test") },
                3 => CodegenError::SemanticMismatch { detail: String::from("test") },
                4 => CodegenError::Io(std::io::Error::other("io")),
                _ => CodegenError::TrybuildFixture { detail: String::from("test") },
            };
            let message = error.to_string();
            prop_assert!(!message.is_empty(), "error display must never be empty");
        }

        #[test]
        fn generated_source_always_forbids_unsafe(slot_count in 1u16..10u16) {
            let parts = WorkflowParts {
                name: Box::<str>::from("prop_test"),
                digest: WorkflowDigest::from_bytes([0x42; 32]),
                nodes: vec![
                    CompiledNode {
                        id: StepIdx::new(0),
                        output: None,
                        next: None,
                        on_error: None,
                        error_slot: None,
                        kind: CompiledNodeKind::Finish { result: SlotIdx::new(0) },
                    },
                ].into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: Box::new([]),
                slot_count,
                symbols_count: 0,
                entry: StepIdx::new(0),
                resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
            };
            if let Ok(workflow) = CompiledWorkflow::try_from_parts(parts)
                && let Ok(source) = emit_rust_workflow(&workflow)
            {
                prop_assert!(source.contains("#![forbid(unsafe_code)]"));
                prop_assert!(source.contains("#![deny(unused_must_use)]"));
            }
        }
    }

    // =======================================================================
    // Adversarial equivalence tests — codegen vs IR engine divergence
    // =======================================================================

    /// Verify that Exists is now supported by validate_generated_subset.
    #[test]
    fn exists_expression_now_supported_by_generated_subset() -> Result<(), String> {
        let ops = vec![
            vb_core::ExprOp::LoadConst(ConstIdx::new(0)),
            vb_core::ExprOp::Exists,
        ];
        let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;
        let parts = WorkflowParts {
            name: Box::<str>::from("test_exists_rejected"),
            digest: WorkflowDigest::from_bytes([0xDA; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::EvalExpr {
                        expr: vb_core::ExprIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        let result = validate_generated_subset(&workflow);
        // Exists is now supported: validation should succeed
        result.map_err(|e| format!("Exists should be supported but got: {e}"))?;
        Ok(())
    }

    /// Verify that compare_generated_to_ir correctly counts action boundaries
    /// when a workflow contains Do nodes.
    #[test]
    fn compare_generated_to_ir_counts_action_boundaries_for_do_workflow() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("test_do_action"),
            digest: WorkflowDigest::from_bytes([0xEF; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(2)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Do {
                        action: ActionId::new(5),
                        input: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(2),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 3,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
        let source = emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;
        let result = compare_generated_to_ir(&source, &workflow);
        assert!(
            result.is_ok(),
            "compare_generated_to_ir must accept Do workflow with action boundary markers, got: {result:?}"
        );
        // Verify the source actually contains the action boundary marker
        assert!(
            source.contains("Action boundary:"),
            "generated source must contain 'Action boundary:' marker for Do nodes"
        );
        Ok(())
    }

    /// Verify that the action boundary comment includes the correct action and slot IDs.
    #[test]
    fn emit_action_boundary_includes_action_marker_comment() -> Result<(), String> {
        let mut out = String::new();
        emit_action_boundary(&mut out, ActionId::new(5), SlotIdx::new(2))
            .map_err(|e| e.to_string())?;
        assert!(
            out.contains("Action boundary: action_id=5, input_slot=2"),
            "action boundary must include action_id and input_slot in comment, got: {out}"
        );
        Ok(())
    }
}
