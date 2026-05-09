#![forbid(unsafe_code)]
//! Source map building functions.

use crate::YamlResult;
use crate::events::{EventSpan, YamlEvent};
use crate::source_map_types::{SourceMap, SourceSpan};

/// Build a source map from YAML text by parsing the event stream.
pub fn build_source_map(text: &str) -> YamlResult<SourceMap> {
    crate::profile::validate_yaml_profile(text)?;
    let events = crate::events::collect_events(text)?;
    Ok(source_map_from_events(&events))
}

/// Build a source map from a pre-collected event list.
fn source_map_from_events(events: &[YamlEvent]) -> SourceMap {
    let mut spans = Vec::new();

    for event in events {
        let span = event.span();
        match event {
            YamlEvent::MappingStart { .. }
            | YamlEvent::SequenceStart { .. }
            | YamlEvent::Scalar { .. } => {
                spans.push(event_span_to_source_span(span));
            }
            _ => {}
        }
    }

    SourceMap { spans }
}

/// Convert an EventSpan to a SourceSpan.
fn event_span_to_source_span(span: EventSpan) -> SourceSpan {
    SourceSpan::new(span.line, span.column, span.line, span.column)
}
