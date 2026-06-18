#![forbid(unsafe_code)]

//! Production-bound replacement properties for the retired `vb_yaml` Verus
//! mirror specs.
//!
//! Obligations: `P-EMPTY-BODY` from
//! `verification/verus/proof-obligations.planned.jsonl`, plus replacement
//! obligations `RPO-YAML-001`, `RPO-YAML-002`, and `RPO-YAML-003` from
//! `.beads/vb-dzibx/replacement-proof-obligations.planned.jsonl`.
//!
//! These tests intentionally call the public `vb_yaml` production APIs instead
//! of recreating the retired mirror predicates.  The finite case-permutation
//! test is complete for YAML 1.1 ambiguous boolean words because production
//! lowercases ASCII and the only finite alternatives for each ASCII alphabetic
//! byte are its lower/upper forms.

use std::collections::BTreeSet;

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use vb_yaml::events::{self, EventSpan, ScalarStyle, YamlEvent};
use vb_yaml::{profile, YamlError};

fn yaml_plain_key_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z_][A-Za-z0-9_]{0,8}"
}

fn non_core_tag_suffix_strategy() -> impl Strategy<Value = String> {
    "(binary|timestamp|merge|custom[A-Za-z0-9_]{0,6})"
}

fn event_span() -> EventSpan {
    EventSpan {
        start: 0,
        end: 1,
        line: 1,
        column: 1,
    }
}

fn scalar_event_with_tag(tag: String) -> YamlEvent {
    YamlEvent::Scalar {
        value: "value".into(),
        style: ScalarStyle::Plain,
        anchor_id: 0,
        tag: Some(tag.into_boxed_str()),
        span: event_span(),
    }
}

fn ascii_case_permutations(word: &str) -> Vec<String> {
    let mut variants = vec![String::new()];
    for byte in word.bytes() {
        let lower = char::from(byte.to_ascii_lowercase());
        let upper = char::from(byte.to_ascii_uppercase());
        let mut next = Vec::new();
        for prefix in &variants {
            let mut lower_variant = prefix.clone();
            lower_variant.push(lower);
            next.push(lower_variant);

            let mut upper_variant = prefix.clone();
            upper_variant.push(upper);
            next.push(upper_variant);
        }
        variants = next;
    }
    variants
}

fn collect_events_or_fail(source: &str) -> Result<Vec<YamlEvent>, TestCaseError> {
    events::collect_events(source).map_err(|error| {
        TestCaseError::fail(format!(
            "generated YAML did not parse: {error:?}; source={source:?}"
        ))
    })
}

fn is_ambiguous_typed_outcome(result: Result<(), YamlError>) -> bool {
    matches!(result, Ok(()) | Err(YamlError::AmbiguousScalar { .. }))
}

fn is_duplicate_key_for(result: Result<(), YamlError>, expected: &str) -> bool {
    matches!(
        result,
        Err(YamlError::DuplicateKey { key: duplicate }) if duplicate.as_ref() == expected
    )
}

fn is_any_duplicate_key(result: Result<(), YamlError>) -> bool {
    matches!(result, Err(YamlError::DuplicateKey { .. }))
}

fn is_custom_tag(result: Result<(), YamlError>) -> bool {
    matches!(result, Err(YamlError::CustomTag { .. }))
}

#[test]
fn p_empty_body_yaml_11_ambiguous_case_permutations_are_rejected() {
    for word in ["y", "n", "yes", "no", "on", "off"] {
        for variant in ascii_case_permutations(word) {
            let direct_result = vb_yaml::reject_yaml_1_1_ambiguous_scalars(&[variant.as_str()]);
            assert!(
                matches!(
                    direct_result,
                    Err(YamlError::AmbiguousScalar { scalar })
                        if scalar.as_ref() == variant.as_str()
                ),
                "production scalar ambiguity API accepted {variant:?}"
            );

            let yaml = format!("flag: {variant}\n");
            let profile_result = vb_yaml::validate_yaml_profile(&yaml);
            assert!(
                matches!(
                    profile_result,
                    Err(YamlError::AmbiguousScalar { scalar })
                        if scalar.as_ref() == variant.as_str()
                ),
                "production profile validation accepted plain scalar {variant:?}"
            );
        }
    }
}

#[test]
fn p_empty_body_quoted_yaml_11_ambiguous_case_permutations_remain_accepted() {
    for word in ["y", "n", "yes", "no", "on", "off"] {
        for variant in ascii_case_permutations(word) {
            let yaml = format!("flag: '{variant}'\n");
            assert_eq!(vb_yaml::validate_yaml_profile(&yaml), Ok(()));
        }
    }
}

#[test]
fn p_empty_body_rpo_yaml_002_allowed_core_tags_are_accepted() {
    for suffix in ["str", "int", "float", "bool", "null", "seq", "map"] {
        for prefix in ["!!", "tag:yaml.org,2002:"] {
            let tag = format!("{prefix}{suffix}");
            let events = [scalar_event_with_tag(tag)];
            assert_eq!(vb_yaml::reject_forbidden_yaml_features(&events), Ok(()));
        }
    }
}

#[test]
fn p_empty_body_rpo_yaml_002_merge_tags_are_rejected_by_anchor_alias_merge_path() {
    for tag in ["!!merge", "tag:yaml.org,2002:merge"] {
        let events = [scalar_event_with_tag(String::from(tag))];
        assert_eq!(
            profile::reject_anchors_aliases_merges(&events),
            Err(YamlError::AnchorAliasMerge)
        );
    }
}

proptest! {
    #[test]
    fn p_empty_body_ambiguous_scalar_api_is_total_for_bounded_ascii(
        bytes in prop::collection::vec(0u8..=0x7f, 0usize..17usize),
    ) {
        let input = std::str::from_utf8(&bytes).map_err(|error| {
            TestCaseError::fail(format!("ASCII generator produced non-UTF-8: {error}"))
        })?;

        let result = vb_yaml::reject_yaml_1_1_ambiguous_scalars(&[input]);
        let typed = is_ambiguous_typed_outcome(result);
        prop_assert!(typed);
    }

    #[test]
    fn p_empty_body_reject_duplicate_keys_rejects_any_appended_duplicate(
        key in yaml_plain_key_strategy(),
        tail in prop::collection::vec(yaml_plain_key_strategy(), 0usize..8usize),
    ) {
        let mut keys = Vec::new();
        keys.push(key.as_str());
        for entry in &tail {
            keys.push(entry.as_str());
        }
        keys.push(key.as_str());

        let result = vb_yaml::reject_duplicate_keys(&keys);
        let rejected = is_any_duplicate_key(result);
        prop_assert!(rejected);
    }

    #[test]
    fn p_empty_body_rpo_yaml_002_generated_non_core_tags_are_rejected(
        suffix in non_core_tag_suffix_strategy(),
        yaml_org_prefix in any::<bool>(),
    ) {
        let tag = if yaml_org_prefix {
            format!("tag:yaml.org,2002:{suffix}")
        } else {
            format!("!!{suffix}")
        };
        let events = [scalar_event_with_tag(tag)];
        let result = vb_yaml::reject_forbidden_yaml_features(&events);
        let rejected = is_custom_tag(result);

        prop_assert!(rejected);
    }

    #[test]
    fn p_empty_body_reject_duplicate_keys_accepts_generated_unique_sets(
        keys in prop::collection::btree_set(yaml_plain_key_strategy(), 0usize..16usize),
    ) {
        let key_refs: Vec<&str> = keys.iter().map(String::as_str).collect();
        prop_assert_eq!(vb_yaml::reject_duplicate_keys(&key_refs), Ok(()));
    }

    #[test]
    fn p_empty_body_reject_duplicate_mapping_keys_rejects_root_duplicates(
        key in yaml_plain_key_strategy(),
    ) {
        let source = format!("{key}: alpha\n{key}: bravo\n");
        let events = collect_events_or_fail(&source)?;
        let result = profile::reject_duplicate_mapping_keys(&events);

        let rejected = is_duplicate_key_for(result, key.as_str());
        prop_assert!(rejected);
    }

    #[test]
    fn p_empty_body_validate_yaml_profile_rejects_root_duplicates(
        key in yaml_plain_key_strategy(),
    ) {
        let source = format!("{key}: alpha\n{key}: bravo\n");
        let result = vb_yaml::validate_yaml_profile(&source);

        let rejected = is_duplicate_key_for(result, key.as_str());
        prop_assert!(rejected);
    }

    #[test]
    fn p_empty_body_reject_duplicate_mapping_keys_rejects_nested_duplicates(
        key in yaml_plain_key_strategy(),
    ) {
        let source = format!("outer:\n  {key}: alpha\n  {key}: bravo\n");
        let events = collect_events_or_fail(&source)?;
        let result = profile::reject_duplicate_mapping_keys(&events);

        let rejected = is_duplicate_key_for(result, key.as_str());
        prop_assert!(rejected);
    }

    #[test]
    fn p_empty_body_reject_duplicate_mapping_keys_scopes_nested_frames(
        key in yaml_plain_key_strategy(),
    ) {
        let source = format!("left:\n  {key}: alpha\nright:\n  {key}: bravo\n");
        let events = collect_events_or_fail(&source)?;
        let result = profile::reject_duplicate_mapping_keys(&events);

        prop_assert_eq!(result, Ok(()));
    }

    #[test]
    fn p_empty_body_unique_btree_set_matches_direct_duplicate_key_api(
        keys in prop::collection::btree_set(yaml_plain_key_strategy(), 1usize..12usize),
    ) {
        let key_refs: BTreeSet<&str> = keys.iter().map(String::as_str).collect();
        let ordered_refs: Vec<&str> = key_refs.iter().copied().collect();

        prop_assert_eq!(vb_yaml::reject_duplicate_keys(&ordered_refs), Ok(()));
    }
}
