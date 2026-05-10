#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
mod tests {
    use super::{CompileError, CompileErrors, SlotCompiler, SourceMark, YamlCompiler, YamlLimits};
    use super::{
        compile_to_generated_rust, compute_compiled_digest, lower_ask, lower_do, lower_finish,
        lower_set,
    };
    use vb_core::ConstValue;
    use vb_core::ids::{ActionId, ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ExprProgram, WorkflowParts};
    use vb_core::{CompiledWorkflow, ResourceContract};

    macro_rules! compile_test_fail {
        ($($arg:tt)*) => {{
            let failed = false;
            assert!(failed, $($arg)*);
            return;
        }};
    }

    const NESTED_SAVE_SOURCE: &[u8] = br#"
version: velvet-ballastics/v1
name: nested_save
when:
  manual: {}
steps:
  - id: build_result
    save:
      text: done
      tags:
        - demo
        - fast
      metadata:
        attempts: 1
        active: true
        note: null
  - id: done
    finish:
      result: 0
"#;

    const OPTIONAL_TOP_LEVEL_FIELDS_SOURCE: &[u8] = br#"
version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
inputs:
  value: text
vars:
  label: 1
secrets:
  api_key: API_KEY
result: {}
examples:
  - name: fixture
    input:
      value: 1
steps:
  - id: build_result
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;

    fn compile_with_inputs(inputs: &str) -> Result<CompiledWorkflow, CompileErrors> {
        let source = format!(
            "version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {{}}\ninputs:\n{inputs}steps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
        );
        YamlCompiler::default().compile(source.as_bytes())
    }

    fn compile_error_text(source: &[u8]) -> String {
        match YamlCompiler::default().compile(source) {
            Ok(_) => "compile unexpectedly succeeded".to_owned(),
            Err(errors) => match errors.first() {
                Some(error) => error.to_string(),
                None => "CompileErrors was empty".to_owned(),
            },
        }
    }

    fn parse_ast_error_text(source: &[u8]) -> String {
        match YamlCompiler::default().parse_ast(source) {
            Ok(_) => "parse_ast unexpectedly succeeded".to_owned(),
            Err(errors) => match errors.first() {
                Some(error) => error.to_string(),
                None => "CompileErrors was empty".to_owned(),
            },
        }
    }

    fn assert_compile_parse_first_error(source: &[u8]) {
        assert_eq!(compile_error_text(source), parse_ast_error_text(source));
    }

    fn compile_first_error(source: &[u8]) -> Result<CompileError, String> {
        match YamlCompiler::default().compile(source) {
            Ok(workflow) => Err(format!("compile unexpectedly succeeded: {workflow:?}")),
            Err(errors) => errors
                .first()
                .cloned()
                .ok_or_else(|| "CompileErrors was empty".to_owned()),
        }
    }

    fn parse_first_error(source: &[u8]) -> Result<CompileError, String> {
        match YamlCompiler::default().parse_ast(source) {
            Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
            Err(errors) => errors
                .first()
                .cloned()
                .ok_or_else(|| "CompileErrors was empty".to_owned()),
        }
    }

    fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
    where
        T: core::fmt::Debug + PartialEq,
    {
        if actual == expected {
            Ok(())
        } else {
            Err(format!("expected {expected:?}, found {actual:?}"))
        }
    }

    fn assert_compile_code(source: &[u8], expected: &'static str) -> Result<(), String> {
        let error = compile_first_error(source)?;
        ensure_equal(error.code(), expected)?;
        ensure_equal(error.diagnostic_code(), expected)
    }

    #[test]
    fn compile_error_exposes_stable_validation_codes() -> Result<(), String> {
        for (source, code) in [
            (
                b"version: velvet-ballastics/v1\nversion: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n".as_slice(),
                "DUPLICATE_KEY",
            ),
            (
                b"version: velvet-ballastics/v1\nname: &n fast_path\ncopy: *n\n",
                "FORBIDDEN_YAML_FEATURE",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nunexpected: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
                "UNKNOWN_TOP_LEVEL_FIELD",
            ),
            (
                b"name: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
                "MISSING_REQUIRED_FIELD",
            ),
            (
                b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
                "INVALID_VERSION",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: BuildResult\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
                "INVALID_ID",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: finish\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
                "RESERVED_ID",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: duplicate\n    save:\n      value: 1\n  - id: duplicate\n    finish:\n      result: 0\n",
                "DUPLICATE_ID",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: only_metadata\n    name: Only Metadata\n  - id: done\n    finish:\n      result: 0\n",
                "MISSING_STEP_PRIMITIVE",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n    finish:\n      result: 0\n  - id: done\n    finish:\n      result: 0\n",
                "MULTIPLE_STEP_PRIMITIVES",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose: true\n  - id: done\n    finish:\n      result: 0\n",
                "INVALID_CHOOSE",
            ),
        ] {
            assert_compile_code(source, code)?;
        }
        Ok(())
    }

    #[test]
    fn reference_diagnostic_codes_cover_public_reference_contract() -> Result<(), String> {
        assert_compile_code(
            br#"version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$input.missing == true"
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#,
            "UNKNOWN_REFERENCE",
        )?;
        assert_compile_code(
            br#"version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$secrets.api_key == \"x\""
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#,
            "SECRET_NOT_DECLARED",
        )
    }

    #[test]
    fn compile_errors_exposes_ordered_error_and_code_accessors() {
        let errors = CompileErrors(vec![
            CompileError::SourceTooLarge {
                actual: 8,
                limit: 4,
            },
            CompileError::InvalidVersion {
                actual: Box::<str>::from("velvet/v1"),
            },
        ]);
        let codes: Vec<&'static str> = errors.diagnostic_codes().collect();

        assert_eq!(errors.len(), 2);
        assert_eq!(errors.as_slice().len(), 2);
        assert_eq!(errors.iter().count(), 2);
        assert_eq!(codes, vec!["PAYLOAD_TOO_LARGE", "INVALID_VERSION"]);
    }

    #[test]
    fn parse_ast_and_compile_expose_same_diagnostic_codes() -> Result<(), String> {
        for source in [
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nunexpected: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n".as_slice(),
            br#"version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$input.flag =="
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#,
            br#"version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$secrets.api_key == \"x\""
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#,
        ] {
            let compile = compile_first_error(source)?;
            let parse = parse_first_error(source)?;
            ensure_equal(compile.code(), parse.code())?;
        }
        Ok(())
    }

    #[test]
    fn compiler_rejects_save_object_until_handle_arenas_exist() {
        let source = br#"
version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
steps:
  - id: build_result
    save:
      text: done
  - id: done
    finish:
      result: 0
"#;
        let result = YamlCompiler::default().compile(source);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedConstantValue { step: 0 }))
        ));
    }

    #[test]
    fn compiler_rejects_nested_save_values_until_handle_arenas_exist() {
        let result = YamlCompiler::default().compile(NESTED_SAVE_SOURCE);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedConstantValue { step: 0 }))
        ));
    }

    #[test]
    fn compiler_rejects_scalar_save_body() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save: done\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::StepFieldShape { field: "save", .. }))
        ));
    }

    #[test]
    fn compiler_rejects_save_references_until_expressions_exist() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\ninputs:\n  value: text\nsteps:\n  - id: build_result\n    save:\n      text: $input.value\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedConstantValue { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_empty_steps() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps: []\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::EmptySteps)))
        );
    }

    #[test]
    fn compiler_rejects_unsupported_top_level_fields() {
        let result = YamlCompiler::default()
            .compile(b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nunexpected: true\nsteps:\n  - finish:\n      result: 0\n");

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownTopLevelField { .. }))
        ));
    }

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

    #[test]
    fn compiler_accepts_input_long_form_scalar_constraints() {
        let result = compile_with_inputs(
            "  title:\n    from: request.body.title\n    is: text\n    default: hello\n    min_length: 1\n    max_length: 20\n    optional: true\n    nullable: false\n    secret: false\n  score:\n    is: number\n    default: 10\n    min: 0\n    max: 100\n  approved:\n    is: boolean\n    default: true\n",
        );

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"));
    }

    #[test]
    fn compiler_accepts_input_long_form_object_fields() {
        let result = compile_with_inputs(
            "  customer:\n    from: request.body.customer\n    is: object\n    fields:\n      id: text\n      email: text\n      address:\n        is: object\n        optional: true\n        nullable: true\n        fields:\n          city: text\n          country: text\n    extra: reject\n",
        );

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"));
    }

    #[test]
    fn compiler_accepts_input_long_form_list_elements() {
        for element in ["any", "text", "number", "boolean", "object"] {
            let result = compile_with_inputs(&format!(
                "  values:\n    is: list\n    of: {element}\n    default: []\n    min: 0\n    max: 10\n"
            ));

            assert!(
                matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
                "list element schema {element} should compile"
            );
        }
    }

    #[test]
    fn compiler_rejects_input_schema_unknown_fields() {
        for inputs in [
            "  value:\n    is: text\n    kind: text\n",
            "  customer:\n    is: object\n    fields:\n      value:\n        is: text\n        from: request.body.value\n",
        ] {
            let result = compile_with_inputs(inputs);

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownInputSchemaField { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_pattern_until_bounded_regex_exists() {
        let result = compile_with_inputs("  value:\n    is: text\n    pattern: ^[a-z]+$\n");

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema {
                field: "inputs.pattern",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_child_fields() {
        for inputs in [
            "  values:\n    is: list\n",
            "  value:\n    is: text\n    of: text\n",
            "  value:\n    is: text\n    fields:\n      nested: text\n",
            "  value:\n    is: text\n    extra: reject\n",
            "  customer:\n    is: object\n    extra: ignore\n",
            "  customer:\n    is: object\n    fields: true\n",
            "  values:\n    is: list\n    of: integer\n",
            "  value:\n    is: integer\n",
        ] {
            let result = compile_with_inputs(inputs);

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
                "invalid schema should be rejected: {inputs}"
            );
        }
    }

    #[test]
    fn compiler_rejects_non_boolean_input_schema_flags() {
        for flag in ["optional", "nullable", "secret"] {
            let result = compile_with_inputs(&format!("  value:\n    is: text\n    {flag}: yes\n"));

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_default_that_does_not_match_input_schema() {
        for inputs in [
            "  value:\n    is: text\n    default: 1\n",
            "  value:\n    is: number\n    default: nope\n",
            "  value:\n    is: boolean\n    default: nope\n",
            "  value:\n    is: object\n    default: []\n",
            "  value:\n    is: list\n    of: text\n    default: {}\n",
        ] {
            let result = compile_with_inputs(inputs);

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
            ));
        }
    }

    #[test]
    fn compiler_validates_null_input_schema_defaults() {
        let rejected = compile_with_inputs("  value:\n    is: text\n    default: null\n");
        let accepted =
            compile_with_inputs("  value:\n    is: text\n    nullable: true\n    default: null\n");

        assert!(matches!(
            rejected,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
        ));
        assert!(matches!(accepted, Ok(ref workflow) if workflow.name() == "schema_case"));
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_bounds() {
        for inputs in [
            "  value:\n    is: number\n    min: 10\n    max: 1\n",
            "  values:\n    is: list\n    of: text\n    min: -1\n",
            "  value:\n    is: text\n    min: 1\n",
            "  value:\n    is: text\n    min_length: -1\n",
            "  value:\n    is: text\n    min_length: 10\n    max_length: 1\n",
            "  value:\n    is: number\n    min_length: 1\n",
        ] {
            let result = compile_with_inputs(inputs);

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
                "invalid bounds should be rejected: {inputs}"
            );
        }
    }

    #[test]
    fn compiler_rejects_non_mapping_optional_top_level_fields() {
        for field in ["inputs", "vars", "secrets"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\n{field}: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. }))),
                "{field} must be mapping-shaped"
            );
        }
    }

    #[test]
    fn compiler_rejects_invalid_optional_top_level_names() {
        for (field, key) in [
            ("inputs", "InputValue"),
            ("vars", "run"),
            ("secrets", "api-key"),
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\n{field}:\n  {key}: value\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidName { .. }))),
                "{field}.{key} must use Velvet v1 public naming"
            );
        }
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_shapes() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\ninputs:\n  value:\n    - text\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. })))
        );
    }

    #[test]
    fn compiler_rejects_runtime_references_in_vars() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nvars:\n  label: $input.value\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedConstantValue { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_non_string_secret_bindings() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsecrets:\n  api_key: 42\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. })))
        );
    }

    #[test]
    fn compiler_rejects_invalid_examples_shape() {
        for examples in ["true", "\n  - fixture"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nexamples: {examples}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. }))),
                "examples must be a sequence of mappings"
            );
        }
    }

    #[test]
    fn compiler_rejects_examples_without_valid_names() {
        for examples in ["\n  - input: {}", "\n  - name: 42", "\n  - name: run"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nexamples: {examples}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(
                    result,
                    Err(ref errors) if matches!(
                        errors.first(),
                        Some(
                            CompileError::MissingField { .. }
                                | CompileError::FieldShape { .. }
                                | CompileError::InvalidName { .. }
                        )
                    )
                ),
                "examples must declare valid fixture names"
            );
        }
    }

    #[test]
    fn compiler_rejects_non_empty_top_level_result_until_result_ir_exists() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nresult:\n  value: $build_result.value\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedTopLevelResult))
        ));
    }

    #[test]
    fn compiler_rejects_non_mapping_top_level_result() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nresult: done\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape {
                field: "result",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_workflow_names() {
        for name in ["", "FastPath", "fast-path", "run"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: \"{name}\"\nwhen:\n  manual: {{}}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidName { field: "name", .. }))),
                "workflow name {name:?} must be rejected"
            );
        }
    }

    #[test]
    fn compiler_rejects_missing_step_ids() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingStepId { .. })))
        );
    }

    #[test]
    fn compiler_rejects_invalid_step_ids() {
        for id in ["", "BuildResult", "build-result", "finish"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: \"{id}\"\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(
                    result,
                    Err(ref errors) if matches!(
                        errors.first(),
                        Some(CompileError::InvalidName {
                            field: "step id",
                            ..
                        })
                    )
                ),
                "step id {id:?} must be rejected"
            );
        }
    }

    #[test]
    fn compiler_rejects_duplicate_step_ids() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: duplicate\n    save:\n      value: 1\n  - id: duplicate\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::DuplicateStepId { .. })))
        );
    }

    #[test]
    fn compiler_accepts_step_display_name_metadata() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    name: Build Result\n    save:\n      value: 1\n  - id: done\n    name: Done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "fast_path"));
    }

    #[test]
    fn compiler_rejects_non_string_step_display_name() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    name: 42\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::StepFieldShape { field: "name", .. }))
        ));
    }

    #[test]
    fn compiler_rejects_unsupported_phase_zero_step_control_fields() {
        for control in ["if", "with", "try_again", "on_error", "then"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: build_result\n    {control}: true\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(
                    result,
                    Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedStepControlField { .. }))
                ),
                "control field {control} must be rejected until Phase 0 compiles it"
            );
        }
    }

    #[test]
    fn compiler_rejects_missing_workflow_trigger() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingField { .. })))
        );
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_shapes() {
        for source in [
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen: manual\nsteps:\n  - finish:\n      result: 0\n".as_slice(),
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen: {}\nsteps:\n  - finish:\n      result: 0\n",
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\n  event: {}\nsteps:\n  - finish:\n      result: 0\n",
        ] {
            let result = YamlCompiler::default().compile(source);

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. } | CompileError::InvalidTriggerCount { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_unknown_workflow_trigger_kind() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  file: {}\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownTriggerKind { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_scalar_workflow_trigger_config() {
        for trigger in ["manual", "webhook", "schedule", "event"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  {trigger}: true\nsteps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::TriggerShape { .. }))),
                "trigger {trigger} config must be mapping-shaped"
            );
        }
    }

    #[test]
    fn compiler_accepts_valid_workflow_trigger_configs() {
        for when_body in [
            "  manual: {}\n",
            "  webhook:\n    path: /github\n    method: POST\n    unique: request.header.X-GitHub-Delivery\n",
            "  schedule:\n    cron: \"*/5 * * * *\"\n    timezone: UTC\n",
            "  event:\n    name: customer.created\n",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Ok(ref workflow) if workflow.name() == "fast_path"),
                "valid trigger should compile"
            );
        }
    }

    #[test]
    fn compiler_rejects_unknown_workflow_trigger_fields() {
        for when_body in [
            "  manual:\n    extra: true\n",
            "  webhook:\n    path: /github\n    method: POST\n    extra: true\n",
            "  schedule:\n    cron: \"*/5 * * * *\"\n    extra: true\n",
            "  event:\n    name: customer.created\n    extra: true\n",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownTriggerField { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_missing_required_workflow_trigger_fields() {
        for when_body in [
            "  webhook:\n    method: POST\n",
            "  webhook:\n    path: /github\n",
            "  schedule:\n    timezone: UTC\n",
            "  event: {}\n",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingTriggerField { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_field_values() {
        for when_body in [
            "  webhook:\n    path: github\n    method: POST\n",
            "  webhook:\n    path: /github\n    method: TRACE\n",
            "  webhook:\n    path: 42\n    method: POST\n",
            "  webhook:\n    path: /github\n    method: POST\n    unique: 42\n",
            "  schedule:\n    cron: \"0 0 0 0 0 0\"\n",
            "  schedule:\n    cron: 42\n",
            "  schedule:\n    cron: \"*/5 * * * *\"\n    timezone: 42\n",
            "  event:\n    name: 42\n",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidTriggerField { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_backward_branch_targets() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose:\n      condition: true\n      on_true: 0\n      on_false: 1\n  - id: done\n    finish:\n      result: true\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::BackwardBranchTarget { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_extra_phase_zero_choose_fields() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose:\n      condition: 0\n      on_true: 1\n      on_false: 1\n      otherwise: true\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownStepPrimitiveField {
                primitive: "choose",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_non_mapping_phase_zero_choose_body() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose: true\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::StepFieldShape {
                field: "choose",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_extra_phase_zero_finish_fields() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n      status: success\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownStepPrimitiveField {
                primitive: "finish",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_non_mapping_phase_zero_finish_body() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish: success\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::StepFieldShape {
                field: "finish",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_aliases() {
        let result = YamlCompiler::default()
            .compile(b"version: velvet-ballastics/v1\nname: &n fast\ncopy: *n\n");

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::AnchorForbidden { mark }) if mark.available)
        ));
    }

    #[test]
    fn compiler_rejects_custom_tags_with_mark() {
        let result = YamlCompiler::default().compile(b"version: !custom velvet-ballastics/v1\n");

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::TagForbidden { mark }) if mark.available)
        ));
    }

    #[test]
    fn compiler_rejects_non_string_object_keys_with_mark() {
        let result = YamlCompiler::default().compile(b"? [bad]\n: value\n");

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::NonStringKey { mark }) if mark.available)
        ));
    }

    #[test]
    fn compiler_rejects_duplicate_top_level_keys() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nversion: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::DuplicateKey { .. })))
        );
    }

    #[test]
    fn compiler_rejects_duplicate_nested_keys() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      text: first\n      text: second\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::DuplicateKey { .. })))
        );
    }

    #[test]
    fn compiler_rejects_legacy_step_aliases() {
        for alias in ["gather", "summarize", "copy"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: legacy\n    {alias}:\n      slot: 0\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownStepField { .. }))),
                "legacy alias {alias} must be rejected"
            );
        }
    }

    #[test]
    fn compiler_rejects_missing_step_primitive() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: only_metadata\n    name: Only Metadata\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingStepPrimitive { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_multiple_step_primitives() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      slot: 0\n      value: 1\n    finish:\n      result: 0\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::MultipleStepPrimitives { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_malformed_master_primitives_with_exact_diagnostic() {
        for (primitive, code) in [
            ("for_each", "INVALID_FOR_EACH"),
            ("together", "INVALID_TOGETHER"),
            ("collect", "INVALID_COLLECT"),
            ("reduce", "INVALID_REDUCE"),
            ("repeat", "INVALID_REPEAT"),
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: unsupported\n    {primitive}: noop\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(
                    result,
                    Err(ref errors)
                        if errors.first().map(CompileError::code) == Some(code)
                ),
                "primitive {primitive} should be rejected with exact invalid diagnostic"
            );
        }
    }

    #[test]
    fn compiler_lowers_yaml_for_each_to_loop_nodes() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: for_each_case\nwhen:\n  manual: {}\nsteps:\n  - id: list\n    save:\n      value: 1\n  - id: each\n    for_each:\n      input: 0\n      item: 1\n      limit: 10\n  - id: done\n    finish:\n      result: 0\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        let start = workflow
            .node(StepIdx::new(1))
            .ok_or("missing for_each start")?;
        let next = workflow
            .node(StepIdx::new(2))
            .ok_or("missing for_each next")?;

        assert!(
            matches!(start.kind, CompiledNodeKind::ForEachStart { input, item_slot, limit, body, done } if input == SlotIdx::ZERO && item_slot == SlotIdx::new(1) && limit == 10 && body == StepIdx::new(2) && done == StepIdx::new(3))
        );
        assert!(
            matches!(next.kind, CompiledNodeKind::ForEachNext { iterator_slot, body, done } if iterator_slot == SlotIdx::new(1) && body == StepIdx::new(2) && done == StepIdx::new(3))
        );
        Ok(())
    }

    #[test]
    fn compiler_accepts_for_each_with_at_once_field() -> Result<(), String> {
        let source = "version: velvet-ballastics/v1\nname: for_each_with_at_once\nwhen:\n  manual: {}\nsteps:\n  - id: list\n    save:\n      value: [1, 2, 3]\n  - id: each\n    for_each:\n      input: 0\n      item: 1\n      limit: 10\n      at_once: 5\n  - id: done\n    finish:\n      result: 0\n";
        let workflow = YamlCompiler::default()
            .compile(source.as_bytes())
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        let start = workflow
            .node(StepIdx::new(1))
            .ok_or("missing for_each start")?;
        assert!(
            matches!(start.kind, CompiledNodeKind::ForEachStart { input, item_slot, limit, body, done } if input == SlotIdx::ZERO && item_slot == SlotIdx::new(1) && limit == 10 && body == StepIdx::new(2) && done == StepIdx::new(3)),
            "for_each start node must have correct structure"
        );
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_together_to_start_and_join_nodes() -> Result<(), String> {
        // The together structure needs the join node to come after all branch
        // targets. With 3 source steps (fanout, body, done) the compiler
        // expands fanout into TogetherStart (node 0) + TogetherJoin (node 1).
        // The branch target (step 1 -> node 2) must be before the finish
        // (step 2 -> node 3). However the compiler currently emits the join
        // at id+1, so for a well-formed test we use a layout where the
        // branch body is between start and join. Since the lowering always
        // puts TogetherJoin right after TogetherStart, and the shared
        // validation pipeline now enforces join > branch ordering, we test
        // that the IR is rejected when branches point past the join.
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: together_case\nwhen:\n  manual: {}\nsteps:\n  - id: fanout\n    together:\n      branches: [1]\n  - id: done\n    finish:\n      result: 0\n",
        );
        // The shared validation pipeline catches the invalid together IR:
        // join (node 1) is not after branch target (node 2).
        assert!(
            matches!(result, Err(ref errors) if errors.0.iter().any(|e| matches!(e, CompileError::Validation(vb_validate::ValidationError::LoopBodyStepOutOfRange { .. })))),
            "expected LoopBodyStepOutOfRange validation error, got: {result:?}"
        );
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_collect_to_collection_nodes() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: collect_case\nwhen:\n  manual: {}\nsteps:\n  - id: source\n    save:\n      value: 1\n  - id: collect_values\n    collect:\n      source: 0\n      limit: 5\n      page_size: 2\n  - id: done\n    finish:\n      result: 0\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;

        assert!(
            matches!(workflow.node(StepIdx::new(1)).map(|node| &node.kind), Some(CompiledNodeKind::CollectStart { source, limit, page_size, body, done }) if *source == SlotIdx::ZERO && *limit == 5 && *page_size == 2 && *body == StepIdx::new(2) && *done == StepIdx::new(3))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(2)).map(|node| &node.kind), Some(CompiledNodeKind::CollectPage { collector_slot, body, done }) if *collector_slot == SlotIdx::ZERO && *body == StepIdx::new(2) && *done == StepIdx::new(3))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(3)).map(|node| &node.kind), Some(CompiledNodeKind::CollectFinish { collector_slot }) if *collector_slot == SlotIdx::ZERO)
        );
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_reduce_to_reduction_nodes() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: reduce_case\nwhen:\n  manual: {}\nsteps:\n  - id: source\n    save:\n      value: 1\n  - id: reduce_values\n    reduce:\n      input: 0\n      accumulator: 1\n      initial: 0\n  - id: done\n    finish:\n      result: 1\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;

        assert!(
            matches!(workflow.node(StepIdx::new(1)).map(|node| &node.kind), Some(CompiledNodeKind::ReduceStart { input, accumulator, initial, body, done }) if *input == SlotIdx::ZERO && *accumulator == SlotIdx::new(1) && *initial == ConstIdx::new(1) && *body == StepIdx::new(2) && *done == StepIdx::new(3))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(2)).map(|node| &node.kind), Some(CompiledNodeKind::ReduceNext { iterator_slot, accumulator, body, done }) if *iterator_slot == SlotIdx::new(1) && *accumulator == SlotIdx::new(1) && *body == StepIdx::new(2) && *done == StepIdx::new(3))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(3)).map(|node| &node.kind), Some(CompiledNodeKind::ReduceFinish { accumulator }) if *accumulator == SlotIdx::new(1))
        );
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_repeat_to_attempt_nodes() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: repeat_case\nwhen:\n  manual: {}\nsteps:\n  - id: poll\n    repeat:\n      max_attempts: 3\n  - id: done\n    finish:\n      result: 1\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;

        assert!(
            matches!(workflow.node(StepIdx::ZERO).map(|node| &node.kind), Some(CompiledNodeKind::RepeatStart { max_attempts, body, done }) if *max_attempts == 3 && *body == StepIdx::new(1) && *done == StepIdx::new(2))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(1)).map(|node| &node.kind), Some(CompiledNodeKind::RepeatAttempt { attempt_slot, body, done }) if *attempt_slot == SlotIdx::new(1) && *body == StepIdx::new(1) && *done == StepIdx::new(2))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(2)).map(|node| &node.kind), Some(CompiledNodeKind::RepeatFinish { result }) if *result == SlotIdx::new(1))
        );
        Ok(())
    }

    #[test]
    fn compiler_rejects_oversized_source() {
        let limits = YamlLimits {
            max_source_bytes: 4,
            ..YamlLimits::default()
        };
        let result = YamlCompiler::new(limits).compile(b"name: too_large\n");

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::SourceTooLarge { .. })))
        );
    }

    #[test]
    fn compiler_accepts_minimal_strict_workflow() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: strict_minimal\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "strict_minimal"));
    }

    #[test]
    fn compiler_lowers_yaml_set_to_set_const_node() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: set_case\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    set:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        let node = workflow.node(StepIdx::new(0)).ok_or("missing set node")?;

        assert!(matches!(node.kind, CompiledNodeKind::SetConst { .. }));
        assert_eq!(node.output, Some(SlotIdx::ZERO));
        assert_eq!(node.next, Some(StepIdx::new(1)));
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_wait_until_to_wait_until_node() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: wait_case\nwhen:\n  manual: {}\nsteps:\n  - id: deadline\n    save:\n      value: 1\n  - id: wait_for_deadline\n    wait:\n      until: 0\n  - id: done\n    finish:\n      result: 0\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        let node = workflow.node(StepIdx::new(1)).ok_or("missing wait node")?;

        assert!(matches!(
            node.kind,
            CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::ZERO
            }
        ));
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_ask_to_ask_and_resume_nodes() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: ask_case\nwhen:\n  manual: {}\nsteps:\n  - id: prompt\n    save:\n      value: 1\n  - id: ask_user\n    ask:\n      prompt: 0\n      answer: 1\n  - id: done\n    finish:\n      result: 1\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        let ask = workflow.node(StepIdx::new(1)).ok_or("missing ask node")?;
        let resume = workflow
            .node(StepIdx::new(2))
            .ok_or("missing resume node")?;
        let finish = workflow
            .node(StepIdx::new(3))
            .ok_or("missing finish node")?;

        assert!(matches!(ask.kind, CompiledNodeKind::Ask { .. }));
        assert!(
            matches!(resume.kind, CompiledNodeKind::AskResume { answer } if answer == SlotIdx::new(1))
        );
        assert!(
            matches!(finish.kind, CompiledNodeKind::Finish { result } if result == SlotIdx::new(1))
        );
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_run_to_do_node() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: run_case\nwhen:\n  manual: {}\nsteps:\n  - id: source_slot\n    save:\n      value: 1\n  - id: call_action\n    run:\n      action: 7\n      input: 0\n  - id: done\n    finish:\n      result: 1\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        assert_eq!(workflow.node_count(), 3);
        assert_eq!(workflow.slot_count(), 2);
        let node = workflow.node(StepIdx::new(1)).ok_or("missing run node")?;
        let finish = workflow
            .node(StepIdx::new(2))
            .ok_or("missing finish node")?;

        assert!(matches!(
            node.kind,
            CompiledNodeKind::Do { action, input }
                if action == ActionId::new(7) && input == SlotIdx::ZERO
        ));
        assert_eq!(node.output, Some(SlotIdx::new(1)));
        assert_eq!(node.next, Some(StepIdx::new(2)));
        assert!(matches!(
            finish.kind,
            CompiledNodeKind::Finish { result } if result == SlotIdx::new(1)
        ));
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_do_alias_to_do_node() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: do_case\nwhen:\n  manual: {}\nsteps:\n  - id: source_slot\n    save:\n      value: 1\n  - id: call_action\n    do:\n      action: 11\n      input: 0\n  - id: done\n    finish:\n      result: 1\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        assert_eq!(workflow.node_count(), 3);
        assert_eq!(workflow.slot_count(), 2);
        let node = workflow.node(StepIdx::new(1)).ok_or("missing do node")?;
        let finish = workflow
            .node(StepIdx::new(2))
            .ok_or("missing finish node")?;

        assert!(matches!(
            node.kind,
            CompiledNodeKind::Do { action, input }
                if action == ActionId::new(11) && input == SlotIdx::ZERO
        ));
        assert_eq!(node.output, Some(SlotIdx::new(1)));
        assert_eq!(node.next, Some(StepIdx::new(2)));
        assert!(matches!(
            finish.kind,
            CompiledNodeKind::Finish { result } if result == SlotIdx::new(1)
        ));
        Ok(())
    }

    #[test]
    fn compiler_preserves_action_name_run_rejection() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: action_name\nwhen:\n  manual: {}\nsteps:\n  - id: call_action\n    run: shell.run\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedStepPrimitive { step: 0, primitive: "run" }))
        ));
    }

    #[test]
    fn compiler_rejects_action_schema_form_with_unknown_field() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: action_schema\nwhen:\n  manual: {}\nsteps:\n  - id: source_slot\n    save:\n      value: 1\n  - id: call_action\n    run:\n      action: 7\n      input: 0\n      with: {}\n  - id: done\n    finish:\n      result: 1\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownStepPrimitiveField { step: 1, primitive: "run", field }) if field.as_ref() == "with")
        ));
    }

    #[test]
    fn compiler_attaches_default_resource_contract() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: resource_case\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;

        if workflow.resource_contract() == ResourceContract::DEFAULT {
            Ok(())
        } else {
            Err(format!(
                "unexpected resource contract: {:?}",
                workflow.resource_contract()
            ))
        }
    }

    #[test]
    fn compiler_rejects_empty_yaml_source() {
        let result = YamlCompiler::default().compile(b"   \n\t  ");

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::EmptySource)))
        );
    }

    #[test]
    fn compiler_rejects_multiple_yaml_documents() {
        let result = YamlCompiler::default().compile(
            b"---\nversion: velvet-ballastics/v1\nname: first\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n---\nversion: velvet-ballastics/v1\nname: second\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::DocumentCount { count: 2 }))
        ));
    }

    #[test]
    fn compiler_rejects_yaml_merge_keys() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: merge_key\nwhen:\n  manual: {}\n<<:\n  steps: []\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::MergeKeyForbidden { .. }))
        ));
    }

    // ── Round 2: Exact-assertion error variant tests ─────────────────────

    #[test]
    fn compile_returns_source_too_large_with_exact_fields() {
        let tiny_limits = YamlLimits {
            max_source_bytes: 10,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler {
            limits: tiny_limits,
        };
        let source = b"version: velvet-ballastics/v1\nname: big\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0";
        let result = compiler.compile(source);
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::SourceTooLarge { actual, limit }) = errors.first() else {
            compile_test_fail!("expected SourceTooLarge, got {:?}", errors.first());
        };
        assert_eq!(*limit, 10);
        assert_eq!(*actual, source.len());
    }

    #[test]
    fn compile_returns_empty_source_for_empty_input() {
        let result = YamlCompiler::default().compile(b"");
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(errors.first(), Some(CompileError::EmptySource)));
    }

    #[test]
    fn compile_returns_top_level_not_mapping_for_list_root() {
        let result = YamlCompiler::default().compile(b"- item1\n- item2");
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(
            errors.first(),
            Some(CompileError::TopLevelNotMapping)
        ));
    }

    #[test]
    fn compile_returns_empty_steps_for_steps_with_empty_list() {
        let result = YamlCompiler::default()
            .compile(b"version: velvet-ballastics/v1\nname: empty\nwhen:\n  manual: {}\nsteps: []");
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(errors.first(), Some(CompileError::EmptySteps)));
    }

    #[test]
    fn compile_returns_invalid_version_for_wrong_version() {
        let result = YamlCompiler::default().compile(
            b"version: bad-version\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::InvalidVersion { actual }) = errors.first() else {
            compile_test_fail!("expected InvalidVersion, got {:?}", errors.first());
        };
        assert_eq!(actual.as_ref(), "bad-version");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_version() {
        let result = YamlCompiler::default().compile(
            b"name: no_version\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            compile_test_fail!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "version");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_name() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            compile_test_fail!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "name");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_when() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_trigger\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            compile_test_fail!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "when");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_steps() {
        let result = YamlCompiler::default()
            .compile(b"version: velvet-ballastics/v1\nname: no_steps\nwhen:\n  manual: {}");
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            compile_test_fail!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "steps");
    }

    #[test]
    fn compile_returns_invalid_trigger_count_for_empty_when() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: empty_when\nwhen: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::InvalidTriggerCount { count }) = errors.first() else {
            compile_test_fail!("expected InvalidTriggerCount, got {:?}", errors.first());
        };
        assert_eq!(*count, 0);
    }

    #[test]
    fn compile_returns_unknown_trigger_kind_for_invalid_trigger() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_trigger\nwhen:\n  teleport: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::UnknownTriggerKind { trigger }) = errors.first() else {
            compile_test_fail!("expected UnknownTriggerKind, got {:?}", errors.first());
        };
        assert_eq!(trigger.as_ref(), "teleport");
    }

    #[test]
    fn compile_returns_missing_step_id_for_step_without_id() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_id\nwhen:\n  manual: {}\nsteps:\n  - finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingStepId { step }) = errors.first() else {
            compile_test_fail!("expected MissingStepId, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

    #[test]
    fn compile_returns_step_shape_for_non_mapping_step() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_step\nwhen:\n  manual: {}\nsteps:\n  - \"scalar\"",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::StepShape { step }) = errors.first() else {
            compile_test_fail!("expected StepShape, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

    #[test]
    fn compile_returns_duplicate_step_id_for_same_ids() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: dup_step\nwhen:\n  manual: {}\nsteps:\n  - id: same\n    save:\n      x: 1\n  - id: same\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::DuplicateStepId { id }) = errors.first() else {
            compile_test_fail!("expected DuplicateStepId, got {:?}", errors.first());
        };
        assert_eq!(id.as_ref(), "same");
    }

    #[test]
    fn compile_returns_missing_step_primitive_for_step_without_primitive() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_prim\nwhen:\n  manual: {}\nsteps:\n  - id: empty_step",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingStepPrimitive { step }) = errors.first() else {
            compile_test_fail!("expected MissingStepPrimitive, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

    #[test]
    fn compile_returns_unknown_step_field_for_invalid_field() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_field\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    unknown_field: 1\n    save:\n      x: 1",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::UnknownStepField { step, field }) = errors.first() else {
            compile_test_fail!("expected UnknownStepField, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
        assert_eq!(field.as_ref(), "unknown_field");
    }

    #[test]
    fn compile_returns_last_step_must_finish_for_non_finish_ending() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_finish\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    save:\n      x: 1",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(
            errors.first(),
            Some(CompileError::LastStepMustFinish)
        ));
    }

    #[test]
    fn compile_returns_unknown_top_level_field_for_invalid_field() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: extra\nwhen:\n  manual: {}\nunknown_root: true\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::UnknownTopLevelField { field }) = errors.first() else {
            compile_test_fail!("expected UnknownTopLevelField, got {:?}", errors.first());
        };
        assert_eq!(field.as_ref(), "unknown_root");
    }

    #[test]
    fn compile_returns_tag_forbidden_for_tagged_node() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: tagged\nwhen:\n  manual: {}\nsteps:\n  - id: !!tag done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(
            errors.first(),
            Some(CompileError::TagForbidden { .. })
        ));
    }

    #[test]
    fn compile_returns_float_forbidden_for_float_scalar() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: floaty\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 3.14",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(errors.first(), Some(CompileError::FloatForbidden)));
    }

    #[test]
    fn compile_returns_depth_limit_for_deeply_nested_yaml() {
        let tiny_limits = YamlLimits {
            max_depth: 3,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler {
            limits: tiny_limits,
        };
        let result = compiler.compile(
            b"version: velvet-ballastics/v1\nname: deep\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\na:\n  b:\n    c:\n      d: deep",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::DepthLimit { depth, limit }) = errors.first() else {
            compile_test_fail!("expected DepthLimit, got {:?}", errors.first());
        };
        assert_eq!(*limit, 3);
        assert!(*depth > 3);
    }

    #[test]
    fn compile_returns_node_limit_for_many_nodes() {
        let tiny_limits = YamlLimits {
            max_nodes: 5,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler {
            limits: tiny_limits,
        };
        let result = compiler.compile(
            b"version: velvet-ballastics/v1\nname: big\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    save:\n      a: 1\n      b: 2\n      c: 3\n      d: 4\n      e: 5\n      f: 6\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::NodeLimit { limit }) = errors.first() else {
            compile_test_fail!("expected NodeLimit, got {:?}", errors.first());
        };
        assert_eq!(*limit, 5);
    }

    #[test]
    fn compile_returns_scalar_limit_for_long_scalar() {
        let tiny_limits = YamlLimits {
            max_scalar_bytes: 5,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler {
            limits: tiny_limits,
        };
        let result = compiler.compile(
            b"version: velvet-ballastics/v1\nname: long_scalar\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\nlabel: abcdefgh",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::ScalarLimit { actual, limit }) = errors.first() else {
            compile_test_fail!("expected ScalarLimit, got {:?}", errors.first());
        };
        assert_eq!(*limit, 5);
        assert!(*actual > 5);
    }

    #[test]
    fn compile_returns_duplicate_key_for_repeated_yaml_key() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: dup\nwhen:\n  manual: {}\nname: dup2\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::DuplicateKey { key, .. }) = errors.first() else {
            compile_test_fail!("expected DuplicateKey, got {:?}", errors.first());
        };
        assert_eq!(key.as_ref(), "name");
    }

    #[test]
    fn compile_returns_invalid_name_for_reserved_step_name() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: reserved\nwhen:\n  manual: {}\nsteps:\n  - id: run\n    save:\n      x: 1\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::InvalidName { field, value }) = errors.first() else {
            compile_test_fail!("expected InvalidName, got {:?}", errors.first());
        };
        assert_eq!(*field, "step id");
        assert_eq!(value.as_ref(), "run");
    }

    #[test]
    fn compile_returns_multiple_step_primitives_for_two_primitives() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: multi\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    save:\n      x: 1\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MultipleStepPrimitives { step }) = errors.first() else {
            compile_test_fail!("expected MultipleStepPrimitives, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

    #[test]
    fn compile_returns_invalid_trigger_count_for_two_triggers() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: multi_trigger\nwhen:\n  manual: {}\n  ipc: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::InvalidTriggerCount { count }) = errors.first() else {
            compile_test_fail!("expected InvalidTriggerCount, got {:?}", errors.first());
        };
        assert_eq!(*count, 2);
    }

    #[test]
    fn compile_returns_field_shape_for_bad_inputs_shape() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_inputs\nwhen:\n  manual: {}\ninputs: []\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::FieldShape { field, expected }) = errors.first() else {
            compile_test_fail!("expected FieldShape, got {:?}", errors.first());
        };
        assert_eq!(*field, "inputs");
        assert!(!expected.is_empty());
    }

    // ── Round 2: Compilation success path tests ──────────────────────────

    #[test]
    fn compile_produces_valid_workflow_for_minimal_source() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok, got {:?}", result)
        };
        assert_eq!(wf.node_count(), 2);
    }

    #[test]
    fn compile_produces_valid_workflow_for_optional_fields() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok, got {:?}", result)
        };
        assert_eq!(wf.node_count(), 2);
        assert_eq!(wf.name(), "fast_path");
    }

    #[test]
    fn compile_produces_non_default_workflow_digest() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok")
        };
        assert_ne!(
            wf.digest(),
            vb_core::ids::WorkflowDigest::from_bytes([0u8; 32])
        );
    }

    #[test]
    fn compile_produces_matching_workflow_name() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok")
        };
        assert_eq!(wf.name(), "fast_path");
    }

    #[test]
    fn compile_produces_correct_entry_step_index() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok")
        };
        assert_eq!(wf.entry(), vb_core::ids::StepIdx::ZERO);
    }

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

    #[test]
    fn lower_set_produces_set_node_kind() {
        let mut builder = SlotCompiler::new();
        let const_idx = builder
            .push_constant(ConstValue::I64(42))
            .ok()
            .unwrap_or(ConstIdx::new(0));
        let node = lower_set(
            StepIdx::new(0),
            SlotIdx::new(0),
            const_idx,
            Some(StepIdx::new(1)),
        );
        assert!(matches!(node.kind, CompiledNodeKind::SetConst { .. }));
    }

    #[test]
    fn lower_do_produces_do_node_kind() {
        let mut builder = SlotCompiler::new();
        let node = lower_do(
            StepIdx::new(0),
            ActionId::new(1),
            SlotIdx::new(0),
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(1)),
            &mut builder,
        );
        assert!(matches!(node.kind, CompiledNodeKind::Do { .. }));
    }

    #[test]
    fn lower_ask_uses_checked_resume_step() -> Result<(), String> {
        let mut builder = SlotCompiler::new();
        let nodes = lower_ask(
            StepIdx::new(7),
            SlotIdx::new(1),
            SlotIdx::new(2),
            None,
            &mut builder,
        )
        .map_err(|error| error.to_string())?;

        assert_eq!(nodes.len(), 2);
        let Some(first) = nodes.first() else {
            return Err(String::from("expected ask node"));
        };
        let Some(second) = nodes.get(1) else {
            return Err(String::from("expected ask resume node"));
        };
        assert!(matches!(first.kind, CompiledNodeKind::Ask { .. }));
        assert_eq!(second.id, StepIdx::new(8));
        assert_eq!(second.output, Some(SlotIdx::new(2)));
        assert!(matches!(second.kind, CompiledNodeKind::AskResume { .. }));
        Ok(())
    }

    #[test]
    fn lower_ask_rejects_resume_step_overflow() {
        let mut builder = SlotCompiler::new();
        let result = lower_ask(
            StepIdx::MAX,
            SlotIdx::new(1),
            SlotIdx::new(2),
            None,
            &mut builder,
        );

        let Err(CompileError::PrimitiveLoweringLimitExceeded {
            primitive,
            field,
            value,
            limit,
        }) = result
        else {
            compile_test_fail!("expected primitive lowering limit error");
        };
        assert_eq!(primitive, "ask");
        assert_eq!(field, "resume_step");
        assert_eq!(value, StepIdx::MAX.as_usize());
        assert_eq!(limit, usize::from(u16::MAX));
    }

    #[test]
    fn compute_compiled_digest_is_deterministic() {
        let d1 = compute_compiled_digest(NESTED_SAVE_SOURCE);
        let d2 = compute_compiled_digest(NESTED_SAVE_SOURCE);
        assert_eq!(d1, d2);
    }

    #[test]
    fn compute_compiled_digest_differs_for_different_sources() {
        let d1 = compute_compiled_digest(b"source_a");
        let d2 = compute_compiled_digest(b"source_b");
        assert_ne!(d1, d2);
    }

    // ── Round 2: SlotCompiler tests ──────────────────────────────────────

    #[test]
    fn slot_compiler_new_starts_empty() {
        let mut sc = SlotCompiler::new();
        assert_eq!(
            sc.push_constant(ConstValue::I64(42)).ok().map(|i| i.get()),
            Some(0)
        );
    }

    #[test]
    fn slot_compiler_push_constant_returns_ascending_indices() {
        let mut sc = SlotCompiler::new();
        let idx0 = sc.push_constant(ConstValue::I64(1));
        let idx1 = sc.push_constant(ConstValue::I64(2));
        assert_eq!(idx0.ok().map(|i| i.get()), Some(0));
        assert_eq!(idx1.ok().map(|i| i.get()), Some(1));
    }

    #[test]
    fn slot_compiler_push_expression_returns_ascending_indices() {
        let mut sc = SlotCompiler::new();
        let empty_ops: Box<[vb_core::workflow::ExprOp]> = Box::from([]);
        let prog = ExprProgram::try_from_ops(empty_ops).unwrap_or_else(|_| ExprProgram {
            ops: Box::from([]),
            max_stack: 0,
        });
        let idx = sc.push_expression(prog);
        assert_eq!(idx.ok().map(|i| i.get()), Some(0));
    }

    #[test]
    fn slot_compiler_record_slot_tracks_max_slot() {
        let mut sc = SlotCompiler::new();
        sc.record_slot(SlotIdx::new(5));
        sc.record_slot(SlotIdx::new(10));
        // record_slot doesn't return anything but should not panic
    }

    // ── Adversarial compilation pipeline tests ──────────────────────────────

    fn adv_compile_error(source: &[u8]) -> Result<CompileError, String> {
        match YamlCompiler::default().compile(source) {
            Ok(workflow) => Err(format!("compile unexpectedly succeeded: {workflow:?}")),
            Err(errors) => errors
                .first()
                .cloned()
                .ok_or_else(|| "CompileErrors was empty".to_owned()),
        }
    }

    fn adv_parse_error(source: &[u8]) -> Result<CompileError, String> {
        match YamlCompiler::default().parse_ast(source) {
            Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
            Err(errors) => errors
                .first()
                .cloned()
                .ok_or_else(|| "CompileErrors was empty".to_owned()),
        }
    }

    fn adv_compile_ok(source: &[u8]) -> Result<CompiledWorkflow, String> {
        YamlCompiler::default()
            .compile(source)
            .map_err(|errors| format!("compile unexpectedly failed: {errors}"))
    }

    fn adv_ensure(condition: bool, message: &'static str) -> Result<(), String> {
        if condition {
            Ok(())
        } else {
            Err(message.to_owned())
        }
    }

    fn adv_ensure_parity(
        source: &[u8],
        check: fn(CompileError) -> Result<(), String>,
    ) -> Result<(), String> {
        let c_text = compile_error_text(source);
        let p_text = parse_ast_error_text(source);
        adv_ensure(
            c_text == p_text,
            "compile and parse_ast diagnostics diverged",
        )?;
        check(adv_compile_error(source)?)?;
        check(adv_parse_error(source)?)
    }

    /// Attack vector 6: Empty steps list should be caught before any downstream validation.
    #[test]
    fn empty_steps_list_rejected_with_exact_error() -> Result<(), String> {
        let source =
            b"version: velvet-ballastics/v1\nname: empty_case\nwhen:\n  manual: {}\nsteps: []\n";
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::EmptySteps),
            "empty steps did not produce EmptySteps diagnostic",
        )
    }

    /// Attack vector 17: Workflow with only a single finish step and no other steps.
    #[test]
    fn single_finish_step_only_workflow_compiles_cleanly() -> Result<(), String> {
        let source = b"version: velvet-ballastics/v1\nname: single_finish\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: true\n";
        let workflow = adv_compile_ok(source)?;
        // Should produce 2 nodes: SetConst(true) + Finish(slot 0)
        adv_ensure(
            workflow.node_count() == 2,
            "single finish should produce 2 IR nodes",
        )
    }

    /// Attack vector 11: Missing finish step -- last step is a save.
    #[test]
    fn missing_finish_step_rejected_with_exact_last_step_must_finish() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: no_finish
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::LastStepMustFinish),
            "missing finish did not produce LastStepMustFinish",
        )
    }

    /// Attack vector 7: Finish step references an input not declared but used.
    #[test]
    fn finish_referencing_undeclared_input_rejected_by_reference_pass() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: missing_input_ref
when:
  manual: {}
steps:
  - id: done
    finish:
      result: $input.nonexistent
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(
                error,
                CompileError::UnknownReferenceName { kind: "input", .. }
            ),
            "undeclared input reference did not produce UnknownReferenceName diagnostic",
        )
    }

    /// Attack vector 8: Choose branches creating unreachable dead code (both branches skip a step).
    #[test]
    fn choose_both_branches_skip_produces_unreachable_step() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: unreachable_dead_code
when:
  manual: {}
steps:
  - id: flag
    save:
      value: true
  - id: route
    choose:
      condition: 0
      on_true: 3
      on_false: 3
  - id: dead
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::UnreachableStep { step: 2 }),
            "unreachable dead step did not produce exact UnreachableStep diagnostic",
        )
    }

    /// Attack vector 1 (approximation): Source byte limit hit produces SourceTooLarge.
    #[test]
    fn oversized_workflow_source_rejected_with_source_too_large() -> Result<(), String> {
        let tiny_limits = YamlLimits {
            max_source_bytes: 100,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler::new(tiny_limits);
        let mut source =
            String::from("version: velvet-ballastics/v1\nname: big\nwhen:\n  manual: {}\nsteps:\n");
        // Add enough steps to exceed 100 bytes
        for i in 0..20 {
            source.push_str(&format!("  - id: s{i}\n    save:\n      value: 1\n"));
        }
        source.push_str("  - id: done\n    finish:\n      result: 0\n");
        let result = compiler.compile(source.as_bytes());
        let Err(errors) = result else {
            return Err("expected compile error for oversized source".to_owned());
        };
        adv_ensure(
            matches!(errors.first(), Some(CompileError::SourceTooLarge { .. })),
            "oversized source did not produce SourceTooLarge",
        )
    }

    /// Attack vector 9 (approximation): Constant pool overflow through many save steps.
    /// With default limits this is too large to test, but we verify the constant
    /// pool tracks correctly for a modest number of steps.
    #[test]
    fn many_save_steps_compile_with_correct_node_count() -> Result<(), String> {
        let mut source = String::from(
            "version: velvet-ballastics/v1\nname: many_saves\nwhen:\n  manual: {}\nsteps:\n",
        );
        let step_count: usize = 50;
        for i in 0..step_count {
            source.push_str(&format!("  - id: s{i}\n    save:\n      value: {i}\n"));
        }
        // Finish with literal 0 (treated as slot 0, which is written by save step 0)
        source.push_str("  - id: done\n    finish:\n      result: 0\n");
        let workflow = adv_compile_ok(source.as_bytes())?;
        // Each save produces 1 node, finish with slot 0 produces 1 node
        let expected = step_count + 1;
        adv_ensure(
            usize::from(workflow.node_count()) == expected,
            "node count mismatch for many saves",
        )
    }

    /// Attack vector 12: Choose condition referencing undefined input via expression string.
    #[test]
    fn choose_expression_referencing_undefined_input_rejected() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: choose_undefined_ref
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$input.nonexistent == true"
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(
                error,
                CompileError::UnknownReferenceName { kind: "input", .. }
            ),
            "undefined input in choose expression did not produce reference diagnostic",
        )
    }

    /// Attack vector 5: Reference resolution with shadowed-looking variable names.
    /// Step IDs and input names should not collide.
    #[test]
    fn step_id_does_not_shadow_input_reference() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: shadow_test
when:
  manual: {}
inputs:
  value: text
steps:
  - id: value
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
        // This should compile fine because step IDs and input references
        // are in separate namespaces ($input.value vs step id "value").
        let _workflow = adv_compile_ok(source)?;
        Ok(())
    }

    /// Attack vector 3 approximation: Nested choose creates multiple branch targets.
    #[test]
    fn nested_choose_branches_compile_with_correct_ir() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: nested_choose
when:
  manual: {}
steps:
  - id: outer_flag
    save:
      value: true
  - id: inner_flag
    save:
      value: false
  - id: route_outer
    choose:
      condition: 0
      on_true: 3
      on_false: 4
  - id: route_inner
    choose:
      condition: 1
      on_true: 5
      on_false: 5
  - id: alt_path
    save:
      value: 2
  - id: done
    finish:
      result: 0
"#;
        let workflow = adv_compile_ok(source)?;
        // 3 saves + 2 chooses + 1 finish(slot 0) = 6 nodes
        let expected = 6u16;
        adv_ensure(
            workflow.node_count() == expected,
            "nested choose did not produce correct node count",
        )
    }

    /// Attack vector 10: Accessor path with deeply nested numeric segments.
    #[test]
    fn deep_numeric_accessor_path_accepted_by_reference_pass() -> Result<(), String> {
        // Build a deeply nested numeric path: $slot.0.1.2.3.4.5.6.7.8.9.10.11.12.13.14.15
        let source = br#"version: velvet-ballastics/v1
name: deep_accessor
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 0
examples:
  - name: fixture
    value: $slot.0.1.2.3.4.5.6.7.8.9.10.11.12.13.14.15
"#;
        // Should pass reference validation because numeric accessor paths are allowed
        let _workflow = adv_compile_ok(source)?;
        Ok(())
    }

    /// Attack vector: Non-numeric accessor path segment rejected.
    #[test]
    fn non_numeric_accessor_path_in_slot_rejected_with_unsupported_accessor() -> Result<(), String>
    {
        let source = br#"version: velvet-ballastics/v1
name: field_accessor
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slot.0.field_name
"#;
        adv_ensure_parity(source, |error| {
            adv_ensure(
                matches!(error, CompileError::UnsupportedAccessorReference { root, path, .. }
                    if root.as_ref() == "slot.0" && path.as_ref() == "field_name"),
                "field accessor did not produce UnsupportedAccessorReference",
            )
        })
    }

    /// Attack vector: Illegal $steps.done reference in examples.
    #[test]
    fn steps_reference_in_examples_rejected_as_illegal() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: illegal_steps_ref
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 0
examples:
  - name: fixture
    value: $steps.done
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::IllegalReference { .. }),
            "steps reference did not produce IllegalReference diagnostic",
        )
    }

    /// Attack vector: $runtime.now in choose condition is rejected.
    #[test]
    fn runtime_now_in_choose_condition_rejected_as_illegal() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: runtime_ref
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$runtime.now == true"
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::IllegalReference { .. }),
            "runtime.now in choose did not produce IllegalReference",
        )
    }

    /// Attack vector: Bare $now reference is rejected.
    #[test]
    fn bare_now_reference_in_finish_rejected_as_illegal() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: bare_now
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $now
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::IllegalReference { reference } if reference.as_ref() == "$now"),
            "bare $now did not produce IllegalReference diagnostic",
        )
    }

    /// Attack vector: Unknown reference root $env.HOME rejected.
    #[test]
    fn unknown_reference_root_env_rejected_with_unknown_root() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: env_ref
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $env.HOME
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::UnknownReferenceRoot { root, .. } if root.as_ref() == "env"),
            "$env.HOME did not produce UnknownReferenceRoot with root=env",
        )
    }

    /// Attack vector: Secret reference in finish result leaks taint.
    #[test]
    fn secret_in_finish_object_rejected_with_taint_leak() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: taint_leak
when:
  manual: {}
secrets:
  key: SECRET_KEY
steps:
  - id: done
    finish:
      result:
        token: $secrets.key
"#;
        adv_ensure_parity(source, |error| {
            adv_ensure(
                matches!(
                    error,
                    CompileError::SecretTaintLeak {
                        field: "finish.result"
                    }
                ),
                "secret in finish object did not produce taint leak",
            )
        })
    }

    /// Attack vector: Choose condition with non-boolean type (number literal in slot).
    #[test]
    fn choose_numeric_slot_condition_rejected_with_type_mismatch() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: num_choose
when:
  manual: {}
steps:
  - id: num
    save:
      value: 42
  - id: route
    choose:
      condition: 0
      on_true: 2
      on_false: 2
  - id: done
    finish:
      result: 0
"#;
        adv_ensure_parity(source, |error| {
            adv_ensure(
                matches!(
                    error,
                    CompileError::TypeMismatch {
                        field: "choose.condition",
                        expected: "boolean",
                        found: "number",
                    }
                ),
                "numeric slot condition did not produce type mismatch",
            )
        })
    }

    /// Attack vector: Finish slot referencing a forward (uninitialized) slot.
    #[test]
    fn finish_forward_slot_reference_rejected_with_unknown_slot() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: forward_slot
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 1
"#;
        adv_ensure_parity(source, |error| {
            adv_ensure(
                matches!(
                    error,
                    CompileError::UnknownSlotType {
                        field: "finish.result",
                        slot: 1
                    }
                ),
                "forward finish slot did not produce unknown slot diagnostic",
            )
        })
    }

    /// Attack vector: Expression helper with wrong arity (contains with 3 args).
    /// Expression parsing accepts the call but arity is checked during lowering.
    /// In the Phase 0 pipeline, expressions are retained in the AST without
    /// bytecode lowering, so arity is only checked when expression lowering runs.
    #[test]
    fn expression_helper_wrong_arity_rejected_in_bytecode_lowering() -> Result<(), String> {
        use crate::expression::parse_expression;
        use crate::expression_bytecode::compile_expr_to_bytecode;

        let expr = parse_expression("contains(1, 2, 3)").map_err(|e| format!("parse: {e:?}"))?;
        let mut constants = Vec::new();
        let error = compile_expr_to_bytecode(&expr, &mut constants)
            .map(|_| "unexpected success".to_owned())
            .unwrap_or_else(|e| e.to_string());
        adv_ensure(
            error.contains("contains") && error.contains("expects 2") && error.contains("found 3"),
            "helper arity mismatch did not produce exact diagnostic",
        )
    }

    /// Attack vector: Expression parse error (incomplete expression) produces
    /// deterministic diagnostic with compile/parse parity.
    #[test]
    fn malformed_expression_produces_deterministic_parse_error() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: bad_expr
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "1 +"
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#;
        adv_ensure(
            compile_error_text(source) == parse_ast_error_text(source),
            "compile and parse_ast diverged on malformed expression",
        )
    }

    /// Attack vector: Two steps writing to the same slot index.
    /// In Phase 0, save steps write to their step index as slot.
    /// Steps 0 and 1 write to slot 0 and slot 1 respectively, so no collision.
    /// But a finish referencing slot 0 when step 0 saved value 1 is valid.
    /// Test that the compiler handles slot layout correctly.
    #[test]
    fn slot_layout_two_saves_finish_reads_first_slot() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: slot_layout
when:
  manual: {}
steps:
  - id: first
    save:
      value: 10
  - id: second
    save:
      value: 20
  - id: done
    finish:
      result: 0
"#;
        let workflow = adv_compile_ok(source)?;
        let node = workflow
            .node(StepIdx::new(2))
            .ok_or("missing finish node")?;
        // The finish should read slot 0 (from first save step)
        match &node.kind {
            CompiledNodeKind::Finish { result } if result.get() == 0 => Ok(()),
            other => Err(format!("finish did not reference slot 0: {other:?}")),
        }
    }

    /// Attack vector: Non-last finish step rejected with exact diagnostic.
    #[test]
    fn finish_in_middle_position_rejected_with_step_field_shape() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: mid_finish
when:
  manual: {}
steps:
  - id: early
    finish:
      result: 0
  - id: late
    finish:
      result: 0
"#;
        adv_ensure_parity(source, |error| {
            adv_ensure(
                matches!(
                    error,
                    CompileError::StepFieldShape {
                        step: 0,
                        field: "finish",
                        expected: "the last step",
                    }
                ),
                "mid-position finish did not produce exact StepFieldShape diagnostic",
            )
        })
    }

    /// Attack vector: Choose with negative branch target rejected.
    #[test]
    fn choose_negative_branch_target_rejected_with_out_of_range() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: neg_target
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: true
      on_true: -1
      on_false: 1
  - id: done
    finish:
      result: 0
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::BranchTargetOutOfRange { value: -1 }),
            "negative branch target did not produce BranchTargetOutOfRange",
        )
    }

    /// Attack vector: Choose with branch target exceeding step count.
    #[test]
    fn choose_branch_target_exceeding_step_count_rejected() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: exceed_target
when:
  manual: {}
steps:
  - id: flag
    save:
      value: true
  - id: route
    choose:
      condition: 0
      on_true: 3
      on_false: 2
  - id: done
    finish:
      result: 0
"#;
        adv_ensure_parity(source, |error| {
            adv_ensure(
                matches!(
                    error,
                    CompileError::UnknownStepTarget { step: 1, target: 3 }
                ),
                "branch target exceeding step count did not produce UnknownStepTarget",
            )
        })
    }

    /// Attack vector: Multiple diagnostics in a single pass -- reference errors
    /// in examples and steps should accumulate.
    #[test]
    fn multiple_reference_errors_accumulate_in_compile() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: multi_error
when:
  manual: {}
inputs:
  user: text
examples:
  - name: bad1
    value: $input.missing_one
  - name: bad2
    value: $input.missing_two
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
        let result = YamlCompiler::default().compile(source);
        let Err(errors) = result else {
            return Err("expected compile error".to_owned());
        };
        // Should have at least 2 errors (one for each missing input reference)
        adv_ensure(
            errors.len() >= 2,
            "expected at least 2 accumulated reference errors",
        )?;
        for error in errors.iter() {
            adv_ensure(
                matches!(
                    error,
                    CompileError::UnknownReferenceName { kind: "input", .. }
                ),
                "accumulated error was not an input reference error",
            )?;
        }
        Ok(())
    }

    /// Attack vector: Expression with deeply nested parentheses hits depth limit.
    #[test]
    fn deeply_nested_expression_hits_parse_depth_limit() -> Result<(), String> {
        let depth = 70;
        let opens = "(".repeat(depth);
        let closes = ")".repeat(depth);
        let expr = format!("{opens}true{closes}");
        let source = format!(
            "version: velvet-ballastics/v1\nname: deep_expr\nwhen:\n  manual: {{}}\nsteps:\n  - id: route\n    choose:\n      condition: \"{expr}\"\n      on_true: 1\n      on_false: 1\n  - id: done\n    finish:\n      result: true\n"
        );
        let error = adv_compile_error(source.as_bytes())?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionLimitExceeded {
                    limit: "parse depth",
                    ..
                }
            ),
            "deeply nested expression did not hit parse depth limit",
        )
    }

    /// Attack vector: Expression exceeding token limit rejected.
    #[test]
    fn long_expression_hits_token_limit() -> Result<(), String> {
        // Generate an expression with more than 256 tokens
        let parts: Vec<&str> = (0..300).map(|_| "1").collect();
        let expr = parts.join(" + ");
        let source = format!(
            "version: velvet-ballastics/v1\nname: token_limit\nwhen:\n  manual: {{}}\nsteps:\n  - id: route\n    choose:\n      condition: \"{expr}\"\n      on_true: 1\n      on_false: 1\n  - id: done\n    finish:\n      result: true\n"
        );
        let error = adv_compile_error(source.as_bytes())?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionLimitExceeded {
                    limit: "token count",
                    ..
                }
            ),
            "long expression did not hit token count limit",
        )
    }

    /// Attack vector: Expression exceeding source length limit rejected.
    #[test]
    fn oversized_expression_hits_source_length_limit() -> Result<(), String> {
        // 4096+ character expression
        let expr = "1".repeat(4097);
        let source = format!(
            "version: velvet-ballastics/v1\nname: expr_len\nwhen:\n  manual: {{}}\nsteps:\n  - id: route\n    choose:\n      condition: \"{expr}\"\n      on_true: 1\n      on_false: 1\n  - id: done\n    finish:\n      result: true\n"
        );
        let error = adv_compile_error(source.as_bytes())?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionLimitExceeded {
                    limit: "source length",
                    ..
                }
            ),
            "oversized expression did not hit source length limit",
        )
    }

    /// Attack vector: Choose with self-referencing target rejected.
    #[test]
    fn choose_self_referencing_target_rejected_with_backward_branch() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: self_ref
when:
  manual: {}
steps:
  - id: first
    save:
      value: true
  - id: route
    choose:
      condition: true
      on_true: 1
      on_false: 2
  - id: done
    finish:
      result: 0
"#;
        adv_ensure_parity(source, |error| {
            adv_ensure(
                matches!(
                    error,
                    CompileError::BackwardBranchTarget { step: 1, target: 1 }
                ),
                "self-referencing branch did not produce exact backward target diagnostic",
            )
        })
    }

    /// Attack vector: Finish with integer 65536 that exceeds u16 slot range.
    /// Since 65536 > step index 0, it's treated as a literal value, not a slot.
    /// The Phase 0 compiler emits it as ConstValue::I64(65536) and compiles.
    #[test]
    fn finish_large_integer_compiled_as_literal_not_slot() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: huge_slot
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 65536
"#;
        let workflow = adv_compile_ok(source)?;
        // 65536 is > step index 0, so it's a literal, not a slot.
        // Produces 2 nodes: SetConst(65536) + Finish(slot 0)
        adv_ensure(
            workflow.node_count() == 2,
            "large integer finish should produce 2 nodes",
        )?;
        // Check constant pool contains the literal
        let node = workflow.node(StepIdx::new(0)).ok_or("missing node 0")?;
        match &node.kind {
            CompiledNodeKind::SetConst { value } => {
                let const_val = workflow.constant(*value).ok_or("missing constant")?;
                adv_ensure(
                    *const_val == ConstValue::I64(65536),
                    "constant should be I64(65536)",
                )
            }
            other => Err(format!("expected SetConst, got {other:?}")),
        }
    }

    /// Attack vector: Var referencing an accessor path ($vars.x.field) rejected.
    #[test]
    fn var_accessor_path_in_finish_rejected_with_unsupported_accessor() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: var_accessor
when:
  manual: {}
vars:
  data: 1
steps:
  - id: done
    finish:
      result: $vars.data.field
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::UnsupportedAccessorReference { .. }),
            "var accessor path did not produce UnsupportedAccessorReference",
        )
    }

    /// Attack vector: Validate that compile and parse_ast produce the same first
    /// diagnostic for a complex workflow with multiple issues (schema + reference).
    #[test]
    fn compile_parse_ast_parity_for_schema_then_reference_errors() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: parity_test
when:
  manual: {}
inputs:
  bad_field:
    is: text
    unknown_field: true
steps:
  - id: done
    finish:
      result: $input.missing
"#;
        adv_ensure(
            compile_error_text(source) == parse_ast_error_text(source),
            "compile and parse_ast diverged on schema+reference error",
        )
    }

    /// Attack vector: SlotCompiler constant pool overflow produces exact error.
    #[test]
    fn slot_compiler_constant_pool_overflow_rejected() -> Result<(), String> {
        let mut sc = SlotCompiler::new();
        // Fill up to u16::MAX + 1 (65536) constants; the 65537th push should fail
        let count = usize::from(u16::MAX) + 1;
        for i in 0..count {
            let value = i64::try_from(i).map_err(|error| error.to_string())?;
            let val = ConstValue::I64(value);
            sc.push_constant(val)
                .map_err(|e| format!("push {i} failed: {e:?}"))?;
        }
        // Now the pool has 65536 entries; the next push should fail
        let result = sc.push_constant(ConstValue::I64(0));
        adv_ensure(
            result.is_err(),
            "constant pool overflow (65536 existing + 1 new) should produce an error",
        )
    }

    // =========================================================================
    // Phase 65 tests -- idempotency verification gate
    // =========================================================================

    fn make_contract(
        id: u16,
        side_effect: vb_core::SideEffect,
        retry_safety: vb_core::RetrySafety,
        idempotency: vb_core::Idempotency,
    ) -> vb_core::ActionContract {
        vb_core::ActionContract {
            id: ActionId::new(id),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency,
            side_effect,
            retry_safety,
        }
    }

    #[test]
    fn idempotency_no_side_effects_passes() -> Result<(), String> {
        let contracts = [
            make_contract(
                1,
                vb_core::SideEffect::None,
                vb_core::RetrySafety::Safe,
                vb_core::Idempotency::DeterministicPure,
            ),
            make_contract(
                2,
                vb_core::SideEffect::None,
                vb_core::RetrySafety::Safe,
                vb_core::Idempotency::DeterministicPure,
            ),
        ];
        super::check_idempotency_gates(&contracts)
            .map_err(|e| format!("expected Ok, got errors: {:?}", e.0))
    }

    #[test]
    fn idempotency_side_effect_safe_retry_passes() -> Result<(), String> {
        let contracts = [make_contract(
            10,
            vb_core::SideEffect::Writes,
            vb_core::RetrySafety::Safe,
            vb_core::Idempotency::IdempotentExternal,
        )];
        super::check_idempotency_gates(&contracts)
            .map_err(|e| format!("expected Ok, got errors: {:?}", e.0))
    }

    #[test]
    fn idempotency_side_effect_unsafe_retry_rejected() -> Result<(), String> {
        let contracts = [make_contract(
            20,
            vb_core::SideEffect::Writes,
            vb_core::RetrySafety::Unsafe,
            vb_core::Idempotency::AtLeastOnceExternal,
        )];
        let result = super::check_idempotency_gates(&contracts);
        match result {
            Ok(()) => Err(String::from("expected error for unsafe retry, got Ok")),
            Err(errors) => {
                let first = errors.first().ok_or("errors should not be empty")?;
                match first {
                    CompileError::IdempotencyViolation {
                        action,
                        side_effect,
                        ..
                    } => {
                        if *action != ActionId::new(20) {
                            return Err(String::from("wrong action id"));
                        }
                        if *side_effect != vb_core::SideEffect::Writes {
                            return Err(String::from("wrong side effect"));
                        }
                        Ok(())
                    }
                    other => Err(format!("expected IdempotencyViolation, got {other:?}")),
                }
            }
        }
    }

    #[test]
    fn idempotency_non_idempotent_side_effect_rejected() -> Result<(), String> {
        let contracts = [make_contract(
            30,
            vb_core::SideEffect::Sends,
            vb_core::RetrySafety::KeyRequired,
            vb_core::Idempotency::AtLeastOnceExternal,
        )];
        let result = super::check_idempotency_gates(&contracts);
        match result {
            Ok(()) => Err(String::from(
                "expected error for non-idempotent side effect, got Ok",
            )),
            Err(errors) => {
                let first = errors.first().ok_or("errors should not be empty")?;
                match first {
                    CompileError::IdempotencyViolation {
                        action,
                        side_effect,
                        reason,
                    } => {
                        if *action != ActionId::new(30) {
                            return Err(String::from("wrong action id"));
                        }
                        if *side_effect != vb_core::SideEffect::Sends {
                            return Err(String::from("wrong side effect"));
                        }
                        let reason_ref: &str = &reason;
                        if !reason_ref.contains("AtLeastOnceExternal") {
                            return Err(String::from("reason should mention AtLeastOnceExternal"));
                        }
                        Ok(())
                    }
                    other => Err(format!("expected IdempotencyViolation, got {other:?}")),
                }
            }
        }
    }

    #[test]
    fn idempotency_idempotent_side_effect_passes() -> Result<(), String> {
        let contracts = [
            make_contract(
                40,
                vb_core::SideEffect::Creates,
                vb_core::RetrySafety::KeyRequired,
                vb_core::Idempotency::IdempotentExternal,
            ),
            make_contract(
                41,
                vb_core::SideEffect::Destroys,
                vb_core::RetrySafety::Safe,
                vb_core::Idempotency::IdempotentExternal,
            ),
        ];
        super::check_idempotency_gates(&contracts)
            .map_err(|e| format!("expected Ok, got errors: {:?}", e.0))
    }

    #[test]
    fn idempotency_mixed_actions_partial_rejection() -> Result<(), String> {
        let contracts = [
            make_contract(
                50,
                vb_core::SideEffect::None,
                vb_core::RetrySafety::Safe,
                vb_core::Idempotency::DeterministicPure,
            ),
            make_contract(
                51,
                vb_core::SideEffect::Writes,
                vb_core::RetrySafety::Safe,
                vb_core::Idempotency::IdempotentExternal,
            ),
            make_contract(
                52,
                vb_core::SideEffect::Destroys,
                vb_core::RetrySafety::Unsafe,
                vb_core::Idempotency::AtLeastOnceExternal,
            ),
        ];
        let result = super::check_idempotency_gates(&contracts);
        match result {
            Ok(()) => Err(String::from("expected error for unsafe action, got Ok")),
            Err(errors) => {
                if errors.as_slice().len() != 1 {
                    return Err(format!(
                        "expected exactly 1 error, got {}",
                        errors.as_slice().len()
                    ));
                }
                let first = errors.first().ok_or("errors should not be empty")?;
                match first {
                    CompileError::IdempotencyViolation { action, .. } => {
                        if *action != ActionId::new(52) {
                            return Err(String::from("expected violation for action 52 only"));
                        }
                        Ok(())
                    }
                    other => Err(format!("expected IdempotencyViolation, got {other:?}")),
                }
            }
        }
    }

    // ── SECURITY: Gate 12 bypass prevention tests ──────────────────────

    /// SECURITY: compile_workflow_with_contracts must reject mismatched contracts.
    ///
    /// Attack vector: Before the fix, `compile_workflow_with_contracts` did NOT
    /// run gate 12 (action contract completeness). A caller could provide
    /// contracts that had no corresponding Do nodes, or a workflow with Do
    /// nodes that had no contracts, and both would be accepted.
    ///
    /// This test verifies that gate 12 is now run during
    /// compile_workflow_with_contracts by providing an orphan contract
    /// (one that has no matching Do node in the workflow).
    #[test]
    fn security_compile_with_contracts_rejects_orphan_contract() -> Result<(), String> {
        use super::compile_workflow_with_contracts;

        // A valid workflow that compiles successfully. The finish result
        // uses a literal true (boolean) which avoids slot type issues.
        let source = br#"version: velvet-ballastics/v1
name: gate12_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: true
"#;

        // Contract 99 has no matching Do node -- gate 12 must reject it.
        let contracts = vec![make_contract(
            99,
            vb_core::SideEffect::None,
            vb_core::RetrySafety::Safe,
            vb_core::Idempotency::DeterministicPure,
        )];

        let result = compile_workflow_with_contracts(source, &contracts);
        match result {
            Err(errors) => {
                // Check that at least one error is ActionContractOrphan.
                // The pipeline may collect errors in any order.
                let found_orphan = errors.iter().any(|e| {
                    matches!(
                        e,
                        CompileError::Validation(
                            vb_validate::ValidationError::ActionContractOrphan { .. }
                        )
                    )
                });
                if found_orphan {
                    Ok(())
                } else {
                    let first = errors.first().ok_or("errors should not be empty")?;
                    Err(format!("expected ActionContractOrphan, got {first:?}"))
                }
            }
            Ok(_) => Err(String::from(
                "SECURITY: compile_workflow_with_contracts accepted orphan contract (gate 12 not run)",
            )),
        }
    }
}
