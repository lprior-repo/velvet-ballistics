#![forbid(unsafe_code)]
//! PO-006: Kani harness verifying every production `YamlError` variant
//! maps to a registered code name in the production `CODE_REGISTRY`.
//!
//! Uses the actual production types:
//!   - `crate::YamlError` (production enum)
//!   - `YamlError::symbolic_code_name()` → `&'static str`
//!   - `vb_core::is_registered_symbolic()` to verify registry membership
//!
//! REPAIR-9 (F-R8-001): Replaced model `enum YamlError` +
//! `code_name()` with production `YamlError` + `symbolic_code_name()`.
//! The `symbolic_code_name()` method is defined on the production
//! `YamlError` type in `crates/vb_yaml/src/error.rs`.
//!
//! Bound: 20 variants (split into 2 sub-harnesses, unwind=160 each)
//! to mitigate `iter().find()` over CODE_REGISTRY (157 entries).

use crate::YamlError;

/// Verify using the production registry function.
fn is_registered(name: &str) -> bool {
    vb_core::is_registered_symbolic(name)
}

/// Sub-harness 1 (10 variants): Duplicate key, forbidden features,
/// anchors, tags, source/limit errors.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(160)]
fn kani_yaml_error_code_registered_1() {
    let variants: [YamlError; 10] = [
        YamlError::DuplicateKey {
            key: Box::from("test_key"),
        },
        YamlError::ForbiddenFeature { detail: "test" },
        YamlError::AnchorAliasMerge,
        YamlError::CustomTag {
            tag: Box::from("!test"),
        },
        YamlError::BinaryScalar,
        YamlError::MultipleDocuments { count: 2 },
        YamlError::AmbiguousScalar {
            scalar: Box::from("yes"),
        },
        YamlError::SourceTooLarge {
            size: 9999,
            max: 1000,
        },
        YamlError::NestingTooDeep {
            depth: 100,
            max: 50,
        },
        YamlError::NodeLimitExceeded {
            count: 9999,
            max: 1000,
        },
    ];
    for (i, variant) in variants.iter().enumerate() {
        let name = variant.symbolic_code_name();
        assert!(
            is_registered(name),
            "YamlError variant {}: code name '{}' must be registered",
            i,
            name
        );
        assert!(
            !name.is_empty(),
            "YamlError variant {}: code must not be empty",
            i
        );
    }
}

/// Sub-harness 2 (10 variants): Length limits, missing/unknown fields,
/// parse errors, unsupported features.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(160)]
fn kani_yaml_error_code_registered_2() {
    let variants: [YamlError; 10] = [
        YamlError::ScalarTooLong {
            len: 9999,
            max: 1000,
        },
        YamlError::SequenceTooLong {
            len: 9999,
            max: 1000,
        },
        YamlError::MappingTooLarge {
            count: 9999,
            max: 1000,
        },
        YamlError::UnknownField {
            field: Box::from("unknown"),
        },
        YamlError::EmptySource,
        YamlError::MissingField {
            field: "required_field",
        },
        YamlError::FieldShape {
            field: "test",
            expected: "mapping",
        },
        YamlError::ParseError {
            line: 1,
            reason: Box::from("syntax error"),
        },
        YamlError::UnsupportedFeature {
            feature: "legacy",
        },
        YamlError::UnsupportedTrigger {
            trigger: "unknown",
        },
    ];
    for (i, variant) in variants.iter().enumerate() {
        let name = variant.symbolic_code_name();
        assert!(
            is_registered(name),
            "YamlError variant {}: code name '{}' must be registered",
            i,
            name
        );
        assert!(
            !name.is_empty(),
            "YamlError variant {}: code must not be empty",
            i
        );
    }
}
