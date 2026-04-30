//! Source location tracking for YAML nodes.
//!
//! This module provides [`SourceMap`] which maps YAML node indices to
//! (line, column) spans extracted from the parser event stream.

use crate::events::{EventSpan, YamlEvent};
use crate::YamlResult;

/// A (line, column) span in the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    /// One-indexed start line.
    pub start_line: usize,
    /// One-indexed start column.
    pub start_col: usize,
    /// One-indexed end line.
    pub end_line: usize,
    /// One-indexed end column.
    pub end_col: usize,
}

impl SourceSpan {
    /// Creates a new source span.
    #[must_use]
    pub const fn new(start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }
}

/// A source map that tracks YAML node positions.
///
/// Nodes are indexed in the order they appear in the event stream.
/// Container nodes (mappings, sequences) are assigned indices at their
/// start event. Scalar and leaf nodes are also indexed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMap {
    /// Mapping from node index to (line, col) span.
    spans: Vec<SourceSpan>,
}

impl SourceMap {
    /// Creates an empty source map.
    #[must_use]
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Returns the number of tracked nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// Returns true if there are no tracked nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// Looks up the span for a node by index.
    #[must_use]
    pub fn span_for_node(&self, node_index: u32) -> Option<SourceSpan> {
        usize::try_from(node_index)
            .ok()
            .and_then(|i| self.spans.get(i).copied())
    }

    /// Returns an iterator over all (index, span) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (u32, SourceSpan)> + '_ {
        self.spans
            .iter()
            .enumerate()
            .map(|(i, span)| (u32::try_from(i).unwrap_or(u32::MAX), *span))
    }
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a source map from YAML text by parsing the event stream.
pub fn build_source_map(text: &str) -> YamlResult<SourceMap> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_map() {
        let map = SourceMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
        assert_eq!(map.span_for_node(0), None);
    }

    #[test]
    fn build_from_simple_yaml() {
        let yaml = "key: value\n";
        let map = build_source_map(yaml).unwrap();
        assert!(!map.is_empty());
    }

    #[test]
    fn node_indices_are_sequential() {
        let yaml = "a: 1\nb: 2\n";
        let map = build_source_map(yaml).unwrap();
        let count = map.len();
        assert!(count >= 2);

        let mut found = Vec::new();
        for (idx, _span) in map.iter() {
            found.push(idx);
        }
        // Indices should be 0, 1, 2, ...
        for (i, idx) in found.iter().enumerate() {
            assert_eq!(*idx, u32::try_from(i).unwrap_or(u32::MAX));
        }
    }

    #[test]
    fn default_is_empty() {
        let map = SourceMap::default();
        assert!(map.is_empty());
    }
}
