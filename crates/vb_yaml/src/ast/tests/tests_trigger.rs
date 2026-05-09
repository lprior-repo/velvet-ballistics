#![forbid(unsafe_code)]
//! Trigger parsing tests.

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

    #[test]
    fn parse_ipc_trigger() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: ipc-test
            when:
              ipc:
                name: my-channel
            steps: []
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(
            wf.trigger,
            TriggerAst::Ipc {
                name: "my-channel".to_string()
            }
        );
    }

    #[test]
    fn parse_canonical_when_ipc_trigger() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: ipc-test
            when:
              ipc:
                name: issue_triage
            steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert!(matches!(
            result,
            Ok(WorkflowSource {
                trigger: TriggerAst::Ipc { name },
                ..
            }) if name == "issue_triage"
        ));
    }

    #[test]
    fn canonical_when_http_trigger_is_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: http-test
            when:
              http: {}
            steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert!(
            matches!(result, Err(YamlError::UnsupportedFeature { feature }) if feature == "http trigger")
        );
    }

    #[test]
    fn parse_ipc_trigger_exact_fields() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: ipc-exact
            when:
              ipc:
                name: my-channel
            steps: []
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(
            wf.trigger,
            TriggerAst::Ipc {
                name: "my-channel".to_string()
            }
        );
    }
}