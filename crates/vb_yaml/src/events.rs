//! Typed YAML event stream wrapping saphyr-parser events.
//!
//! This module provides a [`YamlEvent`] enum that mirrors the saphyr-parser
//! event types but owns all data and carries source location information.

use crate::{YamlError, YamlResult};

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
            | Self::MappingStart { anchor_id, .. } => *anchor_id,
            Self::Alias { anchor_id, .. } => *anchor_id,
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

/// Collect all YAML events from the given source text.
///
/// Returns an owned vector of [`YamlEvent`] values.
pub(crate) fn collect_events(text: &str) -> YamlResult<Vec<YamlEvent>> {
    let mut parser = saphyr_parser::Parser::new_from_str(text);
    let mut events = Vec::new();

    while let Some(result) = parser.next_event() {
        let (event, span) = result.map_err(|e| YamlError::ParseError {
            line: e.marker().line(),
            reason: e.info().into(),
        })?;
        events.push(convert_event(event, span));
    }

    Ok(events)
}

/// Convert a saphyr-parser event into our owned YamlEvent.
pub(crate) fn convert_event(
    event: saphyr_parser::Event<'_>,
    span: saphyr_parser::Span,
) -> YamlEvent {
    let span = EventSpan::from_parser_span(span);
    match event {
        saphyr_parser::Event::StreamStart => YamlEvent::StreamStart { span },
        saphyr_parser::Event::StreamEnd => YamlEvent::StreamEnd { span },
        saphyr_parser::Event::DocumentStart(explicit) => {
            YamlEvent::DocumentStart { explicit, span }
        }
        saphyr_parser::Event::DocumentEnd => YamlEvent::DocumentEnd { span },
        saphyr_parser::Event::Alias(anchor_id) => YamlEvent::Alias { anchor_id, span },
        saphyr_parser::Event::Scalar(value, style, anchor_id, tag) => YamlEvent::Scalar {
            value: value.into(),
            style: ScalarStyle::from_parser(style),
            anchor_id,
            tag: tag.map(|t| format_tag(&t)),
            span,
        },
        saphyr_parser::Event::SequenceStart(anchor_id, tag) => YamlEvent::SequenceStart {
            anchor_id,
            tag: tag.map(|t| format_tag(&t)),
            span,
        },
        saphyr_parser::Event::SequenceEnd => YamlEvent::SequenceEnd { span },
        saphyr_parser::Event::MappingStart(anchor_id, tag) => YamlEvent::MappingStart {
            anchor_id,
            tag: tag.map(|t| format_tag(&t)),
            span,
        },
        saphyr_parser::Event::MappingEnd => YamlEvent::MappingEnd { span },
        saphyr_parser::Event::Nothing => YamlEvent::StreamEnd { span },
    }
}

/// Format a saphyr-parser Tag into a display string.
fn format_tag(tag: &saphyr_parser::Tag) -> Box<str> {
    format!("{}{}", tag.handle, tag.suffix).into_boxed_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! collect_ok {
        ($yaml:expr) => {
            match collect_events($yaml) {
                Ok(value) => value,
                Err(error) => {
                    assert!(false, "event collection failed: {error}");
                    return;
                }
            }
        };
    }

    #[test]
    fn collect_events_empty_mapping() {
        let yaml = "key: value\n";
        let events = collect_ok!(yaml);
        assert!(events.len() >= 6);
    }

    #[test]
    fn scalar_event_carries_value() {
        let yaml = "hello\n";
        let events = collect_ok!(yaml);
        let scalar = events.iter().find_map(|e| match e {
            YamlEvent::Scalar { value, .. } => Some(value.clone()),
            _ => None,
        });
        assert_eq!(scalar.as_deref(), Some("hello"));
    }

    #[test]
    fn document_start_is_explicit() {
        let yaml = "---\nkey: value\n";
        let events = collect_ok!(yaml);
        let doc_start = events.iter().find(|e| e.is_document_start());
        assert!(matches!(
            doc_start,
            Some(YamlEvent::DocumentStart { explicit: true, .. })
        ));
    }

    #[test]
    fn anchor_id_is_zero_by_default() {
        let yaml = "a: b\n";
        let events = collect_ok!(yaml);
        for event in &events {
            assert_eq!(event.anchor_id(), 0);
        }
    }

    #[test]
    fn span_has_nonzero_line_for_content() {
        let yaml = "a: b\n";
        let events = collect_ok!(yaml);
        let scalar = events.iter().find(|e| e.as_scalar() == Some("a"));
        let Some(scalar) = scalar else {
            assert!(false, "missing scalar");
            return;
        };
        assert!(scalar.span().line > 0);
    }
}
