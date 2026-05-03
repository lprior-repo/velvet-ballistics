use super::helpers::*;

    #[test]
    fn compile_with_limits_respects_custom_source_limit() {
        let source = OPTIONAL_TOP_LEVEL_FIELDS_SOURCE;
        let limits = YamlLimits {
            max_source_bytes: source.len() + 1,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler { limits };
        let result = compiler.compile(source);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok, got {:?}", result)
        };
        assert_eq!(wf.node_count(), 2);
    }

    #[test]
    fn compile_to_generated_rust_accepts_supported_subset() -> Result<(), String> {
        let workflow = supported_codegen_workflow()?;

        let source = compile_to_generated_rust(&workflow).map_err(|e| e.to_string())?;

        assert!(
            source.contains("pub fn drive"),
            "generated source must contain drive function"
        );
        Ok(())
    }

    #[test]
    fn compile_to_generated_rust_rejects_unsupported_ir_before_emit() -> Result<(), String> {
        let workflow = unsupported_codegen_workflow()?;

        let error = compile_to_generated_rust(&workflow)
            .err()
            .ok_or("unsupported IR unexpectedly generated source")?;

        assert!(
            error.to_string().contains("BuildList"),
            "unsupported IR error must name rejected feature, got: {error}"
        );
        Ok(())
    }

    #[test]
    fn compile_to_generated_rust_reports_subset_rejection_as_compile_error() -> Result<(), String> {
        let workflow = unsupported_codegen_workflow()?;

        let errors = compile_to_generated_rust(&workflow)
            .err()
            .ok_or("unsupported IR unexpectedly generated source")?;
        let first = errors
            .0
            .first()
            .ok_or("unsupported IR must produce a compile error")?;

        assert_eq!(first.diagnostic_code(), "INVALID_EXPRESSION");
        assert!(
            first
                .to_string()
                .contains("unsupported generated Rust IR feature"),
            "generated-mode subset rejection must be explicit, got: {first}"
        );
        Ok(())
    }

    fn supported_codegen_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("compile_codegen_supported"),
            digest: WorkflowDigest::from_bytes([0x31; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    error_slot: None,
                    on_error: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    error_slot: None,
                    on_error: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(7)].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn unsupported_codegen_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("compile_codegen_unsupported"),
            digest: WorkflowDigest::from_bytes([0x32; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(1)),
                    error_slot: None,
                    on_error: None,
                    kind: CompiledNodeKind::BuildList {
                        items: vec![SlotIdx::new(0)].into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    error_slot: None,
                    on_error: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    // ── Round 2: CompileError::code() tests ──────────────────────────────

    #[test]
    fn compile_error_code_returns_payload_too_large_for_source_too_large() {
        let err = CompileError::SourceTooLarge {
            actual: 100,
            limit: 50,
        };
        assert_eq!(err.code(), "PAYLOAD_TOO_LARGE");
    }

    #[test]
    fn compile_error_code_returns_missing_required_field_for_empty_source() {
        let err = CompileError::EmptySource;
        assert_eq!(err.code(), "MISSING_REQUIRED_FIELD");
    }

    #[test]
    fn compile_error_code_returns_type_mismatch_for_top_level_not_mapping() {
        let err = CompileError::TopLevelNotMapping;
        assert_eq!(err.code(), "TYPE_MISMATCH");
    }

    #[test]
    fn compile_error_code_returns_duplicate_key_for_duplicate_key() {
        let err = CompileError::DuplicateKey {
            key: Box::from("test"),
            mark: SourceMark {
                index: 0,
                end_index: 0,
                line: 1,
                column: 1,
                available: true,
            },
        };
        assert_eq!(err.code(), "DUPLICATE_KEY");
    }

    #[test]
    fn compile_error_code_returns_limit_exceeded_for_depth_limit() {
        let err = CompileError::DepthLimit {
            depth: 10,
            limit: 5,
        };
        assert_eq!(err.code(), "LIMIT_EXCEEDED");
    }

    #[test]
    fn compile_error_code_returns_limit_exceeded_for_node_limit() {
        let err = CompileError::NodeLimit { limit: 100 };
        assert_eq!(err.code(), "LIMIT_EXCEEDED");
    }

