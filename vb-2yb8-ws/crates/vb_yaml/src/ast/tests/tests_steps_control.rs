#![forbid(unsafe_code)]
//! Control flow step parsing tests - Choose.

#[cfg(test)]
mod tests {
    use super::super::parse::parse_workflow_ast;

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
    fn parse_choose_step() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: choose-test
            when:
              manual: {}
            steps:
              - id: c1
                choose:
                  branches:
                    - when: x > 0
                      steps:
                        - id: pos
                          set:
                            output: sign
                            value: \"1\"
                  otherwise: handle_zero
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Choose {
                branches,
                otherwise,
            } => {
                assert_eq!(branches.len(), 1);
                let first_branch = first_item!(branches, "branch");
                assert_eq!(first_branch.when, "x > 0");
                assert_eq!(first_branch.steps.len(), 1);
                assert_eq!(otherwise.as_deref(), Some("handle_zero"));
            }
            other => fail_assert!("expected Choose, got {other:?}"),
        }
    }

    #[test]
    fn parse_choose_with_multiple_branches() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: choose-multi
            when:
              manual: {}
            steps:
              - id: c1
                choose:
                  branches:
                    - when: x > 0
                      steps: []
                    - when: x < 0
                      steps: []
                  otherwise: zero
        "};
        let wf = parse_ok!(yaml);
        let first_step = first_item!(wf.steps, "step");
        match &first_step.primitive {
            StepPrimitive::Choose {
                branches,
                otherwise,
            } => {
                assert_eq!(branches.len(), 2);
                assert_eq!(
                    branches.first().map(|branch| branch.when.as_str()),
                    Some("x > 0")
                );
                assert_eq!(
                    branches.get(1).map(|branch| branch.when.as_str()),
                    Some("x < 0")
                );
                assert_eq!(otherwise.as_deref(), Some("zero"));
            }
            other => fail_assert!("expected Choose, got {other:?}"),
        }
    }
}