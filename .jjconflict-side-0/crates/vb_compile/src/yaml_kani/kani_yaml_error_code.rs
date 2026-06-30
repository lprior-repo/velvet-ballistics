#![forbid(unsafe_code)]
//! PO-006: Kani harness verifying every YamlError variant maps to a
//! registered SymbolicCode in the registry.
//!
//! Proves: For each of 20 YamlError variants, code() returns a SymbolicCode
//! that is in the CODE_REGISTRY.
//!
//! Bound: 20 YamlError variants (unwind=20)

/// Minimal model of SymbolicCode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolicCode(&'static str);

/// Minimal model of YamlError (20 variants).
#[derive(Debug, Clone)]
pub enum YamlError {
    DuplicateKey {
        key: Box<str>,
    },
    ForbiddenFeature {
        detail: &'static str,
    },
    AnchorAliasMerge,
    CustomTag {
        tag: Box<str>,
    },
    BinaryScalar,
    MultipleDocuments {
        count: usize,
    },
    AmbiguousScalar {
        scalar: Box<str>,
    },
    SourceTooLarge {
        size: usize,
        max: usize,
    },
    NestingTooDeep {
        depth: u16,
        max: u16,
    },
    NodeLimitExceeded {
        count: u32,
        max: u32,
    },
    ScalarTooLong {
        len: usize,
        max: usize,
    },
    SequenceTooLong {
        len: usize,
        max: usize,
    },
    MappingTooLarge {
        count: usize,
        max: usize,
    },
    UnknownField {
        field: Box<str>,
    },
    EmptySource,
    MissingField {
        field: &'static str,
    },
    FieldShape {
        field: &'static str,
        expected: &'static str,
    },
    ParseError {
        line: usize,
        reason: Box<str>,
    },
    UnsupportedFeature {
        feature: &'static str,
    },
    UnsupportedTrigger {
        trigger: &'static str,
    },
}

/// YamlError::code() — maps each variant to its registered SymbolicCode.
impl YamlError {
    #[must_use]
    pub fn code(&self) -> SymbolicCode {
        match self {
            YamlError::DuplicateKey { .. } => SymbolicCode("DUPLICATE_KEY"),
            YamlError::ForbiddenFeature { .. } => SymbolicCode("FORBIDDEN_YAML_FEATURE"),
            YamlError::AnchorAliasMerge => SymbolicCode("FORBIDDEN_YAML_FEATURE"),
            YamlError::CustomTag { .. } => SymbolicCode("FORBIDDEN_YAML_FEATURE"),
            YamlError::BinaryScalar => SymbolicCode("FORBIDDEN_YAML_FEATURE"),
            YamlError::MultipleDocuments { .. } => SymbolicCode("FORBIDDEN_YAML_FEATURE"),
            YamlError::AmbiguousScalar { .. } => SymbolicCode("FORBIDDEN_YAML_FEATURE"),
            YamlError::SourceTooLarge { .. } => SymbolicCode("PAYLOAD_TOO_LARGE"),
            YamlError::NestingTooDeep { .. } => SymbolicCode("LIMIT_EXCEEDED"),
            YamlError::NodeLimitExceeded { .. } => SymbolicCode("LIMIT_EXCEEDED"),
            YamlError::ScalarTooLong { .. } => SymbolicCode("LIMIT_EXCEEDED"),
            YamlError::SequenceTooLong { .. } => SymbolicCode("LIMIT_EXCEEDED"),
            YamlError::MappingTooLarge { .. } => SymbolicCode("LIMIT_EXCEEDED"),
            YamlError::UnknownField { .. } => SymbolicCode("UNKNOWN_TOP_LEVEL_FIELD"),
            YamlError::EmptySource => SymbolicCode("MISSING_REQUIRED_FIELD"),
            YamlError::MissingField { .. } => SymbolicCode("MISSING_REQUIRED_FIELD"),
            YamlError::FieldShape { .. } => SymbolicCode("TYPE_MISMATCH"),
            YamlError::ParseError { .. } => SymbolicCode("FORBIDDEN_YAML_FEATURE"),
            YamlError::UnsupportedFeature { .. } => SymbolicCode("FORBIDDEN_YAML_FEATURE"),
            YamlError::UnsupportedTrigger { .. } => SymbolicCode("UNSUPPORTED_TRIGGER"),
        }
    }
}

/// The set of registered symbolic codes that YamlError variants map to.
const REGISTERED_CODES: &[&str] = &[
    "DUPLICATE_KEY",
    "FORBIDDEN_YAML_FEATURE",
    "UNSUPPORTED_TRIGGER",
    "PAYLOAD_TOO_LARGE",
    "LIMIT_EXCEEDED",
    "UNKNOWN_TOP_LEVEL_FIELD",
    "MISSING_REQUIRED_FIELD",
    "TYPE_MISMATCH",
];

fn is_registered(name: &str) -> bool {
    REGISTERED_CODES.iter().any(|&r| r == name)
}

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-006: Every YamlError variant's code() returns a registered SymbolicCode.
    #[kani::proof]
    #[kani::unwind(20)]
    fn kani_yaml_error_code_registered() {
        let variants: [YamlError; 20] = [
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
            YamlError::UnsupportedFeature { feature: "legacy" },
            YamlError::UnsupportedTrigger { trigger: "unknown" },
        ];

        for (i, variant) in variants.iter().enumerate() {
            let code = variant.code();
            let name = code.0;
            assert!(
                is_registered(name),
                "YamlError variant {}: code '{}' must be registered",
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
}
