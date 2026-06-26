#![forbid(unsafe_code)]
//! Source map building functions.

use super::source_map_types::{SemanticSourceMap, SourceMap, SourceSpan};
use crate::yaml_error::YamlResult;
use crate::yaml_events::{EventSpan, YamlEvent};

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingKey {
    value: Box<str>,
    span: EventSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathContext {
    Mapping {
        path: String,
        pending_key: Option<PendingKey>,
    },
    Sequence {
        path: String,
        next_index: usize,
    },
}

/// Build a source map from YAML text by parsing the event stream.
pub fn build_source_map(text: &str) -> YamlResult<SourceMap> {
    crate::yaml_profile::validate_yaml_profile(text)?;
    let events = crate::yaml_events::collect_events(text)?;
    Ok(source_map_from_events(text, &events))
}

/// Build a source map from a pre-collected event list.
fn source_map_from_events(text: &str, events: &[YamlEvent]) -> SourceMap {
    let mut spans = Vec::new();

    for event in events {
        let span = event.span();
        match event {
            YamlEvent::MappingStart { .. }
            | YamlEvent::SequenceStart { .. }
            | YamlEvent::Scalar { .. } => {
                spans.push(event_span_to_source_span(text, span));
            }
            _ => {}
        }
    }

    SourceMap { spans }
}

/// Convert an EventSpan to a SourceSpan.
fn event_span_to_source_span(text: &str, span: EventSpan) -> SourceSpan {
    let (end_line, end_col) = line_col(text, span.end);
    SourceSpan::new(
        span.start,
        span.end,
        span.line,
        span.column,
        end_line,
        end_col,
    )
}

/// Build a cold semantic source map for commonly diagnosed author paths.
pub fn build_semantic_source_map(text: &str) -> YamlResult<SemanticSourceMap> {
    crate::yaml_profile::validate_yaml_profile(text)?;
    let events = crate::yaml_events::collect_events(text)?;
    Ok(semantic_source_map_from_events(text, &events))
}

fn semantic_source_map_from_events(text: &str, events: &[YamlEvent]) -> SemanticSourceMap {
    let mut map = SemanticSourceMap::default();
    let mut stack = Vec::<PathContext>::new();

    for event in events {
        match event {
            YamlEvent::MappingStart { span, .. } => {
                let path = begin_container_path(text, &mut stack, *span, &mut map);
                stack.push(PathContext::Mapping {
                    path,
                    pending_key: None,
                });
            }
            YamlEvent::SequenceStart { span, .. } => {
                let path = begin_container_path(text, &mut stack, *span, &mut map);
                stack.push(PathContext::Sequence {
                    path,
                    next_index: 0,
                });
            }
            YamlEvent::MappingEnd { .. } | YamlEvent::SequenceEnd { .. } => {
                let _popped = stack.pop();
            }
            YamlEvent::Scalar { value, span, .. } => {
                visit_scalar(text, &mut stack, value.as_ref(), *span, &mut map);
            }
            YamlEvent::StreamStart { .. }
            | YamlEvent::StreamEnd { .. }
            | YamlEvent::DocumentStart { .. }
            | YamlEvent::DocumentEnd { .. }
            | YamlEvent::Alias { .. } => {}
        }
    }

    map
}

fn begin_container_path(
    text: &str,
    stack: &mut [PathContext],
    span: EventSpan,
    map: &mut SemanticSourceMap,
) -> String {
    let Some(parent) = stack.last_mut() else {
        return "$".to_string();
    };
    match parent {
        PathContext::Mapping { path, pending_key } => {
            let Some(key) = pending_key.take() else {
                return path.clone();
            };
            let child = append_mapping_path(path, key.value.as_ref());
            if is_trigger_container_path(&child) {
                map.push(child.clone(), event_span_to_source_span(text, key.span));
            } else {
                map.push(child.clone(), event_span_to_source_span(text, span));
            }
            child
        }
        PathContext::Sequence { path, next_index } => {
            let child = append_sequence_path(path, *next_index);
            *next_index = next_index.saturating_add(1);
            map.push(child.clone(), event_span_to_source_span(text, span));
            child
        }
    }
}

fn visit_scalar(
    text: &str,
    stack: &mut [PathContext],
    value: &str,
    span: EventSpan,
    map: &mut SemanticSourceMap,
) {
    let Some(parent) = stack.last_mut() else {
        map.push("$".to_string(), event_span_to_source_span(text, span));
        return;
    };
    match parent {
        PathContext::Mapping { path, pending_key } => {
            if let Some(key) = pending_key.take() {
                let child = append_mapping_path(path, key.value.as_ref());
                map.push(child, event_span_to_source_span(text, span));
            } else {
                *pending_key = Some(PendingKey {
                    value: Box::<str>::from(value),
                    span,
                });
            }
        }
        PathContext::Sequence { path, next_index } => {
            let child = append_sequence_path(path, *next_index);
            *next_index = next_index.saturating_add(1);
            map.push(child, event_span_to_source_span(text, span));
        }
    }
}

fn append_mapping_path(parent: &str, key: &str) -> String {
    if parent == "$" {
        format!("$.{key}")
    } else {
        format!("{parent}.{key}")
    }
}

fn append_sequence_path(parent: &str, index: usize) -> String {
    format!("{parent}[{index}]")
}

fn is_trigger_container_path(path: &str) -> bool {
    matches!(
        path,
        "$.when.manual" | "$.when.schedule" | "$.when.event" | "$.when.webhook"
    )
}

fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    let mut cursor = 0usize;
    for ch in text.chars() {
        if cursor >= offset {
            break;
        }
        if ch == '\n' {
            line = line.saturating_add(1);
            col = 1;
        } else {
            col = col.saturating_add(1);
        }
        cursor = match cursor.checked_add(ch.len_utf8()) {
            Some(next) => next,
            None => break,
        };
    }
    (line, col)
}

/// Look up the span for a node by index in a pre-built source map.
pub fn span_for_node(
    map: &SourceMap,
    node_index: u32,
) -> Option<SourceSpan> {
    map.span_for_node(node_index)
}
