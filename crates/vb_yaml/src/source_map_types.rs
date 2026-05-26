#![forbid(unsafe_code)]
//! Source location tracking for YAML nodes.
//!
//! This module provides [`SourceMap`] which maps YAML node indices to
//! (line, column) spans extracted from the parser event stream.

/// A (line, column) span in the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    /// Start byte offset in UTF-8 source.
    pub start_offset: usize,
    /// Exclusive end byte offset in UTF-8 source.
    pub end_offset: usize,
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
    pub const fn new(
        start_offset: usize,
        end_offset: usize,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    ) -> Self {
        Self {
            start_offset,
            end_offset,
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }
}

/// Cold semantic source map keyed by JSONPath-like author paths.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SemanticSourceMap {
    spans: Vec<(String, SourceSpan)>,
}

impl SemanticSourceMap {
    pub(crate) fn push(&mut self, path: String, span: SourceSpan) {
        self.spans.push((path, span));
    }

    #[must_use]
    pub fn span_for_path(&self, path: &str) -> Option<SourceSpan> {
        self.spans
            .iter()
            .find_map(|(candidate, span)| if candidate == path { Some(*span) } else { None })
    }

    /// Reverse lookup: find the YAML author path for a byte-offset span.
    ///
    /// This is used during diagnostic rendering to annotate error messages
    /// with the YAML author path (e.g., `$.inputs.name`) when available.
    #[must_use]
    pub fn find_path_for_offset(&self, start_offset: usize, end_offset: usize) -> Option<&str> {
        self.spans.iter().find_map(|(path, span)| {
            if span.start_offset <= start_offset && span.end_offset >= end_offset {
                Some(path.as_str())
            } else {
                None
            }
        })
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
    pub(crate) spans: Vec<SourceSpan>,
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
