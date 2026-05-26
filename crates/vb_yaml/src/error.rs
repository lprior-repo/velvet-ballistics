#![forbid(unsafe_code)]

//! YAML parsing error types.

use crate::source_map::SourceSpan;
use thiserror::Error;

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
    ForbiddenFeature {
        detail: &'static str,
        #[doc(hidden)]
        span: Option<SourceSpan>,
    },
}

impl YamlError {
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
    }
}

/// Alias for results using [`YamlError`].
pub type YamlResult<T> = Result<T, YamlError>;
