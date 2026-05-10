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
    fn compiler_accepts_allowed_input_schema_shorthand_text() {
        let shorthand = "text";
        let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
            "schema shorthand {shorthand} should compile"
        );
    }

    #[test]
    fn compiler_accepts_allowed_input_schema_shorthand_number() {
        let shorthand = "number";
        let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
            "schema shorthand {shorthand} should compile"
        );
    }

    #[test]
    fn compiler_accepts_allowed_input_schema_shorthand_boolean() {
        let shorthand = "boolean";
        let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
            "schema shorthand {shorthand} should compile"
        );
    }

    #[test]
    fn compiler_accepts_allowed_input_schema_shorthand_object() {
        let shorthand = "object";
        let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
            "schema shorthand {shorthand} should compile"
        );
    }

    #[test]
    fn compiler_accepts_allowed_input_schema_shorthand_any() {
        let shorthand = "any";
        let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
            "schema shorthand {shorthand} should compile"
        );
    }

    #[test]
    fn compiler_accepts_allowed_input_schema_shorthand_list_any() {
        let shorthand = "list<any>";
        let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
            "schema shorthand {shorthand} should compile"
        );
    }

    #[test]
    fn compiler_accepts_allowed_input_schema_shorthand_list_text() {
        let shorthand = "list<text>";
        let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
            "schema shorthand {shorthand} should compile"
        );
    }

    #[test]
    fn compiler_accepts_allowed_input_schema_shorthand_list_number() {
        let shorthand = "list<number>";
        let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
            "schema shorthand {shorthand} should compile"
        );
    }

    #[test]
    fn compiler_accepts_allowed_input_schema_shorthand_list_boolean() {
        let shorthand = "list<boolean>";
        let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
            "schema shorthand {shorthand} should compile"
        );
    }

    #[test]
    fn compiler_rejects_unknown_input_schema_shorthand_integer() {
        let shorthand = "integer";
        let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "schema shorthand {shorthand} should be rejected"
        );
    }

    #[test]
    fn compiler_rejects_unknown_input_schema_shorthand_string() {
        let shorthand = "string";
        let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "schema shorthand {shorthand} should be rejected"
        );
    }

    #[test]
    fn compiler_rejects_unknown_input_schema_shorthand_list() {
        let shorthand = "list";
        let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "schema shorthand {shorthand} should be rejected"
        );
    }

    #[test]
    fn compiler_rejects_unknown_input_schema_shorthand_list_object() {
        let shorthand = "list<object>";
        let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "schema shorthand {shorthand} should be rejected"
        );
    }

    #[test]
    fn compiler_and_ast_report_same_schema_diagnostics_inline_kind() {
        let inputs = "  value: integer\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {{}}\ninputs:\n{inputs}steps:\n  - id: done\n    finish:\n      result: 0\n"
        );

        assert_compile_parse_first_error(source.as_bytes());
    }

    #[test]
    fn compiler_and_ast_report_same_schema_diagnostics_is_and_kind() {
        let inputs = "  value:\n    is: text\n    kind: text\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {{}}\ninputs:\n{inputs}steps:\n  - id: done\n    finish:\n      result: 0\n"
        );

        assert_compile_parse_first_error(source.as_bytes());
    }

    #[test]
    fn compiler_and_ast_report_same_schema_diagnostics_is_and_default() {
        let inputs = "  value:\n    is: text\n    default: 1\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {{}}\ninputs:\n{inputs}steps:\n  - id: done\n    finish:\n      result: 0\n"
        );

        assert_compile_parse_first_error(source.as_bytes());
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
        let source = b"version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {}\ninputs:\n  value: integer\nsteps:\n  - id: early\n    finish:\n      result: 0
      status: success
  - id: done\n    finish:\n      result: 0\n";

        assert!(compile_error_text(source).contains("field finish"));
        assert_compile_parse_first_error(source);
    }
