//! Collect, Reduce, and Repeat step parsing tests.

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
    fn parse_collect_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: collect-test
            when:
              manual: {}
            steps:
              - id: col1
                collect:
                  variable: page
                  source: api.list
                  pages: 10
                  items: 50
                  steps:
                    - id: process
                      set:
                        output: buf
                        value: page
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Collect {
                variable,
                source,
                pages,
                items,
                body,
            } => {
                assert_eq!(variable, "page");
                assert_eq!(source, "api.list");
                assert_eq!(*pages, Some(10));
                assert_eq!(*items, Some(50));
                assert_eq!(body.len(), 1);
            }
            other => fail_assert!("expected Collect, got {other:?}"),
        }
    }

    #[test]
    fn parse_collect_without_optional_fields() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: collect-simple
            when:
              manual: {}
            steps:
              - id: c1
                collect:
                  variable: page
                  source: api.list
                  steps: []
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Collect {
                variable,
                source,
                pages,
                items,
                body,
            } => {
                assert_eq!(variable, "page");
                assert_eq!(source, "api.list");
                assert_eq!(*pages, None);
                assert_eq!(*items, None);
                assert!(body.is_empty());
            }
            other => fail_assert!("expected Collect, got {other:?}"),
        }
    }

    #[test]
    fn parse_reduce_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: reduce-test
            when:
              manual: {}
            steps:
              - id: r1
                reduce:
                  variable: acc
                  input: items
                  initial: \"0\"
                  steps: []
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Reduce {
                variable,
                input,
                initial,
                body,
            } => {
                assert_eq!(variable, "acc");
                assert_eq!(input, "items");
                assert_eq!(initial, "0");
                assert!(body.is_empty());
            }
            other => fail_assert!("expected Reduce, got {other:?}"),
        }
    }

    #[test]
    fn parse_repeat_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: repeat-test
            when:
              manual: {}
            steps:
              - id: rp1
                repeat:
                  max_attempts: 3
                  steps:
                    - id: attempt
                      do:
                        action: http.post
                        input: body
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Repeat { max_attempts, body } => {
                assert_eq!(*max_attempts, 3);
                assert_eq!(body.len(), 1);
            }
            other => fail_assert!("expected Repeat, got {other:?}"),
        }
    }

    #[test]
    fn parse_repeat_with_max_attempts() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: repeat-simple
            when:
              manual: {}
            steps:
              - id: r1
                repeat:
                  max_attempts: 5
                  steps: []
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Repeat { max_attempts, body } => {
                assert_eq!(*max_attempts, 5);
                assert!(body.is_empty());
            }
            other => fail_assert!("expected Repeat, got {other:?}"),
        }
    }
}