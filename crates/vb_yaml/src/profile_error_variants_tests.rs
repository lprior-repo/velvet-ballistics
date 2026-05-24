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

// ---------------------------------------------------------------------------
// YamlError variant smoke tests — variants defined but not fully exercised
// ---------------------------------------------------------------------------

/// `BinaryScalar` is **unreachable through the public API**.
///
/// `reject_binary_scalar` is a no-op because the binary-tag check in
/// `reject_forbidden_features` catches `!!binary` as a `CustomTag` variant
/// before the scalar body is ever examined.  No production code path
/// constructs `YamlError::BinaryScalar`.
///
/// This test verifies the `Display` impl so that error-reporting tooling
/// (e.g. logs, diagnostics) produces a well-formed message if the variant
/// is ever wired into a reachable path in the future.
#[test]
fn yaml_error_binary_scalar_variant_exists() {
    let err = YamlError::BinaryScalar;
    let msg = err.to_string();
    assert!(
        msg.contains("binary"),
        "BinaryScalar error message should mention 'binary', got: {msg}"
    );
}

/// `UnsupportedFeature` is **unreachable through the public API**.
///
/// No production code path constructs `YamlError::UnsupportedFeature`.
/// Null-byte checks in `check_null_bytes` and `check_null_bytes_in_source`
/// return `ForbiddenFeature` instead.  Other feature-related rejections
/// use narrower variants (`CustomTag`, `AnchorAliasMerge`, etc.).
/// Until a code path is wired in, this variant can never be returned by
/// `parse_workflow_source`, `validate_yaml_profile`, or any other public
/// function.
///
/// This test verifies the `Display` impl so that the error message is
/// well-formed if a future feature adds a reachable constructor.
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
        version: velvet-ballastics/v1
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
        version: velvet-ballastics/v1
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
        version: velvet-ballastics/v1
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
    assert_eq!(
        result,
        Err(YamlError::ParseError {
            line: 2,
            reason: "while parsing a flow sequence, expected ',' or ']'".into()
        })
    );
}

/// `parse_workflow_source` surfaces `ParseError` for unclosed strings.
#[test]
fn parse_error_unclosed_double_quote() {
    let yaml = "key: \"unclosed\n";
    let result = parse_workflow_source(yaml);
    assert_eq!(
        result,
        Err(YamlError::ParseError {
            line: 1,
            reason: "while scanning a quoted scalar, found unexpected end of stream".into()
        })
    );
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
    assert_eq!(result, Err(YamlError::CustomTag { tag: "!seq".into() }));
}

/// A custom tag on a mapping (e.g., !map) is rejected as CustomTag.
#[test]
fn forbidden_feature_custom_tag_on_mapping_rejected() {
    let yaml = "data: !map\n  k: v\n";
    let result = validate_yaml_profile(yaml);
    assert_eq!(result, Err(YamlError::CustomTag { tag: "!map".into() }));
}
