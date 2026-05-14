#![forbid(unsafe_code)]
//! Basic workflow parsing tests.

#[cfg(test)]
mod tests {
    use super::super::parse::parse_workflow_ast;
    use super::super::types::*;
    use crate::YamlError;

    fn assertion_failed(_message: std::fmt::Arguments<'_>) -> bool {
        false
    }

    macro_rules! fail_assert {
        ($($arg:tt)*) => {
            assert!(assertion_failed(format_args!($($arg)*)), $($arg)*)
        };
    }

    macro_rules! parse_ok {
        ($yaml:expr) => {
            match parse_workflow_ast($yaml) {
                Ok(value) => value,
                Err(error) => {
                    fail_assert!("parse failed: {error}");
                    return;
                }
            }
        };
    }

    macro_rules! first_item {
        ($values:expr, $label:expr) => {
            match $values.first() {
                Some(value) => value,
                None => {
                    fail_assert!("missing {}", $label);
                    return;
                }
            }
        };
    }

    #[test]
    fn parse_minimal_workflow() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: minimal
            when:
              manual: {}
            steps:
              - id: s1
                set:
                  output: x
                  value: \"42\"
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(wf.version, "velvet-ballastics/v1");
        assert_eq!(wf.name, "minimal");
        assert_eq!(wf.trigger, TriggerAst::Manual);
        assert_eq!(wf.steps.len(), 1);
        let first_step = first_item!(wf.steps, "step");
        assert_eq!(first_step.id, "s1");
        assert!(matches!(
            &first_step.primitive,
            StepPrimitive::Set { output, value } if output == "x" && value == "42"
        ));
    }

    #[test]
    fn parse_canonical_when_manual_trigger() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: manual-test
            when:
              manual: {}
            steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert!(matches!(
            result,
            Ok(WorkflowSource {
                trigger: TriggerAst::Manual,
                ..
            })
        ));
    }

    #[test]
    fn parse_empty_steps_list() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: empty-steps
            when:
              manual: {}
            steps: []
        "};
        let wf = parse_ok!(yaml);
        assert!(wf.steps.is_empty());
    }

    #[test]
    fn parse_workflow_without_result() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-result
            when:
              manual: {}
            steps: []
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(wf.result, None);
    }

    #[test]
    fn parse_workflow_without_examples() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-ex
            when:
              manual: {}
            steps: []
        "};
        let wf = parse_ok!(yaml);
        assert!(wf.examples.is_empty());
    }

    #[test]
    fn parse_step_without_optional_fields() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: minimal-step
            when:
              manual: {}
            steps:
              - id: s1
                set:
                  output: x
                  value: \"1\"
        "};
        let wf = parse_ok!(yaml);
        let step = first_item!(wf.steps, "step");
        assert_eq!(step.name, None);
        assert_eq!(step.condition, None);
        assert_eq!(step.with, None);
        assert_eq!(step.retry, None);
        assert_eq!(step.on_error, None);
        assert_eq!(step.then, None);
    }

    // -----------------------------------------------------------------------
    // AST BDD tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_workflow_ast_produces_typed_nodes_for_valid_input() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: typed
            when:
              manual: {}
            steps:
              - id: s1
                set:
                  output: x
                  value: \"1\"
        "};
        let result = parse_workflow_ast(yaml);
        match result {
            Ok(wf) => {
                assert_eq!(wf.version, "velvet-ballastics/v1");
                assert_eq!(wf.name, "typed");
                assert_eq!(wf.steps.len(), 1);
            }
            Err(e) => fail_assert!("expected Ok, got Err: {e}"),
        }
    }

    #[test]
    fn parse_workflow_ast_returns_scalar_kind_for_scalar_nodes() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: scalar-test
            when:
              manual: {}
            steps:
              - id: s1
                set:
                  output: x
                  value: \"hello\"
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Set { output, value } => {
                assert_eq!(output, "x");
                assert_eq!(value, "hello");
            }
            other => fail_assert!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn parse_workflow_ast_returns_mapping_for_mapping_nodes() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: mapping-test
            when:
              manual: {}
            steps:
              - id: s1
                retry:
                  max_attempts: 5
                set:
                  output: x
                  value: \"1\"
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        let Some(retry) = first_step.retry.as_ref() else {
            fail_assert!("missing retry");
            return;
        };
        assert_eq!(retry.max_attempts, 5);
        assert_eq!(retry.delay, None);
    }

    #[test]
    fn parse_workflow_ast_returns_sequence_for_sequence_nodes() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: seq-test
            when:
              manual: {}
            steps:
              - id: s1
                set:
                  output: x
                  value: \"1\"
              - id: s2
                set:
                  output: y
                  value: \"2\"
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(wf.steps.len(), 2);
        assert_eq!(wf.steps.first().map(|step| step.id.as_str()), Some("s1"));
        assert_eq!(wf.steps.get(1).map(|step| step.id.as_str()), Some("s2"));
    }

    #[test]
    fn parse_preserves_span_information_in_nodes() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: span-test
            when:
              manual: {}
            steps: []
        "};
        let result = crate::source_map::build_source_map(yaml);
        match result {
            Ok(map) => {
                assert!(!map.is_empty());
                let first_span = map.span_for_node(0);
                let Some(span) = first_span else {
                    fail_assert!("expected Some span for node 0");
                    return;
                };
                assert!(span.start_line > 0);
            }
            Err(e) => fail_assert!("expected Ok, got Err: {e}"),
        }
    }
}