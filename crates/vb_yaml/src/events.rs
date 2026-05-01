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

    fn assertion_failed(message: std::fmt::Arguments<'_>) -> bool {
        let _ = message;
        false
    }

    macro_rules! fail_assert {
        ($($arg:tt)*) => {
            assert!(assertion_failed(format_args!($($arg)*)), $($arg)*)
        };
    }

    macro_rules! collect_ok {
        ($yaml:expr) => {
            match collect_events($yaml) {
                Ok(value) => value,
                Err(error) => {
                    fail_assert!("event collection failed: {error}");
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
            fail_assert!("missing scalar");
            return;
        };
        assert!(scalar.span().line > 0);
    }

    // -----------------------------------------------------------------------
    // AST Node Behavior Tests (YamlEvent accessors)
    // -----------------------------------------------------------------------

    #[test]
    fn typed_node_scalar_returns_value_via_as_scalar() {
        // Given: YAML with a scalar value "hello"
        let yaml = "hello\n";
        let events = collect_ok!(yaml);
        // When: finding the scalar event and calling as_scalar
        let scalar_event = events.iter().find(|e| e.as_scalar().is_some());
        let Some(evt) = scalar_event else {
            fail_assert!("missing scalar event");
            return;
        };
        // Then: as_scalar returns Some with exact value "hello"
        assert_eq!(evt.as_scalar(), Some("hello"));
    }

    #[test]
    fn typed_node_scalar_returns_none_for_non_scalar() {
        // Given: a StreamStart event (non-scalar)
        let yaml = "a: 1\n";
        let events = collect_ok!(yaml);
        let stream_start = events
            .iter()
            .find(|e| matches!(e, YamlEvent::StreamStart { .. }));
        let Some(evt) = stream_start else {
            fail_assert!("missing stream start");
            return;
        };
        // When: calling as_scalar on non-scalar event
        // Then: returns None
        assert_eq!(evt.as_scalar(), None);
    }

    #[test]
    fn typed_node_mapping_start_anchor_id_returns_zero() {
        // Given: YAML with an unanchored mapping
        let yaml = "a: 1\n";
        let events = collect_ok!(yaml);
        let mapping_start = events
            .iter()
            .find(|e| matches!(e, YamlEvent::MappingStart { .. }));
        let Some(evt) = mapping_start else {
            fail_assert!("missing mapping start");
            return;
        };
        // When: calling anchor_id
        // Then: returns 0
        assert_eq!(evt.anchor_id(), 0);
    }

    #[test]
    fn typed_node_is_document_start_for_doc_start_event() {
        // Given: YAML with explicit document start
        let yaml = "---\nkey: value\n";
        let events = collect_ok!(yaml);
        let doc_start = events.iter().find(|e| e.is_document_start());
        // When: checking is_document_start
        // Then: found a doc start and it returns true
        let Some(evt) = doc_start else {
            fail_assert!("missing document start");
            return;
        };
        assert!(evt.is_document_start());
    }

    #[test]
    fn typed_node_is_document_start_for_non_doc_start() {
        // Given: a scalar event
        let yaml = "hello\n";
        let events = collect_ok!(yaml);
        let scalar = events.iter().find(|e| e.as_scalar().is_some());
        let Some(evt) = scalar else {
            fail_assert!("missing scalar");
            return;
        };
        // When: calling is_document_start
        // Then: returns false
        assert!(!evt.is_document_start());
    }

    #[test]
    fn typed_node_is_alias_returns_true_for_alias() {
        // Given: YAML events produced from text with an alias
        let yaml = "a: &anc value\nb: *anc\n";
        let events = collect_ok!(yaml);
        let alias_event = events.iter().find(|e| e.is_alias());
        // When: checking is_alias
        let Some(evt) = alias_event else {
            fail_assert!("missing alias event");
            return;
        };
        // Then: returns true
        assert!(evt.is_alias());
    }

    #[test]
    fn typed_node_is_alias_returns_false_for_scalar() {
        // Given: YAML with only scalars
        let yaml = "a: b\n";
        let events = collect_ok!(yaml);
        let scalar = events.iter().find(|e| e.as_scalar().is_some());
        let Some(evt) = scalar else {
            fail_assert!("missing scalar");
            return;
        };
        // When: checking is_alias
        // Then: returns false
        assert!(!evt.is_alias());
    }

    #[test]
    fn typed_node_span_returns_correct_line_column() {
        // Given: YAML on a single line
        let yaml = "a: b\n";
        let events = collect_ok!(yaml);
        let scalar = events.iter().find(|e| e.as_scalar() == Some("a"));
        let Some(evt) = scalar else {
            fail_assert!("missing scalar");
            return;
        };
        // When: getting the span
        let span = evt.span();
        // Then: line > 0 (column may be 0 depending on parser)
        assert!(span.line > 0);
    }

    #[test]
    fn typed_node_tag_returns_none_for_untagged() {
        // Given: YAML with no tags
        let yaml = "a: b\n";
        let events = collect_ok!(yaml);
        let scalar = events.iter().find(|e| e.as_scalar().is_some());
        let Some(evt) = scalar else {
            fail_assert!("missing scalar");
            return;
        };
        // When: calling tag()
        // Then: returns None
        assert_eq!(evt.tag(), None);
    }

    #[test]
    fn typed_node_tag_returns_some_for_tagged() {
        // Given: YAML with a custom tag
        let yaml = "a: !mytag b\n";
        let events = collect_ok!(yaml);
        let scalar = events.iter().find_map(|e| match e {
            YamlEvent::Scalar { tag: Some(_), .. } => Some(e.clone()),
            _ => None,
        });
        let Some(evt) = scalar else {
            fail_assert!("missing tagged scalar");
            return;
        };
        // When: calling tag()
        let tag = evt.tag();
        // Then: returns Some with the tag string
        let Some(tag_str) = tag else {
            fail_assert!("expected Some tag");
            return;
        };
        assert!(
            tag_str.contains("mytag"),
            "tag should contain 'mytag', got: {tag_str}"
        );
    }

    #[test]
    fn typed_node_anchor_id_returns_nonzero_for_anchored() {
        // Given: YAML with an anchor on a scalar
        let yaml = "a: &anc value\n";
        let events = collect_ok!(yaml);
        let anchored = events.iter().find(|e| e.anchor_id() != 0);
        // When: checking anchor_id
        let Some(evt) = anchored else {
            fail_assert!("missing anchored event");
            return;
        };
        // Then: anchor_id is non-zero
        assert!(evt.anchor_id() > 0);
    }

    #[test]
    fn typed_node_anchor_id_returns_zero_for_unanchored() {
        // Given: YAML with no anchors
        let yaml = "a: b\n";
        let events = collect_ok!(yaml);
        // When: checking all events
        // Then: all anchor_id values are 0
        let all_zero = events.iter().all(|e| e.anchor_id() == 0);
        assert!(all_zero);
    }

    #[test]
    fn event_span_fields_are_populated() {
        // Given: a StreamStart event
        let yaml = "a: b\n";
        let events = collect_ok!(yaml);
        let stream_start = events
            .iter()
            .find(|e| matches!(e, YamlEvent::StreamStart { .. }));
        let Some(YamlEvent::StreamStart { span }) = stream_start else {
            fail_assert!("missing stream start");
            return;
        };
        // When: inspecting the span
        // Then: start < end (span has range)
        assert!(span.end >= span.start);
    }

    #[test]
    fn collect_events_produces_stream_lifecycle() {
        // Given: valid YAML
        let yaml = "a: 1\n";
        let events = collect_ok!(yaml);
        // When: inspecting the event stream
        let has_stream_start = events
            .iter()
            .any(|e| matches!(e, YamlEvent::StreamStart { .. }));
        let has_stream_end = events
            .iter()
            .any(|e| matches!(e, YamlEvent::StreamEnd { .. }));
        // Then: both stream start and stream end present
        assert!(has_stream_start, "missing StreamStart");
        assert!(has_stream_end, "missing StreamEnd");
    }

    #[test]
    fn document_start_carries_explicit_flag() {
        // Given: YAML with explicit document marker
        let yaml = "---\nkey: value\n";
        let events = collect_ok!(yaml);
        // When: finding the DocumentStart
        let doc_start = events.iter().find_map(|e| match e {
            YamlEvent::DocumentStart { explicit, .. } => Some(*explicit),
            _ => None,
        });
        // Then: explicit = true
        assert_eq!(doc_start, Some(true));
    }

    #[test]
    fn implicit_document_start_is_not_explicit() {
        // Given: YAML without explicit document marker
        let yaml = "key: value\n";
        let events = collect_ok!(yaml);
        // When: finding the DocumentStart
        let doc_start = events.iter().find_map(|e| match e {
            YamlEvent::DocumentStart { explicit, .. } => Some(*explicit),
            _ => None,
        });
        // Then: explicit = false
        assert_eq!(doc_start, Some(false));
    }

    #[test]
    fn sequence_events_have_start_and_end() {
        // Given: YAML with a sequence
        let yaml = "items:\n  - a\n  - b\n";
        let events = collect_ok!(yaml);
        let has_start = events
            .iter()
            .any(|e| matches!(e, YamlEvent::SequenceStart { .. }));
        let has_end = events
            .iter()
            .any(|e| matches!(e, YamlEvent::SequenceEnd { .. }));
        // When: checking for sequence events
        // Then: both start and end present
        assert!(has_start, "missing SequenceStart");
        assert!(has_end, "missing SequenceEnd");
    }

    #[test]
    fn mapping_events_have_start_and_end() {
        // Given: YAML with a mapping
        let yaml = "a: 1\n";
        let events = collect_ok!(yaml);
        let has_start = events
            .iter()
            .any(|e| matches!(e, YamlEvent::MappingStart { .. }));
        let has_end = events
            .iter()
            .any(|e| matches!(e, YamlEvent::MappingEnd { .. }));
        // When: checking for mapping events
        // Then: both start and end present
        assert!(has_start, "missing MappingStart");
        assert!(has_end, "missing MappingEnd");
    }

    #[test]
    fn scalar_style_plain_for_unquoted() {
        // Given: YAML with plain scalar
        let yaml = "key: value\n";
        let events = collect_ok!(yaml);
        let scalar = events.iter().find_map(|e| match e {
            YamlEvent::Scalar { value, style, .. } if value.as_ref() == "value" => Some(*style),
            _ => None,
        });
        // When: checking style
        // Then: Plain
        assert_eq!(scalar, Some(ScalarStyle::Plain));
    }

    #[test]
    fn scalar_style_single_quoted() {
        // Given: YAML with single-quoted scalar
        let yaml = "key: 'value'\n";
        let events = collect_ok!(yaml);
        let scalar = events.iter().find_map(|e| match e {
            YamlEvent::Scalar { value, style, .. } if value.as_ref() == "value" => Some(*style),
            _ => None,
        });
        // When: checking style
        // Then: SingleQuoted
        assert_eq!(scalar, Some(ScalarStyle::SingleQuoted));
    }

    #[test]
    fn scalar_style_double_quoted() {
        // Given: YAML with double-quoted scalar
        let yaml = "key: \"value\"\n";
        let events = collect_ok!(yaml);
        let scalar = events.iter().find_map(|e| match e {
            YamlEvent::Scalar { value, style, .. } if value.as_ref() == "value" => Some(*style),
            _ => None,
        });
        // When: checking style
        // Then: DoubleQuoted
        assert_eq!(scalar, Some(ScalarStyle::DoubleQuoted));
    }

    #[test]
    fn event_span_from_parser_span_fields() {
        // Given: an EventSpan created manually
        let span = EventSpan {
            start: 0,
            end: 10,
            line: 1,
            column: 1,
        };
        // When: inspecting fields
        // Then: exact values
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 10);
        assert_eq!(span.line, 1);
        assert_eq!(span.column, 1);
    }

    #[test]
    fn scalar_event_has_zero_anchor_when_unanchored() {
        // Given: a plain scalar without anchor
        let yaml = "key: value\n";
        let events = collect_ok!(yaml);
        let scalar = events.iter().find_map(|e| match e {
            YamlEvent::Scalar {
                value, anchor_id, ..
            } if value.as_ref() == "value" => Some(*anchor_id),
            _ => None,
        });
        // When: checking anchor_id
        // Then: 0
        assert_eq!(scalar, Some(0));
    }

    #[test]
    fn sequence_start_tag_returns_none_when_untagged() {
        // Given: YAML with an untagged sequence
        let yaml = "items:\n  - a\n";
        let events = collect_ok!(yaml);
        let seq_start = events.iter().find_map(|e| match e {
            YamlEvent::SequenceStart { tag, .. } => Some(tag.clone()),
            _ => None,
        });
        let Some(tag) = seq_start else {
            fail_assert!("missing sequence start");
            return;
        };
        // When: checking tag
        // Then: None
        assert_eq!(tag, None);
    }

    // -----------------------------------------------------------------------
    // Adversarial BDD tests - event layer attack vectors
    // -----------------------------------------------------------------------

    #[test]
    fn adversarial_events_null_byte_accepted_by_parser_but_rejected_by_profile() {
        // Given: YAML with a null byte embedded in a scalar
        let yaml = "key: val\x00ue\n";
        // When: collecting events (raw parser layer)
        let result = collect_events(yaml);
        // Then: The parser itself accepts null bytes in scalars, but the
        // profile validation layer (validate_yaml_profile) rejects them.
        // At the raw event collection level we simply verify that events
        // are produced and may contain the null byte.
        match result {
            Ok(events) => {
                let _scalar = events.iter().find_map(|e| match e {
                    YamlEvent::Scalar { value, .. } if value.contains('\x00') => {
                        Some(value.clone())
                    }
                    _ => None,
                });
                assert!(!events.is_empty(), "events should not be empty");
                // Verify the profile layer catches the null byte at source level
                let profile_result = crate::profile::validate_yaml_profile(yaml);
                assert!(
                    matches!(
                        profile_result,
                        Err(crate::YamlError::ForbiddenFeature {
                            detail: "null_byte_in_source"
                        })
                    ),
                    "profile validation must reject null bytes, got: {profile_result:?}"
                );
            }
            Err(e) => {
                // If saphyr rejects it in a future version, that's also fine
                let _ = e;
            }
        }
    }

    #[test]
    fn adversarial_events_unicode_zero_width_char_accepted_as_events() {
        // Given: YAML with a zero-width joiner character in a value
        let yaml = "key: hello\u{200D}world\n";
        // When: collecting events
        let result = collect_events(yaml);
        // Then: Ok - the parser accepts Unicode; profile validation rejects ambiguity
        match result {
            Ok(events) => {
                let scalar = events.iter().find_map(|e| match e {
                    YamlEvent::Scalar { value, .. } if value.contains('\u{200D}') => {
                        Some(value.clone())
                    }
                    _ => None,
                });
                assert!(scalar.is_some(), "expected scalar with zero-width joiner");
            }
            Err(e) => fail_assert!("expected Ok events, got Err: {e}"),
        }
    }

    #[test]
    fn adversarial_events_rtl_override_in_scalar_parsed() {
        // Given: YAML with RTL override character in a value
        let yaml = "key: hello\u{202E}world\n";
        // When: collecting events
        let result = collect_events(yaml);
        // Then: Ok - the parser preserves the raw scalar value
        match result {
            Ok(events) => {
                let scalar = events.iter().find_map(|e| match e {
                    YamlEvent::Scalar { value, .. } if value.contains('\u{202E}') => {
                        Some(value.clone())
                    }
                    _ => None,
                });
                assert!(scalar.is_some(), "expected scalar with RTL override");
            }
            Err(e) => fail_assert!("expected Ok events, got Err: {e}"),
        }
    }

    #[test]
    fn adversarial_events_tagged_scalar_carries_tag() {
        // Given: YAML with a custom-tagged scalar
        let yaml = "key: !mytag value\n";
        // When: collecting events
        let events = collect_ok!(yaml);
        // Then: the scalar event carries the tag string
        let tagged = events.iter().find_map(|e| match e {
            YamlEvent::Scalar { tag: Some(t), .. } => Some(t.clone()),
            _ => None,
        });
        let Some(tag) = tagged else {
            fail_assert!("missing tagged scalar event");
            return;
        };
        assert!(tag.contains("mytag"), "expected 'mytag' in tag, got: {tag}");
    }

    #[test]
    fn adversarial_events_anchor_produces_nonzero_anchor_id() {
        // Given: YAML with an anchor on a scalar
        let yaml = "a: &anc value\n";
        // When: collecting events
        let events = collect_ok!(yaml);
        // Then: at least one event has a non-zero anchor_id
        let anchored = events.iter().find(|e| e.anchor_id() != 0);
        assert!(anchored.is_some(), "expected anchored event");
    }

    #[test]
    fn adversarial_events_alias_produces_alias_variant() {
        // Given: YAML with an alias reference
        let yaml = "a: &anc value\nb: *anc\n";
        // When: collecting events
        let events = collect_ok!(yaml);
        // Then: an Alias event is present
        let alias = events.iter().find(|e| e.is_alias());
        assert!(alias.is_some(), "expected Alias event in stream");
    }

    #[test]
    fn adversarial_events_multi_doc_produces_multiple_document_starts() {
        // Given: YAML with two documents
        let yaml = "---\na: 1\n---\nb: 2\n";
        // When: collecting events
        let events = collect_ok!(yaml);
        // Then: two DocumentStart events
        let doc_starts: Vec<_> = events.iter().filter(|e| e.is_document_start()).collect();
        assert_eq!(doc_starts.len(), 2, "expected 2 document starts");
    }

    #[test]
    fn adversarial_events_block_scalar_preserves_value() {
        // Given: YAML with a literal block scalar
        let yaml = "key: |\n  line1\n  line2\n";
        // When: collecting events
        let events = collect_ok!(yaml);
        // Then: scalar has Literal style with multi-line content
        let scalar = events.iter().find_map(|e| match e {
            YamlEvent::Scalar {
                value,
                style: ScalarStyle::Literal,
                ..
            } => Some(value.clone()),
            _ => None,
        });
        let Some(val) = scalar else {
            fail_assert!("missing literal block scalar");
            return;
        };
        assert!(val.contains("line1"), "expected 'line1' in block scalar");
        assert!(val.contains("line2"), "expected 'line2' in block scalar");
    }

    #[test]
    fn adversarial_events_folded_scalar_preserves_value() {
        // Given: YAML with a folded block scalar
        let yaml = "key: >\n  line1\n  line2\n";
        // When: collecting events
        let events = collect_ok!(yaml);
        // Then: scalar has Folded style
        let scalar = events.iter().find_map(|e| match e {
            YamlEvent::Scalar {
                style: ScalarStyle::Folded,
                ..
            } => Some(true),
            _ => None,
        });
        assert_eq!(scalar, Some(true), "expected Folded scalar");
    }
}
