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
    let yaml = indoc::indoc! {"
        version: velvet-ballastics/v1
        name: t
        when:
          http: {}
        steps: []
    "};
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::UnsupportedFeature {
            feature: "http trigger"
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
    match result {
        Err(YamlError::CustomTag { tag }) => {
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
    let result = parse_workflow_source(yaml);
    match result {
        Ok(wf) => {
            assert_eq!(wf.name, "minimal");
            assert_eq!(wf.steps.len(), 1);
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
            assert_eq!(wf.version, "velvet-ballastics/v1");
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
            assert_eq!(wf.steps.len(), 3);
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
        Ok(wf) => assert_eq!(wf.name, "fixture"),
        Err(e) => fail_assert!("expected Ok, got Err: {e}"),
    }
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
    let yaml = "version: velvet-ballastics/v1\nname: dup\nwhen:\n  manual: {}\nname: dup2\nsteps: []\n";
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
fn adversarial_api_workflow_with_unknown_trigger_field_rejected() {
    let yaml = indoc::indoc! {"
        version: velvet-ballastics/v1
        name: bad-trigger
        when:
          webhook: {}
        steps: []
    "};
    let result = parse_workflow_source(yaml);
    assert!(
        matches!(result, Err(YamlError::FieldShape { .. })),
        "expected FieldShape for unknown trigger, got: {result:?}"
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
    assert!(result.is_err(), "expected error for non-mapping when");
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
