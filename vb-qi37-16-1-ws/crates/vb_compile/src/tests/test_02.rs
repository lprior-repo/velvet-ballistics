#![forbid(unsafe_code)]
use super::helpers::*;

    #[test]
    fn compiler_rejects_missing_workflow_version() {
        let result = YamlCompiler::default().compile(
            b"name: fast_path\nwhen:\n  manual: {}\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingField { .. })))
        );
    }

    #[test]
    fn compiler_rejects_non_canonical_workflow_version() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidVersion { .. })))
        );
    }

    #[test]
    fn compiler_accepts_optional_top_level_fields() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "fast_path"));
    }

    #[test]
    fn compiler_accepts_allowed_input_schema_shorthand() {
        for shorthand in [
            "text",
            "number",
            "boolean",
            "object",
            "any",
            "list<any>",
            "list<text>",
            "list<number>",
            "list<boolean>",
        ] {
            let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

            assert!(
                matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
                "schema shorthand {shorthand} should compile"
            );
        }
    }

    #[test]
    fn compiler_rejects_unknown_input_schema_shorthand() {
        for shorthand in ["integer", "string", "list", "list<object>"] {
            let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
                "schema shorthand {shorthand} should be rejected"
            );
        }
    }

    #[test]
    fn compiler_and_ast_report_same_schema_diagnostics() {
        for inputs in [
            "  value: integer\n",
            "  value:\n    is: text\n    kind: text\n",
            "  value:\n    is: text\n    default: 1\n",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {{}}\ninputs:\n{inputs}steps:\n  - id: done\n    finish:\n      result: 0\n"
            );

            assert_compile_parse_first_error(source.as_bytes());
        }
    }

    #[test]
    fn schema_validation_does_not_preempt_yaml_profile_errors() {
        assert_compile_parse_first_error(
            b"version: velvet-ballastics/v1\nname: &n schema_case\ninputs:\n  value: integer\ncopy: *n\n",
        );
    }

    #[test]
    fn schema_validation_does_not_preempt_duplicate_key_errors() {
        assert_compile_parse_first_error(
            b"version: velvet-ballastics/v1\nversion: velvet-ballastics/v1\nname: schema_case\ninputs:\n  value: integer\n",
        );
    }

    #[test]
    fn schema_validation_does_not_preempt_lowering_errors() {
        let source = b"version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {}\ninputs:\n  value: integer\nsteps:\n  - id: route\n    choose: true\n";

        assert_eq!(
            compile_error_text(source),
            CompileError::LastStepMustFinish.to_string()
        );
        assert_compile_parse_first_error(source);
    }

    #[test]
    fn schema_validation_does_not_preempt_finish_position_errors() {
        let source = b"version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {}\ninputs:\n  value: integer\nsteps:\n  - id: early\n    finish:\n      result: 0\n      status: success\n  - id: done\n    finish:\n      result: 0\n";

        assert!(compile_error_text(source).contains("field finish"));
        assert_compile_parse_first_error(source);
    }

