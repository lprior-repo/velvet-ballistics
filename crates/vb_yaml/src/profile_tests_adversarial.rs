#![forbid(unsafe_code)]
//! Profile module adversarial tests.

use crate::profile_validation::{
    validate_yaml_profile, validate_yaml_profile_with_limits, check_source_size,
};
use crate::{YamlError, YamlLimits};

fn assertion_failed(_message: std::fmt::Arguments<'_>) -> bool {
    false
}

macro_rules! fail_assert {
    ($($arg:tt)*) => {
        assert!(assertion_failed(format_args!($($arg)*)), $($arg)*)
    };
}

/// Generate deeply nested YAML for testing depth limits.
fn generate_nested_yaml(depth: usize) -> String {
    let mut yaml = String::from("a:\n");
    for i in 0..depth {
        let indent = "  ".repeat(i);
        yaml.push_str(&format!("{indent}b:\n"));
    }
    yaml
}

/// Generate YAML with many key-value pairs under a root key for testing node limits.
fn generate_many_keys_under_root_yaml(key_count: usize) -> String {
    let mut yaml = String::from("root:\n");
    for i in 0..key_count {
        yaml.push_str(&format!("  k{i}: v{i}\n"));
    }
    yaml
}

// -----------------------------------------------------------------------
// Adversarial BDD tests - attack vector validation
// -----------------------------------------------------------------------

#[test]
fn adversarial_duplicate_keys_nested_deep_mapping_rejected() {
    let yaml = "a:\n  b:\n    c: 1\n    c: 2\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Err(YamlError::DuplicateKey { key: "c".into() }));
}

#[test]
fn adversarial_duplicate_keys_top_level_rejected() {
    let yaml = "x: 1\nx: 2\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Err(YamlError::DuplicateKey { key: "x".into() }));
}

#[test]
fn adversarial_alias_without_anchor_rejected() {
    let yaml = "a: &anc value\nb: *anc\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Err(YamlError::AnchorAliasMerge));
}

#[test]
fn adversarial_anchor_on_sequence_rejected() {
    let yaml = "items: &seq\n  - a\n  - b\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Err(YamlError::AnchorAliasMerge));
}

#[test]
fn adversarial_anchor_on_mapping_rejected() {
    let yaml = "base: &map\n  k: v\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Err(YamlError::AnchorAliasMerge));
}

#[test]
fn adversarial_custom_tag_double_bang_timestamp_rejected() {
    let yaml = "date: !!timestamp 2024-01-01\n";
    let result = validate_yaml_profile(yaml);
    match result {
        Err(YamlError::CustomTag { tag }) => {
            assert!(
                tag.contains("timestamp"),
                "expected 'timestamp' in tag, got: {tag}"
            );
        }
        other => fail_assert!("expected CustomTag, got {other:?}"),
    }
}

#[test]
fn adversarial_custom_tag_local_bang_rejected() {
    let yaml = "val: !myapp/special data\n";
    let result = validate_yaml_profile(yaml);
    match result {
        Err(YamlError::CustomTag { tag }) => {
            assert!(tag.contains("myapp"), "expected 'myapp' in tag, got: {tag}");
        }
        other => fail_assert!("expected CustomTag, got {other:?}"),
    }
}

#[test]
fn adversarial_multi_document_with_explicit_markers_rejected() {
    let yaml = "---\na: 1\n...\n---\nb: 2\n";
    let result = validate_yaml_profile(yaml);
    assert!(
        matches!(result, Err(YamlError::MultipleDocuments { count }) if count >= 2),
        "expected MultipleDocuments, got: {result:?}"
    );
}

#[test]
fn adversarial_yaml_11_yes_mixed_case_rejected() {
    let yaml = "flag: Yes\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar {
            scalar: "Yes".into()
        })
    );
}

#[test]
fn adversarial_yaml_11_no_uppercase_rejected() {
    let yaml = "flag: NO\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar {
            scalar: "NO".into()
        })
    );
}

#[test]
fn adversarial_yaml_11_on_uppercase_rejected() {
    let yaml = "flag: ON\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar {
            scalar: "ON".into()
        })
    );
}

#[test]
fn adversarial_yaml_11_off_mixed_case_rejected() {
    let yaml = "flag: Off\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar {
            scalar: "Off".into()
        })
    );
}

#[test]
fn adversarial_yaml_11_y_lowercase_rejected() {
    let yaml = "flag: y\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar { scalar: "y".into() })
    );
}

#[test]
fn adversarial_yaml_11_n_lowercase_rejected() {
    let yaml = "flag: n\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(
        result,
        Err(YamlError::AmbiguousScalar { scalar: "n".into() })
    );
}

#[test]
fn adversarial_yaml_11_boolean_quoted_accepted() {
    let yaml = "flag: 'yes'\nother: \"no\"\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_comments_only_rejected_as_empty() {
    let yaml = "# just a comment\n# another comment\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Err(YamlError::EmptySource));
}

#[test]
fn adversarial_empty_string_rejected() {
    let result = validate_yaml_profile("");
    assert_eq!(result, Err(YamlError::EmptySource));
}

#[test]
fn adversarial_scalar_over_limit_rejected() {
    let long_val = "x".repeat(70_000);
    let yaml = format!("key: \"{long_val}\"\n");
    let result = validate_yaml_profile(&yaml);
    match result {
        Err(YamlError::ScalarTooLong { len, max }) => {
            assert!(len > 65_536, "expected len > 65536, got {len}");
            assert_eq!(max, 65_536);
        }
        other => fail_assert!("expected ScalarTooLong, got {other:?}"),
    }
}

#[test]
fn adversarial_node_limit_exceeded() {
    let yaml = generate_many_keys_under_root_yaml(5_000);
    let limits = YamlLimits {
        max_nodes: 100,
        ..YamlLimits::default()
    };
    let result = validate_yaml_profile_with_limits(&yaml, &limits);
    match result {
        Err(YamlError::NodeLimitExceeded { count, max }) => {
            assert!(count > 100);
            assert_eq!(max, 100);
        }
        other => fail_assert!("expected NodeLimitExceeded, got {other:?}"),
    }
}

#[test]
fn adversarial_duplicate_key_in_sequence_context_rejected() {
    let yaml = "items:\n  - name: a\n    name: b\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Err(YamlError::DuplicateKey { key: "name".into() }));
}

#[test]
fn adversarial_depth_limit_exact_boundary_accepted() {
    let yaml = generate_nested_yaml(9);
    let limits = YamlLimits {
        max_depth: 10,
        ..YamlLimits::default()
    };
    let result = validate_yaml_profile_with_limits(&yaml, &limits);
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_depth_limit_one_over_rejected() {
    let yaml = generate_nested_yaml(11);
    let limits = YamlLimits {
        max_depth: 10,
        ..YamlLimits::default()
    };
    let result = validate_yaml_profile_with_limits(&yaml, &limits);
    assert!(
        matches!(result, Err(YamlError::NestingTooDeep { depth, max }) if depth > 10 && max == 10),
        "expected NestingTooDeep, got: {result:?}"
    );
}

#[test]
fn adversarial_tag_on_sequence_rejected() {
    let yaml = "items: !seq\n  - a\n  - b\n";
    let result = validate_yaml_profile(yaml);
    match result {
        Err(YamlError::CustomTag { tag }) => {
            assert!(tag.contains("seq"), "expected 'seq' in tag, got: {tag}");
        }
        other => fail_assert!("expected CustomTag, got {other:?}"),
    }
}

#[test]
fn adversarial_tag_on_mapping_rejected() {
    let yaml = "data: !map\n  k: v\n";
    let result = validate_yaml_profile(yaml);
    match result {
        Err(YamlError::CustomTag { tag }) => {
            assert!(tag.contains("map"), "expected 'map' in tag, got: {tag}");
        }
        other => fail_assert!("expected CustomTag, got {other:?}"),
    }
}

#[test]
fn adversarial_source_exactly_at_size_limit_accepted() {
    let base = "a: b\n";
    let max = base.len();
    let result = check_source_size(base, max);
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_source_one_byte_over_limit_rejected() {
    let text = "a: bcd\n";
    let result = check_source_size(text, 6);
    assert_eq!(result, Err(YamlError::SourceTooLarge { size: 7, max: 6 }));
}

#[test]
fn adversarial_three_documents_rejected_with_count() {
    let yaml = "---\na: 1\n---\nb: 2\n---\nc: 3\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Err(YamlError::MultipleDocuments { count: 3 }));
}
