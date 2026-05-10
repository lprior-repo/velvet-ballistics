#![forbid(unsafe_code)]
use super::helpers::*;

    #[test]
    fn compiler_rejects_non_mapping_optional_top_level_fields_inputs() {
        let field = "inputs";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\n{field}: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. }))),
            "{field} must be mapping-shaped"
        );
    }

    #[test]
    fn compiler_rejects_non_mapping_optional_top_level_fields_vars() {
        let field = "vars";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\n{field}: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. }))),
            "{field} must be mapping-shaped"
        );
    }

    #[test]
    fn compiler_rejects_non_mapping_optional_top_level_fields_secrets() {
        let field = "secrets";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\n{field}: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. }))),
            "{field} must be mapping-shaped"
        );
    }

    #[test]
    fn compiler_rejects_invalid_optional_top_level_names_inputs() {
        let (field, key) = ("inputs", "InputValue");
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\n{field}:\n  {key}: value\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidName { .. }))),
            "{field}.{key} must use Velvet v1 public naming"
        );
    }

    #[test]
    fn compiler_rejects_invalid_optional_top_level_names_vars() {
        let (field, key) = ("vars", "run");
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\n{field}:\n  {key}: value\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidName { .. }))),
            "{field}.{key} must use Velvet v1 public naming"
        );
    }

    #[test]
    fn compiler_rejects_invalid_optional_top_level_names_secrets() {
        let (field, key) = ("secrets", "api-key");
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\n{field}:\n  {key}: value\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidName { .. }))),
            "{field}.{key} must use Velvet v1 public naming"
        );
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
    fn compiler_rejects_invalid_examples_shape_boolean() {
        let examples = "true";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nexamples: {examples}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. }))),
            "examples must be a sequence of mappings"
        );
    }

    #[test]
    fn compiler_rejects_invalid_examples_shape_missing_mapping() {
        let examples = "\n  - fixture";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nexamples: {examples}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. }))),
            "examples must be a sequence of mappings"
        );
    }

    #[test]
    fn compiler_rejects_examples_without_valid_names_empty_input() {
        let examples = "\n  - input: {}";
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

    #[test]
    fn compiler_rejects_examples_without_valid_names_numeric_name() {
        let examples = "\n  - name: 42";
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

    #[test]
    fn compiler_rejects_examples_without_valid_names_reserved_name() {
        let examples = "\n  - name: run";
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
    fn compiler_rejects_invalid_workflow_names_empty() {
        let name = "";
        let source = format!(
            "version: velvet-ballastics/v1\nname: \"{name}\"\nwhen:\n  manual: {{}}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidName { field: "name", .. }))),
            "workflow name {name:?} must be rejected"
        );
    }

    #[test]
    fn compiler_rejects_invalid_workflow_names_camel_case() {
        let name = "FastPath";
        let source = format!(
            "version: velvet-ballastics/v1\nname: \"{name}\"\nwhen:\n  manual: {{}}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidName { field: "name", .. }))),
            "workflow name {name:?} must be rejected"
        );
    }

    #[test]
    fn compiler_rejects_invalid_workflow_names_kebab_case() {
        let name = "fast-path";
        let source = format!(
            "version: velvet-ballastics/v1\nname: \"{name}\"\nwhen:\n  manual: {{}}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidName { field: "name", .. }))),
            "workflow name {name:?} must be rejected"
        );
    }

    #[test]
    fn compiler_rejects_invalid_workflow_names_reserved() {
        let name = "run";
        let source = format!(
            "version: velvet-ballastics/v1\nname: \"{name}\"\nwhen:\n  manual: {{}}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidName { field: "name", .. }))),
            "workflow name {name:?} must be rejected"
        );
    }
