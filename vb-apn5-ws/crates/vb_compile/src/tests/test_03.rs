#![forbid(unsafe_code)]
use super::helpers::*;

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

