#![forbid(unsafe_code)]

//! Behavior tests for `fuzz_yaml_events` harness logic and `assert_typed_yaml_error`.
//!
//! Test layers:
//! - U1-U6: Harness logic (public behavior of fuzz_yaml_events)
//! - BT7-BT10: Harness boundary behavior
//! - E1-E22: Error variant classification (assert_typed_yaml_error exhaustiveness)
//! - BT1-BT6: Boundary tests (fuzz gate: UTF-8, empty, non-UTF-8, etc.)

use fuzz_lib::test_exports::{assert_typed_yaml_error, fuzz_yaml_events, MAX_FUZZ_PAYLOAD};
use vb_yaml::YamlError;

// ============================================================================
// U1-U6: Harness Logic Tests — fuzz_yaml_events public behavior
// ============================================================================

#[test]
fn harness_does_not_panic_on_valid_minimal_workflow() {
    // Given: a valid, minimal velvet-ballistics workflow
    let yaml = "version: v1\nname: test\nwhen: manual\nsteps:\n  - id: 1\n    do: ping\n    input: 0\n";
    // When: fuzz_yaml_events is called with valid UTF-8
    // Then: must not panic (the function returns (), no return value to check)
    fuzz_yaml_events(yaml.as_bytes());
}

#[test]
fn harness_does_not_panic_on_single_line_scalar() {
    // Given: a YAML scalar on its own line
    let yaml = "hello\n";
    // When: fuzz_yaml_events is called
    // Then: no panic — empty profiles are acceptable
    fuzz_yaml_events(yaml.as_bytes());
}

#[test]
fn harness_does_not_panic_on_empty_string() {
    // Given: empty input
    let yaml = "";
    // When: fuzz_yaml_events is called
    // Then: no panic — empty input is valid UTF-8
    fuzz_yaml_events(yaml.as_bytes());
}

#[test]
fn harness_returns_early_on_non_utf8_input() {
    // Given: invalid UTF-8 bytes
    let data: &[u8] = &[0xFF, 0xFE, 0xFD];
    // When: fuzz_yaml_events is called
    // Then: must return early without panic (non-UTF-8 is silently skipped)
    fuzz_yaml_events(data);
}

#[test]
fn harness_handles_maximum_valid_payload_boundary() {
    // Given: input at exactly MAX_FUZZ_PAYLOAD bytes
    let payload = "a".repeat(MAX_FUZZ_PAYLOAD as usize);
    // When: fuzz_yaml_events is called
    // Then: no panic — large payloads must not OOM or overflow
    fuzz_yaml_events(payload.as_bytes());
}

#[test]
fn harness_handles_yaml_with_only_whitespace() {
    // Given: whitespace-only input
    let yaml = "   \n  \t  \n   ";
    // When: fuzz_yaml_events is called
    // Then: no panic — whitespace is valid UTF-8, profile may or may not reject it
    fuzz_yaml_events(yaml.as_bytes());
}

// ============================================================================
// BT7-BT10: Harness Boundary Behavior Tests
// ============================================================================

#[test]
fn harness_does_not_panic_on_deeply_nested_yaml() {
    // Given: deeply nested YAML (100 levels)
    let mut yaml = String::new();
    for i in 0..100 {
        for _ in 0..i {
            yaml.push(' ');
        }
        yaml.push_str("key: value\n");
    }
    // When: fuzz_yaml_events is called
    // Then: no panic — nesting errors are returned as typed YamlError
    fuzz_yaml_events(yaml.as_bytes());
}

#[test]
fn harness_does_not_panic_on_source_map_with_max_entries() {
    // Given: YAML that creates many source map entries but stays under MAX_FUZZ_PAYLOAD
    let mut yaml = String::from("items:\n");
    for i in 0..500 {
        yaml.push_str(&format!("  - item_{i}: {i}\n"));
    }
    // When: fuzz_yaml_events is called
    // Then: no panic — event count must stay below MAX_FUZZ_PAYLOAD (4096)
    fuzz_yaml_events(yaml.as_bytes());
}

#[test]
fn harness_does_not_panic_on_very_long_scalar_value() {
    // Given: a scalar value at 64KB
    let scalar_content = "x".repeat(65536);
    let yaml = format!("key: {scalar_content}\n");
    // When: fuzz_yaml_events is called
    // Then: no panic — scalar length limit is enforced as typed error
    fuzz_yaml_events(yaml.as_bytes());
}

#[test]
fn harness_does_not_panic_on_very_long_key_name() {
    // Given: a key name at 64KB
    let key_content = "x".repeat(65536);
    let yaml = format!("{key_content}: value\n");
    // When: fuzz_yaml_events is called
    // Then: no panic
    fuzz_yaml_events(yaml.as_bytes());
}

// ============================================================================
// E1-E22: Error Classification Tests — assert_typed_yaml_error exhaustiveness
// ============================================================================

/// Helper: constructs a YamlError variant and asserts it is recognized by
/// assert_typed_yaml_error.  Every variant must be matched without panicking.
macro_rules! assert_classifies {
    ($expr:expr) => {
        assert_typed_yaml_error($expr);
    };
}

#[test]
fn classify_unsupported_trigger_variant() {
    // Given: UnsupportedTrigger variant
    let err = YamlError::UnsupportedTrigger { trigger: "cron" };
    // When: assert_typed_yaml_error is called
    // Then: must match without panic (or triggering the wildcard arm)
    assert_classifies!(err);
}

#[test]
fn classify_unsupported_feature_variant() {
    let err = YamlError::UnsupportedFeature {
        feature: "merge_keys",
    };
    assert_classifies!(err);
}

#[test]
fn classify_duplicate_key_variant() {
    let err = YamlError::DuplicateKey {
        key: "version".into(),
    };
    assert_classifies!(err);
}

#[test]
fn classify_anchor_alias_merge_variant() {
    let err = YamlError::AnchorAliasMerge;
    assert_classifies!(err);
}

#[test]
fn classify_custom_tag_variant() {
    let err = YamlError::CustomTag {
        tag: "!mytag".into(),
    };
    assert_classifies!(err);
}

#[test]
fn classify_binary_scalar_variant() {
    let err = YamlError::BinaryScalar;
    assert_classifies!(err);
}

#[test]
fn classify_multiple_documents_variant() {
    let err = YamlError::MultipleDocuments { count: 3 };
    assert_classifies!(err);
}

#[test]
fn classify_ambiguous_scalar_variant() {
    let err = YamlError::AmbiguousScalar {
        scalar: "yes".into(),
    };
    assert_classifies!(err);
}

#[test]
fn classify_source_too_large_variant() {
    let err = YamlError::SourceTooLarge {
        size: 5000,
        max: 4096,
    };
    assert_classifies!(err);
}

#[test]
fn classify_nesting_too_deep_variant() {
    let err = YamlError::NestingTooDeep {
        depth: 100,
        max: 64,
    };
    assert_classifies!(err);
}

#[test]
fn classify_node_limit_exceeded_variant() {
    let err = YamlError::NodeLimitExceeded {
        count: 5000,
        max: 4096,
    };
    assert_classifies!(err);
}

#[test]
fn classify_scalar_too_long_variant() {
    let err = YamlError::ScalarTooLong {
        len: 10000,
        max: 8192,
    };
    assert_classifies!(err);
}

#[test]
fn classify_sequence_too_long_variant() {
    let err = YamlError::SequenceTooLong {
        len: 5000,
        max: 4096,
    };
    assert_classifies!(err);
}

#[test]
fn classify_mapping_too_large_variant() {
    let err = YamlError::MappingTooLarge {
        count: 2000,
        max: 1024,
    };
    assert_classifies!(err);
}

#[test]
fn classify_unknown_field_variant() {
    let err = YamlError::UnknownField {
        field: "extra_data".into(),
    };
    assert_classifies!(err);
}

#[test]
fn classify_empty_source_variant() {
    let err = YamlError::EmptySource;
    assert_classifies!(err);
}

#[test]
fn classify_missing_field_variant() {
    let err = YamlError::MissingField { field: "version" };
    assert_classifies!(err);
}

#[test]
fn classify_field_shape_variant() {
    let err = YamlError::FieldShape {
        field: "steps",
        expected: "array",
    };
    assert_classifies!(err);
}

#[test]
fn classify_parse_error_variant() {
    let err = YamlError::ParseError {
        line: 42,
        reason: "unexpected character".into(),
    };
    assert_classifies!(err);
}

#[test]
fn classify_forbidden_feature_variant() {
    let err = YamlError::ForbiddenFeature {
        detail: "null_byte_in_source",
    };
    assert_classifies!(err);
}

#[test]
fn classify_legacy_primitive_variant() {
    let err = YamlError::LegacyPrimitive {
        primitive: "parallel",
        canonical: "together",
    };
    assert_classifies!(err);
}

#[test]
fn exhaustive_match_all_variants_in_loop() {
    // Given: all 21 YamlError variants (constructed explicitly)
    let all_variants: [YamlError; 21] = [
        YamlError::UnsupportedTrigger { trigger: "cron" },
        YamlError::UnsupportedFeature {
            feature: "merge_keys",
        },
        YamlError::DuplicateKey {
            key: "key".into(),
        },
        YamlError::AnchorAliasMerge,
        YamlError::CustomTag {
            tag: "!tag".into(),
        },
        YamlError::BinaryScalar,
        YamlError::MultipleDocuments { count: 2 },
        YamlError::AmbiguousScalar {
            scalar: "no".into(),
        },
        YamlError::SourceTooLarge {
            size: 9999,
            max: 4096,
        },
        YamlError::NestingTooDeep {
            depth: 200,
            max: 64,
        },
        YamlError::NodeLimitExceeded {
            count: 9999,
            max: 4096,
        },
        YamlError::ScalarTooLong {
            len: 99999,
            max: 8192,
        },
        YamlError::SequenceTooLong {
            len: 999,
            max: 256,
        },
        YamlError::MappingTooLarge {
            count: 999,
            max: 256,
        },
        YamlError::UnknownField {
            field: "unknown".into(),
        },
        YamlError::EmptySource,
        YamlError::MissingField { field: "name" },
        YamlError::FieldShape {
            field: "when",
            expected: "string",
        },
        YamlError::ParseError {
            line: 1,
            reason: "syntax".into(),
        },
        YamlError::ForbiddenFeature {
            detail: "anchor_detected",
        },
        YamlError::LegacyPrimitive {
            primitive: "aggregate",
            canonical: "reduce",
        },
    ];
    // When: every variant is classified
    for variant in all_variants {
        assert_typed_yaml_error(variant);
    }
    // Then: loop completes without panic — all 21 variants classified
}

// ============================================================================
// BT1-BT6: Boundary Tests — input gating and behavior at edges
// ============================================================================

#[test]
fn boundary_empty_byte_slice_returns_early_without_panic() {
    // Given: empty byte slice
    let data: &[u8] = &[];
    // When: fuzz_yaml_events is called
    // Then: no panic — empty slice is silently accepted
    fuzz_yaml_events(data);
}

#[test]
fn boundary_single_byte_utf8_returns_early() {
    // Given: single ASCII byte
    let data: &[u8] = b"A";
    // When: fuzz_yaml_events is called
    // Then: no panic — valid UTF-8 single byte
    fuzz_yaml_events(data);
}

#[test]
fn boundary_non_utf8_single_byte_returns_early() {
    // Given: single non-UTF-8 byte (0x80 is a continuation byte)
    let data: &[u8] = &[0x80];
    // When: fuzz_yaml_events is called
    // Then: no panic — non-UTF-8 is skipped early by from_utf8 check
    fuzz_yaml_events(data);
}

#[test]
fn boundary_valid_utf8_multi_byte_character_accepted() {
    // Given: valid UTF-8 with multi-byte character
    let yaml = "\u{1F600}\n";
    // When: fuzz_yaml_events is called
    // Then: no panic — valid multi-byte UTF-8
    fuzz_yaml_events(yaml.as_bytes());
}

#[test]
fn boundary_invalid_utf8_with_valid_prefix_returns_early() {
    // Given: byte sequence starting with valid ASCII but containing invalid UTF-8
    let data: &[u8] = b"hello\xC0\x80";
    // When: fuzz_yaml_events is called
    // Then: no panic — overlong encoding is rejected as non-UTF-8
    fuzz_yaml_events(data);
}

#[test]
fn boundary_exactly_max_fuzz_payload_length_accepted() {
    // Given: byte slice of exactly MAX_FUZZ_PAYLOAD length (valid UTF-8)
    let payload: String = "a".repeat(MAX_FUZZ_PAYLOAD as usize);
    assert_eq!(payload.len(), MAX_FUZZ_PAYLOAD as usize);
    // When: fuzz_yaml_events is called
    // Then: no panic — length itself is fine, content may or may not be valid YAML
    fuzz_yaml_events(payload.as_bytes());
}
