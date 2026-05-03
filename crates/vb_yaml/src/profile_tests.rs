//! Profile module tests.

use crate::profile_validation::{
    validate_yaml_profile, validate_yaml_profile_with_limits, check_source_size,
    reject_forbidden_features, reject_anchors_aliases_merges, reject_multiple_documents,
    reject_yaml_1_1_ambiguous_scalars,
};
use crate::profile_dupkeys::reject_duplicate_keys;
use crate::{YamlError, YamlLimits};

fn assertion_failed(_message: std::fmt::Arguments<'_>) -> bool {
    false
}

macro_rules! fail_assert {
    ($($arg:tt)*) => {
        assert!(assertion_failed(format_args!($($arg)*)), $($arg)*)
    };
}

#[test]
fn empty_source_rejected() {
    let result = validate_yaml_profile("");
    assert!(matches!(result, Err(YamlError::EmptySource)));
}

#[test]
fn single_document_accepted() {
    let result = validate_yaml_profile("a: 1\n");
    assert_eq!(result, Ok(()));
}

#[test]
fn multiple_documents_rejected() {
    let yaml = "---\na: 1\n---\nb: 2\n";
    let result = validate_yaml_profile(yaml);
    assert!(matches!(result, Err(YamlError::MultipleDocuments { .. })));
}

#[test]
fn anchor_rejected() {
    let yaml = "a: &anchor value\nb: *anchor\n";
    let result = validate_yaml_profile(yaml);
    assert!(matches!(result, Err(YamlError::AnchorAliasMerge)));
}

#[test]
fn ambiguous_yes_rejected() {
    let result = validate_yaml_profile("flag: yes\n");
    assert!(matches!(result, Err(YamlError::AmbiguousScalar { .. })));
}

#[test]
fn quoted_yes_accepted() {
    let yaml = "flag: 'yes'\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Ok(()));
}

#[test]
fn true_false_accepted() {
    let yaml = "flag: true\nother: false\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Ok(()));
}

#[test]
fn reject_duplicate_keys_finds_dup() {
    let keys = vec!["a", "b", "a"];
    let result = reject_duplicate_keys(&keys);
    assert!(matches!(result, Err(YamlError::DuplicateKey { key }) if key.as_ref() == "a"));
}

#[test]
fn strict_profile_rejects_duplicate_top_level_key() {
    let yaml = "version: velvet-ballastics/v1\nname: first\nname: second\nwhen:\n  manual: {}\nsteps: []\n";
    let result = validate_yaml_profile(yaml);
    assert!(matches!(result, Err(YamlError::DuplicateKey { key }) if key.as_ref() == "name"));
}

#[test]
fn strict_profile_rejects_duplicate_nested_key() {
    let yaml = "version: velvet-ballastics/v1\nname: wf\nwhen:\n  ipc:\n    name: a\n    name: b\nsteps: []\n";
    let result = validate_yaml_profile(yaml);
    assert!(matches!(result, Err(YamlError::DuplicateKey { key }) if key.as_ref() == "name"));
}

#[test]
fn reject_duplicate_keys_allows_unique() {
    let keys = vec!["a", "b", "c"];
    assert_eq!(reject_duplicate_keys(&keys), Ok(()));
}

#[test]
fn depth_limit_enforced() {
    let mut yaml = String::from("a:\n");
    for i in 0..70 {
        let indent = "  ".repeat(i);
        yaml.push_str(&format!("{indent}b:\n"));
    }
    let limits = YamlLimits {
        max_depth: 10,
        ..YamlLimits::default()
    };
    let result = validate_yaml_profile_with_limits(&yaml, &limits);
    assert!(matches!(result, Err(YamlError::NestingTooDeep { .. })));
}

#[test]
fn source_too_large_rejected() {
    let big = "x".repeat(2_000_000);
    let limits = YamlLimits {
        max_source_bytes: 1_000_000,
        ..YamlLimits::default()
    };
    let result = validate_yaml_profile_with_limits(&big, &limits);
    assert!(matches!(result, Err(YamlError::SourceTooLarge { .. })));
}

#[test]
fn scalar_too_long_rejected() {
    let long_scalar = "x".repeat(100);
    let yaml = format!("key: \"{long_scalar}\"\n");
    let limits = YamlLimits {
        max_scalar_bytes: 50,
        ..YamlLimits::default()
    };
    let result = validate_yaml_profile_with_limits(&yaml, &limits);
    assert!(matches!(result, Err(YamlError::ScalarTooLong { .. })));
}

// -----------------------------------------------------------------------
// Profile exact-assertion tests
// -----------------------------------------------------------------------

#[test]
fn empty_source_returns_empty_source_error() {
    let result = validate_yaml_profile("");
    assert_eq!(result, Err(YamlError::EmptySource));
}

#[test]
fn single_document_accepted_exact() {
    let yaml = "a: 1\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Ok(()));
}

#[test]
fn multiple_documents_returns_exact_count() {
    let yaml = "---\na: 1\n---\nb: 2\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Err(YamlError::MultipleDocuments { count: 2 }));
}

#[test]
fn anchor_rejected_exact() {
    let yaml = "a: &anc value\nb: *anc\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Err(YamlError::AnchorAliasMerge));
}

#[test]
fn ambiguous_yes_rejected_exact() {
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
fn ambiguous_no_rejected_exact() {
    let yaml = "flag: no\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar {
            scalar: "no".into()
        })
    );
}

#[test]
fn ambiguous_on_rejected_exact() {
    let yaml = "flag: on\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar {
            scalar: "on".into()
        })
    );
}

#[test]
fn ambiguous_off_rejected_exact() {
    let yaml = "flag: off\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar {
            scalar: "off".into()
        })
    );
}

#[test]
fn quoted_yes_accepted_exact() {
    let yaml = "flag: 'yes'\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Ok(()));
}

#[test]
fn true_false_accepted_exact() {
    let yaml = "flag: true\nother: false\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Ok(()));
}

#[test]
fn depth_limit_exact_values() {
    let mut yaml = String::from("a:\n");
    for i in 0..15 {
        let indent = "  ".repeat(i);
        yaml.push_str(&format!("{indent}b:\n"));
    }
    let limits = YamlLimits {
        max_depth: 10,
        ..YamlLimits::default()
    };
    let result = validate_yaml_profile_with_limits(&yaml, &limits);
    match result {
        Err(YamlError::NestingTooDeep { depth, max }) => {
            assert!(depth > 10);
            assert_eq!(max, 10);
        }
        other => fail_assert!("expected NestingTooDeep, got {other:?}"),
    }
}

#[test]
fn source_too_large_exact_values() {
    let big = "x".repeat(200);
    let limits = YamlLimits {
        max_source_bytes: 100,
        ..YamlLimits::default()
    };
    let result = validate_yaml_profile_with_limits(&big, &limits);
    assert_eq!(
        result,
        Err(YamlError::SourceTooLarge {
            size: 200,
            max: 100
        })
    );
}

#[test]
fn scalar_too_long_exact_values() {
    let long_scalar = "x".repeat(100);
    let yaml = format!("key: \"{long_scalar}\"\n");
    let limits = YamlLimits {
        max_scalar_bytes: 50,
        ..YamlLimits::default()
    };
    let result = validate_yaml_profile_with_limits(&yaml, &limits);
    match result {
        Err(YamlError::ScalarTooLong { len, max }) => {
            assert!(len > 50);
            assert_eq!(max, 50);
        }
        other => fail_assert!("expected ScalarTooLong, got {other:?}"),
    }
}

#[test]
fn node_limit_exceeded_exact_values() {
    let mut yaml = String::from("root:\n");
    for i in 0..20 {
        yaml.push_str(&format!("  key{i}: val{i}\n"));
    }
    let limits = YamlLimits {
        max_nodes: 5,
        ..YamlLimits::default()
    };
    let result = validate_yaml_profile_with_limits(&yaml, &limits);
    match result {
        Err(YamlError::NodeLimitExceeded { count, max }) => {
            assert!(count > 5);
            assert_eq!(max, 5);
        }
        other => fail_assert!("expected NodeLimitExceeded, got {other:?}"),
    }
}

#[test]
fn reject_duplicate_keys_returns_exact_key() {
    let keys = vec!["a", "b", "a"];
    let result = reject_duplicate_keys(&keys);
    assert_eq!(result, Err(YamlError::DuplicateKey { key: "a".into() }));
}

#[test]
fn reject_duplicate_keys_allows_unique_exact() {
    let keys = vec!["a", "b", "c"];
    let result = reject_duplicate_keys(&keys);
    assert_eq!(result, Ok(()));
}

#[test]
fn reject_forbidden_features_rejects_custom_tag() {
    let yaml = "key: !mytag value\n";
    let Ok(events) = crate::events::collect_events(yaml) else {
        fail_assert!("collect_events failed");
        return;
    };
    let result = reject_forbidden_features(&events);
    match result {
        Err(YamlError::CustomTag { tag }) => {
            assert!(tag.contains("mytag"), "expected 'mytag' in tag, got: {tag}");
        }
        other => fail_assert!("expected CustomTag, got {other:?}"),
    }
}

#[test]
fn reject_forbidden_features_allows_core_tags() {
    let yaml = "key: value\n";
    let Ok(events) = crate::events::collect_events(yaml) else {
        fail_assert!("collect_events failed");
        return;
    };
    let result = reject_forbidden_features(&events);
    assert_eq!(result, Ok(()));
}

#[test]
fn reject_anchors_aliases_merges_rejects_anchor() {
    let yaml = "a: &anc value\n";
    let Ok(events) = crate::events::collect_events(yaml) else {
        fail_assert!("collect_events failed");
        return;
    };
    let result = reject_anchors_aliases_merges(&events);
    assert_eq!(result, Err(YamlError::AnchorAliasMerge));
}

#[test]
fn reject_anchors_aliases_merges_allows_clean_yaml() {
    let yaml = "a: 1\n";
    let Ok(events) = crate::events::collect_events(yaml) else {
        fail_assert!("collect_events failed");
        return;
    };
    let result = reject_anchors_aliases_merges(&events);
    assert_eq!(result, Ok(()));
}

#[test]
fn reject_multiple_documents_rejects_two_docs() {
    let yaml = "---\na: 1\n---\nb: 2\n";
    let Ok(events) = crate::events::collect_events(yaml) else {
        fail_assert!("collect_events failed");
        return;
    };
    let result = reject_multiple_documents(&events);
    assert_eq!(result, Err(YamlError::MultipleDocuments { count: 2 }));
}

#[test]
fn reject_multiple_documents_allows_single_doc() {
    let yaml = "a: 1\n";
    let Ok(events) = crate::events::collect_events(yaml) else {
        fail_assert!("collect_events failed");
        return;
    };
    let result = reject_multiple_documents(&events);
    assert_eq!(result, Ok(()));
}

#[test]
fn reject_yaml_1_1_ambiguous_rejects_yes_exact() {
    let scalars = vec!["yes"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar {
            scalar: "yes".into()
        })
    );
}

#[test]
fn reject_yaml_1_1_ambiguous_rejects_y_exact() {
    let scalars = vec!["y"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar { scalar: "y".into() })
    );
}

#[test]
fn reject_yaml_1_1_ambiguous_rejects_n_exact() {
    let scalars = vec!["n"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar { scalar: "n".into() })
    );
}

#[test]
fn reject_yaml_1_1_ambiguous_allows_true_exact() {
    let scalars = vec!["true"];
    let result = reject_yaml_1_1_ambiguous_scalars(&scalars);
    assert_eq!(result, Ok(()));
}

#[test]
fn duplicate_top_level_key_exact() {
    let yaml = "version: velvet-ballastics/v1\nname: first\nname: second\nwhen:\n  manual: {}\nsteps: []\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Err(YamlError::DuplicateKey { key: "name".into() }));
}

#[test]
fn duplicate_nested_key_exact() {
    let yaml = "version: velvet-ballastics/v1\nname: wf\nwhen:\n  ipc:\n    name: a\n    name: b\nsteps: []\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Err(YamlError::DuplicateKey { key: "name".into() }));
}

#[test]
fn check_source_size_allows_within_limit() {
    let text = "a: b\n";
    let result = check_source_size(text, 1_000);
    assert_eq!(result, Ok(()));
}

#[test]
fn check_source_size_rejects_over_limit_exact() {
    let text = "abcde";
    let result = check_source_size(text, 4);
    assert_eq!(result, Err(YamlError::SourceTooLarge { size: 5, max: 4 }));
}

#[test]
fn validate_accepts_nested_mapping() {
    let yaml = "a:\n  b: 1\n  c: 2\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_accepts_sequence() {
    let yaml = "items:\n  - a\n  - b\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_rejects_whitespace_only() {
    let yaml = "   \n  \n";
    let result = validate_yaml_profile(yaml);
    assert!(matches!(result, Err(YamlError::EmptySource)));
}

#[test]
fn custom_tag_rejected_exact() {
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

