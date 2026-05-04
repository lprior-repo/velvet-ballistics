//! Basic step type parsing tests - Set and Do.

#[cfg(test)]
mod tests {
    use super::super::parse::parse_workflow_ast;
    use super::super::types::*;

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
    fn parse_do_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: do-test
            when:
              manual: {}
            steps:
              - id: do1
                do:
                  action: http.get
                  input: '\"https://example.com\"'
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        assert!(matches!(
            &first_step.primitive,
            StepPrimitive::Do { action, input }
            if action == "http.get" && input == "\"https://example.com\""
        ));
    }

    #[test]
    fn parse_do_step_with_input() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: do-test
            when:
              manual: {}
            steps:
              - id: d1
                do:
                  action: http.post
                  input: payload
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Do { action, input } => {
                assert_eq!(action, "http.post");
                assert_eq!(input, "payload");
            }
            other => fail_assert!("expected Do, got {other:?}"),
        }
    }
}