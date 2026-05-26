#![forbid(unsafe_code)]

//! YAML parsing error types.

use crate::source_map::SourceSpan;
use thiserror::Error;
use vb_core::diagnostic::{HasSymbolicCode, SymbolicCode};

/// YAML parsing error type.
///
/// Parse-level variants carry an optional [`SourceSpan`] extracted from the
/// parser event stream. Limit-exceeded variants that apply to the whole
/// document (`SourceTooLarge`, `NestingTooDeep`, `NodeLimitExceeded`,
/// `EmptySource`) omit the span.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum YamlError {
    #[error("unsupported trigger: {trigger}")]
    UnsupportedTrigger {
        trigger: &'static str,
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },

    #[error("unsupported YAML feature: {feature}")]
    UnsupportedFeature {
        feature: &'static str,
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },

    #[error("duplicate key found: {key}")]
    DuplicateKey {
        key: Box<str>,
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },

    #[error("anchor/alias/merge key rejected")]
    AnchorAliasMerge {
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },

    #[error("custom tag rejected: {tag}")]
    CustomTag {
        tag: Box<str>,
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },

    #[error("binary scalar rejected")]
    BinaryScalar {
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },

    #[error("multiple documents rejected")]
    MultipleDocuments {
        count: usize,
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },

    #[error("YAML 1.1 ambiguous scalar rejected: {scalar}")]
    AmbiguousScalar {
        scalar: Box<str>,
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },

    #[error("source too large: {size} bytes, max {max}")]
    SourceTooLarge { size: usize, max: usize },

    #[error("nesting too deep: {depth}, max {max}")]
    NestingTooDeep { depth: u16, max: u16 },

    #[error("node limit exceeded: {count}, max {max}")]
    NodeLimitExceeded { count: u32, max: u32 },

    #[error("scalar too long: {len} bytes, max {max}")]
    ScalarTooLong {
        len: usize,
        max: usize,
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },

    #[error("sequence too long: {len}, max {max}")]
    SequenceTooLong {
        len: usize,
        max: usize,
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },

    #[error("mapping too large: {count} entries, max {max}")]
    MappingTooLarge {
        count: usize,
        max: usize,
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },

    #[error("unknown field: {field}")]
    UnknownField {
        field: Box<str>,
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },

    #[error("empty source")]
    EmptySource,

    #[error("missing required field: {field}")]
    MissingField {
        field: &'static str,
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },

    #[error("field shape error: {field} expected {expected}")]
    FieldShape {
        field: &'static str,
        expected: &'static str,
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },

    #[error("parse error at line {line}: {reason}")]
    ParseError {
        line: usize,
        reason: Box<str>,
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },

    #[error("forbidden YAML feature: {detail}")]
<<<<<<< HEAD
    ForbiddenFeature { detail: &'static str },

    #[error("legacy primitive not supported: {primitive} (use {canonical} instead)")]
    LegacyPrimitive {
        primitive: &'static str,
        canonical: &'static str,
=======
    ForbiddenFeature {
        detail: &'static str,
        #[doc(hidden)]
        span: Option<SourceSpan>,
>>>>>>> landing/vb-xi2f.9
    },
}

impl YamlError {
<<<<<<< HEAD
    /// Returns the stable symbolic diagnostic code for this YAML error.
    ///
    /// Mapping matches error-taxonomy §2.3:
    /// - `DUPLICATE_KEY`: DuplicateKey
    /// - `FORBIDDEN_YAML_FEATURE`: ForbiddenFeature, AnchorAliasMerge, CustomTag, BinaryScalar, AmbiguousScalar, UnsupportedFeature, MultipleDocuments, ParseError, LegacyPrimitive
    /// - `UNSUPPORTED_TRIGGER`: UnsupportedTrigger
    /// - `PAYLOAD_TOO_LARGE`: SourceTooLarge
    /// - `LIMIT_EXCEEDED`: NestingTooDeep, NodeLimitExceeded, ScalarTooLong, SequenceTooLong, MappingTooLarge
    /// - `UNKNOWN_TOP_LEVEL_FIELD`: UnknownField
    /// - `MISSING_REQUIRED_FIELD`: EmptySource, MissingField
    /// - `TYPE_MISMATCH`: FieldShape
    #[must_use]
    pub fn code(&self) -> SymbolicCode {
        let s: &'static str = match self {
            Self::DuplicateKey { .. } => "DUPLICATE_KEY",
            Self::ForbiddenFeature { .. }
            | Self::AnchorAliasMerge
            | Self::CustomTag { .. }
            | Self::BinaryScalar
            | Self::MultipleDocuments { .. }
            | Self::AmbiguousScalar { .. }
            | Self::UnsupportedFeature { .. }
            | Self::ParseError { .. }
            | Self::LegacyPrimitive { .. } => "FORBIDDEN_YAML_FEATURE",
            Self::UnsupportedTrigger { .. } => "UNSUPPORTED_TRIGGER",
            Self::SourceTooLarge { .. } => "PAYLOAD_TOO_LARGE",
            Self::NestingTooDeep { .. }
            | Self::NodeLimitExceeded { .. }
            | Self::ScalarTooLong { .. }
            | Self::SequenceTooLong { .. }
            | Self::MappingTooLarge { .. } => "LIMIT_EXCEEDED",
            Self::UnknownField { .. } => "UNKNOWN_TOP_LEVEL_FIELD",
            Self::EmptySource | Self::MissingField { .. } => "MISSING_REQUIRED_FIELD",
            Self::FieldShape { .. } => "TYPE_MISMATCH",
        };
        // Safety invariant: all YamlError symbolic codes are registered
        // in vb_core::CODE_REGISTRY (verified by Kani).
        if let Some(code) = SymbolicCode::from_static(s) {
            return code;
        }
        // Unreachable: all match arms use registered symbolic names.
        SymbolicCode::INTERNAL_INVARIANT
    }
}

impl HasSymbolicCode for YamlError {
    fn symbolic_code(&self) -> SymbolicCode {
        self.code()
    }
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
=======
    /// Returns the source span associated with this error, if any.
    #[must_use]
    pub fn span(&self) -> Option<SourceSpan> {
        match self {
            Self::UnsupportedTrigger { span, .. }
            | Self::UnsupportedFeature { span, .. }
            | Self::DuplicateKey { span, .. }
            | Self::AnchorAliasMerge { span }
            | Self::CustomTag { span, .. }
            | Self::BinaryScalar { span }
            | Self::MultipleDocuments { span, .. }
            | Self::AmbiguousScalar { span, .. }
            | Self::ScalarTooLong { span, .. }
            | Self::SequenceTooLong { span, .. }
            | Self::MappingTooLarge { span, .. }
            | Self::UnknownField { span, .. }
            | Self::MissingField { span, .. }
            | Self::FieldShape { span, .. }
            | Self::ParseError { span, .. }
            | Self::ForbiddenFeature { span, .. } => *span,
            Self::SourceTooLarge { .. }
            | Self::NestingTooDeep { .. }
            | Self::NodeLimitExceeded { .. }
            | Self::EmptySource => None,
        }
>>>>>>> landing/vb-xi2f.9
    }
}

/// Alias for results using [`YamlError`].
pub type YamlResult<T> = Result<T, YamlError>;
