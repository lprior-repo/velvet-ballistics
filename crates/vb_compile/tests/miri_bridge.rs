// Miri test: usize → u32 bridge UB check
// PO-M01: Span bridge conversion (C9.3)
//
// The span bridge is now implemented:
//  - vb_compile::span_bridge::clamp_u32
//  - vb_compile::span_bridge::span_from_source_span
//  - From<SourceMark> for Span
//
// Command:
//   cargo +nightly miri test --test miri_bridge -- --nocapture

use vb_compile::SourceMark;
use vb_compile::span_bridge::{clamp_u32, span_from_source_span};
use vb_core::span::Span;
use vb_yaml::source_map::SourceSpan;

// ---------------------------------------------------------------------------
// clamp_u32: no UB for edge-case values
// ---------------------------------------------------------------------------

#[test]
fn clamp_u32_edge_cases_no_ub() {
    // Zero
    let r0 = clamp_u32(0);
    assert_eq!(r0, 0_u32);

    // Within range
    let r = clamp_u32(42);
    assert_eq!(r, 42_u32);

    // u32::MAX exact
    let r_max = clamp_u32(u32::MAX as usize);
    assert_eq!(r_max, u32::MAX);

    // u32::MAX + 1 (saturates)
    let r_over = clamp_u32(u32::MAX as usize + 1);
    assert_eq!(r_over, u32::MAX);

    // usize::MAX (saturates)
    let r_usize_max = clamp_u32(usize::MAX);
    assert_eq!(r_usize_max, u32::MAX);
}

// ---------------------------------------------------------------------------
// SourceSpan → Span: no UB for extreme values
// ---------------------------------------------------------------------------

#[test]
fn source_span_to_span_edge_cases_no_ub() {
    // Typical case
    let ss = SourceSpan::new(10, 20, 3, 5, 3, 9);
    let span = span_from_source_span(ss);
    assert_eq!(span.start, 10_u32);
    assert_eq!(span.end, 20_u32);
    assert_eq!(span.line, Some(3_u32));
    assert_eq!(span.column, Some(5_u32));

    // Large values — clamped
    let big = u32::MAX as usize + 100;
    let ss2 = SourceSpan::new(big, big, big, big, big, big);
    let span2 = span_from_source_span(ss2);
    assert_eq!(span2.start, u32::MAX);
    assert_eq!(span2.end, u32::MAX);
    assert_eq!(span2.line, Some(u32::MAX));
    assert_eq!(span2.column, Some(u32::MAX));

    // usize::MAX — no UB
    let ss3 = SourceSpan::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    );
    let span3 = span_from_source_span(ss3);
    assert_eq!(span3.start, u32::MAX);
    assert_eq!(span3.end, u32::MAX);
    assert_eq!(span3.line, Some(u32::MAX));
    assert_eq!(span3.column, Some(u32::MAX));

    // Minimal values
    let ss4 = SourceSpan::new(0, 0, 1, 1, 1, 1);
    let span4 = span_from_source_span(ss4);
    assert_eq!(span4.start, 0_u32);
    assert_eq!(span4.end, 0_u32);
    assert_eq!(span4.line, Some(1_u32));
    assert_eq!(span4.column, Some(1_u32));
}

// ---------------------------------------------------------------------------
// SourceMark → Span: no UB for edge cases
// ---------------------------------------------------------------------------

#[test]
fn source_mark_to_span_edge_cases_no_ub() {
    // Available mark
    let mark = SourceMark {
        index: 100,
        end_index: 200,
        line: 5,
        column: 10,
        available: true,
    };
    let span: Span = mark.into();
    assert_eq!(span.start, 100_u32);
    assert_eq!(span.end, 200_u32);
    assert_eq!(span.line, Some(5_u32));
    assert_eq!(span.column, Some(10_u32));

    // Unavailable mark (constructed inline since unavailable() is pub(crate))
    let mark2 = SourceMark {
        index: 0,
        end_index: 0,
        line: 0,
        column: 0,
        available: false,
    };
    let span2: Span = mark2.into();
    assert_eq!(span2.start, 0_u32);
    assert_eq!(span2.end, 0_u32);
    assert_eq!(span2.line, None);
    assert_eq!(span2.column, None);

    // Unavailable with non-zero fields — UB check
    let mark3 = SourceMark {
        index: usize::MAX,
        end_index: usize::MAX,
        line: usize::MAX,
        column: usize::MAX,
        available: false,
    };
    let span3: Span = mark3.into();
    assert_eq!(span3.start, u32::MAX);
    assert_eq!(span3.end, u32::MAX);
    assert_eq!(span3.line, None);
    assert_eq!(span3.column, None);

    // Available with usize::MAX — UB check
    let mark4 = SourceMark {
        index: usize::MAX,
        end_index: usize::MAX,
        line: usize::MAX,
        column: usize::MAX,
        available: true,
    };
    let span4: Span = mark4.into();
    assert_eq!(span4.start, u32::MAX);
    assert_eq!(span4.end, u32::MAX);
    assert_eq!(span4.line, Some(u32::MAX));
    assert_eq!(span4.column, Some(u32::MAX));
}

// ---------------------------------------------------------------------------
// Core Span invariants under Miri
// ---------------------------------------------------------------------------

#[test]
fn span_invariants_under_miri() {
    // ZERO invariants
    let zero = Span::ZERO;
    assert!(zero.is_empty());
    assert_eq!(zero.line, None);
    assert_eq!(zero.column, None);
    assert_eq!(zero.location(), None);
    assert_eq!(zero.start, 0);
    assert_eq!(zero.end, 0);

    // Span::new invariants
    let span = Span::new(10, 20);
    assert!(!span.is_empty());
    assert_eq!(span.start, 10);
    assert_eq!(span.end, 20);
    assert_eq!(span.line, None);
    assert_eq!(span.column, None);

    // Span::with_location invariants
    let loc = Span::with_location(5, 15, 3, 7);
    assert_eq!(loc.line, Some(3));
    assert_eq!(loc.column, Some(7));
    assert_eq!(loc.location(), Some((3, 7)));

    // Copy + Clone safety
    let copied = loc;
    assert_eq!(copied, loc);
    let cloned = loc.clone();
    assert_eq!(cloned, loc);

    // Edge cases
    let max_span = Span::new(u32::MAX, u32::MAX);
    assert!(max_span.is_empty());
    assert_eq!(max_span.start, u32::MAX);

    let max_loc = Span::with_location(0, 100, u32::MAX, u32::MAX);
    assert_eq!(max_loc.line, Some(u32::MAX));
    assert_eq!(max_loc.column, Some(u32::MAX));
    assert_eq!(max_loc.location(), Some((u32::MAX, u32::MAX)));
}

// ---------------------------------------------------------------------------
// Composite bridge test: full end-to-end UB check
// ---------------------------------------------------------------------------

#[test]
fn usize_bridge_no_ub() {
    // clamp_u32 on all edge values
    clamp_u32_edge_cases_no_ub();

    // SourceSpan → Span on all edge values
    source_span_to_span_edge_cases_no_ub();

    // SourceMark → Span on all edge values
    source_mark_to_span_edge_cases_no_ub();

    // Core Span invariants
    span_invariants_under_miri();
}
