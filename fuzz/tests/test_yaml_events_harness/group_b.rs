//! Continuation of yaml events harness tests (group B: tests 19-38).
use fuzz_lib::test_exports::{MAX_FUZZ_PAYLOAD, assert_typed_yaml_error, fuzz_yaml_events};
use vb_yaml::YamlError;

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
fn classify_legacy_primitive_deprecated_variant() {
    let err = YamlError::LegacyPrimitiveDeprecated {
        name: String::from("parallel"),
        replacement: String::from("together"),
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
        YamlError::DuplicateKey { key: "key".into() },
        YamlError::AnchorAliasMerge,
        YamlError::CustomTag { tag: "!tag".into() },
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
        YamlError::SequenceTooLong { len: 999, max: 256 },
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
        YamlError::LegacyPrimitiveDeprecated {
            name: String::from("aggregate"),
            replacement: String::from("reduce"),
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
