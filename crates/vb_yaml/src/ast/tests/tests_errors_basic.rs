#![forbid(unsafe_code)]
//! Basic error case parsing tests.

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
    fn missing_version_is_error() {
        let yaml = "name: test\nwhen:\n  manual: {}\nsteps: []\n";
        let result = parse_workflow_ast(yaml);
        assert!(matches!(
            result,
            Err(YamlError::MissingField { field: "version" })
        ));
    }

    #[test]
    fn missing_name_is_error() {
        let yaml = "version: velvet-ballastics/v1\nwhen:\n  manual: {}\nsteps: []\n";
        let result = parse_workflow_ast(yaml);
        assert!(matches!(
            result,
            Err(YamlError::MissingField { field: "name" })
        ));
    }

    #[test]
    fn missing_when_is_error() {
        let yaml = "version: velvet-ballastics/v1\nname: test\nsteps: []\n";
        let result = parse_workflow_ast(yaml);
        assert!(matches!(
            result,
            Err(YamlError::MissingField { field: "when" })
        ));
    }

    #[test]
    fn missing_step_primitive_is_error() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: test
            when:
              manual: {}
            steps:
              - id: s1
        "};
        let result = parse_workflow_ast(yaml);
        assert!(matches!(
            result,
            Err(YamlError::MissingField {
                field: "step primitive (set/save/do/choose/foreach/together/collect/reduce/repeat/wait/ask/finish)"
            })
        ));
    }

    #[test]
    fn empty_source_is_error() {
        let result = parse_workflow_ast("");
        assert!(matches!(result, Err(YamlError::EmptySource)));
    }

    #[test]
    fn missing_version_returns_missing_field_exact() {
        let yaml = "name: test\nwhen:\n  manual: {}\nsteps: []\n";
        let result = parse_workflow_ast(yaml);
        assert_eq!(result, Err(YamlError::MissingField { field: "version" }));
    }

    #[test]
    fn missing_name_returns_missing_field_exact() {
        let yaml = "version: velvet-ballastics/v1\nwhen:\n  manual: {}\nsteps: []\n";
        let result = parse_workflow_ast(yaml);
        assert_eq!(result, Err(YamlError::MissingField { field: "name" }));
    }

    #[test]
    fn missing_when_returns_missing_field_exact() {
        let yaml = "version: velvet-ballastics/v1\nname: test\nsteps: []\n";
        let result = parse_workflow_ast(yaml);
        assert_eq!(result, Err(YamlError::MissingField { field: "when" }));
    }

    #[test]
    fn missing_step_primitive_returns_error_exact() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: test
            when:
              manual: {}
            steps:
              - id: s1
        "};
        let result = parse_workflow_ast(yaml);
        match result {
            Err(YamlError::MissingField { field }) => {
                assert!(
                    field.contains("step primitive"),
                    "expected step primitive field, got: {field}"
                );
            }
            other => fail_assert!("expected MissingField, got {other:?}"),
        }
    }

    #[test]
    fn empty_version_returns_field_shape_error() {
        let yaml = "version: ''\nname: test\nwhen:\n  manual: {}\nsteps: []\n";
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::FieldShape {
                field: "version",
                expected: "non-empty string"
            })
        );
    }

    #[test]
    fn empty_name_returns_field_shape_error() {
        let yaml = "version: velvet-ballastics/v1\nname: ''\nwhen:\n  manual: {}\nsteps: []\n";
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::FieldShape {
                field: "name",
                expected: "non-empty string"
            })
        );
    }

    #[test]
    fn non_mapping_root_returns_field_shape_error() {
        let yaml = "just a string\n";
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
    fn http_trigger_returns_unsupported_feature_exact() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: t
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
}