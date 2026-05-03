use super::helpers::*;

    #[test]
    fn compile_error_code_returns_forbidden_yaml_for_alias() {
        let err = CompileError::AliasForbidden {
            mark: SourceMark {
                index: 0,
                end_index: 0,
                line: 1,
                column: 1,
                available: true,
            },
        };
        assert_eq!(err.code(), "FORBIDDEN_YAML_FEATURE");
    }

    #[test]
    fn compile_error_code_returns_forbidden_yaml_for_float() {
        let err = CompileError::FloatForbidden;
        assert_eq!(err.code(), "FORBIDDEN_YAML_FEATURE");
    }

    #[test]
    fn compile_error_code_returns_unknown_step_for_unsupported_primitive() {
        let err = CompileError::UnsupportedStepPrimitive {
            step: 0,
            primitive: "custom",
        };
        assert_eq!(err.code(), "UNKNOWN_STEP_FIELD");
    }

    #[test]
    fn compile_error_code_returns_backward_branch_for_backward_target() {
        let err = CompileError::BackwardBranchTarget { step: 2, target: 0 };
        assert_eq!(err.code(), "INVALID_THEN_TARGET");
    }

    #[test]
    fn compile_error_code_returns_type_mismatch_for_type_mismatch() {
        let err = CompileError::TypeMismatch {
            field: "test",
            expected: "text",
            found: "number",
        };
        assert_eq!(err.code(), "TYPE_MISMATCH");
    }

    #[test]
    fn compile_error_code_returns_expression_error_for_unexpected_char() {
        let err = CompileError::ExpressionUnexpectedChar {
            expression: Box::from("$x"),
            index: 1,
            found: '@',
        };
        assert_eq!(err.code(), "INVALID_EXPRESSION");
    }

    #[test]
    fn compile_error_code_returns_expression_error_for_helper_arity() {
        let err = CompileError::ExpressionHelperArity {
            helper: "len",
            expected: 1,
            actual: 2,
        };
        assert_eq!(err.code(), "INVALID_EXPRESSION");
    }

    // ── Round 2: YamlLimits and Compiler config tests ────────────────────

    #[test]
    fn yaml_limits_default_has_reasonable_values() {
        let defaults = YamlLimits::default();
        assert!(defaults.max_source_bytes > 0);
        assert!(defaults.max_depth > 0);
        assert!(defaults.max_nodes > 0);
        assert!(defaults.max_scalar_bytes > 0);
    }

    #[test]
    fn yaml_compiler_default_uses_default_limits() {
        let compiler = YamlCompiler::default();
        assert_eq!(
            compiler.limits.max_source_bytes,
            YamlLimits::default().max_source_bytes
        );
    }

    // ── Round 2: Lowering function tests ─────────────────────────────────

    #[test]
    fn lower_finish_produces_finish_node_kind() {
        let mut builder = SlotCompiler::new();
        let node = lower_finish(StepIdx::new(0), SlotIdx::new(0), &mut builder);
        assert!(matches!(node.kind, CompiledNodeKind::Finish { .. }));
    }

