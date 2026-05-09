#![forbid(unsafe_code)]
//! Typed YAML event stream wrapping saphyr-parser events.
//!
//! This module provides a [`YamlEvent`] enum that mirrors the saphyr-parser
//! event types but owns all data and carries source location information.

/// Scalar style preserved from the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarStyle {
    /// Plain (unquoted) scalar.
    Plain,
    /// Single-quoted scalar.
    SingleQuoted,
    /// Double-quoted scalar.
    DoubleQuoted,
    /// Literal block scalar (`|`).
    Literal,
    /// Folded block scalar (`>`).
    Folded,
}

impl ScalarStyle {
    /// Convert from saphyr-parser's ScalarStyle.
    pub(crate) fn from_parser(style: saphyr_parser::ScalarStyle) -> Self {
        match style {
            saphyr_parser::ScalarStyle::Plain => Self::Plain,
            saphyr_parser::ScalarStyle::SingleQuoted => Self::SingleQuoted,
            saphyr_parser::ScalarStyle::DoubleQuoted => Self::DoubleQuoted,
            saphyr_parser::ScalarStyle::Literal => Self::Literal,
            saphyr_parser::ScalarStyle::Folded => Self::Folded,
        }
    }
}

/// Source location for an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventSpan {
    /// Byte offset where the event starts.
    pub start: usize,
    /// Byte offset where the event ends.
    pub end: usize,
    /// One-indexed source line.
    pub line: usize,
    /// One-indexed source column.
    pub column: usize,
}

impl EventSpan {
    /// Creates a span from a saphyr-parser Span.
    pub(crate) fn from_parser_span(span: saphyr_parser::Span) -> Self {
        Self {
            start: span.start.index(),
            end: span.end.index(),
            line: span.start.line(),
            column: span.start.col(),
        }
    }
}

/// A typed YAML event carrying owned data and source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YamlEvent {
    /// Start of the YAML stream.
    StreamStart {
        /// Source span.
        span: EventSpan,
    },
    /// End of the YAML stream.
    StreamEnd {
        /// Source span.
        span: EventSpan,
    },
    /// Start of a YAML document.
    DocumentStart {
        /// Whether the document start was explicit (`---`).
        explicit: bool,
        /// Source span.
        span: EventSpan,
    },
    /// End of a YAML document.
    DocumentEnd {
        /// Source span.
        span: EventSpan,
    },
    /// An alias reference.
    Alias {
        /// Anchor ID the alias refers to.
        anchor_id: usize,
        /// Source span.
        span: EventSpan,
    },
    /// A scalar value.
    Scalar {
        /// The scalar value.
        value: Box<str>,
        /// The scalar style.
        style: ScalarStyle,
        /// Anchor ID (0 if none).
        anchor_id: usize,
        /// Tag string, if present.
        tag: Option<Box<str>>,
        /// Source span.
        span: EventSpan,
    },
    /// Start of a sequence.
    SequenceStart {
        /// Anchor ID (0 if none).
        anchor_id: usize,
        /// Tag string, if present.
        tag: Option<Box<str>>,
        /// Source span.
        span: EventSpan,
    },
    /// End of a sequence.
    SequenceEnd {
        /// Source span.
        span: EventSpan,
    },
    /// Start of a mapping.
    MappingStart {
        /// Anchor ID (0 if none).
        anchor_id: usize,
        /// Tag string, if present.
        tag: Option<Box<str>>,
        /// Source span.
        span: EventSpan,
    },
    /// End of a mapping.
    MappingEnd {
        /// Source span.
        span: EventSpan,
    },
}

impl YamlEvent {
    /// Returns the source span for this event.
    #[must_use]
    pub fn span(&self) -> EventSpan {
        match self {
            Self::StreamStart { span }
            | Self::StreamEnd { span }
            | Self::DocumentStart { span, .. }
            | Self::DocumentEnd { span }
            | Self::Alias { span, .. }
            | Self::Scalar { span, .. }
            | Self::SequenceStart { span, .. }
            | Self::SequenceEnd { span }
            | Self::MappingStart { span, .. }
            | Self::MappingEnd { span } => *span,
        }
    }

    /// Returns the anchor ID if this event carries one, else 0.
    #[must_use]
    pub fn anchor_id(&self) -> usize {
        match self {
            Self::Scalar { anchor_id, .. }
            | Self::SequenceStart { anchor_id, .. }
            | Self::MappingStart { anchor_id, .. }
            | Self::Alias { anchor_id, .. } => *anchor_id,
            _ => 0,
        }
    }

    /// Returns the tag string if this event carries one.
    #[must_use]
    pub fn tag(&self) -> Option<&str> {
        match self {
            Self::Scalar { tag, .. }
            | Self::SequenceStart { tag, .. }
            | Self::MappingStart { tag, .. } => tag.as_deref(),
            _ => None,
        }
    }

    /// Returns true if this event is a document start.
    #[must_use]
    pub fn is_document_start(&self) -> bool {
        matches!(self, Self::DocumentStart { .. })
    }

    /// Returns true if this event is an alias.
    #[must_use]
    pub fn is_alias(&self) -> bool {
        matches!(self, Self::Alias { .. })
    }

    /// Returns the scalar value if this is a scalar event.
    #[must_use]
    pub fn as_scalar(&self) -> Option<&str> {
        match self {
            Self::Scalar { value, .. } => Some(value.as_ref()),
            _ => None,
        }
    }
}
