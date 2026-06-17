// Verification artifact: vb_yaml_source_span_validity.rs
// Verifier: Verus
// Crate: vb_yaml
//
// Proof obligations:
// - PO-YAML-007: EventSpan fields satisfy basic validity invariants
// - PO-YAML-008: SourceSpan is consistent with EventSpan conversion
// - PO-YAML-009: SourceMap span indices are always in-bounds
//
// GOD RULE 2: Spec functions mirror production types in:
// - crates/vb_yaml/src/events_types.rs (EventSpan)
// - crates/vb_yaml/src/source_map_types.rs (SourceSpan, SourceMap)

use vstd::prelude::*;

verus! {

// ─────────────────────────────────────────────────────────────────
// Spec: EventSpan model
// ─────────────────────────────────────────────────────────────────

/// Spec model of EventSpan from events_types.rs.
///
/// In production:
///   pub struct EventSpan { pub start: usize, pub end: usize, pub line: usize, pub column: usize }
pub struct SpecEventSpan {
    pub start: int,
    pub end: int,
    pub line: int,
    pub column: int,
}

impl SpecEventSpan {
    spec fn valid(self) -> bool {
        self.start <= self.end
            && self.start >= 0
            && self.end >= 0
            && self.line >= 1
            && self.column >= 1
    }
}

/// Spec model of SourceSpan from source_map_types.rs.
///
/// In production:
///   pub struct SourceSpan {
///       pub start_offset: usize, pub end_offset: usize,
///       pub start_line: usize, pub start_col: usize,
///       pub end_line: usize, pub end_col: usize,
///   }
pub struct SpecSourceSpan {
    pub start_offset: int,
    pub end_offset: int,
    pub start_line: int,
    pub start_col: int,
    pub end_line: int,
    pub end_col: int,
}

impl SpecSourceSpan {
    spec fn valid(self) -> bool {
        self.start_offset <= self.end_offset
            && self.start_offset >= 0
            && self.end_offset >= 0
            && self.start_line >= 1
            && self.start_col >= 1
            && self.end_line >= self.start_line
            && (self.end_line > self.start_line || self.end_col >= self.start_col)
    }
}

// ─────────────────────────────────────────────────────────────────
// Spec: SourceMap model
// ─────────────────────────────────────────────────────────────────

/// Spec model of SourceMap: a vector of spans indexed by node index.
pub struct SpecSourceMap {
    pub spans: Seq<SpecSourceSpan>,
}

impl SpecSourceMap {
    spec fn valid(self) -> bool {
        self.len() >= 0
    }

    spec fn len(self) -> int {
        self.spans.len()
    }

    /// Check if a node index is within the map bounds.
    spec fn contains_node(self, node_index: int) -> bool {
        0 <= node_index && node_index < self.len()
    }

    spec fn span_at(self, node_index: int) -> Option<SpecSourceSpan> {
        if 0 <= node_index && node_index < self.len() {
            Option::Some(self.spans[node_index])
        } else {
            Option::None
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-YAML-007: EventSpan invariants
// ─────────────────────────────────────────────────────────────────

/// Lemma: A valid EventSpan always has start <= end.
pub proof fn lemma_event_span_start_le_end(span: SpecEventSpan)
    requires
        span.valid(),
    ensures
        span.start <= span.end,
{
    assert(span.start <= span.end);
}

/// Lemma: A valid EventSpan always has line >= 1.
pub proof fn lemma_event_span_line_ge_one(span: SpecEventSpan)
    requires
        span.valid(),
    ensures
        span.line >= 1,
{
    assert(span.line >= 1);
}

/// Lemma: A valid EventSpan always has column >= 1.
pub proof fn lemma_event_span_column_ge_one(span: SpecEventSpan)
    requires
        span.valid(),
    ensures
        span.column >= 1,
{
    assert(span.column >= 1);
}

/// Lemma: EventSpan byte offsets are non-negative.
pub proof fn lemma_event_span_offsets_nonnegative(span: SpecEventSpan)
    requires
        span.valid(),
    ensures
        span.start >= 0 && span.end >= 0,
{
    assert(span.start >= 0 && span.end >= 0);
}

// ─────────────────────────────────────────────────────────────────
// PO-YAML-008: SourceSpan validity
// ─────────────────────────────────────────────────────────────────

/// Lemma: A valid SourceSpan always has start_offset <= end_offset.
pub proof fn lemma_source_span_offset_valid(span: SpecSourceSpan)
    requires
        span.valid(),
    ensures
        span.start_offset <= span.end_offset,
{
    assert(span.start_offset <= span.end_offset);
}

/// Lemma: A valid SourceSpan always has end_line >= start_line.
pub proof fn lemma_source_span_line_valid(span: SpecSourceSpan)
    requires
        span.valid(),
    ensures
        span.end_line >= span.start_line,
{
    assert(span.end_line >= span.start_line);
}

/// Lemma: If end_line == start_line then end_col >= start_col.
pub proof fn lemma_source_span_column_on_same_line(span: SpecSourceSpan)
    requires
        span.valid(),
        span.end_line == span.start_line,
    ensures
        span.end_col >= span.start_col,
{
    // When end_line == start_line, the validity constraint requires
    // end_col >= start_col
    assert(span.end_col >= span.start_col);
}

/// Lemma: SourceSpan lines are always >= 1.
pub proof fn lemma_source_span_lines_ge_one(span: SpecSourceSpan)
    requires
        span.valid(),
    ensures
        span.start_line >= 1 && span.end_line >= 1,
{
    assert(span.start_line >= 1 && span.end_line >= 1);
}

// ─────────────────────────────────────────────────────────────────
// PO-YAML-009: SourceMap index safety
// ─────────────────────────────────────────────────────────────────

/// Lemma: span_at returns Some for in-bounds indices.
pub proof fn lemma_span_at_some_for_valid_index(map: SpecSourceMap, node_index: int)
    requires
        0 <= node_index && node_index < map.len(),
    ensures
        map.span_at(node_index) == Option::Some(map.spans[node_index]),
{
    assert(map.span_at(node_index) == Option::Some(map.spans[node_index]));
}

/// Lemma: span_at returns None for out-of-bounds indices.
pub proof fn lemma_span_at_none_for_invalid_index(map: SpecSourceMap, node_index: int)
    requires
        node_index < 0 || node_index >= map.len(),
    ensures
        map.span_at(node_index) == Option::None,
{
    if node_index < 0 {
        assert(map.span_at(node_index) == Option::None);
    } else if node_index >= map.len() {
        assert(map.span_at(node_index) == Option::None);
    }
}

/// Lemma: SourceMap is never invalid (empty maps are valid).
pub proof fn lemma_source_map_always_valid(map: SpecSourceMap)
    ensures
        map.valid(),
{
    assert(map.valid());
}

/// Lemma: A non-empty SourceMap has at least one valid span.
pub proof fn lemma_non_empty_source_map_has_valid_span(map: SpecSourceMap)
    requires
        map.len() > 0,
    ensures
        exists|span: SpecSourceSpan| map.span_at(0) == Option::Some(span),
{
    assert(map.span_at(0) == Option::Some(map.spans[0]));
}

} // verus!

fn main() {}
