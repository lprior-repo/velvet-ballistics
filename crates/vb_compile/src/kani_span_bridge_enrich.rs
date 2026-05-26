// Kani proof: Span bridge conversion panic-freedom and correctness
// PO-K07: Span bridging (C9.1-C9.3)
//
// The span bridge infrastructure is now implemented:
//  - vb_compile::span_bridge::clamp_u32
//  - vb_compile::span_bridge::span_from_source_span
//  - From<SourceMark> for Span
//
// Proves:
//   1. clamp_u32 never panics for any usize value
//   2. clamp_u32 correctly clamps values > u32::MAX to u32::MAX
//   3. span_from_source_span produces correct clamped values
//   4. SourceMark→Span conversion preserves available flag behavior

#![forbid(unsafe_code)]

use crate::SourceMark;
use crate::span_bridge::{clamp_u32, span_from_source_span};
use vb_core::span::Span;
use vb_yaml::source_map::SourceSpan;

// ---------------------------------------------------------------------------
// clamp_u32 correctness and panic-freedom
// ---------------------------------------------------------------------------

/// clamp_u32 never panics for any usize value, and values within u32 range
/// are identity-mapped.
#[kani::proof]
#[kani::unwind(3)]
fn clamp_u32_identity_and_no_panic() {
    let val: usize = kani::any();
    // No assumption — exercise the full usize range including extreme values.
    let result = clamp_u32(val);
    if val <= u32::MAX as usize {
        assert_eq!(
            result, val as u32,
            "within-range values must be identity-mapped"
        );
    } else {
        assert_eq!(
            result,
            u32::MAX,
            "out-of-range values must clamp to u32::MAX"
        );
    }
}

/// clamp_u32 at specific boundary values.
#[kani::proof]
fn clamp_u32_boundary_values() {
    // Zero
    assert_eq!(clamp_u32(0), 0_u32);
    // One
    assert_eq!(clamp_u32(1), 1_u32);
    // u32::MAX exact
    assert_eq!(clamp_u32(u32::MAX as usize), u32::MAX);
    // u32::MAX + 1 → clamped
    assert_eq!(clamp_u32(u32::MAX as usize + 1), u32::MAX);
}

// ---------------------------------------------------------------------------
// SourceSpan → Span conversion
// ---------------------------------------------------------------------------

/// span_from_source_span never panics and preserves data within u32 range.
#[kani::proof]
#[kani::unwind(5)]
fn source_span_to_span_no_panic() {
    let start_offset: usize = kani::any();
    let end_offset: usize = kani::any();
    let start_line: usize = kani::any();
    let start_col: usize = kani::any();
    let end_line: usize = kani::any();
    let end_col: usize = kani::any();

    let ss = SourceSpan::new(
        start_offset,
        end_offset,
        start_line,
        start_col,
        end_line,
        end_col,
    );

    // Conversion must never panic for any input.
    let span = span_from_source_span(ss);

    // Offsets are clamped.
    assert_eq!(span.start, clamp_u32(start_offset));
    assert_eq!(span.end, clamp_u32(end_offset));

    // Line and column are always Some (SourceSpan always carries them).
    assert_eq!(span.line, Some(clamp_u32(start_line)));
    assert_eq!(span.column, Some(clamp_u32(start_col)));

    // Paired invariant must hold.
    assert_eq!(span.line.is_some(), span.column.is_some());
}

/// span_from_source_span with specific boundary values.
#[kani::proof]
fn source_span_boundary_values() {
    // Within-range values
    let ss = SourceSpan::new(10, 20, 3, 5, 3, 9);
    let span = span_from_source_span(ss);
    assert_eq!(span.start, 10_u32);
    assert_eq!(span.end, 20_u32);
    assert_eq!(span.line, Some(3_u32));
    assert_eq!(span.column, Some(5_u32));

    // Large values clamped
    let big = u32::MAX as usize + 100;
    let ss2 = SourceSpan::new(big, big, big, big, big, big);
    let span2 = span_from_source_span(ss2);
    assert_eq!(span2.start, u32::MAX);
    assert_eq!(span2.end, u32::MAX);
    assert_eq!(span2.line, Some(u32::MAX));
    assert_eq!(span2.column, Some(u32::MAX));
}

// ---------------------------------------------------------------------------
// SourceMark → Span conversion (From impl)
// ---------------------------------------------------------------------------

/// From<SourceMark> for Span preserves the available flag.
#[kani::proof]
#[kani::unwind(5)]
fn source_mark_available_produces_some_line_col() {
    let index: usize = kani::any();
    let end_index: usize = kani::any();
    let line: usize = kani::any();
    let column: usize = kani::any();

    let mark = SourceMark {
        index,
        end_index,
        line,
        column,
        available: true,
    };
    let span: Span = mark.into();

    assert_eq!(span.start, clamp_u32(index));
    assert_eq!(span.end, clamp_u32(end_index));
    assert_eq!(span.line, Some(clamp_u32(line)));
    assert_eq!(span.column, Some(clamp_u32(column)));
    // Paired invariant holds.
    assert_eq!(span.line.is_some(), span.column.is_some());
}

/// From<SourceMark> for Span with unavailable produces None line/col.
#[kani::proof]
#[kani::unwind(5)]
fn source_mark_unavailable_produces_none_line_col() {
    let index: usize = kani::any();
    let end_index: usize = kani::any();

    let mark = SourceMark {
        index,
        end_index,
        line: 0,
        column: 0,
        available: false,
    };
    let span: Span = mark.into();

    assert_eq!(span.start, clamp_u32(index));
    assert_eq!(span.end, clamp_u32(end_index));
    assert_eq!(span.line, None);
    assert_eq!(span.column, None);
}

/// SourceMark unavailable ignores even non-zero line/col values.
#[kani::proof]
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

/// SourceMark::unavailable() → Span produces zero offsets and None line/col.
#[kani::proof]
fn source_mark_unavailable_constructor_to_span() {
    let span: Span = SourceMark::unavailable().into();
    assert_eq!(span.start, 0_u32);
    assert_eq!(span.end, 0_u32);
    assert_eq!(span.line, None);
    assert_eq!(span.column, None);
    assert!(span.is_empty());
}

// ---------------------------------------------------------------------------
// Exhaustive range bridge: all usize values tested
// ---------------------------------------------------------------------------

/// Bridge conversion never panics with max-value SourceSpan.
#[kani::proof]
fn bridge_max_values_no_panic() {
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
