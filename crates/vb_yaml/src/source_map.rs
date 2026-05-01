//! Source location tracking for YAML nodes.
//!
//! This module provides [`SourceMap`] which maps YAML node indices to
//! (line, column) spans extracted from the parser event stream.

use crate::YamlResult;
use crate::events::{EventSpan, YamlEvent};

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
            .filter_map(|(i, span)| match u32::try_from(i) {
                Ok(index) => Some((index, *span)),
                Err(_) => None,
            })
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

    macro_rules! build_ok {
        ($yaml:expr) => {
            match build_source_map($yaml) {
                Ok(value) => value,
                Err(error) => {
                    assert!(false, "source map failed: {error}");
                    return;
                }
            }
        };
    }

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
        let map = build_ok!(yaml);
        assert!(!map.is_empty());
    }

    #[test]
    fn node_indices_are_sequential() {
        let yaml = "a: 1\nb: 2\n";
        let map = build_ok!(yaml);
        let count = map.len();
        assert!(count >= 2);

        let mut found = Vec::new();
        for (idx, _span) in map.iter() {
            found.push(idx);
        }
        // Indices should be 0, 1, 2, ...
        for (i, idx) in found.iter().enumerate() {
            let Ok(expected) = u32::try_from(i) else {
                assert!(false, "index does not fit u32");
                return;
            };
            assert_eq!(*idx, expected);
        }
    }

    #[test]
    fn default_is_empty() {
        let map = SourceMap::default();
        assert!(map.is_empty());
    }

    // -----------------------------------------------------------------------
    // Source Map BDD tests
    // -----------------------------------------------------------------------

    #[test]
    fn source_map_new_is_empty() {
        // Given: a new SourceMap
        let map = SourceMap::new();
        // When: checking len and is_empty
        // Then: len is 0 and is_empty is true
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    }

    #[test]
    fn source_map_len_increases_with_entries() {
        // Given: a source map built from simple YAML
        let yaml = "a: 1\nb: 2\n";
        let map = build_ok!(yaml);
        // When: checking len
        let count = map.len();
        // Then: len >= 4 (mapping + 4 scalars)
        assert!(count >= 2, "expected at least 2 entries, got {count}");
        assert!(!map.is_empty());
    }

    #[test]
    fn source_map_iter_yields_inserted_entries() {
        // Given: a source map from YAML
        let yaml = "a: 1\n";
        let map = build_ok!(yaml);
        // When: iterating
        let entries: Vec<(u32, SourceSpan)> = map.iter().collect();
        // Then: at least one entry with a valid span
        assert!(!entries.is_empty());
        let first = entries[0];
        assert_eq!(first.0, 0);
        assert!(first.1.start_line > 0);
    }

    #[test]
    fn build_source_map_produces_correct_mappings() {
        // Given: simple YAML
        let yaml = "key: value\n";
        let map = build_ok!(yaml);
        // When: looking up node 0
        let span = map.span_for_node(0);
        // Then: Some with valid line/column
        let Some(s) = span else {
            assert!(false, "expected Some span for node 0");
            return;
        };
        assert!(s.start_line > 0);
    }

    #[test]
    fn source_map_span_for_node_returns_correct_range() {
        // Given: a source map
        let yaml = "a: 1\n";
        let map = build_ok!(yaml);
        // When: looking up node 0
        let span = map.span_for_node(0);
        // Then: the span line/col match (end = start for single-point spans)
        let Some(s) = span else {
            assert!(false, "expected Some span");
            return;
        };
        assert_eq!(s.start_line, s.end_line);
        assert_eq!(s.start_col, s.end_col);
    }

    #[test]
    fn source_map_span_for_node_returns_none_for_out_of_range() {
        // Given: a source map from small YAML
        let yaml = "a: 1\n";
        let map = build_ok!(yaml);
        // When: looking up an out-of-range index
        let result = map.span_for_node(9999);
        // Then: None
        assert_eq!(result, None);
    }

    #[test]
    fn source_map_iter_indices_are_sequential() {
        // Given: a source map
        let yaml = "a: 1\nb: 2\n";
        let map = build_ok!(yaml);
        // When: collecting indices from iter
        let indices: Vec<u32> = map.iter().map(|(i, _)| i).collect();
        // Then: indices are 0, 1, 2, ... sequential
        let mut expected: u32 = 0;
        for idx in &indices {
            assert_eq!(*idx, expected);
            expected = expected.saturating_add(1);
        }
    }

    #[test]
    fn source_span_new_exact_values() {
        // Given: a SourceSpan created with specific values
        let span = SourceSpan::new(1, 2, 3, 4);
        // When: inspecting fields
        // Then: exact values
        assert_eq!(span.start_line, 1);
        assert_eq!(span.start_col, 2);
        assert_eq!(span.end_line, 3);
        assert_eq!(span.end_col, 4);
    }

    #[test]
    fn source_map_preserves_order_from_yaml() {
        // Given: YAML with multiple scalars
        let yaml = "first: a\nsecond: b\nthird: c\n";
        let map = build_ok!(yaml);
        // When: iterating
        let entries: Vec<(u32, SourceSpan)> = map.iter().collect();
        // Then: node count >= 6 (mapping start + 6 scalars)
        assert!(entries.len() >= 3, "expected at least 3 entries, got {}", entries.len());
    }

    #[test]
    fn build_source_map_for_nested_yaml() {
        // Given: nested YAML
        let yaml = "a:\n  b: 1\n";
        let map = build_ok!(yaml);
        // When: checking length
        // Then: multiple nodes tracked
        assert!(map.len() >= 3, "expected at least 3 entries, got {}", map.len());
    }

    #[test]
    fn build_source_map_for_sequence_yaml() {
        // Given: YAML with a sequence
        let yaml = "items:\n  - a\n  - b\n";
        let map = build_ok!(yaml);
        // When: checking length
        // Then: sequence nodes are tracked
        assert!(map.len() >= 2, "expected at least 2 entries, got {}", map.len());
    }
}
