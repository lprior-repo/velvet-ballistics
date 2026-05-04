//! Adversarial error case parsing tests - workflow and trigger validation.

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
    fn adversarial_ast_http_trigger_rejected_by_ast_layer() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: http-trigger
            when:
              http: {}
            steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::UnsupportedFeature {
                feature: "http trigger"
            })
        );
    }

    #[test]
    fn adversarial_ast_scalar_root_rejected() {
        let yaml = "42\n";
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::FieldShape {
                field: "workflow",
                expected: "mapping"
            })
        );
    }

    #[test]
    fn adversarial_ast_sequence_root_rejected() {
        let yaml = "- a\n- b\n";
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::FieldShape {
                field: "workflow",
                expected: "mapping"
            })
        );
    }

    #[test]
    fn adversarial_ast_when_with_empty_mapping_rejected() {
        let yaml = "version: velvet-ballastics/v1\nname: bad\nwhen: {}\nsteps: []\n";
        let result = parse_workflow_ast(yaml);
        assert!(
            matches!(result, Err(YamlError::FieldShape { field, .. }) if field == "when"),
            "expected FieldShape for empty when, got: {result:?}"
        );
    }

    #[test]
    fn adversarial_ast_ipc_trigger_missing_name_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-ipc-name
            when:
              ipc: {}
            steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "when.ipc.name"
            })
        );
    }
}