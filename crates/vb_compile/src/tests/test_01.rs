#![forbid(unsafe_code)]
use super::helpers::*;

    #[test]
    fn compile_error_exposes_stable_validation_codes_duplicate_key() -> Result<(), String> {
        assert_compile_code(
            b"version: velvet-ballastics/v1\nversion: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n".as_slice(),
            "DUPLICATE_KEY",
        )
    }

    #[test]
    fn compile_error_exposes_stable_validation_codes_forbidden_yaml() -> Result<(), String> {
        assert_compile_code(
            b"version: velvet-ballastics/v1\nname: &n fast_path\ncopy: *n\n",
            "FORBIDDEN_YAML_FEATURE",
        )
    }

    #[test]
    fn compile_error_exposes_stable_validation_codes_unknown_top_level_field(
    ) -> Result<(), String> {
        assert_compile_code(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nunexpected: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
            "UNKNOWN_TOP_LEVEL_FIELD",
        )
    }

    #[test]
    fn compile_error_exposes_stable_validation_codes_missing_required_field(
    ) -> Result<(), String> {
        assert_compile_code(
            b"name: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
            "MISSING_REQUIRED_FIELD",
        )
    }

    #[test]
    fn compile_error_exposes_stable_validation_codes_invalid_version() -> Result<(), String> {
        assert_compile_code(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
            "INVALID_VERSION",
        )
    }

    #[test]
    fn compile_error_exposes_stable_validation_codes_invalid_id() -> Result<(), String> {
        assert_compile_code(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: BuildResult\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
            "INVALID_ID",
        )
    }

    #[test]
    fn compile_error_exposes_stable_validation_codes_reserved_id() -> Result<(), String> {
        assert_compile_code(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: finish\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
            "RESERVED_ID",
        )
    }

    #[test]
    fn compile_error_exposes_stable_validation_codes_duplicate_id() -> Result<(), String> {
        assert_compile_code(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: duplicate\n    save:\n      value: 1\n  - id: duplicate\n    finish:\n      result: 0\n",
            "DUPLICATE_ID",
        )
    }

    #[test]
    fn compile_error_exposes_stable_validation_codes_missing_step_primitive() -> Result<(), String> {
        assert_compile_code(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: only_metadata\n    name: Only Metadata\n  - id: done\n    finish:\n      result: 0\n",
            "MISSING_STEP_PRIMITIVE",
        )
    }

    #[test]
    fn compile_error_exposes_stable_validation_codes_multiple_step_primitives() -> Result<(), String> {
        assert_compile_code(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n    finish:\n      result: 0\n  - id: done\n    finish:\n      result: 0\n",
            "MULTIPLE_STEP_PRIMITIVES",
        )
    }

    #[test]
    fn compile_error_exposes_stable_validation_codes_invalid_choose() -> Result<(), String> {
        assert_compile_code(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose: true\n  - id: done\n    finish:\n      result: 0\n",
            "INVALID_CHOOSE",
        )
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
    fn parse_ast_and_compile_expose_same_diagnostic_codes_unknown_top_level_field(
    ) -> Result<(), String> {
        let source = b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nunexpected: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n".as_slice();
        let compile = compile_first_error(source)?;
        let parse = parse_first_error(source)?;
        ensure_equal(compile.code(), parse.code())
    }

    #[test]
    fn parse_ast_and_compile_expose_same_diagnostic_codes_invalid_reference(
    ) -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
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
"#;
        let compile = compile_first_error(source)?;
        let parse = parse_first_error(source)?;
        ensure_equal(compile.code(), parse.code())
    }

    #[test]
    fn parse_ast_and_compile_expose_same_diagnostic_codes_undeclared_secret(
    ) -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
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
"#;
        let compile = compile_first_error(source)?;
        let parse = parse_first_error(source)?;
        ensure_equal(compile.code(), parse.code())
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
