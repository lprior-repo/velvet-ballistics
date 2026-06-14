#![forbid(unsafe_code)]
//! Tests for untested YamlError variants and branch coverage.
//!
//! These tests exist because the test-reviewer density audit found
//! YamlError variants and parsing branches without exact-assertion coverage.
//!
//! Target: 5x density (215 tests / 43 pub fns).

use crate::events::{EventSpan, ScalarStyle, YamlEvent};
use crate::parse_workflow_source;
use crate::profile::{reject_forbidden_features, validate_yaml_profile};
use crate::{YamlError, profile::reject_anchors_aliases_merges};

fn assertion_failed(_message: std::fmt::Arguments<'_>) -> bool {
    false
}

macro_rules! fail_assert {
    ($($arg:tt)*) => {
        assert!(assertion_failed(format_args!($($arg)*)), $($arg)*)
    };
}

// ---------------------------------------------------------------------------
// YamlError variant smoke tests — variants defined but not fully exercised
// ---------------------------------------------------------------------------

/// `BinaryScalar` is defined but the actual rejection path runs through
/// `CustomTag` (the tag check in reject_forbidden_features). We test the
/// variant exists and formats correctly.
#[test]
fn yaml_error_binary_scalar_variant_exists() {
    let err = YamlError::BinaryScalar;
    let msg = err.to_string();
    assert!(
        msg.contains("binary"),
        "BinaryScalar error message should mention 'binary', got: {msg}"
    );
}

/// `UnsupportedFeature` is defined but no current code path returns it
/// (check_null_bytes returns ForbiddenFeature instead). We test the variant
/// exists and formats correctly.
#[test]
fn yaml_error_unsupported_feature_variant_exists() {
    let err = YamlError::UnsupportedFeature {
        feature: "some_feature",
    };
    let msg = err.to_string();
    assert!(
        msg.contains("some_feature"),
        "UnsupportedFeature message should contain feature name, got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// ForbiddenFeature — null byte in source (scalar-level path unreachable)
// ---------------------------------------------------------------------------

/// `check_null_bytes_in_source` runs before scalar parsing and returns
/// `ForbiddenFeature { detail: "null_byte_in_source" }` for any null byte
/// in the raw source text. The scalar-level path
/// (`null_byte_in_scalar`) is unreachable through normal YAML parsing
/// because saphyr validates source text before building scalar values.
#[test]
fn forbidden_feature_null_byte_in_source_rejected() {
    let yaml = "key: \"has\x00null\"\n";
    let result = validate_yaml_profile(yaml);
    assert!(
        matches!(
            result,
            Err(YamlError::ForbiddenFeature {
                detail: "null_byte_in_source"
            })
        ),
        "expected ForbiddenFeature(null_byte_in_source), got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// FieldShape — non-string mapping keys in inputs / vars / result
// ---------------------------------------------------------------------------

/// `parse_inputs` returns `FieldShape { field: "inputs key", expected: "string" }`
/// when a key in the inputs mapping is not a string (e.g., integer).
#[test]
fn field_shape_inputs_key_not_string() {
    let yaml = indoc::indoc! {"
        version: velvet-ballistics/v1
        name: test
        when: { manual: {} }
        inputs:
          123: string
        steps: []
    "};
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::FieldShape {
            field: "inputs key",
            expected: "string"
        })
    );
}

/// `parse_vars` returns `FieldShape { field: "vars key", expected: "string" }`
/// when a key in the vars mapping is not a string.
#[test]
fn field_shape_vars_key_not_string() {
    let yaml = indoc::indoc! {"
        version: velvet-ballistics/v1
        name: test
        when: { manual: {} }
        vars:
          456: \"value\"
        steps: []
    "};
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::FieldShape {
            field: "vars key",
            expected: "string"
        })
    );
}

/// `parse_result` returns `FieldShape { field: "result key", expected: "string" }`
/// when a key in the result mapping is not a string.
#[test]
fn field_shape_result_key_not_string() {
    let yaml = indoc::indoc! {"
        version: velvet-ballistics/v1
        name: test
        when: { manual: {} }
        steps: []
        result:
          789: \"some value\"
    "};
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::FieldShape {
            field: "result key",
            expected: "string"
        })
    );
}

// ---------------------------------------------------------------------------
// ParseError — malformed YAML
// ---------------------------------------------------------------------------

/// `parse_workflow_ast` returns `ParseError` when the YAML document is
/// syntactically malformed (e.g., mismatched brackets).
#[test]
fn parse_error_for_malformed_yaml() {
    // Mismatched flow indicators — saphyr reports this as a parse error.
    let yaml = "key: [1, 2\n";
    let result = parse_workflow_source(yaml);
    match result {
        Err(YamlError::ParseError { line, reason }) => {
            assert!(line > 0, "ParseError line should be > 0, got {line}");
            assert!(!reason.is_empty(), "ParseError reason should be non-empty");
        }
        other => fail_assert!("expected ParseError, got {other:?}"),
    }
}

/// `parse_workflow_source` surfaces `ParseError` for unclosed strings.
#[test]
fn parse_error_unclosed_double_quote() {
    let yaml = "key: \"unclosed\n";
    let result = parse_workflow_source(yaml);
    match result {
        Err(YamlError::ParseError { line, .. }) => {
            assert!(line > 0, "ParseError line should be > 0, got {line}");
        }
        other => fail_assert!("expected ParseError, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Merge-key detection via reject_anchors_aliases_merges
// ---------------------------------------------------------------------------

/// In `reject_anchors_aliases_merges`, a Scalar event with anchor_id == 0
/// and a merge key tag (yaml.org/2002:merge) is rejected as AnchorAliasMerge.
#[test]
fn reject_anchors_aliases_merges_rejects_yaml_org_merge_via_scalar() {
    // Scalar with yaml.org/2002:merge tag, zero anchor_id → merge key path
    let events = &[YamlEvent::Scalar {
        value: "<<".into(),
        style: ScalarStyle::Plain,
        anchor_id: 0,
        tag: Some("tag:yaml.org,2002:merge".into()),
        span: EventSpan {
            start: 0,
            end: 2,
            line: 1,
            column: 1,
        },
    }];
    let result = reject_anchors_aliases_merges(events);
    assert_eq!(result, Err(YamlError::AnchorAliasMerge));
}

/// In `reject_anchors_aliases_merges`, a Scalar event with anchor_id == 0
/// and a !!merge tag is rejected as AnchorAliasMerge.
#[test]
fn reject_anchors_aliases_merges_rejects_double_bang_merge_via_scalar() {
    // Scalar with !!merge tag, zero anchor_id → merge key path
    let events = &[YamlEvent::Scalar {
        value: "<<".into(),
        style: ScalarStyle::Plain,
        anchor_id: 0,
        tag: Some("!!merge".into()),
        span: EventSpan {
            start: 0,
            end: 2,
            line: 1,
            column: 1,
        },
    }];
    let result = reject_anchors_aliases_merges(events);
    assert_eq!(result, Err(YamlError::AnchorAliasMerge));
}

// ---------------------------------------------------------------------------
// is_allowed_tag branch coverage
// ---------------------------------------------------------------------------

/// Allowed yaml.org/2002:str tag suffix passes through
/// `reject_forbidden_features` without error.
#[test]
fn is_allowed_tag_accepts_yaml_org_2002_str() {
    let events = &[YamlEvent::Scalar {
        value: "hello".into(),
        style: ScalarStyle::Plain,
        anchor_id: 0,
        tag: Some("tag:yaml.org,2002:str".into()),
        span: EventSpan {
            start: 0,
            end: 5,
            line: 1,
            column: 1,
        },
    }];
    let result = reject_forbidden_features(events);
    assert_eq!(result, Ok(()));
}

/// Allowed !!int tag suffix passes through
/// `reject_forbidden_features` without error.
#[test]
fn is_allowed_tag_accepts_double_bang_int() {
    let events = &[YamlEvent::Scalar {
        value: "42".into(),
        style: ScalarStyle::Plain,
        anchor_id: 0,
        tag: Some("!!int".into()),
        span: EventSpan {
            start: 0,
            end: 2,
            line: 1,
            column: 1,
        },
    }];
    let result = reject_forbidden_features(events);
    assert_eq!(result, Ok(()));
}

/// A custom tag on a sequence (e.g., !seq) is rejected as CustomTag
/// by `reject_forbidden_features` (not BinaryScalar, even though it could
/// theoretically be a binary tag — the actual binary-scalar path is a no-op).
#[test]
fn forbidden_feature_custom_tag_on_sequence_rejected() {
    let yaml = "items: !seq\n  - a\n";
    let result = validate_yaml_profile(yaml);
    match result {
        Err(YamlError::CustomTag { tag }) => {
            assert!(tag.contains("seq"), "expected 'seq' in tag, got: {tag}");
        }
        other => fail_assert!("expected CustomTag, got {other:?}"),
    }
}

/// A custom tag on a mapping (e.g., !map) is rejected as CustomTag.
#[test]
fn forbidden_feature_custom_tag_on_mapping_rejected() {
    let yaml = "data: !map\n  k: v\n";
    let result = validate_yaml_profile(yaml);
    match result {
        Err(YamlError::CustomTag { tag }) => {
            assert!(tag.contains("map"), "expected 'map' in tag, got: {tag}");
        }
        other => fail_assert!("expected CustomTag, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// YamlError::kind() — exhaustive classifier coverage
// ---------------------------------------------------------------------------

/// Every YamlError variant must return a YamlErrorKind variant.
/// This is a structural invariant: if a new YamlErrorKind is added,
/// the compiler will force the kind() match to be exhaustive.
#[test]
fn kind_unsupported_trigger() {
    let err = YamlError::UnsupportedTrigger { trigger: "cron" };
    assert_eq!(err.kind(), crate::YamlErrorKind::UnsupportedTrigger);
}

#[test]
fn kind_unsupported_feature() {
    let err = YamlError::UnsupportedFeature { feature: "anchors" };
    assert_eq!(err.kind(), crate::YamlErrorKind::UnsupportedFeature);
}

#[test]
fn kind_duplicate_key() {
    let err = YamlError::DuplicateKey {
        key: Box::from("name"),
    };
    assert_eq!(err.kind(), crate::YamlErrorKind::DuplicateKey);
}

#[test]
fn kind_anchor_alias_merge() {
    let err = YamlError::AnchorAliasMerge;
    assert_eq!(err.kind(), crate::YamlErrorKind::AnchorAliasMerge);
}

#[test]
fn kind_custom_tag() {
    let err = YamlError::CustomTag {
        tag: Box::from("!mytag"),
    };
    assert_eq!(err.kind(), crate::YamlErrorKind::CustomTag);
}

#[test]
fn kind_binary_scalar() {
    let err = YamlError::BinaryScalar;
    assert_eq!(err.kind(), crate::YamlErrorKind::BinaryScalar);
}

#[test]
fn kind_multiple_documents() {
    let err = YamlError::MultipleDocuments { count: 3 };
    assert_eq!(err.kind(), crate::YamlErrorKind::MultipleDocuments);
}

#[test]
fn kind_ambiguous_scalar() {
    let err = YamlError::AmbiguousScalar {
        scalar: Box::from("yes"),
    };
    assert_eq!(err.kind(), crate::YamlErrorKind::AmbiguousScalar);
}

#[test]
fn kind_source_too_large() {
    let err = YamlError::SourceTooLarge {
        size: 2_000_000,
        max: 1_000_000,
    };
    assert_eq!(err.kind(), crate::YamlErrorKind::SourceTooLarge);
}

#[test]
fn kind_nesting_too_deep() {
    let err = YamlError::NestingTooDeep { depth: 65, max: 64 };
    assert_eq!(err.kind(), crate::YamlErrorKind::NestingTooDeep);
}

#[test]
fn kind_node_limit_exceeded() {
    let err = YamlError::NodeLimitExceeded {
        count: 200_000,
        max: 100_000,
    };
    assert_eq!(err.kind(), crate::YamlErrorKind::NodeLimitExceeded);
}

#[test]
fn kind_scalar_too_long() {
    let err = YamlError::ScalarTooLong {
        len: 100_000,
        max: 65_536,
    };
    assert_eq!(err.kind(), crate::YamlErrorKind::ScalarTooLong);
}

#[test]
fn kind_sequence_too_long() {
    let err = YamlError::SequenceTooLong {
        len: 20_000,
        max: 10_000,
    };
    assert_eq!(err.kind(), crate::YamlErrorKind::SequenceTooLong);
}

#[test]
fn kind_mapping_too_large() {
    let err = YamlError::MappingTooLarge {
        count: 2_000,
        max: 1_024,
    };
    assert_eq!(err.kind(), crate::YamlErrorKind::MappingTooLarge);
}

#[test]
fn kind_unknown_field() {
    let err = YamlError::UnknownField {
        field: Box::from("bogus"),
    };
    assert_eq!(err.kind(), crate::YamlErrorKind::UnknownField);
}

#[test]
fn kind_empty_source() {
    let err = YamlError::EmptySource;
    assert_eq!(err.kind(), crate::YamlErrorKind::EmptySource);
}

#[test]
fn kind_missing_field() {
    let err = YamlError::MissingField { field: "name" };
    assert_eq!(err.kind(), crate::YamlErrorKind::MissingField);
}

#[test]
fn kind_field_shape() {
    let err = YamlError::FieldShape {
        field: "inputs key",
        expected: "string",
    };
    assert_eq!(err.kind(), crate::YamlErrorKind::FieldShape);
}

#[test]
fn kind_parse_error() {
    let err = YamlError::ParseError {
        line: 5,
        reason: Box::from("unexpected token"),
    };
    assert_eq!(err.kind(), crate::YamlErrorKind::ParseError);
}

#[test]
fn kind_forbidden_feature() {
    let err = YamlError::ForbiddenFeature {
        detail: "null_byte_in_source",
    };
    assert_eq!(err.kind(), crate::YamlErrorKind::ForbiddenFeature);
}

#[test]
fn kind_legacy_primitive_deprecated() {
    let err = YamlError::LegacyPrimitiveDeprecated {
        name: "old_name".to_string(),
        replacement: "new_name".to_string(),
    };
    assert_eq!(
        err.kind(),
        crate::YamlErrorKind::LegacyPrimitiveDeprecated
    );
}

/// Verify kind() → code() mapping consistency for each error variant.
/// This tests the invariants from the doc comment on YamlError::code():
/// - DUPLICATE_KEY: DuplicateKey
/// - FORBIDDEN_YAML_FEATURE: ForbiddenFeature, AnchorAliasMerge, CustomTag, BinaryScalar,
///   AmbiguousScalar, UnsupportedFeature, MultipleDocuments, ParseError, LegacyPrimitiveDeprecated
/// - UNSUPPORTED_TRIGGER: UnsupportedTrigger
/// - PAYLOAD_TOO_LARGE: SourceTooLarge
/// - LIMIT_EXCEEDED: NestingTooDeep, NodeLimitExceeded, ScalarTooLong, SequenceTooLong, MappingTooLarge
/// - UNKNOWN_TOP_LEVEL_FIELD: UnknownField
/// - MISSING_REQUIRED_FIELD: EmptySource, MissingField
/// - TYPE_MISMATCH: FieldShape
#[test]
fn kind_to_code_mapping_consistent() {
    assert_eq!(
        YamlError::DuplicateKey { key: Box::from("x") }.code().as_str(),
        "DUPLICATE_KEY"
    );
    assert_eq!(
        YamlError::ForbiddenFeature { detail: "x" }.code().as_str(),
        "FORBIDDEN_YAML_FEATURE"
    );
    assert_eq!(
        YamlError::AnchorAliasMerge.code().as_str(),
        "FORBIDDEN_YAML_FEATURE"
    );
    assert_eq!(
        YamlError::CustomTag { tag: Box::from("!x") }.code().as_str(),
        "FORBIDDEN_YAML_FEATURE"
    );
    assert_eq!(
        YamlError::BinaryScalar.code().as_str(),
        "FORBIDDEN_YAML_FEATURE"
    );
    assert_eq!(
        YamlError::AmbiguousScalar { scalar: Box::from("x") }.code().as_str(),
        "FORBIDDEN_YAML_FEATURE"
    );
    assert_eq!(
        YamlError::UnsupportedFeature { feature: "x" }.code().as_str(),
        "FORBIDDEN_YAML_FEATURE"
    );
    assert_eq!(
        YamlError::MultipleDocuments { count: 2 }.code().as_str(),
        "FORBIDDEN_YAML_FEATURE"
    );
    assert_eq!(
        YamlError::ParseError { line: 1, reason: Box::from("x") }.code().as_str(),
        "FORBIDDEN_YAML_FEATURE"
    );
    assert_eq!(
        YamlError::LegacyPrimitiveDeprecated { name: "x".into(), replacement: "y".into() }.code().as_str(),
        "FORBIDDEN_YAML_FEATURE"
    );
    assert_eq!(
        YamlError::UnsupportedTrigger { trigger: "x" }.code().as_str(),
        "UNSUPPORTED_TRIGGER"
    );
    assert_eq!(
        YamlError::SourceTooLarge { size: 1, max: 0 }.code().as_str(),
        "PAYLOAD_TOO_LARGE"
    );
    assert_eq!(
        YamlError::NestingTooDeep { depth: 1, max: 0 }.code().as_str(),
        "LIMIT_EXCEEDED"
    );
    assert_eq!(
        YamlError::NodeLimitExceeded { count: 1, max: 0 }.code().as_str(),
        "LIMIT_EXCEEDED"
    );
    assert_eq!(
        YamlError::ScalarTooLong { len: 1, max: 0 }.code().as_str(),
        "LIMIT_EXCEEDED"
    );
    assert_eq!(
        YamlError::SequenceTooLong { len: 1, max: 0 }.code().as_str(),
        "LIMIT_EXCEEDED"
    );
    assert_eq!(
        YamlError::MappingTooLarge { count: 1, max: 0 }.code().as_str(),
        "LIMIT_EXCEEDED"
    );
    assert_eq!(
        YamlError::UnknownField { field: Box::from("x") }.code().as_str(),
        "UNKNOWN_TOP_LEVEL_FIELD"
    );
    assert_eq!(
        YamlError::EmptySource.code().as_str(),
        "MISSING_REQUIRED_FIELD"
    );
    assert_eq!(
        YamlError::MissingField { field: "x" }.code().as_str(),
        "MISSING_REQUIRED_FIELD"
    );
    assert_eq!(
        YamlError::FieldShape { field: "x", expected: "y" }.code().as_str(),
        "TYPE_MISMATCH"
    );
}
