#![forbid(unsafe_code)]
//! Step metadata and result/examples parsing tests.

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
    fn parse_step_with_metadata() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: meta-test
            when:
              manual: {}
            steps:
              - id: s1
                name: My Step
                if: x > 0
                with: http_connector
                retry:
                  max_attempts: 3
                  delay: 1s
                on_error:
                  handler: fallback
                then: next_step
                set:
                  output: y
                  value: \"hello\"
        "};
        let wf = parse_ok!(yaml);
        let step = first_item!(wf.steps, "step");
        assert_eq!(step.name.as_deref(), Some("My Step"));
        assert_eq!(step.condition.as_deref(), Some("x > 0"));
        assert_eq!(step.with.as_deref(), Some("http_connector"));
        assert_eq!(step.then.as_deref(), Some("next_step"));

        let Some(retry) = step.retry.as_ref() else {
            fail_assert!("missing retry");
            return;
        };
        assert_eq!(retry.max_attempts, 3);
        assert_eq!(retry.delay.as_deref(), Some("1s"));

        let Some(on_error) = step.on_error.as_ref() else {
            fail_assert!("missing on_error");
            return;
        };
        assert_eq!(on_error.handler, "fallback");
    }

    #[test]
    fn parse_step_with_condition() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: cond
            when:
              manual: {}
            steps:
              - id: s1
                if: x > 10
                set:
                  output: y
                  value: \"1\"
        "};
        let wf = parse_ok!(yaml);
        let step = first_item!(wf.steps, "step");
        assert_eq!(step.condition.as_deref(), Some("x > 10"));
    }

    #[test]
    fn parse_step_with_then() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: then-test
            when:
              manual: {}
            steps:
              - id: s1
                then: next_step
                set:
                  output: y
                  value: \"1\"
        "};
        let wf = parse_ok!(yaml);
        let step = first_item!(wf.steps, "step");
        assert_eq!(step.then.as_deref(), Some("next_step"));
    }

    #[test]
    fn parse_step_with_on_error_handler() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: error-handler
            when:
              manual: {}
            steps:
              - id: s1
                on_error:
                  handler: fallback
                set:
                  output: x
                  value: \"1\"
        "};
        let wf = parse_ok!(yaml);
        let step = first_item!(wf.steps, "step");
        let Some(ref on_error) = step.on_error else {
            fail_assert!("missing on_error");
            return;
        };
        assert_eq!(on_error.handler, "fallback");
    }

    #[test]
    fn parse_result_and_examples() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: result-test
            when:
              manual: {}
            steps: []
            result:
              value: final_output
            examples:
              - description: basic test
                input: '{\"x\": 1}'
                expected: \"2\"
        "};
        let wf = parse_ok!(yaml);
        let Some(result) = wf.result.as_ref() else {
            fail_assert!("missing result");
            return;
        };
        assert_eq!(result.value, "final_output");

        assert_eq!(wf.examples.len(), 1);
        let first_example = first_item!(wf.examples, "example");
        assert_eq!(first_example.description.as_deref(), Some("basic test"));
        assert_eq!(first_example.input.as_deref(), Some("{\"x\": 1}"));
        assert_eq!(first_example.expected.as_deref(), Some("2"));
    }

    #[test]
    fn parse_workflow_with_result() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: result-test
            when:
              manual: {}
            steps: []
            result:
              value: final_output
        "};
        let wf = parse_ok!(yaml);
        let Some(ref result) = wf.result else {
            fail_assert!("missing result");
            return;
        };
        assert_eq!(result.value, "final_output");
    }

    #[test]
    fn parse_workflow_with_examples() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: ex-test
            when:
              manual: {}
            steps: []
            examples:
              - description: basic
                input: '{\"x\": 1}'
                expected: \"2\"
              - description: empty
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(wf.examples.len(), 2);
        assert_eq!(
            wf.examples
                .first()
                .and_then(|example| example.description.as_deref()),
            Some("basic")
        );
        assert_eq!(
            wf.examples
                .first()
                .and_then(|example| example.input.as_deref()),
            Some("{\"x\": 1}")
        );
        assert_eq!(
            wf.examples
                .first()
                .and_then(|example| example.expected.as_deref()),
            Some("2")
        );
        assert_eq!(
            wf.examples
                .get(1)
                .and_then(|example| example.description.as_deref()),
            Some("empty")
        );
        assert_eq!(
            wf.examples.get(1).and_then(|example| example.input.as_ref()),
            None
        );
        assert_eq!(
            wf.examples.get(1).and_then(|example| example.expected.as_ref()),
            None
        );
    }
}