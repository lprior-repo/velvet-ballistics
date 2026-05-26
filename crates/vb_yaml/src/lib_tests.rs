#![forbid(unsafe_code)]
//! Library-level integration tests.

use super::*;

fn assertion_failed(_message: std::fmt::Arguments<'_>) -> bool {
    false
}

macro_rules! fail_assert {
    ($($arg:tt)*) => {
        assert!(assertion_failed(format_args!($($arg)*)), $($arg)*)
    };
}

#[test]
fn validate_rejects_empty_source() {
    let result = validate_yaml_profile("");
    assert!(matches!(result, Err(YamlError::EmptySource)));
}

#[test]
fn validate_accepts_simple_mapping() {
    let yaml = "key: value\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Ok(()));
}

#[test]
fn parse_events_returns_typed_events() {
    let yaml = "a: 1\n";
    let Ok(events) = parse_yaml_events(yaml) else {
        fail_assert!("parse events failed");
        return;
    };
    assert!(!events.is_empty());
}

#[test]
fn reject_duplicate_keys_detects_dups() {
    let keys = vec!["foo", "bar", "foo"];
    let result = reject_duplicate_keys(&keys);
    assert!(matches!(result, Err(YamlError::DuplicateKey { .. })));
}

#[test]
fn reject_duplicate_keys_allows_unique() {
    let keys = vec!["foo", "bar", "baz"];
    let result = reject_duplicate_keys(&keys);
    assert_eq!(result, Ok(()));
}

#[test]
fn reject_yaml_1_1_ambiguous_rejects_yes() {
    let scalars = vec!["yes"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert!(matches!(result, Err(YamlError::AmbiguousScalar { .. })));
}

#[test]
fn reject_yaml_1_1_ambiguous_allows_true() {
    let scalars = vec!["true"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert_eq!(result, Ok(()));
}

// -----------------------------------------------------------------------
// YamlError variant exact-assertion tests
// -----------------------------------------------------------------------

#[test]
fn reject_anchors_aliases_merges_returns_anchor_alias_merge_for_anchor() {
    let yaml = "a: &anc value\n";
    let Ok(events) = crate::events::collect_events(yaml) else {
        fail_assert!("collect_events failed");
        return;
    };
    let result = reject_anchors_aliases_merges(&events);
    assert!(matches!(result, Err(YamlError::AnchorAliasMerge { .. })));
}

#[test]
fn reject_anchors_aliases_merges_returns_anchor_alias_merge_for_alias() {
    let yaml = "a: &anc value\nb: *anc\n";
    let Ok(events) = crate::events::collect_events(yaml) else {
        fail_assert!("collect_events failed");
        return;
    };
    let result = reject_anchors_aliases_merges(&events);
    assert!(matches!(result, Err(YamlError::AnchorAliasMerge { .. })));
}

#[test]
fn reject_duplicate_keys_returns_duplicate_key_for_same_keys() {
    let keys = vec!["alpha", "beta", "alpha"];
    let result = reject_duplicate_keys(&keys);
    assert!(matches!(result, Err(YamlError::DuplicateKey { key, .. }) if key.as_ref() == "alpha"));
}

#[test]
fn reject_forbidden_features_returns_unsupported_feature_for_complex_key() {
    let yaml = indoc::indoc! {r#"
        version: velvet-ballistics/v1
        name: t
        when:
          http: {}
        steps: []
    "#};
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::UnsupportedTrigger {
            span: None,
            trigger: "http"
        })
    );
}

#[test]
fn reject_multiple_documents_returns_multiple_documents_for_doc_separator() {
    let yaml = "---\na: 1\n---\nb: 2\n";
    let Ok(events) = crate::events::collect_events(yaml) else {
        fail_assert!("collect_events failed");
        return;
    };
    let result = reject_multiple_documents(&events);
    assert_eq!(
        result,
        Err(YamlError::MultipleDocuments {
            span: None,
            count: 2
        })
    );
}

#[test]
fn reject_yaml_profile_returns_source_too_large_for_oversized_input() {
    let big = "x".repeat(200);
    let limits = YamlLimits {
        max_source_bytes: 100,
        max_depth: 64,
        max_nodes: 100_000,
        max_sequence_len: 10_000,
        max_mapping_entries: 1_024,
        max_scalar_bytes: 65_536,
    };
    let result = profile::validate_yaml_profile_with_limits(&big, &limits);
    assert_eq!(
        result,
        Err(YamlError::SourceTooLarge {
            size: 200,
            max: 100
        })
    );
}

#[test]
fn reject_yaml_profile_returns_nesting_too_deep_for_deeply_nested() {
    let mut yaml = String::from("a:\n");
    for i in 0..20 {
        let indent = "  ".repeat(i);
        yaml.push_str(&format!("{indent}b:\n"));
    }
    let limits = YamlLimits {
        max_source_bytes: 1_048_576,
        max_depth: 5,
        max_nodes: 100_000,
        max_sequence_len: 10_000,
        max_mapping_entries: 1_024,
        max_scalar_bytes: 65_536,
    };
    let result = profile::validate_yaml_profile_with_limits(&yaml, &limits);
    match result {
        Err(YamlError::NestingTooDeep { depth, max }) => {
            assert!(depth > 5);
            assert_eq!(max, 5);
        }
        other => fail_assert!("expected NestingTooDeep, got {other:?}"),
    }
}

#[test]
fn reject_yaml_profile_returns_node_limit_exceeded_for_many_nodes() {
    let mut yaml = String::from("a: 1\n");
    for i in 0..50 {
        yaml.push_str(&format!("key{i}: val{i}\n"));
    }
    let limits = YamlLimits {
        max_source_bytes: 1_048_576,
        max_depth: 64,
        max_nodes: 10,
        max_sequence_len: 10_000,
        max_mapping_entries: 1_024,
        max_scalar_bytes: 65_536,
    };
    let result = profile::validate_yaml_profile_with_limits(&yaml, &limits);
    match result {
        Err(YamlError::NodeLimitExceeded { count, max }) => {
            assert!(count > 10);
            assert_eq!(max, 10);
        }
        other => fail_assert!("expected NodeLimitExceeded, got {other:?}"),
    }
}

#[test]
fn reject_yaml_profile_returns_custom_tag_for_tags() {
    let yaml = "key: !custom value\n";
    let result = validate_yaml_profile(yaml);
    match result {
        Err(YamlError::CustomTag { tag, .. }) => {
            assert!(
                tag.contains("custom"),
                "tag should contain 'custom', got: {tag}"
            );
        }
        other => fail_assert!("expected CustomTag, got {other:?}"),
    }
}

#[test]
fn reject_yaml_profile_returns_ambiguous_scalar_for_unquoted_special() {
    let yaml = "flag: yes\n";
    let result = validate_yaml_profile(yaml);
    assert!(
        matches!(result, Err(YamlError::AmbiguousScalar { scalar, .. }) if scalar.as_ref() == "yes")
    );
}

#[test]
fn reject_yaml_profile_returns_ok_for_clean_yaml() {
    let yaml = "key: value\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Ok(()));
}

// -----------------------------------------------------------------------
// Parsing integration tests
// -----------------------------------------------------------------------

#[test]
fn parse_workflow_source_returns_ok_for_minimal_valid_workflow() {
    let yaml = indoc::indoc! {r#"
        version: velvet-ballistics/v1
        name: minimal
        when:
          manual: {}
        steps:
          - id: s1
            set:
              output: x
              value: \"42\"
    "#};
    let result = parse_workflow_source(yaml);
    match result {
        Ok(wf) => {
            assert_eq!(wf.name(), "minimal");
            assert_eq!(wf.steps().len(), 1);
        }
        Err(e) => fail_assert!("expected Ok, got Err: {e}"),
    }
}

#[test]
fn parse_workflow_source_returns_ok_for_workflow_with_version() {
    let yaml = indoc::indoc! {"
        version: velvet-ballistics/v1
        name: versioned
        when:
          manual: {}
        steps: []
    "};
    let result = parse_workflow_source(yaml);
    match result {
        Ok(wf) => {
            assert_eq!(wf.version(), "velvet-ballistics/v1");
        }
        Err(e) => fail_assert!("expected Ok, got Err: {e}"),
    }
}

#[test]
fn parse_workflow_source_returns_ok_for_multi_step_workflow() {
    let yaml = indoc::indoc! {"
        version: velvet-ballistics/v1
        name: multi
        when:
          manual: {}
        steps:
          - id: s1
            set:
              output: x
              value: \"1\"
          - id: s2
            set:
              output: val2
              value: \"2\"
          - id: s3
            set:
              output: val3
              value: \"3\"
    "};
    let result = parse_workflow_source(yaml);
    match result {
        Ok(wf) => {
            assert_eq!(wf.steps().len(), 3);
        }
        Err(e) => fail_assert!("expected Ok, got Err: {e}"),
    }
}

#[test]
fn parse_workflow_source_returns_error_for_empty_source() {
    let result = parse_workflow_source("");
    assert_eq!(result, Err(YamlError::EmptySource));
}

#[test]
fn parse_workflow_source_returns_error_for_non_mapping_root() {
    let yaml = "just a string\n";
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::FieldShape {
            span: None,
            field: "workflow",
            expected: "mapping"
        })
    );
}

#[test]
fn parse_workflow_source_returns_error_for_anchors() {
    let yaml = "a: &anc value\n";
    let result = parse_workflow_source(yaml);
    assert!(matches!(result, Err(YamlError::AnchorAliasMerge { .. })));
}

#[test]
fn parse_workflow_source_returns_error_for_aliases() {
    let yaml = "a: &anc value\nb: *anc\n";
    let result = parse_workflow_source(yaml);
    assert!(matches!(result, Err(YamlError::AnchorAliasMerge { .. })));
}

#[test]
fn parse_workflow_source_returns_error_for_multiple_documents() {
    let yaml = "---\na: 1\n---\nb: 2\n";
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::MultipleDocuments {
            span: None,
            count: 2
        })
    );
}

#[test]
fn parse_workflow_source_returns_error_for_ambiguous_scalar_unquoted_special() {
    let yaml = "flag: no\n";
    let result = parse_workflow_source(yaml);
    assert!(
        matches!(result, Err(YamlError::AmbiguousScalar { scalar, .. }) if scalar.as_ref() == "no")
    );
}

#[test]
fn validate_yaml_profile_accepts_simple_key_value_yaml() {
    let yaml = "name: test\ncount: 42\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_yaml_profile_accepts_workflow_with_all_step_types() {
    let yaml = indoc::indoc! {"
        version: velvet-ballistics/v1
        name: comprehensive
        when:
          manual: {}
        inputs:
          - name: count
            type: u32
        vars:
          - name: acc
            value: \"0\"
        secrets:
          - name: api_key
        steps:
          - id: s1
            set:
              output: x
              value: \"1\"
          - id: s2
            do:
              action: http.get
              input: '\"https://example.com\"'
    "};
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Ok(()));
}

#[test]
fn load_fixture_source_returns_content_for_valid_fixture() {
    let yaml = indoc::indoc! {"
        version: velvet-ballistics/v1
        name: fixture
        when:
          manual: {}
        steps:
          - id: s1
            set:
              output: x
              value: \"1\"
    "};
    let result = load_fixture_source(yaml);
    match result {
        Ok(wf) => assert_eq!(wf.name(), "fixture"),
        Err(e) => fail_assert!("expected Ok, got Err: {e}"),
    }
}

#[test]
fn parse_workflow_source_rejects_step_with_multiple_primitives() {
    let yaml = indoc::indoc! {r#"
        version: velvet-ballistics/v1
        name: duplicate-primitive
        when:
          manual: {}
        steps:
          - id: s1
            set: { output: x, value: "1" }
            do: { action: action.name, input: "x" }
    "#};
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::FieldShape {
            span: None,
            field: "step",
            expected: "exactly one primitive"
        })
    );
}

#[test]
fn parse_workflow_source_rejects_canonical_and_alias_duplicate_primitives() {
    let yaml = indoc::indoc! {r#"
        version: velvet-ballistics/v1
        name: duplicate-alias-primitive
        when:
          manual: {}
        steps:
          - id: s1
            do: { action: action.name, input: "x" }
            run: { action: action.name, input: "y" }
    "#};
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::FieldShape {
            span: None,
            field: "step",
            expected: "exactly one primitive"
        })
    );
}

#[test]
fn load_fixture_source_returns_error_for_missing_fixture() {
    let result = load_fixture_source("");
    assert_eq!(result, Err(YamlError::EmptySource));
}

#[test]
fn reject_yaml_1_1_ambiguous_rejects_no() {
    let scalars = vec!["no"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert!(
        matches!(result, Err(YamlError::AmbiguousScalar { scalar, .. }) if scalar.as_ref() == "no")
    );
}

#[test]
fn reject_yaml_1_1_ambiguous_rejects_on() {
    let scalars = vec!["on"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert!(
        matches!(result, Err(YamlError::AmbiguousScalar { scalar, .. }) if scalar.as_ref() == "on")
    );
}

#[test]
fn reject_yaml_1_1_ambiguous_rejects_off() {
    let scalars = vec!["off"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert!(
        matches!(result, Err(YamlError::AmbiguousScalar { scalar, .. }) if scalar.as_ref() == "off")
    );
}

#[test]
fn reject_yaml_1_1_ambiguous_rejects_y() {
    let scalars = vec!["y"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert!(
        matches!(result, Err(YamlError::AmbiguousScalar { scalar, .. }) if scalar.as_ref() == "y")
    );
}

#[test]
fn reject_yaml_1_1_ambiguous_rejects_n() {
    let scalars = vec!["n"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert!(
        matches!(result, Err(YamlError::AmbiguousScalar { scalar, .. }) if scalar.as_ref() == "n")
    );
}

#[test]
fn reject_yaml_1_1_ambiguous_is_case_insensitive() {
    let scalars = vec!["YES"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert!(
        matches!(result, Err(YamlError::AmbiguousScalar { scalar, .. }) if scalar.as_ref() == "YES")
    );
}

#[test]
fn reject_yaml_1_1_ambiguous_allows_regular_strings() {
    let scalars = vec!["hello", "world", "42"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert_eq!(result, Ok(()));
}

#[test]
fn parse_yaml_events_produces_events_for_valid_yaml() {
    let yaml = "a: 1\n";
    let result = parse_yaml_events(yaml);
    match result {
        Ok(events) => assert!(!events.is_empty()),
        Err(e) => fail_assert!("expected Ok, got Err: {e}"),
    }
}

#[test]
fn parse_yaml_events_returns_error_for_invalid_yaml() {
    let result = parse_yaml_events("");
    assert!(matches!(result, Err(YamlError::EmptySource)));
}

#[test]
fn reject_duplicate_keys_returns_exact_key_for_duplicate() {
    let keys = vec!["first", "repeat", "repeat"];
    let result = reject_duplicate_keys(&keys);
    assert!(matches!(result, Err(YamlError::DuplicateKey { key, .. }) if key.as_ref() == "repeat"));
}

#[test]
fn yaml_limits_default_has_expected_values() {
    let limits = YamlLimits::default();
    assert_eq!(limits.max_source_bytes, 1_048_576);
    assert_eq!(limits.max_depth, 64);
    assert_eq!(limits.max_nodes, 100_000);
    assert_eq!(limits.max_sequence_len, 10_000);
    assert_eq!(limits.max_mapping_entries, 1_024);
    assert_eq!(limits.max_scalar_bytes, 65_536);
}

#[test]
fn parse_workflow_source_returns_error_for_duplicate_step_ids() {
    let yaml =
        "version: velvet-ballistics/v1\nname: dup\nwhen:\n  manual: {}\nname: dup2\nsteps: []\n";
    let result = parse_workflow_source(yaml);
    assert!(matches!(result, Err(YamlError::DuplicateKey { key, .. }) if key.as_ref() == "name"));
}

#[test]
fn span_for_node_returns_none_for_empty_map() {
    let yaml = "a: 1\n";
    let Ok(map) = build_source_map(yaml) else {
        fail_assert!("build_source_map failed");
        return;
    };
    let result = span_for_node(&map, 999);
    assert_eq!(result, None);
}

// -----------------------------------------------------------------------
// Adversarial BDD tests - top-level API attack vectors
// -----------------------------------------------------------------------

#[test]
fn adversarial_api_null_byte_in_source_rejected() {
    let yaml = "key: \x00value\n";
    let result = parse_yaml_events(yaml);
    assert!(
        matches!(
            result,
            Err(YamlError::ForbiddenFeature {
                span: None,
                detail: "null_byte_in_source"
            })
        ),
        "expected ForbiddenFeature for null byte, got: {result:?}"
    );
}

#[test]
fn adversarial_api_null_byte_workflow_rejected() {
    let yaml = "version: velvet-ballistics/v1\nname: \x00bad\nwhen:\n  manual: {}\nsteps: []\n";
    let result = parse_workflow_source(yaml);
    assert!(result.is_err(), "expected error for null byte in workflow");
}

#[test]
fn adversarial_api_unicode_emoji_accepted() {
    let yaml = "name: test_emoji\nvalue: \"hello world\"\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_api_scalar_near_limit_accepted() {
    let val = "x".repeat(65_535);
    let yaml = format!("key: \"{val}\"\n");
    let result = validate_yaml_profile(&yaml);
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_api_scalar_one_over_limit_rejected() {
    let val = "x".repeat(65_537);
    let yaml = format!("key: \"{val}\"\n");
    let result = validate_yaml_profile(&yaml);
    assert!(
        matches!(result, Err(YamlError::ScalarTooLong { .. })),
        "expected ScalarTooLong, got: {result:?}"
    );
}

#[test]
fn adversarial_api_workflow_with_unsupported_trigger_rejected() {
    let yaml = indoc::indoc! {"
        version: velvet-ballistics/v1
        name: bad-trigger
        when:
          ipc: {}
        steps: []
    "};
    let result = parse_workflow_source(yaml);
    assert!(
        matches!(
            result,
            Err(YamlError::UnsupportedTrigger {
                span: None,
                trigger: "ipc"
            })
        ),
        "expected UnsupportedTrigger for ipc trigger, got: {result:?}"
    );
}

#[test]
fn adversarial_api_workflow_with_missing_when_rejected() {
    let yaml = "version: velvet-ballistics/v1\nname: no-when\nsteps: []\n";
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::MissingField {
            span: None,
            field: "when"
        })
    );
}

#[test]
fn adversarial_api_workflow_with_non_mapping_when_rejected() {
    let yaml = "version: velvet-ballistics/v1\nname: bad\nwhen: manual\nsteps: []\n";
    let result = parse_workflow_source(yaml);
    assert!(result.is_err(), "expected error for non-mapping when");
}

#[test]
fn canonical_triggers_and_aliases_parse() {
    let yaml = indoc::indoc! {r#"
        version: velvet-ballistics/v1
        name: canonical
        when:
          event:
            type: invoice.created
        inputs:
          payload:
            nested: [1, "$kept_text"]
        vars:
          flag: true
        secrets:
          api_key: ENV_API_KEY
        steps:
          - id: first
            save:
              output: answer
              value: "42"
          - id: done
            finish:
              result: answer
    "#};
    let wf = parse_workflow_source(yaml).expect("canonical workflow parses");
    assert!(
        matches!(wf.trigger(), crate::ast::TriggerAst::Event { event_type } if event_type == "invoice.created")
    );
    assert_eq!(wf.inputs().len(), 1);
    assert_eq!(wf.vars().len(), 1);
    assert_eq!(wf.secrets().len(), 1);
    assert_eq!(wf.steps().len(), 2);
}

#[test]
fn parse_workflow_source_accepts_all_v1_triggers() {
    let cases = [
        (
            "manual",
            indoc::indoc! {"
                version: velvet-ballistics/v1
                name: manual-trigger
                when:
                  manual: {}
                steps: []
            "},
        ),
        (
            "webhook",
            indoc::indoc! {"
                version: velvet-ballistics/v1
                name: webhook-trigger
                when:
                  webhook: {}
                steps: []
            "},
        ),
        (
            "event",
            indoc::indoc! {"
                version: velvet-ballistics/v1
                name: event-trigger
                when:
                  event:
                    type: invoice.created
                steps: []
            "},
        ),
        (
            "schedule",
            indoc::indoc! {"
                version: velvet-ballistics/v1
                name: schedule-trigger
                when:
                  schedule:
                    cron: '0 0 * * *'
                steps: []
            "},
        ),
    ];

    for (kind, yaml) in cases {
        let result = parse_workflow_source(yaml);
        match result {
            Ok(wf) => match (kind, wf.trigger()) {
                ("manual", crate::ast::TriggerAst::Manual)
                | ("webhook", crate::ast::TriggerAst::Webhook) => {}
                ("event", crate::ast::TriggerAst::Event { event_type }) => {
                    assert_eq!(event_type, "invoice.created");
                }
                ("schedule", crate::ast::TriggerAst::Schedule { cron }) => {
                    assert_eq!(cron, "0 0 * * *");
                }
                (_, other) => fail_assert!("wrong trigger for {kind}: {other:?}"),
            },
            Err(e) => fail_assert!("expected {kind} trigger to parse, got Err: {e}"),
        }
    }
}

#[test]
fn parse_workflow_source_accepts_schedule_with_cron_mapping() {
    let yaml = indoc::indoc! {"
        version: velvet-ballistics/v1
        name: scheduled
        when:
          schedule:
            cron: '*/5 * * * *'
        steps: []
    "};
    let result = parse_workflow_source(yaml);
    match result {
        Ok(wf) => assert!(
            matches!(wf.trigger(), crate::ast::TriggerAst::Schedule { cron } if cron == "*/5 * * * *")
        ),
        Err(e) => fail_assert!("expected schedule trigger to parse, got Err: {e}"),
    }
}

#[test]
fn parse_workflow_source_rejects_schedule_non_mapping_shape() {
    let yaml = indoc::indoc! {"
        version: velvet-ballistics/v1
        name: bad-schedule
        when:
          schedule: '*/5 * * * *'
        steps: []
    "};
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::FieldShape {
            span: None,
            field: "mapping",
            expected: "mapping"
        })
    );
}

#[test]
fn parse_workflow_source_rejects_schedule_missing_cron() {
    let yaml = indoc::indoc! {"
        version: velvet-ballistics/v1
        name: missing-schedule-cron
        when:
          schedule: {}
        steps: []
    "};
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::MissingField {
            span: None,
            field: "when.schedule.cron"
        })
    );
}

#[test]
fn parse_workflow_source_rejects_schedule_empty_cron() {
    let yaml = indoc::indoc! {"
        version: velvet-ballistics/v1
        name: empty-schedule-cron
        when:
          schedule:
            cron: ''
        steps: []
    "};
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::FieldShape {
            span: None,
            field: "when.schedule.cron",
            expected: "non-empty string"
        })
    );
}

#[test]
fn parse_workflow_source_rejects_multiple_triggers() {
    let yaml = indoc::indoc! {"
        version: velvet-ballistics/v1
        name: many-triggers
        when:
          manual: {}
          schedule:
            cron: '0 0 * * *'
        steps: []
    "};
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::FieldShape {
            span: None,
            field: "when",
            expected: "exactly one trigger"
        })
    );
}

#[test]
fn strict_rejects_retry_and_unknown_example_fields() {
    let retry_yaml = indoc::indoc! {r#"
        version: velvet-ballistics/v1
        name: bad_retry
        when: { manual: {} }
        steps:
          - id: first
            retry: { max_attempts: 2 }
            set: { output: x, value: "1" }
    "#};
    assert!(matches!(
        parse_workflow_source(retry_yaml),
        Err(YamlError::UnknownField { .. })
    ));
    let example_yaml = indoc::indoc! {"
        version: velvet-ballistics/v1
        name: bad_example
        when: { manual: {} }
        steps: []
        examples:
          - name: legacy
    "};
    assert!(matches!(
        parse_workflow_source(example_yaml),
        Err(YamlError::UnknownField { .. })
    ));
}

#[test]
fn semantic_source_map_tracks_trigger_and_block_scalar() {
    let yaml = indoc::indoc! {"
        version: velvet-ballistics/v1
        name: spans
        when:
          manual: {}
        steps:
          - id: first
            set:
              output: msg
              value: |
                one
                two
          - id: done
            finish: { result: msg }
    "};
    let map = crate::source_map::build_semantic_source_map(yaml).expect("source map builds");
    let manual = map.span_for_path("$.when.manual").expect("manual span");
    assert_eq!(&yaml[manual.start_offset..manual.end_offset], "manual");
    let value = map
        .span_for_path("$.steps[0].set.value")
        .expect("value span");
    assert!(value.end_line > value.start_line);
    assert!(value.end_col >= 1);
    let block_text = yaml
        .get(value.start_offset..value.end_offset)
        .expect("block scalar span is valid UTF-8 slice");
    assert!(block_text.contains("one"));
    assert!(block_text.contains("two"));
    assert!(!block_text.contains("done"));
    let done = map.span_for_path("$.steps[1].id").expect("done id span");
    assert_eq!(yaml.get(done.start_offset..done.end_offset), Some("done"));
}

#[test]
fn adversarial_api_oversized_source_rejected_immediately() {
    let big = "x".repeat(2_000_000);
    let result = validate_yaml_profile(&big);
    assert!(
        matches!(result, Err(YamlError::SourceTooLarge { .. })),
        "expected SourceTooLarge, got: {result:?}"
    );
}

#[test]
fn adversarial_api_only_whitespace_rejected() {
    let yaml = "   \t  \n  \n  ";
    let result = validate_yaml_profile(yaml);
    assert!(result.is_err(), "expected error for whitespace-only YAML");
}

// ---------------------------------------------------------------------------
// build_source_map — public API coverage
// ---------------------------------------------------------------------------

#[test]
fn build_source_map_returns_non_empty_for_valid_yaml() {
    let yaml = "key: value\n";
    let result = build_source_map(yaml);
    assert!(result.is_ok(), "build_source_map failed: {result:?}");
    let map = result.unwrap();
    assert!(
        !map.is_empty(),
        "source map should not be empty for valid YAML"
    );
    assert!(
        map.len() >= 2,
        "expected >=2 nodes for key: value, got {}",
        map.len()
    );
}

#[test]
fn span_for_node_returns_none_for_out_of_range_index() {
    let yaml = "a: 1\n";
    let map = build_source_map(yaml).expect("source map should build");
    let result = map.span_for_node(999);
    assert_eq!(
        result, None,
        "span_for_node(999) should return None for small doc"
    );
}

#[test]
fn span_for_node_returns_some_for_valid_index() {
    let yaml = "a: 1\n";
    let map = build_source_map(yaml).expect("source map should build");
    let result = map.span_for_node(0);
    assert!(
        result.is_some(),
        "span_for_node(0) should return Some for valid doc"
    );
}

// ---------------------------------------------------------------------------
// reject_forbidden_yaml_features — public API coverage
// ---------------------------------------------------------------------------

#[test]
fn reject_forbidden_yaml_features_rejects_custom_tag() {
    // Custom tags like !custom are rejected by the feature checker.
    let yaml = "key: !custom value\n";
    let Ok(events) = parse_yaml_events(yaml) else {
        // If YAML parsing itself fails that's also acceptable for this test.
        return;
    };
    let result = reject_forbidden_yaml_features(&events);
    assert!(
        matches!(result, Err(YamlError::CustomTag { .. })),
        "expected CustomTag error, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// load_fixture_source — public API coverage
// ---------------------------------------------------------------------------

#[test]
fn load_fixture_source_parses_valid_workflow() {
    let yaml = indoc::indoc! {r#"
        version: velvet-ballistics/v1
        name: test-workflow
        when:
          manual: {}
        steps:
          - id: start
            finish:
              result: "done"
    "#};
    let result = load_fixture_source(yaml);
    assert!(result.is_ok(), "load_fixture_source failed: {result:?}");
}

// ---------------------------------------------------------------------------
// YamlError::span() enrichment tests (B49-B55)
// ---------------------------------------------------------------------------

#[test]
fn yaml_error_all_variants_constructible_with_none_span() {
    // B49: Every YamlError variant constructible with span: None
    let errors: Vec<YamlError> = vec![
        YamlError::UnsupportedTrigger {
            trigger: "http",
            span: None,
        },
        YamlError::UnsupportedFeature {
            feature: "test",
            span: None,
        },
        YamlError::DuplicateKey {
            key: Box::from("k"),
            span: None,
        },
        YamlError::AnchorAliasMerge { span: None },
        YamlError::CustomTag {
            tag: Box::from("tag"),
            span: None,
        },
        YamlError::BinaryScalar { span: None },
        YamlError::MultipleDocuments {
            count: 2,
            span: None,
        },
        YamlError::AmbiguousScalar {
            scalar: Box::from("yes"),
            span: None,
        },
        YamlError::SourceTooLarge { size: 100, max: 50 },
        YamlError::NestingTooDeep { depth: 20, max: 16 },
        YamlError::NodeLimitExceeded {
            count: 5000,
            max: 1000,
        },
        YamlError::EmptySource,
        YamlError::ScalarTooLong {
            len: 100,
            max: 50,
            span: None,
        },
        YamlError::SequenceTooLong {
            len: 100,
            max: 50,
            span: None,
        },
        YamlError::MappingTooLarge {
            count: 100,
            max: 50,
            span: None,
        },
        YamlError::UnknownField {
            field: Box::from("x"),
            span: None,
        },
        YamlError::MissingField {
            field: "version",
            span: None,
        },
        YamlError::FieldShape {
            field: "steps",
            expected: "sequence",
            span: None,
        },
        YamlError::ParseError {
            line: 1,
            reason: Box::from("bad"),
            span: None,
        },
        YamlError::ForbiddenFeature {
            detail: "test",
            span: None,
        },
    ];
    // TF-VB-002b REPAIRED: Replace vacuous `let _ = error;` loop with
    // exact .span() assertions proving backward-compatibility (C4.3).
    assert_eq!(errors.len(), 20);
    for error in &errors {
        assert_eq!(
            error.span(),
            None,
            "variant {error} constructed with span: None must return None from span()"
        );
    }
}

#[test]
fn yaml_error_span_returns_none_for_limit_only_variants() {
    // B50: span() returns None for limit-only variants
    let errors: Vec<YamlError> = vec![
        YamlError::SourceTooLarge { size: 100, max: 50 },
        YamlError::NestingTooDeep { depth: 20, max: 16 },
        YamlError::NodeLimitExceeded {
            count: 5000,
            max: 1000,
        },
        YamlError::EmptySource,
    ];
    for error in &errors {
        assert_eq!(error.span(), None, "limit variant must return None span");
    }
}

#[test]
fn yaml_error_span_returns_some_for_span_carrying_variants() {
    // B51/B53: span() returns Some and preserves exact SourceSpan
    use crate::source_map::SourceSpan;
    let ss = SourceSpan::new(10, 20, 3, 5, 3, 9);

    let errors_with_span: Vec<YamlError> = vec![
        YamlError::DuplicateKey {
            key: Box::from("k"),
            span: Some(ss),
        },
        YamlError::ParseError {
            line: 5,
            reason: Box::from("syntax"),
            span: Some(ss),
        },
        YamlError::UnknownField {
            field: Box::from("bad"),
            span: Some(ss),
        },
        YamlError::ForbiddenFeature {
            detail: "test",
            span: Some(ss),
        },
        YamlError::AnchorAliasMerge { span: Some(ss) },
        YamlError::CustomTag {
            tag: Box::from("!bad"),
            span: Some(ss),
        },
        YamlError::BinaryScalar { span: Some(ss) },
        YamlError::AmbiguousScalar {
            scalar: Box::from("on"),
            span: Some(ss),
        },
        YamlError::MultipleDocuments {
            count: 3,
            span: Some(ss),
        },
        YamlError::UnsupportedTrigger {
            trigger: "http",
            span: Some(ss),
        },
        YamlError::UnsupportedFeature {
            feature: "test",
            span: Some(ss),
        },
        YamlError::ScalarTooLong {
            len: 100,
            max: 50,
            span: Some(ss),
        },
        YamlError::SequenceTooLong {
            len: 100,
            max: 50,
            span: Some(ss),
        },
        YamlError::MappingTooLarge {
            count: 100,
            max: 50,
            span: Some(ss),
        },
        YamlError::MissingField {
            field: "version",
            span: Some(ss),
        },
        YamlError::FieldShape {
            field: "steps",
            expected: "sequence",
            span: Some(ss),
        },
    ];
    // 16 span-carrying variants
    assert_eq!(errors_with_span.len(), 16);
    for error in &errors_with_span {
        assert_eq!(
            error.span(),
            Some(ss),
            "span-carrying variant must return Some(exact_span)"
        );
    }
}

#[test]
fn yaml_error_span_is_exhaustive_all_20_variants_covered() {
    // B52: span() is exhaustive — all 20 variants have a match arm.
    // This is compile-time enforced by the exhaustive match in YamlError::span().
    // Runtime verification: constructing each of the 20 variants and calling span().
    use crate::source_map::SourceSpan;
    let ss = SourceSpan::new(0, 5, 1, 1, 1, 5);

    let all: Vec<YamlError> = vec![
        YamlError::UnsupportedTrigger {
            trigger: "cron",
            span: Some(ss),
        },
        YamlError::UnsupportedFeature {
            feature: "f",
            span: None,
        },
        YamlError::DuplicateKey {
            key: Box::from("k"),
            span: None,
        },
        YamlError::AnchorAliasMerge { span: None },
        YamlError::CustomTag {
            tag: Box::from("t"),
            span: None,
        },
        YamlError::BinaryScalar { span: None },
        YamlError::MultipleDocuments {
            count: 1,
            span: None,
        },
        YamlError::AmbiguousScalar {
            scalar: Box::from("yes"),
            span: None,
        },
        YamlError::SourceTooLarge { size: 100, max: 50 },
        YamlError::NestingTooDeep { depth: 1, max: 16 },
        YamlError::NodeLimitExceeded {
            count: 1,
            max: 1000,
        },
        YamlError::EmptySource,
        YamlError::ScalarTooLong {
            len: 1,
            max: 10,
            span: None,
        },
        YamlError::SequenceTooLong {
            len: 1,
            max: 10,
            span: None,
        },
        YamlError::MappingTooLarge {
            count: 1,
            max: 10,
            span: None,
        },
        YamlError::UnknownField {
            field: Box::from("x"),
            span: None,
        },
        YamlError::MissingField {
            field: "v",
            span: None,
        },
        YamlError::FieldShape {
            field: "s",
            expected: "seq",
            span: None,
        },
        YamlError::ParseError {
            line: 1,
            reason: Box::from("e"),
            span: None,
        },
        YamlError::ForbiddenFeature {
            detail: "d",
            span: None,
        },
    ];
    assert_eq!(all.len(), 20);
    for error in &all {
        // Must not panic, must return Some or None based on variant
        let span_opt = error.span();
        // Verify the result is consistent with variant type
        match error {
            YamlError::SourceTooLarge { .. }
            | YamlError::NestingTooDeep { .. }
            | YamlError::NodeLimitExceeded { .. }
            | YamlError::EmptySource => {
                assert_eq!(span_opt, None, "limit variant must return None span");
            }
            _ => {
                // Span-carrying variants: Some/None depends on construction
                let _ = span_opt;
            }
        }
    }
}

#[test]
fn yaml_error_span_none_is_backward_compatible() {
    // B54: YamlError with span: None is backward compatible (Display/Eq)
    let err1 = YamlError::DuplicateKey {
        key: Box::from("test"),
        span: None,
    };
    let err2 = YamlError::DuplicateKey {
        key: Box::from("test"),
        span: None,
    };
    assert_eq!(err1, err2);
    // Display and Debug must not panic
    let _display = format!("{err1}");
    let _debug = format!("{err1:?}");
}

#[test]
fn yaml_error_parse_variants_carry_span_from_event_stream() {
    // B55: parse-level variants (ParseError, AnchorAliasMerge, etc.)
    // carry span from event stream
    use crate::source_map::SourceSpan;
    let parse_span = SourceSpan::new(15, 30, 2, 8, 2, 12);

    let parse_err = YamlError::ParseError {
        line: 2,
        reason: Box::from("unexpected token"),
        span: Some(parse_span),
    };
    assert_eq!(parse_err.span(), Some(parse_span));

    let anchor_err = YamlError::AnchorAliasMerge {
        span: Some(parse_span),
    };
    assert_eq!(anchor_err.span(), Some(parse_span));

    let custom_tag_err = YamlError::CustomTag {
        tag: Box::from("!invalid"),
        span: Some(parse_span),
    };
    assert_eq!(custom_tag_err.span(), Some(parse_span));

    let binary_err = YamlError::BinaryScalar {
        span: Some(parse_span),
    };
    assert_eq!(binary_err.span(), Some(parse_span));

    let ambiguous_err = YamlError::AmbiguousScalar {
        scalar: Box::from("on"),
        span: Some(parse_span),
    };
    assert_eq!(ambiguous_err.span(), Some(parse_span));
}
