//! Property test: Every YamlError variant maps to a registered SymbolicCode
//! in CODE_REGISTRY (21 variants, 6 distinct codes).
//!
//! PO-009 / PS-009: Error code stability — YamlError code() returns registered SymbolicCode.
//!
//! Invariants:
//!   - Every YamlError::code() returns a SymbolicCode.
//!   - Every returned symbolic name exists in CODE_REGISTRY.
//!   - All 6 distinct codes (DUPLICATE_KEY, FORBIDDEN_YAML_FEATURE, UNKNOWN_TOP_LEVEL_FIELD,
//!     MISSING_REQUIRED_FIELD, PAYLOAD_TOO_LARGE, LIMIT_EXCEEDED, UNSUPPORTED_TRIGGER,
//!     TYPE_MISMATCH) cover all 21 variants.

use vb_core::diagnostic::{CODE_REGISTRY, SymbolicCode};
use vb_yaml::YamlError;

fn all_yaml_error_variants() -> Vec<YamlError> {
    vec![
        YamlError::UnsupportedTrigger { trigger: "http" },
        YamlError::UnsupportedFeature { feature: "anchors" },
        YamlError::DuplicateKey {
            key: Box::from("steps"),
        },
        YamlError::AnchorAliasMerge,
        YamlError::CustomTag {
            tag: Box::from("!secret"),
        },
        YamlError::BinaryScalar,
        YamlError::MultipleDocuments { count: 3 },
        YamlError::AmbiguousScalar {
            scalar: Box::from("yes"),
        },
        YamlError::SourceTooLarge {
            size: 1024 * 1024,
            max: 1024,
        },
        YamlError::NestingTooDeep {
            depth: 100,
            max: 50,
        },
        YamlError::NodeLimitExceeded {
            count: 10000,
            max: 5000,
        },
        YamlError::ScalarTooLong {
            len: 10000,
            max: 1000,
        },
        YamlError::SequenceTooLong { len: 500, max: 100 },
        YamlError::MappingTooLarge {
            count: 500,
            max: 100,
        },
        YamlError::UnknownField {
            field: Box::from("extra"),
        },
        YamlError::EmptySource,
        YamlError::MissingField { field: "steps" },
        YamlError::FieldShape {
            field: "id",
            expected: "string",
        },
        YamlError::ParseError {
            line: 1,
            reason: Box::from("unexpected token"),
        },
        YamlError::ForbiddenFeature {
            detail: "custom YAML tag",
        },
        YamlError::LegacyPrimitiveDeprecated {
            name: String::from("old_op"),
            replacement: String::from("new_op"),
        },
    ]
}

#[test]
fn all_21_yaml_error_variants_enumerated() {
    let variants = all_yaml_error_variants();
    assert_eq!(variants.len(), 21, "Expected 21 YamlError variants");
}

#[test]
fn every_yaml_error_code_is_registered_symbolic_code() {
    for error in &all_yaml_error_variants() {
        let code = error.code();
        // Verify the code can be reconstructed from its string representation
        let reconstructed = SymbolicCode::from_static(code.as_str());
        assert!(
            reconstructed.is_some(),
            "YamlError::code() returned '{}' which is not a registered SymbolicCode. \
             Variant: {:?}",
            code.as_str(),
            error
        );
        // Verify the code appears in CODE_REGISTRY
        assert!(
            CODE_REGISTRY.iter().any(|e| e.symbolic == code.as_str()),
            "YamlError code '{}' not found in CODE_REGISTRY. Variant: {:?}",
            code.as_str(),
            error
        );
    }
}

#[test]
fn yaml_error_codes_cover_expected_distinct_symbols() {
    let mut codes: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for error in &all_yaml_error_variants() {
        codes.insert(error.code().as_str());
    }
    let expected: &[&str] = &[
        "DUPLICATE_KEY",
        "FORBIDDEN_YAML_FEATURE",
        "UNKNOWN_TOP_LEVEL_FIELD",
        "MISSING_REQUIRED_FIELD",
        "PAYLOAD_TOO_LARGE",
        "LIMIT_EXCEEDED",
        "UNSUPPORTED_TRIGGER",
        "TYPE_MISMATCH",
    ];
    for name in expected {
        assert!(
            codes.contains(name),
            "Expected code '{}' to be covered by YamlError variants, got codes: {:?}",
            name,
            codes
        );
    }
}

#[test]
fn yaml_error_duplicate_key_maps_correctly() {
    let err = YamlError::DuplicateKey {
        key: Box::from("steps"),
    };
    assert_eq!(err.code().as_str(), "DUPLICATE_KEY");
}

#[test]
fn yaml_error_forbidden_yaml_feature_variants() {
    // All these variants should map to FORBIDDEN_YAML_FEATURE
    let forbidden_variants: &[YamlError] = &[
        YamlError::ForbiddenFeature {
            detail: "custom tag",
        },
        YamlError::AnchorAliasMerge,
        YamlError::CustomTag {
            tag: Box::from("!secret"),
        },
        YamlError::BinaryScalar,
        YamlError::MultipleDocuments { count: 3 },
        YamlError::AmbiguousScalar {
            scalar: Box::from("yes"),
        },
        YamlError::UnsupportedFeature { feature: "anchors" },
        YamlError::ParseError {
            line: 1,
            reason: Box::from("bad"),
        },
        YamlError::LegacyPrimitiveDeprecated {
            name: String::from("old"),
            replacement: String::from("new"),
        },
    ];
    for err in forbidden_variants {
        assert_eq!(
            err.code().as_str(),
            "FORBIDDEN_YAML_FEATURE",
            "Expected FORBIDDEN_YAML_FEATURE for {:?}",
            err
        );
    }
}
