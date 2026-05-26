// Proptest: Span bridge round-trip and correctness
// PO-P05: Span bridge properties (C9.1-C9.2)
//
// The span bridge is now implemented in vb_compile::span_bridge:
//  - clamp_u32: usize → u32 with saturation
//  - span_from_source_span: SourceSpan → Span
//  - From<SourceMark> for Span
//
// Verifies:
//  1. clamp_u32 identity for values ≤ u32::MAX
//  2. clamp_u32 saturation for values > u32::MAX
//  3. span_from_source_span preserves data within u32 range
//  4. span_from_source_span clamps values exceeding u32::MAX
//  5. SourceMark (available=true) → Span produces Some(line) and Some(col)
//  6. SourceMark (available=false) → Span produces None, None
//  7. Core Span properties (paired invariant, etc.)

use proptest::prelude::*;
use vb_compile::SourceMark;
use vb_compile::span_bridge::{clamp_u32, span_from_source_span};
use vb_core::span::Span;
use vb_yaml::source_map::SourceSpan;

// ---------------------------------------------------------------------------
// clamp_u32 properties
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn clamp_u32_identity_for_any_usize(val: usize) {
        let result = clamp_u32(val);
        if val <= u32::MAX as usize {
            prop_assert_eq!(result, val as u32);
        } else {
            prop_assert_eq!(result, u32::MAX);
        }
    }

    #[test]
    fn clamp_u32_result_always_lte_u32_max(val: usize) {
        let result = clamp_u32(val);
        prop_assert!(result <= u32::MAX);
    }

    // -----------------------------------------------------------------------
    // SourceSpan → Span properties
    // -----------------------------------------------------------------------

    #[test]
    fn source_span_to_span_within_range(
        start_off in 0usize..(u32::MAX as usize),
        end_off in 0usize..(u32::MAX as usize),
        s_line in 1usize..(u32::MAX as usize),
        s_col in 1usize..(u32::MAX as usize),
        e_line in 1usize..(u32::MAX as usize),
        e_col in 1usize..(u32::MAX as usize),
    ) {
        let ss = SourceSpan::new(start_off, end_off, s_line, s_col, e_line, e_col);
        let span = span_from_source_span(ss);

        prop_assert_eq!(span.start, start_off as u32);
        prop_assert_eq!(span.end, end_off as u32);
        prop_assert_eq!(span.line, Some(s_line as u32));
        prop_assert_eq!(span.column, Some(s_col as u32));
        // Paired invariant
        prop_assert_eq!(span.line.is_some(), span.column.is_some());
    }

    #[test]
    fn source_span_to_span_paired_invariant(
        start_off: usize,
        end_off: usize,
        s_line: usize,
        s_col: usize,
        e_line: usize,
        e_col: usize,
    ) {
        let ss = SourceSpan::new(start_off, end_off, s_line, s_col, e_line, e_col);
        let span = span_from_source_span(ss);
        // Line and column are always Some because SourceSpan always carries them
        prop_assert!(span.line.is_some());
        prop_assert!(span.column.is_some());
        prop_assert_eq!(span.line.is_some(), span.column.is_some());
    }

    // -----------------------------------------------------------------------
    // Core Span paired invariant
    // -----------------------------------------------------------------------

    #[test]
    fn span_new_paired_invariant(
        start in 0u32..=u32::MAX,
        end in 0u32..=u32::MAX,
    ) {
        let span = Span::new(start, end);
        prop_assert_eq!(span.line.is_some(), span.column.is_some());
    }

    #[test]
    fn span_with_location_round_trip(
        start in 0u32..=u32::MAX,
        end in 0u32..=u32::MAX,
        line in 1u32..=u32::MAX,
        col in 1u32..=u32::MAX,
    ) {
        prop_assume!(start <= end);
        let span = Span::with_location(start, end, line, col);
        prop_assert_eq!(span.line, Some(line));
        prop_assert_eq!(span.column, Some(col));
        prop_assert_eq!(span.location(), Some((line, col)));
    }

    #[test]
    fn span_with_location_paired_invariant(
        start in 0u32..=u32::MAX,
        end in 0u32..=u32::MAX,
        line in 1u32..=u32::MAX,
        col in 1u32..=u32::MAX,
    ) {
        prop_assume!(start <= end);
        let span = Span::with_location(start, end, line, col);
        prop_assert_eq!(span.line.is_some(), span.column.is_some());
    }
}

// ---------------------------------------------------------------------------
// SourceMark → Span properties (outside proptest! macro)
// ---------------------------------------------------------------------------

#[test]
fn source_mark_available_produces_some_line_col() {
    let mark = SourceMark {
        index: 5,
        end_index: 15,
        line: 3,
        column: 8,
        available: true,
    };
    let span: Span = mark.into();

    assert_eq!(span.start, 5_u32);
    assert_eq!(span.end, 15_u32);
    assert_eq!(span.line, Some(3_u32));
    assert_eq!(span.column, Some(8_u32));
}

#[test]
fn source_mark_unavailable_produces_none_line_col() {
    // SourceMark::unavailable() is pub(crate); construct equivalent directly.
    let mark = SourceMark {
        index: 0,
        end_index: 0,
        line: 0,
        column: 0,
        available: false,
    };
    let span: Span = mark.into();

    assert_eq!(span.start, 0_u32);
    assert_eq!(span.end, 0_u32);
    assert_eq!(span.line, None);
    assert_eq!(span.column, None);
    assert!(span.is_empty());
}

#[test]
fn source_mark_unavailable_ignores_line_col_fields() {
    let mark = SourceMark {
        index: 100,
        end_index: 200,
        line: 5,
        column: 10,
        available: false,
    };
    let span: Span = mark.into();

    assert_eq!(span.start, 100_u32);
    assert_eq!(span.end, 200_u32);
    assert_eq!(span.line, None);
    assert_eq!(span.column, None);
}

#[test]
fn source_mark_available_with_large_values_clamps() {
    let big = u32::MAX as usize + 1;
    let mark = SourceMark {
        index: big,
        end_index: big,
        line: big,
        column: big,
        available: true,
    };
    let span: Span = mark.into();

    assert_eq!(span.start, u32::MAX);
    assert_eq!(span.end, u32::MAX);
    assert_eq!(span.line, Some(u32::MAX));
    assert_eq!(span.column, Some(u32::MAX));
}

#[test]
fn bridge_edge_case_usize_max_no_panic() {
    let ss = SourceSpan::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    );
    let _ = span_from_source_span(ss);

    let mark = SourceMark {
        index: usize::MAX,
        end_index: usize::MAX,
        line: usize::MAX,
        column: usize::MAX,
        available: true,
    };
    let _: Span = mark.into();

    let mark_unavail = SourceMark {
        index: usize::MAX,
        end_index: usize::MAX,
        line: usize::MAX,
        column: usize::MAX,
        available: false,
    };
    let _: Span = mark_unavail.into();
}

// ---------------------------------------------------------------------------
// Zero-parameter Span tests
// ---------------------------------------------------------------------------

#[test]
fn span_zero_is_minimal() {
    let span = Span::ZERO;
    assert_eq!(span.start, 0);
    assert_eq!(span.end, 0);
    assert!(span.is_empty());
    assert!(span.line.is_none());
    assert!(span.column.is_none());
}

#[test]
fn span_default_equals_zero() {
    assert_eq!(Span::default(), Span::ZERO);
}
