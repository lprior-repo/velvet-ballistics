//! Tests for the AST parsing.
//!
//! This module contains comprehensive tests for the AST types and parsing.

#[cfg(test)]
mod ast_tests {
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
    fn parse_inputs_vars_secrets() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: full
            when:
              manual: {}
            inputs:
              - name: count
                type: u32
                default: \"10\"
            vars:
              - name: acc
                value: \"0\"
            secrets:
              - name: api_key
                key: vault/api_key
            steps: []
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(wf.inputs.len(), 1);
        let first_input = first_item!(wf.inputs, "input");
        assert_eq!(first_input.name, "count");
        assert_eq!(first_input.field_type.as_deref(), Some("u32"));
        assert_eq!(first_input.default.as_deref(), Some("10"));

        assert_eq!(wf.vars.len(), 1);
        let first_var = first_item!(wf.vars, "var");
        assert_eq!(first_var.name, "acc");

        assert_eq!(wf.secrets.len(), 1);
        let first_secret = first_item!(wf.secrets, "secret");
        assert_eq!(first_secret.name, "api_key");
        assert_eq!(first_secret.key.as_deref(), Some("vault/api_key"));
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
    fn missing_version_is_error() {
        let yaml = "name: test\nwhen:\n  manual: {}\nsteps: []\n";
        let result = parse_workflow_ast(yaml);
        assert!(matches!(
            result,
            Err(YamlError::MissingField { field: "version" })
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
    fn parse_workflow_with_inputs_and_defaults() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: inputs-test
            when:
              manual: {}
            inputs:
              - name: count
                type: u32
                default: \"10\"
              - name: name
                type: string
            steps: []
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(wf.inputs.len(), 2);
        assert_eq!(
            wf.inputs.first().map(|input| input.name.as_str()),
            Some("count")
        );
        assert_eq!(
            wf.inputs
                .first()
                .and_then(|input| input.field_type.as_deref()),
            Some("u32")
        );
        assert_eq!(
            wf.inputs.first().and_then(|input| input.default.as_deref()),
            Some("10")
        );
        assert_eq!(
            wf.inputs.get(1).map(|input| input.name.as_str()),
            Some("name")
        );
        assert_eq!(
            wf.inputs
                .get(1)
                .and_then(|input| input.field_type.as_deref()),
            Some("string")
        );
        assert_eq!(
            wf.inputs.get(1).and_then(|input| input.default.as_ref()),
            None
        );
    }

    #[test]
    fn parse_workflow_with_vars() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: vars-test
            when:
              manual: {}
            vars:
              - name: acc
                value: \"0\"
              - name: buf
            steps: []
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(wf.vars.len(), 2);
        assert_eq!(wf.vars.first().map(|var| var.name.as_str()), Some("acc"));
        assert_eq!(
            wf.vars.first().and_then(|var| var.value.as_deref()),
            Some("0")
        );
        assert_eq!(wf.vars.get(1).map(|var| var.name.as_str()), Some("buf"));
        assert_eq!(wf.vars.get(1).and_then(|var| var.value.as_ref()), None);
    }

    #[test]
    fn parse_workflow_with_secrets() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: secrets-test
            when:
              manual: {}
            secrets:
              - name: api_key
                key: vault/api_key
              - name: db_pass
            steps: []
        "};
        let wf = parse_ok!(yaml);
        assert_eq!(wf.secrets.len(), 2);
        assert_eq!(
            wf.secrets.first().map(|secret| secret.name.as_str()),
            Some("api_key")
        );
        assert_eq!(
            wf.secrets.first().and_then(|secret| secret.key.as_deref()),
            Some("vault/api_key")
        );
        assert_eq!(
            wf.secrets.get(1).map(|secret| secret.name.as_str()),
            Some("db_pass")
        );
        assert_eq!(
            wf.secrets.get(1).and_then(|secret| secret.key.as_ref()),
            None
        );
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
            wf.examples
                .get(1)
                .and_then(|example| example.input.as_ref()),
            None
        );
        assert_eq!(
            wf.examples
                .get(1)
                .and_then(|example| example.expected.as_ref()),
            None
        );
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

    // -----------------------------------------------------------------------
    // Adversarial BDD tests - AST layer attack vectors
    // -----------------------------------------------------------------------

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
    fn adversarial_ast_invalid_input_type_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: bad-inputs
            when:
              manual: {}
            inputs: not_a_list
            steps: []
        "};
        let wf = parse_ok!(yaml);
        assert!(
            wf.inputs.is_empty(),
            "non-sequence inputs should be treated as empty"
        );
    }

    #[test]
    fn adversarial_ast_non_mapping_input_item_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: bad-input-item
            when:
              manual: {}
            inputs:
              - just_a_string
            steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::FieldShape {
                field: "inputs",
                expected: "mapping"
            })
        );
    }

    #[test]
    fn adversarial_ast_http_trigger_rejected_by_ast_layer() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: http-trigger
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

    #[test]
    fn adversarial_ast_scalar_root_rejected() {
        let yaml = "42\n";
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
    fn adversarial_ast_sequence_root_rejected() {
        let yaml = "- a\n- b\n";
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

    #[test]
    fn adversarial_ast_ipc_trigger_missing_name_rejected() {
        let yaml = indoc::indoc! {"
            version: velvet-ballastics/v1
            name: no-ipc-name
            when:
              ipc: {}
            steps: []
        "};
        let result = parse_workflow_ast(yaml);
        assert_eq!(
            result,
            Err(YamlError::MissingField {
                field: "when.ipc.name"
            })
        );
    }

    #[test]
    fn adversarial_ast_when_with_empty_mapping_rejected() {
        let yaml = "version: velvet-ballastics/v1\nname: bad\nwhen: {}\nsteps: []\n";
        let result = parse_workflow_ast(yaml);
        assert!(
            matches!(result, Err(YamlError::FieldShape { field, .. }) if field == "when"),
            "expected FieldShape for empty when, got: {result:?}"
        );
    }
}
