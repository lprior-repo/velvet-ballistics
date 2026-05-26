#![forbid(unsafe_code)]

//! YAML parsing error types.

use thiserror::Error;
use vb_core::diagnostic::{HasSymbolicCode, SymbolicCode};

/// YAML parsing error type.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum YamlError {
    #[error("unsupported trigger: {trigger}")]
    UnsupportedTrigger { trigger: &'static str },

    #[error("unsupported YAML feature: {feature}")]
    UnsupportedFeature { feature: &'static str },

    #[error("duplicate key found: {key}")]
    DuplicateKey { key: Box<str> },

    #[error("anchor/alias/merge key rejected")]
    AnchorAliasMerge,

    #[error("custom tag rejected: {tag}")]
    CustomTag { tag: Box<str> },

    #[error("binary scalar rejected")]
    BinaryScalar,

    #[error("multiple documents rejected")]
    MultipleDocuments { count: usize },

    #[error("YAML 1.1 ambiguous scalar rejected: {scalar}")]
    AmbiguousScalar { scalar: Box<str> },

    #[error("source too large: {size} bytes, max {max}")]
    SourceTooLarge { size: usize, max: usize },

    #[error("nesting too deep: {depth}, max {max}")]
    NestingTooDeep { depth: u16, max: u16 },

    #[error("node limit exceeded: {count}, max {max}")]
    NodeLimitExceeded { count: u32, max: u32 },

    #[error("scalar too long: {len} bytes, max {max}")]
    ScalarTooLong { len: usize, max: usize },

    #[error("sequence too long: {len}, max {max}")]
    SequenceTooLong { len: usize, max: usize },

    #[error("mapping too large: {count} entries, max {max}")]
    MappingTooLarge { count: usize, max: usize },

    #[error("unknown field: {field}")]
    UnknownField { field: Box<str> },

    #[error("empty source")]
    EmptySource,

    #[error("missing required field: {field}")]
    MissingField { field: &'static str },

    #[error("field shape error: {field} expected {expected}")]
    FieldShape {
        field: &'static str,
        expected: &'static str,
    },

    #[error("parse error at line {line}: {reason}")]
    ParseError { line: usize, reason: Box<str> },

    #[error("forbidden YAML feature: {detail}")]
    ForbiddenFeature { detail: &'static str },
}

impl HasSymbolicCode for YamlError {
    /// Returns the [`SymbolicCode`] for this YAML error variant.
    ///
    /// Every variant maps to a code name registered in
    /// `vb_core::CODE_REGISTRY`. Multiple YAML-specific variants
    /// (e.g., `AnchorAliasMerge`, `CustomTag`, `BinaryScalar`,
    /// `AmbiguousScalar`, `ParseError`, `UnsupportedFeature`) share the
    /// `"FORBIDDEN_YAML_FEATURE"` code because they all represent
    /// YAML constructs rejected by the strict profile.
    fn symbolic_code(&self) -> SymbolicCode {
        let s: &'static str = match self {
            YamlError::DuplicateKey { .. } => "DUPLICATE_KEY",
            YamlError::ForbiddenFeature { .. }
            | YamlError::AnchorAliasMerge
            | YamlError::CustomTag { .. }
            | YamlError::BinaryScalar
            | YamlError::MultipleDocuments { .. }
            | YamlError::AmbiguousScalar { .. }
            | YamlError::ParseError { .. }
            | YamlError::UnsupportedFeature { .. } => "FORBIDDEN_YAML_FEATURE",
            YamlError::SourceTooLarge { .. } => "PAYLOAD_TOO_LARGE",
            YamlError::NestingTooDeep { .. }
            | YamlError::NodeLimitExceeded { .. }
            | YamlError::ScalarTooLong { .. }
            | YamlError::SequenceTooLong { .. }
            | YamlError::MappingTooLarge { .. } => "LIMIT_EXCEEDED",
            YamlError::UnknownField { .. } => "UNKNOWN_TOP_LEVEL_FIELD",
            YamlError::EmptySource
            | YamlError::MissingField { .. } => "MISSING_REQUIRED_FIELD",
            YamlError::FieldShape { .. } => "TYPE_MISMATCH",
            YamlError::UnsupportedTrigger { .. } => "UNSUPPORTED_TRIGGER",
        };
        SymbolicCode::from_static(s).unwrap_or(SymbolicCode::INTERNAL_INVARIANT)
    }
}

impl YamlError {
    /// Returns the symbolic diagnostic code name for this error variant.
    ///
    /// Compatibility wrapper for callers that need a `&'static str`.
    /// Prefer [`HasSymbolicCode::symbolic_code`] for new code.
    #[must_use]
    pub fn symbolic_code_name(&self) -> &'static str {
        self.symbolic_code().as_str()
    }
}

/// Alias for results using [`YamlError`].
pub type YamlResult<T> = Result<T, YamlError>;
