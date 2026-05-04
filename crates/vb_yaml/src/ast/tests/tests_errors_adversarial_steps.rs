//! Adversarial error case parsing tests - step validation.

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
    fn adversarial_ast_non_mapping_step_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: bad-step
            when:
              manual: {}
            steps:
              - just_a_string
        "};
        let result = parse_workflow_ast(yaml);
        assert!(
            matches!(result, Err(YamlError::FieldShape { field, .. }) if field == "step"),
            "expected FieldShape(step), got: {result:?}"
        );
    }

    #[test]
    fn adversarial_ast_step_missing_id_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-id
            when:
              manual: {}
            steps:
              - set:
                  output: x
                  value: \"1\"
        "};
        let result = parse_workflow_ast(yaml);
        assert_eq!(result, Err(YamlError::MissingField { field: "step.id" }));
    }

    #[test]
    fn adversarial_ast_empty_step_id_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: empty-id
            when:
              manual: {}
            steps:
              - id: ''
                set:
                  output: x
                  value: \"1\"
        "};
        let result = parse_workflow_ast(yaml);
        assert!(
            matches!(result, Err(YamlError::FieldShape { field, .. }) if field == "step.id"),
            "expected FieldShape for empty id, got: {result:?}"
        );
    }

    #[test]
    fn adversarial_ast_set_missing_output_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-output
            when:
              manual: {}
            steps:
              - id: s1
                set:
                  value: \"1\"
        "};
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "set.output"
            })
        );
    }

    #[test]
    fn adversarial_ast_do_missing_action_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-action
            when:
              manual: {}
            steps:
              - id: s1
                do:
                  input: payload
        "};
        let result = parse_workflow_ast(yaml);
        assert_eq!(result, Err(YamlError::MissingField { field: "do.action" }));
    }

    #[test]
    fn adversarial_ast_ask_missing_prompt_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-prompt
            when:
              manual: {}
            steps:
              - id: s1
                ask:
                  timeout: 10s
        "};
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "ask.prompt"
            })
        );
    }

    #[test]
    fn adversarial_ast_repeat_missing_max_attempts_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-max
            when:
              manual: {}
            steps:
              - id: s1
                repeat:
                  steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "max_attempts"
            })
        );
    }

    #[test]
    fn adversarial_ast_reduce_missing_initial_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-init
            when:
              manual: {}
            steps:
              - id: s1
                reduce:
                  variable: acc
                  input: items
                  steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "reduce.initial"
            })
        );
    }

    #[test]
    fn adversarial_ast_collect_missing_source_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-source
            when:
              manual: {}
            steps:
              - id: s1
                collect:
                  variable: page
                  steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "collect.source"
            })
        );
    }

    #[test]
    fn adversarial_ast_finish_missing_result_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-result
            when:
              manual: {}
            steps:
              - id: s1
                finish: {}
        "};
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "finish.result"
            })
        );
    }

    #[test]
    fn adversarial_ast_together_branch_missing_label_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-label
            when:
              manual: {}
            steps:
              - id: t1
                together:
                  branches:
                    - steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "together.branches[].label"
            })
        );
    }

    #[test]
    fn adversarial_ast_choose_branch_missing_when_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-when
            when:
              manual: {}
            steps:
              - id: c1
                choose:
                  branches:
                    - steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "choose.branches[].when"
            })
        );
    }
}