#![forbid(unsafe_code)]
//! Event conversion and collection functions.

use crate::{YamlError, YamlResult};
use crate::events_types::{YamlEvent, EventSpan, ScalarStyle};

/// Collect all YAML events from the given source text.
///
/// Returns an owned vector of [`YamlEvent`] values.
pub fn collect_events(text: &str) -> YamlResult<Vec<YamlEvent>> {
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
pub fn convert_event(
    event: saphyr_parser::Event<'_>,
    span: saphyr_parser::Span,
) -> YamlEvent {
    let span = EventSpan::from_parser_span(span);
    match event {
        saphyr_parser::Event::StreamStart => YamlEvent::StreamStart { span },
        saphyr_parser::Event::StreamEnd | saphyr_parser::Event::Nothing => {
            YamlEvent::StreamEnd { span }
        }
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
    }
}

/// Format a saphyr-parser Tag into a display string.
fn format_tag(tag: &saphyr_parser::Tag) -> Box<str> {
    format!("{}{}", tag.handle, tag.suffix).into_boxed_str()
}
