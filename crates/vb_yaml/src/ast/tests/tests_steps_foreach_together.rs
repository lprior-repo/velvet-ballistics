//! ForEach and Together step parsing tests.

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
    fn parse_foreach_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: foreach-test
            when:
              manual: {}
            steps:
              - id: fe1
                foreach:
                  variable: item
                  input: items
                  at_once: 5
                  steps:
                    - id: inner
                      set:
                        output: out
                        value: item
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::ForEach {
                variable,
                input,
                at_once,
                body,
            } => {
                assert_eq!(variable, "item");
                assert_eq!(input, "items");
                assert_eq!(*at_once, Some(5));
                assert_eq!(body.len(), 1);
            }
            other => fail_assert!("expected ForEach, got {other:?}"),
        }
    }

    #[test]
    fn parse_foreach_without_at_once() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: foreach-simple
            when:
              manual: {}
            steps:
              - id: fe1
                foreach:
                  variable: item
                  input: items
                  steps: []
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::ForEach {
                variable,
                input,
                at_once,
                body,
            } => {
                assert_eq!(variable, "item");
                assert_eq!(input, "items");
                assert_eq!(*at_once, None);
                assert!(body.is_empty());
            }
            other => fail_assert!("expected ForEach, got {other:?}"),
        }
    }

    #[test]
    fn parse_together_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: together-test
            when:
              manual: {}
            steps:
              - id: t1
                together:
                  branches:
                    - label: a
                      steps:
                        - id: sa
                          set:
                            output: x
                            value: \"1\"
                    - label: b
                      steps: []
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Together { branches } => {
                assert_eq!(branches.len(), 2);
                let first_branch = first_item!(branches, "branch");
                assert_eq!(first_branch.label, "a");
                assert_eq!(first_branch.steps.len(), 1);
                let Some(second_branch) = branches.get(1) else {
                    fail_assert!("missing second branch");
                    return;
                };
                assert_eq!(second_branch.label, "b");
            }
            other => fail_assert!("expected Together, got {other:?}"),
        }
    }

    #[test]
    fn parse_together_with_multiple_branches() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: together-multi
            when:
              manual: {}
            steps:
              - id: t1
                together:
                  branches:
                    - label: first
                      steps: []
                    - label: second
                      steps: []
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Together { branches } => {
                assert_eq!(branches.len(), 2);
                assert_eq!(
                    branches.first().map(|branch| branch.label.as_str()),
                    Some("first")
                );
                assert_eq!(
                    branches.get(1).map(|branch| branch.label.as_str()),
                    Some("second")
                );
            }
            other => fail_assert!("expected Together, got {other:?}"),
        }
    }
}