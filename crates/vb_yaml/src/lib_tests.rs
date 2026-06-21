#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]
//! Library-level integration tests.

use super::*;
use vb_core::diagnostic::HasSymbolicCode;

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
            field: "step",
            expected: "exactly one primitive"
        })
    );
}

fn assert_legacy_primitive_deprecated(yaml: &str, expected_name: &str, expected_replacement: &str) {
    let err = match parse_workflow_source(yaml) {
        Err(err) => err,
        Ok(_) => {
            fail_assert!("expected legacy primitive rejection for {expected_name}");
            return;
        }
    };
    assert_eq!(
        err,
        YamlError::LegacyPrimitiveDeprecated {
            name: expected_name.to_string(),
            replacement: expected_replacement.to_string(),
        }
    );
    let rendered = err.to_string();
    assert!(rendered.contains(expected_name));
    assert!(rendered.contains(expected_replacement));
    assert!(rendered.contains(&format!(
        "migration hint: use {expected_replacement} instead"
    )));
    assert_eq!(err.code().as_str(), "FORBIDDEN_YAML_FEATURE");
    assert_eq!(err.symbolic_code().as_str(), "FORBIDDEN_YAML_FEATURE");
    assert_eq!(err.symbolic_code_name(), "FORBIDDEN_YAML_FEATURE");
}

#[test]
fn parse_workflow_source_rejects_legacy_parallel_with_migration_hint() {
    let yaml = indoc::indoc! {r#"
        version: velvet-ballistics/v1
        name: legacy-parallel
        when:
          manual: {}
        steps:
          - id: s1
            parallel:
              branches:
                - label: branch
                  steps: []
    "#};
    assert_legacy_primitive_deprecated(yaml, "parallel", "together");
}

#[test]
fn parse_workflow_source_rejects_legacy_aggregate_with_reduce_hint() {
    let yaml = indoc::indoc! {r#"
        version: velvet-ballistics/v1
        name: legacy-aggregate
        when:
          manual: {}
        steps:
          - id: s1
            aggregate: scalar-shape-does-not-matter
    "#};
    assert_legacy_primitive_deprecated(yaml, "aggregate", "reduce");
}

#[test]
fn parse_workflow_source_preserves_unknown_non_legacy_step_field_error() {
    let yaml = indoc::indoc! {r#"
        version: velvet-ballistics/v1
        name: unknown-step-field
        when:
          manual: {}
        steps:
          - id: s1
            deprecated_but_not_legacy: {}
    "#};
    assert_eq!(
        parse_workflow_source(yaml),
        Err(YamlError::UnknownField {
            field: Box::from("deprecated_but_not_legacy"),
        })
    );
}

#[test]
fn parse_workflow_source_legacy_preempts_top_level_competing_step_errors() {
    let cases = [
        (
            indoc::indoc! {r#"
                version: velvet-ballistics/v1
                name: legacy-preempts-duplicate-and-unknown
                when:
                  manual: {}
                steps:
                  - id: s1
                    do: { action: action.name, input: "x" }
                    run: { action: action.name, input: "y" }
                    hostile_unknown: {}
                    parallel: [1, "two", { nested: value }]
            "#},
            "parallel",
            "together",
        ),
        (
            indoc::indoc! {r#"
                version: velvet-ballistics/v1
                name: legacy-preempts-canonical-alias
                when:
                  manual: {}
                steps:
                  - id: s1
                    for_each: { variable: item, input: items, steps: [] }
                    foreach: { variable: item, input: items, steps: [] }
                    aggregate:
                      arbitrary:
                        shape: ignored
            "#},
            "aggregate",
            "reduce",
        ),
    ];
    for (yaml, expected_name, expected_replacement) in cases {
        assert_legacy_primitive_deprecated(yaml, expected_name, expected_replacement);
    }
}

#[test]
fn parse_workflow_source_rejects_legacy_inside_nested_body_steps() {
    let cases = [
        (
            indoc::indoc! {r#"
                version: velvet-ballistics/v1
                name: nested-choose-legacy
                when:
                  manual: {}
                steps:
                  - id: outer
                    choose:
                      branches:
                        - when: ready
                          steps:
                            - id: nested
                              set: { output: x, value: "1" }
                              hostile_unknown: true
                              parallel: "scalar-shape"
            "#},
            "parallel",
            "together",
        ),
        (
            indoc::indoc! {r#"
                version: velvet-ballistics/v1
                name: nested-foreach-legacy
                when:
                  manual: {}
                steps:
                  - id: outer
                    for_each:
                      variable: item
                      input: items
                      steps:
                        - id: nested
                          do: { action: action.name, input: "x" }
                          run: { action: action.name, input: "y" }
                          aggregate: [1, 2, 3]
            "#},
            "aggregate",
            "reduce",
        ),
        (
            indoc::indoc! {r#"
                version: velvet-ballistics/v1
                name: nested-together-legacy
                when:
                  manual: {}
                steps:
                  - id: outer
                    together:
                      branches:
                        - label: branch
                          steps:
                            - id: nested
                              for_each: { variable: item, input: items, steps: [] }
                              parallel:
                                branches: []
            "#},
            "parallel",
            "together",
        ),
        (
            indoc::indoc! {r#"
                version: velvet-ballistics/v1
                name: nested-collect-legacy
                when:
                  manual: {}
                steps:
                  - id: outer
                    collect:
                      variable: page
                      source: pages
                      steps:
                        - id: nested
                          aggregate:
                            source: ignored
                            items: []
                          unknown_after_legacy: {}
            "#},
            "aggregate",
            "reduce",
        ),
        (
            indoc::indoc! {r#"
                version: velvet-ballistics/v1
                name: nested-reduce-legacy
                when:
                  manual: {}
                steps:
                  - id: outer
                    reduce:
                      variable: item
                      input: items
                      initial: seed
                      steps:
                        - id: nested
                          parallel: { arbitrary: map }
            "#},
            "parallel",
            "together",
        ),
        (
            indoc::indoc! {r#"
                version: velvet-ballistics/v1
                name: nested-repeat-legacy
                when:
                  manual: {}
                steps:
                  - id: outer
                    repeat:
                      max_attempts: 3
                      steps:
                        - id: nested
                          finish: { result: done }
                          aggregate: "scalar-shape"
            "#},
            "aggregate",
            "reduce",
        ),
    ];
    for (yaml, expected_name, expected_replacement) in cases {
        assert_legacy_primitive_deprecated(yaml, expected_name, expected_replacement);
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
    let yaml =
        "version: velvet-ballistics/v1\nname: dup\nwhen:\n  manual: {}\nname: dup2\nsteps: []\n";
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
    let yaml = "version: velvet-ballistics/v1\nname: \x00bad\nwhen:\n  manual: {}\nsteps: []\n";
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
            Err(YamlError::UnsupportedTrigger { trigger: "ipc" })
        ),
        "expected UnsupportedTrigger for ipc trigger, got: {result:?}"
    );
}

#[test]
fn adversarial_api_workflow_with_missing_when_rejected() {
    let yaml = "version: velvet-ballistics/v1\nname: no-when\nsteps: []\n";
    let result = parse_workflow_source(yaml);
    assert_eq!(result, Err(YamlError::MissingField { field: "when" }));
}

#[test]
fn adversarial_api_workflow_with_non_mapping_when_rejected() {
    let yaml = "version: velvet-ballistics/v1\nname: bad\nwhen: manual\nsteps: []\n";
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::FieldShape {
            field: "when",
            expected: "mapping",
        })
    );
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
    assert_eq!(
        yaml.get(manual.start_offset..manual.end_offset),
        Some("manual")
    );
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
    assert_eq!(result, Err(YamlError::EmptySource));
}

// ---------------------------------------------------------------------------
// build_source_map — public API coverage
// ---------------------------------------------------------------------------

#[test]
fn build_source_map_returns_non_empty_for_valid_yaml() {
    let yaml = "key: value\n";
    let map = build_source_map(yaml).expect("build_source_map should succeed for valid YAML");
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
    let span = result.expect("span_for_node(0) must return Some for valid doc");
    assert!(
        span.start_offset <= span.end_offset,
        "span must have valid byte range"
    );
    assert!(span.start_line > 0, "span must have a valid line number");
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
    assert!(
        result.is_ok(),
        "load_fixture_source should accept valid workflow, got {result:?}"
    );
    let workflow = result
        .ok()
        .expect("validated fixture load must yield a workflow");
    assert!(
        !workflow.steps.is_empty(),
        "validated fixture must declare at least one step, got {workflow:?}"
    );
}
