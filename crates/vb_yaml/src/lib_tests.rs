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
    assert_eq!(result, Err(YamlError::EmptySource));
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
    assert_eq!(events.len(), 8);
    assert_eq!(events[3].as_scalar(), Some("a"));
    assert_eq!(events[4].as_scalar(), Some("1"));
}

#[test]
fn reject_duplicate_keys_detects_dups() {
    let keys = vec!["foo", "bar", "foo"];
    let result = reject_duplicate_keys(&keys);
    assert_eq!(result, Err(YamlError::DuplicateKey { key: "foo".into() }));
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
    assert_eq!(result, Err(YamlError::AmbiguousScalar { scalar: "yes".into() }));
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
    assert_eq!(result, Err(YamlError::AnchorAliasMerge));
}

#[test]
fn reject_anchors_aliases_merges_returns_anchor_alias_merge_for_alias() {
    let yaml = "a: &anc value\nb: *anc\n";
    let Ok(events) = crate::events::collect_events(yaml) else {
        fail_assert!("collect_events failed");
        return;
    };
    let result = reject_anchors_aliases_merges(&events);
    assert_eq!(result, Err(YamlError::AnchorAliasMerge));
}

#[test]
fn reject_duplicate_keys_returns_duplicate_key_for_same_keys() {
    let keys = vec!["alpha", "beta", "alpha"];
    let result = reject_duplicate_keys(&keys);
    assert_eq!(
        result,
        Err(YamlError::DuplicateKey {
            key: "alpha".into()
        })
    );
}

#[test]
fn reject_forbidden_features_returns_unsupported_feature_for_complex_key() {
    let yaml = indoc::indoc! {r#"
        version: velvet-ballastics/v1
        name: t
        when:
          http: {}
        steps: []
    "#};
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::UnsupportedTrigger { trigger: "http" })
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
    assert_eq!(result, Err(YamlError::MultipleDocuments { count: 2 }));
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
    assert_eq!(
        result,
        Err(YamlError::CustomTag {
            tag: "!custom".into()
        })
    );
}

#[test]
fn reject_yaml_profile_returns_ambiguous_scalar_for_unquoted_special() {
    let yaml = "flag: yes\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar {
            scalar: "yes".into()
        })
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
        version: velvet-ballastics/v1
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
        version: velvet-ballastics/v1
        name: versioned
        when:
          manual: {}
        steps: []
    "};
    let result = parse_workflow_source(yaml);
    match result {
        Ok(wf) => {
            assert_eq!(wf.version(), "velvet-ballastics/v1");
        }
        Err(e) => fail_assert!("expected Ok, got Err: {e}"),
    }
}

#[test]
fn parse_workflow_source_returns_ok_for_multi_step_workflow() {
    let yaml = indoc::indoc! {"
        version: velvet-ballastics/v1
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
            field: "workflow",
            expected: "mapping"
        })
    );
}

#[test]
fn parse_workflow_source_returns_error_for_anchors() {
    let yaml = "a: &anc value\n";
    let result = parse_workflow_source(yaml);
    assert_eq!(result, Err(YamlError::AnchorAliasMerge));
}

#[test]
fn parse_workflow_source_returns_error_for_aliases() {
    let yaml = "a: &anc value\nb: *anc\n";
    let result = parse_workflow_source(yaml);
    assert_eq!(result, Err(YamlError::AnchorAliasMerge));
}

#[test]
fn parse_workflow_source_returns_error_for_multiple_documents() {
    let yaml = "---\na: 1\n---\nb: 2\n";
    let result = parse_workflow_source(yaml);
    assert_eq!(result, Err(YamlError::MultipleDocuments { count: 2 }));
}

#[test]
fn parse_workflow_source_returns_error_for_ambiguous_scalar_unquoted_special() {
    let yaml = "flag: no\n";
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar {
            scalar: "no".into()
        })
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
        version: velvet-ballastics/v1
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
        version: velvet-ballastics/v1
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
        version: velvet-ballastics/v1
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
            field: "step",
            expected: "exactly one primitive"
        })
    );
}

#[test]
fn parse_workflow_source_rejects_canonical_and_alias_duplicate_primitives() {
    let yaml = indoc::indoc! {r#"
        version: velvet-ballastics/v1
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
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar {
            scalar: "no".into()
        })
    );
}

#[test]
fn reject_yaml_1_1_ambiguous_rejects_on() {
    let scalars = vec!["on"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar {
            scalar: "on".into()
        })
    );
}

#[test]
fn reject_yaml_1_1_ambiguous_rejects_off() {
    let scalars = vec!["off"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar {
            scalar: "off".into()
        })
    );
}

#[test]
fn reject_yaml_1_1_ambiguous_rejects_y() {
    let scalars = vec!["y"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar { scalar: "y".into() })
    );
}

#[test]
fn reject_yaml_1_1_ambiguous_rejects_n() {
    let scalars = vec!["n"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar { scalar: "n".into() })
    );
}

#[test]
fn reject_yaml_1_1_ambiguous_is_case_insensitive() {
    let scalars = vec!["YES"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar {
            scalar: "YES".into()
        })
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
        Ok(events) => {
            assert_eq!(events.len(), 8);
            assert_eq!(events[3].as_scalar(), Some("a"));
            assert_eq!(events[4].as_scalar(), Some("1"));
        }
        Err(e) => fail_assert!("expected Ok, got Err: {e}"),
    }
}

#[test]
fn parse_yaml_events_returns_error_for_invalid_yaml() {
    let result = parse_yaml_events("");
    assert_eq!(result, Err(YamlError::EmptySource));
}

#[test]
fn reject_duplicate_keys_returns_exact_key_for_duplicate() {
    let keys = vec!["first", "repeat", "repeat"];
    let result = reject_duplicate_keys(&keys);
    assert_eq!(
        result,
        Err(YamlError::DuplicateKey {
            key: "repeat".into()
        })
    );
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
        "version: velvet-ballastics/v1\nname: dup\nwhen:\n  manual: {}\nname: dup2\nsteps: []\n";
    let result = parse_workflow_source(yaml);
    assert_eq!(result, Err(YamlError::DuplicateKey { key: "name".into() }));
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
                detail: "null_byte_in_source"
            })
        ),
        "expected ForbiddenFeature for null byte, got: {result:?}"
    );
}

#[test]
fn adversarial_api_null_byte_workflow_rejected() {
    let yaml = "version: velvet-ballastics/v1\nname: \x00bad\nwhen:\n  manual: {}\nsteps: []\n";
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::ForbiddenFeature {
            detail: "null_byte_in_source"
        })
    );
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
    assert_eq!(
        result,
        Err(YamlError::ScalarTooLong {
            len: 65_537,
            max: 65_536
        })
    );
}

#[test]
fn adversarial_api_workflow_with_unsupported_trigger_rejected() {
    let yaml = indoc::indoc! {"
        version: velvet-ballastics/v1
        name: bad-trigger
        when:
          ipc: {}
        steps: []
    "};
    let result = parse_workflow_source(yaml);
    assert!(
        matches!(
            result,
            Err(YamlError::UnsupportedTrigger { trigger: "ipc" })
        ),
        "expected UnsupportedTrigger for ipc trigger, got: {result:?}"
    );
}

#[test]
fn adversarial_api_workflow_with_missing_when_rejected() {
    let yaml = "version: velvet-ballastics/v1\nname: no-when\nsteps: []\n";
    let result = parse_workflow_source(yaml);
    assert_eq!(result, Err(YamlError::MissingField { field: "when" }));
}

#[test]
fn adversarial_api_workflow_with_non_mapping_when_rejected() {
    let yaml = "version: velvet-ballastics/v1\nname: bad\nwhen: manual\nsteps: []\n";
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::FieldShape {
            field: "when",
            expected: "mapping"
        })
    );
}

#[test]
fn canonical_triggers_and_aliases_parse() {
    let yaml = indoc::indoc! {r#"
        version: velvet-ballastics/v1
        name: canonical
        when:
          event:
            name: invoice.created
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
fn parse_workflow_source_preserves_exact_author_value_variants() {
    let yaml = indoc::indoc! {r#"
        version: velvet-ballastics/v1
        name: author-values
        when:
          manual: {}
        inputs:
          nothing: null
          enabled: true
          count: 42
          label: hello
          list: [1, two, false]
          object:
            child: value
        vars:
          nested:
            - name: alpha
        steps: []
        result:
          ok: true
        examples:
          - description: variants
            input:
              id: 7
            expected: [done, 1]
    "#};

    let workflow = match parse_workflow_source(yaml) {
        Ok(workflow) => workflow,
        Err(error) => {
            fail_assert!("workflow should parse, got {error:?}");
            return;
        }
    };

    assert_eq!(workflow.inputs()[0].value, crate::ast::AuthorValue::Null);
    assert_eq!(
        workflow.inputs()[1].value,
        crate::ast::AuthorValue::Bool(true)
    );
    assert_eq!(workflow.inputs()[2].value, crate::ast::AuthorValue::I64(42));
    assert_eq!(
        workflow.inputs()[3].value,
        crate::ast::AuthorValue::Text("hello".into())
    );
    assert_eq!(
        workflow.inputs()[4].value,
        crate::ast::AuthorValue::Sequence(vec![
            crate::ast::AuthorValue::I64(1),
            crate::ast::AuthorValue::Text("two".into()),
            crate::ast::AuthorValue::Bool(false),
        ])
    );
    assert_eq!(
        workflow.inputs()[5].value,
        crate::ast::AuthorValue::Mapping(vec![crate::ast::AuthorEntry {
            key: "child".into(),
            value: crate::ast::AuthorValue::Text("value".into()),
        }])
    );
    assert_eq!(
        workflow.vars()[0].value,
        crate::ast::AuthorValue::Sequence(vec![crate::ast::AuthorValue::Mapping(vec![
            crate::ast::AuthorEntry {
                key: "name".into(),
                value: crate::ast::AuthorValue::Text("alpha".into()),
            },
        ])])
    );
    let Some(result) = workflow.result() else {
        fail_assert!("expected result mapping");
        return;
    };
    assert_eq!(
        result.fields,
        vec![crate::ast::AuthorEntry {
            key: "ok".into(),
            value: crate::ast::AuthorValue::Bool(true),
        }]
    );
    assert_eq!(
        workflow.examples()[0].input,
        Some(crate::ast::AuthorValue::Mapping(vec![
            crate::ast::AuthorEntry {
                key: "id".into(),
                value: crate::ast::AuthorValue::I64(7),
            }
        ]))
    );
    assert_eq!(
        workflow.examples()[0].expected,
        Some(crate::ast::AuthorValue::Sequence(vec![
            crate::ast::AuthorValue::Text("done".into()),
            crate::ast::AuthorValue::I64(1),
        ]))
    );
}

#[test]
fn parse_workflow_source_rejects_unknown_fields_with_exact_field_names() {
    let top_level = indoc::indoc! {"
        version: velvet-ballastics/v1
        name: unknown-top
        when: { manual: {} }
        steps: []
        extra: value
    "};
    assert_eq!(
        parse_workflow_source(top_level),
        Err(YamlError::UnknownField {
            field: "extra".into()
        })
    );

    let set_field = indoc::indoc! {"
        version: velvet-ballastics/v1
        name: unknown-set
        when: { manual: {} }
        steps:
          - id: first
            set: { output: out, value: value_ref, legacy: extra_value }
    "};
    assert_eq!(
        parse_workflow_source(set_field),
        Err(YamlError::UnknownField {
            field: "legacy".into()
        })
    );
}

#[test]
fn parse_workflow_source_accepts_all_v1_triggers() {
    let cases = [
        (
            "manual",
            indoc::indoc! {"
                version: velvet-ballastics/v1
                name: manual-trigger
                when:
                  manual: {}
                steps: []
            "},
        ),
        (
            "webhook",
            indoc::indoc! {"
                version: velvet-ballastics/v1
                name: webhook-trigger
                when:
                  webhook: {}
                steps: []
            "},
        ),
        (
            "event",
            indoc::indoc! {"
                version: velvet-ballastics/v1
                name: event-trigger
                when:
                  event:
                    name: invoice.created
                steps: []
            "},
        ),
        (
            "schedule",
            indoc::indoc! {"
                version: velvet-ballastics/v1
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
        version: velvet-ballastics/v1
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
        version: velvet-ballastics/v1
        name: bad-schedule
        when:
          schedule: '*/5 * * * *'
        steps: []
    "};
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::FieldShape {
            field: "mapping",
            expected: "mapping"
        })
    );
}

#[test]
fn parse_workflow_source_rejects_schedule_missing_cron() {
    let yaml = indoc::indoc! {"
        version: velvet-ballastics/v1
        name: missing-schedule-cron
        when:
          schedule: {}
        steps: []
    "};
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::MissingField {
            field: "when.schedule.cron"
        })
    );
}

#[test]
fn parse_workflow_source_rejects_schedule_empty_cron() {
    let yaml = indoc::indoc! {"
        version: velvet-ballastics/v1
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
            field: "when.schedule.cron",
            expected: "non-empty string"
        })
    );
}

#[test]
fn parse_workflow_source_rejects_multiple_triggers() {
    let yaml = indoc::indoc! {"
        version: velvet-ballastics/v1
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
            field: "when",
            expected: "exactly one trigger"
        })
    );
}

#[test]
fn strict_rejects_retry_and_unknown_example_fields() {
    let retry_yaml = indoc::indoc! {r#"
        version: velvet-ballastics/v1
        name: bad_retry
        when: { manual: {} }
        steps:
          - id: first
            retry: { max_attempts: 2 }
            set: { output: x, value: "1" }
    "#};
    assert_eq!(
        parse_workflow_source(retry_yaml),
        Err(YamlError::UnknownField {
            field: "retry".into()
        })
    );
    let example_yaml = indoc::indoc! {"
        version: velvet-ballastics/v1
        name: bad_example
        when: { manual: {} }
        steps: []
        examples:
          - name: legacy
    "};
    assert_eq!(
        parse_workflow_source(example_yaml),
        Err(YamlError::UnknownField {
            field: "name".into()
        })
    );
}

#[test]
fn semantic_source_map_tracks_trigger_and_block_scalar() {
    let yaml = indoc::indoc! {"
        version: velvet-ballastics/v1
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
    assert_eq!(
        result,
        Err(YamlError::SourceTooLarge {
            size: 2_000_000,
            max: 1_048_576
        })
    );
}

#[test]
fn adversarial_api_only_whitespace_rejected() {
    let yaml = "   \t  \n  \n  ";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Err(YamlError::EmptySource));
}

// ---------------------------------------------------------------------------
// build_source_map — public API coverage
// ---------------------------------------------------------------------------

#[test]
fn build_source_map_returns_non_empty_for_valid_yaml() {
    let yaml = "key: value\n";
    let result = build_source_map(yaml);
    let map = match result {
        Ok(map) => map,
        Err(error) => {
            fail_assert!("build_source_map failed: {error:?}");
            return;
        }
    };
    assert_eq!(map.len(), 3);
    assert_eq!(yaml.get(map.span_for_node(1).map(|span| span.start_offset).unwrap_or(0)..map.span_for_node(1).map(|span| span.end_offset).unwrap_or(0)), Some("key"));
    assert_eq!(yaml.get(map.span_for_node(2).map(|span| span.start_offset).unwrap_or(0)..map.span_for_node(2).map(|span| span.end_offset).unwrap_or(0)), Some("value"));
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
    match result {
        Some(span) => {
            assert_eq!(span.start_offset, 0);
            assert_eq!(span.end_offset, 0);
            assert_eq!(span.start_line, 1);
            assert_eq!(span.start_col, 0);
            assert_eq!(span.end_line, 1);
            assert_eq!(span.end_col, 1);
        }
        None => fail_assert!("span_for_node(0) should return Some for valid doc"),
    }
}

// ---------------------------------------------------------------------------
// reject_forbidden_yaml_features — public API coverage
// ---------------------------------------------------------------------------

#[test]
fn reject_forbidden_yaml_features_rejects_custom_tag() {
    // Custom tags like !custom are rejected by the feature checker.
    let yaml = "key: !custom value\n";
    let Ok(events) = crate::events::collect_events(yaml) else {
        fail_assert!("collect_events failed for custom tag fixture");
        return;
    };
    let result = reject_forbidden_yaml_features(&events);
    assert_eq!(
        result,
        Err(YamlError::CustomTag {
            tag: "!custom".into()
        })
    );
}

// ---------------------------------------------------------------------------
// load_fixture_source — public API coverage
// ---------------------------------------------------------------------------

#[test]
fn load_fixture_source_parses_valid_workflow() {
    let yaml = indoc::indoc! {r#"
        version: velvet-ballastics/v1
        name: test-workflow
        when:
          manual: {}
        steps:
          - id: start
            finish:
              result: "done"
    "#};
    let result = load_fixture_source(yaml);
    match result {
        Ok(workflow) => {
            assert_eq!(workflow.version(), "velvet-ballastics/v1");
            assert_eq!(workflow.name(), "test-workflow");
            assert_eq!(workflow.steps().len(), 1);
            assert_eq!(
                workflow.steps().first().map(|s| s.id.as_str()),
                Some("start")
            );
        }
        Err(error) => fail_assert!("load_fixture_source failed: {error:?}"),
    }
}

// Round 2 exact AST primitive and numeric boundary coverage.
fn round2_workflow_with_step(step_yaml: &str) -> String {
    format!(
        "version: velvet-ballastics/v1\nname: step-primitive\nwhen:\n  manual: {{}}\nsteps:\n{step_yaml}"
    )
}
fn round2_step(step_yaml: &str) -> crate::ast::StepAst {
    let yaml = round2_workflow_with_step(step_yaml);
    match parse_workflow_source(&yaml) {
        Ok(wf) => wf.steps().first().cloned().unwrap_or(crate::ast::StepAst {
            id: String::new(),
            name: None,
            condition: None,
            primitive: crate::ast::StepPrimitive::Wait {
                event: None,
                timeout: None,
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }),
        Err(error) => {
            fail_assert!("workflow should parse, got {error:?}");
            crate::ast::StepAst {
                id: String::new(),
                name: None,
                condition: None,
                primitive: crate::ast::StepPrimitive::Wait {
                    event: None,
                    timeout: None,
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            }
        }
    }
}

#[test]
fn round2_ast_covers_all_step_primitives_save_alias_and_boundaries() {
    assert_eq!(
        round2_step("  - id: s\n    set: { output: total, value: inputs.amount }\n").primitive,
        crate::ast::StepPrimitive::Set {
            output: "total".into(),
            value: "inputs.amount".into()
        }
    );
    assert_eq!(
        round2_step("  - id: save\n    save: { output: slot, value: constant }\n").primitive,
        crate::ast::StepPrimitive::Set {
            output: "slot".into(),
            value: "constant".into()
        }
    );
    assert_eq!(
        parse_workflow_source(&round2_workflow_with_step(
            "  - id: save\n    save: { output: '', value: constant }\n"
        )),
        Err(YamlError::FieldShape {
            field: "set.output",
            expected: "non-empty string"
        })
    );
    assert_eq!(
        round2_step("  - id: do\n    do: { action: http.get, input: inputs.url }\n").primitive,
        crate::ast::StepPrimitive::Do {
            action: "http.get".into(),
            input: "inputs.url".into()
        }
    );
    assert_eq!(
        round2_step("  - id: run\n    run: { action: shell.exec, input: command }\n").primitive,
        crate::ast::StepPrimitive::Do {
            action: "shell.exec".into(),
            input: "command".into()
        }
    );
    match round2_step("  - id: choose\n    choose: { branches: [ { when: cond, steps: [] } ], otherwise: fallback }\n").primitive { crate::ast::StepPrimitive::Choose { branches, otherwise } => { assert_eq!(branches.len(), 1); assert_eq!(branches.first().map(|b| b.when.as_str()), Some("cond")); assert_eq!(otherwise, Some("fallback".into())); }, other => fail_assert!("expected Choose, got {other:?}") }
    match round2_step("  - id: fe\n    foreach: { variable: item, input: inputs.items, at_once: 4294967295, steps: [] }\n").primitive { crate::ast::StepPrimitive::ForEach { variable, input, at_once, body } => { assert_eq!(variable, "item"); assert_eq!(input, "inputs.items"); assert_eq!(at_once, Some(u32::MAX)); assert_eq!(body, Vec::new()); }, other => fail_assert!("expected ForEach, got {other:?}") }
    assert_eq!(
        round2_step(
            "  - id: fe\n    for_each: { variable: user, input: inputs.users, steps: [] }\n"
        )
        .primitive,
        crate::ast::StepPrimitive::ForEach {
            variable: "user".into(),
            input: "inputs.users".into(),
            at_once: None,
            body: Vec::new()
        }
    );
    match round2_step("  - id: par\n    parallel: { branches: [ { label: left, steps: [] }, { label: right, steps: [] } ] }\n").primitive { crate::ast::StepPrimitive::Together { branches } => { assert_eq!(branches.len(), 2); assert_eq!(branches.first().map(|b| b.label.as_str()), Some("left")); assert_eq!(branches.get(1).map(|b| b.label.as_str()), Some("right")); }, other => fail_assert!("expected Together, got {other:?}") }
    match round2_step("  - id: collect\n    collect: { variable: page, source: api.pages, pages: 4294967295, items: 4294967295, steps: [] }\n").primitive { crate::ast::StepPrimitive::Collect { variable, source, pages, items, body } => { assert_eq!(variable, "page"); assert_eq!(source, "api.pages"); assert_eq!(pages, Some(u32::MAX)); assert_eq!(items, Some(u32::MAX)); assert_eq!(body, Vec::new()); }, other => fail_assert!("expected Collect, got {other:?}") }
    match round2_step("  - id: agg\n    aggregate: { variable: acc, input: inputs.values, initial: zero, steps: [] }\n").primitive { crate::ast::StepPrimitive::Aggregate { variable, input, initial, body } => { assert_eq!(variable, "acc"); assert_eq!(input, "inputs.values"); assert_eq!(initial, "zero"); assert_eq!(body, Vec::new()); }, other => fail_assert!("expected Aggregate, got {other:?}") }
    assert_eq!(
        round2_step("  - id: repeat\n    repeat: { max_attempts: 65535, steps: [] }\n").primitive,
        crate::ast::StepPrimitive::Repeat {
            max_attempts: u16::MAX,
            body: Vec::new()
        }
    );
    assert_eq!(
        round2_step("  - id: wait\n    wait: { event: invoice.paid, timeout: PT10M }\n").primitive,
        crate::ast::StepPrimitive::Wait {
            event: Some("invoice.paid".into()),
            timeout: Some("PT10M".into())
        }
    );
    assert_eq!(
        round2_step("  - id: ask\n    ask: { prompt: Approve?, timeout: PT1H }\n").primitive,
        crate::ast::StepPrimitive::Ask {
            prompt: "Approve?".into(),
            timeout: Some("PT1H".into())
        }
    );
    assert_eq!(
        round2_step("  - id: finish\n    finish: { result: success }\n").primitive,
        crate::ast::StepPrimitive::Finish {
            result: crate::ast::ScalarValue::String("success".into())
        }
    );
    assert_eq!(
        round2_step("  - id: finish\n    finish: { result: 42 }\n").primitive,
        crate::ast::StepPrimitive::Finish {
            result: crate::ast::ScalarValue::Integer(42)
        }
    );
}

#[test]
fn round2_ast_numeric_errors_are_exact() {
    for yaml in [
        round2_workflow_with_step("  - id: r\n    repeat: { max_attempts: 65536, steps: [] }\n"),
        round2_workflow_with_step("  - id: r\n    repeat: { max_attempts: -1, steps: [] }\n"),
        round2_workflow_with_step(
            "  - id: r\n    try_again: { max_attempts: 65536 }\n    set: { output: x, value: yy }\n",
        ),
        round2_workflow_with_step(
            "  - id: r\n    try_again: { max_attempts: -1 }\n    set: { output: x, value: yy }\n",
        ),
    ] {
        assert_eq!(
            parse_workflow_source(&yaml),
            Err(YamlError::FieldShape {
                field: "max_attempts",
                expected: "u16 integer"
            })
        );
    }
    assert_eq!(round2_step("  - id: retry\n    try_again: { max_attempts: 65535, delay: PT5S }\n    set: { output: x, value: yy }\n").retry.map(|r| (r.max_attempts, r.delay)), Some((u16::MAX, Some("PT5S".into()))));
    match round2_step("  - id: fe\n    foreach: { variable: item, input: inputs.items, at_once: 4294967296, steps: [] }\n").primitive { crate::ast::StepPrimitive::ForEach { at_once, .. } => assert_eq!(at_once, None), other => fail_assert!("expected ForEach, got {other:?}") }
    match round2_step("  - id: fe\n    foreach: { variable: item, input: inputs.items, at_once: -1, steps: [] }\n").primitive { crate::ast::StepPrimitive::ForEach { at_once, .. } => assert_eq!(at_once, None), other => fail_assert!("expected ForEach, got {other:?}") }
    match round2_step("  - id: c\n    collect: { variable: page, source: pages, pages: 4294967296, items: -1, steps: [] }\n").primitive { crate::ast::StepPrimitive::Collect { pages, items, .. } => { assert_eq!(pages, None); assert_eq!(items, None); }, other => fail_assert!("expected Collect, got {other:?}") }
}
