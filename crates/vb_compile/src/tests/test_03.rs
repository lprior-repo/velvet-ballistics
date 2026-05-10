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
    fn compiler_accepts_input_long_form_list_elements_any() {
        let element = "any";
        let result = compile_with_inputs(&format!(
            "  values:\n    is: list\n    of: {element}\n    default: []\n    min: 0\n    max: 10\n"
        ));

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
            "list element schema {element} should compile"
        );
    }

    #[test]
    fn compiler_accepts_input_long_form_list_elements_text() {
        let element = "text";
        let result = compile_with_inputs(&format!(
            "  values:\n    is: list\n    of: {element}\n    default: []\n    min: 0\n    max: 10\n"
        ));

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
            "list element schema {element} should compile"
        );
    }

    #[test]
    fn compiler_accepts_input_long_form_list_elements_number() {
        let element = "number";
        let result = compile_with_inputs(&format!(
            "  values:\n    is: list\n    of: {element}\n    default: []\n    min: 0\n    max: 10\n"
        ));

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
            "list element schema {element} should compile"
        );
    }

    #[test]
    fn compiler_accepts_input_long_form_list_elements_boolean() {
        let element = "boolean";
        let result = compile_with_inputs(&format!(
            "  values:\n    is: list\n    of: {element}\n    default: []\n    min: 0\n    max: 10\n"
        ));

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
            "list element schema {element} should compile"
        );
    }

    #[test]
    fn compiler_accepts_input_long_form_list_elements_object() {
        let element = "object";
        let result = compile_with_inputs(&format!(
            "  values:\n    is: list\n    of: {element}\n    default: []\n    min: 0\n    max: 10\n"
        ));

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
            "list element schema {element} should compile"
        );
    }

    #[test]
    fn compiler_rejects_input_schema_unknown_fields_is_and_kind() {
        let inputs = "  value:\n    is: text\n    kind: text\n";
        let result = compile_with_inputs(inputs);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownInputSchemaField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_input_schema_unknown_fields_from_and_is() {
        let inputs = "  customer:\n    is: object\n    fields:\n      value:\n        is: text\n        from: request.body.value\n";
        let result = compile_with_inputs(inputs);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownInputSchemaField { .. }))
        ));
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
    fn compiler_rejects_invalid_input_schema_child_fields_list_no_of() {
        let inputs = "  values:\n    is: list\n";
        let result = compile_with_inputs(inputs);

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "invalid schema should be rejected: {inputs}"
        );
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_child_fields_text_with_of() {
        let inputs = "  value:\n    is: text\n    of: text\n";
        let result = compile_with_inputs(inputs);

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "invalid schema should be rejected: {inputs}"
        );
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_child_fields_text_with_fields() {
        let inputs = "  value:\n    is: text\n    fields:\n      nested: text\n";
        let result = compile_with_inputs(inputs);

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "invalid schema should be rejected: {inputs}"
        );
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_child_fields_text_with_extra() {
        let inputs = "  value:\n    is: text\n    extra: reject\n";
        let result = compile_with_inputs(inputs);

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "invalid schema should be rejected: {inputs}"
        );
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_child_fields_object_with_extra() {
        let inputs = "  customer:\n    is: object\n    extra: ignore\n";
        let result = compile_with_inputs(inputs);

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "invalid schema should be rejected: {inputs}"
        );
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_child_fields_object_with_fields_true() {
        let inputs = "  customer:\n    is: object\n    fields: true\n";
        let result = compile_with_inputs(inputs);

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "invalid schema should be rejected: {inputs}"
        );
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_child_fields_list_of_integer() {
        let inputs = "  values:\n    is: list\n    of: integer\n";
        let result = compile_with_inputs(inputs);

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "invalid schema should be rejected: {inputs}"
        );
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_child_fields_integer_type() {
        let inputs = "  value:\n    is: integer\n";
        let result = compile_with_inputs(inputs);

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "invalid schema should be rejected: {inputs}"
        );
    }

    #[test]
    fn compiler_rejects_non_boolean_input_schema_flags_optional() {
        let flag = "optional";
        let result = compile_with_inputs(&format!("  value:\n    is: text\n    {flag}: yes\n"));

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_non_boolean_input_schema_flags_nullable() {
        let flag = "nullable";
        let result = compile_with_inputs(&format!("  value:\n    is: text\n    {flag}: yes\n"));

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_non_boolean_input_schema_flags_secret() {
        let flag = "secret";
        let result = compile_with_inputs(&format!("  value:\n    is: text\n    {flag}: yes\n"));

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_default_that_does_not_match_input_schema_text_default_number() {
        let inputs = "  value:\n    is: text\n    default: 1\n";
        let result = compile_with_inputs(inputs);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_default_that_does_not_match_input_schema_number_default_string() {
        let inputs = "  value:\n    is: number\n    default: nope\n";
        let result = compile_with_inputs(inputs);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_default_that_does_not_match_input_schema_boolean_default_string() {
        let inputs = "  value:\n    is: boolean\n    default: nope\n";
        let result = compile_with_inputs(inputs);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_default_that_does_not_match_input_schema_object_default_array() {
        let inputs = "  value:\n    is: object\n    default: []\n";
        let result = compile_with_inputs(inputs);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_default_that_does_not_match_input_schema_list_default_object() {
        let inputs = "  value:\n    is: list\n    of: text\n    default: {}\n";
        let result = compile_with_inputs(inputs);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
        ));
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
    fn compiler_rejects_invalid_input_schema_bounds_min_greater_than_max() {
        let inputs = "  value:\n    is: number\n    min: 10\n    max: 1\n";
        let result = compile_with_inputs(inputs);

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "invalid bounds should be rejected: {inputs}"
        );
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_bounds_list_negative_min() {
        let inputs = "  values:\n    is: list\n    of: text\n    min: -1\n";
        let result = compile_with_inputs(inputs);

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "invalid bounds should be rejected: {inputs}"
        );
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_bounds_text_with_min() {
        let inputs = "  value:\n    is: text\n    min: 1\n";
        let result = compile_with_inputs(inputs);

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "invalid bounds should be rejected: {inputs}"
        );
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_bounds_text_negative_min_length() {
        let inputs = "  value:\n    is: text\n    min_length: -1\n";
        let result = compile_with_inputs(inputs);

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "invalid bounds should be rejected: {inputs}"
        );
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_bounds_text_min_greater_than_max_length() {
        let inputs = "  value:\n    is: text\n    min_length: 10\n    max_length: 1\n";
        let result = compile_with_inputs(inputs);

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "invalid bounds should be rejected: {inputs}"
        );
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_bounds_number_with_min_length() {
        let inputs = "  value:\n    is: number\n    min_length: 1\n";
        let result = compile_with_inputs(inputs);

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
            "invalid bounds should be rejected: {inputs}"
        );
    }
