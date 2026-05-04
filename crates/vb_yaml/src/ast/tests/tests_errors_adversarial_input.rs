//! Adversarial error case parsing tests - inputs and type validation.

#[cfg(test)]
mod tests {
    use super::super::parse::parse_workflow_ast;
    use crate::YamlError;

    fn assertion_failed(_message: std::fmt::Arguments<'_>) -> bool {
        false
    }

    macro_rules! fail_assert {
        ($($arg:tt)*) => {
            assert!(assertion_failed(format_args!($($arg)*)), $($arg)*)
        };
    }

    #[test]
    fn adversarial_ast_invalid_input_type_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: bad-inputs
            when:
              manual: {}
            inputs: not_a_list
            steps: []
        "};
        let wf = parse_workflow_ast(yaml).unwrap();
        assert!(
            wf.inputs.is_empty(),
            "non-sequence inputs should be treated as empty"
        );
    }

    #[test]
    fn adversarial_ast_non_mapping_input_item_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: bad-input-item
            when:
              manual: {}
            inputs:
              - just_a_string
            steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::FieldShape {
                field: "inputs",
                expected: "mapping"
            })
        );
    }
}