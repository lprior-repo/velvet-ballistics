#![forbid(unsafe_code)]
//! Terminal step parsing tests - Wait, Ask, Finish.

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
    fn parse_wait_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: wait-test
            when:
              manual: {}
            steps:
              - id: w1
                wait:
                  event: approval
                  timeout: 30s
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Wait { event, timeout } => {
                assert_eq!(event.as_deref(), Some("approval"));
                assert_eq!(timeout.as_deref(), Some("30s"));
            }
            other => fail_assert!("expected Wait, got {other:?}"),
        }
    }

    #[test]
    fn parse_wait_step_with_only_timeout() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: wait-only
            when:
              manual: {}
            steps:
              - id: w1
                wait:
                  timeout: 10s
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Wait { event, timeout } => {
                assert_eq!(*event, None);
                assert_eq!(timeout.as_deref(), Some("10s"));
            }
            other => fail_assert!("expected Wait, got {other:?}"),
        }
    }

    #[test]
    fn parse_ask_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: ask-test
            when:
              manual: {}
            steps:
              - id: a1
                ask:
                  prompt: Continue?
                  timeout: 60s
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Ask { prompt, timeout } => {
                assert_eq!(prompt, "Continue?");
                assert_eq!(timeout.as_deref(), Some("60s"));
            }
            other => fail_assert!("expected Ask, got {other:?}"),
        }
    }

    #[test]
    fn parse_ask_step_without_timeout() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: ask-simple
            when:
              manual: {}
            steps:
              - id: a1
                ask:
                  prompt: What?
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Ask { prompt, timeout } => {
                assert_eq!(prompt, "What?");
                assert_eq!(*timeout, None);
            }
            other => fail_assert!("expected Ask, got {other:?}"),
        }
    }

    #[test]
    fn parse_finish_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: finish-test
            when:
              manual: {}
            steps:
              - id: f1
                finish:
                  result: output
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Finish { result } => {
                assert_eq!(result, &ScalarValue::String(String::from("output")));
            }
            other => fail_assert!("expected Finish, got {other:?}"),
        }
    }

    #[test]
    fn parse_finish_step_with_result() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: finish-simple
            when:
              manual: {}
            steps:
              - id: f1
                finish:
                  result: done
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Finish { result } => {
                assert_eq!(result, &ScalarValue::String(String::from("done")));
            }
            other => fail_assert!("expected Finish, got {other:?}"),
        }
    }
}