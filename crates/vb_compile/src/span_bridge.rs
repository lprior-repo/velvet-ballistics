#![forbid(unsafe_code)]

//! Lossless-or-clamping bridge between YAML source spans and core diagnostic spans.
//!
//! This module converts `vb_yaml::SourceSpan` (usize offsets) and
//! `SourceMark` (parser marks) into `vb_core::Span` (u32 offsets with
//! optional line/column). Values exceeding `u32::MAX` are clamped safely.

use crate::SourceMark;
use vb_core::span::Span;
use vb_yaml::source_map::SourceSpan;

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

/// Clamps a `usize` value to `u32::MAX` without panicking.
///
/// Values > `u32::MAX` are saturated to `u32::MAX`. This is the safe
/// bridge between 64-bit parser offsets and 32-bit diagnostic spans.
#[must_use]
pub fn clamp_u32(value: usize) -> u32 {
    // `u32::try_from` returns Err for values > u32::MAX, which we map to
    // u32::MAX. For values <= u32::MAX, the conversion cannot fail.
    u32::try_from(value).unwrap_or(u32::MAX)
}

// ---------------------------------------------------------------------------
// Bridge: SourceSpan → Span
// ---------------------------------------------------------------------------

/// Converts a YAML [`SourceSpan`] into a core [`Span`].
///
/// Byte offsets, line, and column are clamped to `u32` via [`clamp_u32`].
/// Line and column are always `Some` because `SourceSpan` always carries them.
///
/// This is a free function rather than a `From` impl because both `SourceSpan`
/// and `Span` are foreign types to this crate (orphan rule).
#[must_use]
pub fn span_from_source_span(ss: SourceSpan) -> Span {
    Span {
        start: clamp_u32(ss.start_offset),
        end: clamp_u32(ss.end_offset),
        line: Some(clamp_u32(ss.start_line)),
        column: Some(clamp_u32(ss.start_col)),
    }
}

// ---------------------------------------------------------------------------
// Bridge: SourceMark → Span
// ---------------------------------------------------------------------------

/// Converts a parser [`SourceMark`] into a core [`Span`].
///
/// When `available` is `true`, line and column are propagated as `Some`.
/// When `available` is `false`, line and column are `None` — the mark
/// came from tree-only validation where parser marks are unavailable.
/// Byte offsets are always clamped via [`clamp_u32`].
impl From<SourceMark> for Span {
    fn from(mark: SourceMark) -> Self {
        Self {
            start: clamp_u32(mark.index),
            end: clamp_u32(mark.end_index),
            line: if mark.available {
                Some(clamp_u32(mark.line))
            } else {
                None
            },
            column: if mark.available {
                Some(clamp_u32(mark.column))
            } else {
                None
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::as_conversions)]
mod tests {
    use super::*;

    // --- clamp_u32 ---

    #[test]
    fn clamp_u32_zero() {
        assert_eq!(clamp_u32(0), 0_u32);
    }

    #[test]
    fn clamp_u32_within_range() {
        assert_eq!(clamp_u32(42), 42_u32);
        assert_eq!(clamp_u32(u32::MAX as usize), u32::MAX);
    }

    #[test]
    fn clamp_u32_exceeds_max() {
        assert_eq!(clamp_u32(u32::MAX as usize + 1), u32::MAX);
    }

    #[test]
    fn clamp_u32_usize_max() {
        assert_eq!(clamp_u32(usize::MAX), u32::MAX);
    }

    #[test]
    fn clamp_u32_never_panics() {
        // Exercise extreme values to prove panic-freedom.
        for val in [
            0_usize,
            1,
            u32::MAX as usize,
            u32::MAX as usize + 1,
            usize::MAX,
        ] {
            let _ = clamp_u32(val);
        }
    }

    // --- SourceSpan → Span ---

    #[test]
    fn source_span_to_span_typical() {
        let ss = SourceSpan::new(10, 20, 3, 5, 3, 9);
        let span = span_from_source_span(ss);

        assert_eq!(span.start, 10_u32);
        assert_eq!(span.end, 20_u32);
        assert_eq!(span.line, Some(3_u32));
        assert_eq!(span.column, Some(5_u32));
    }

    #[test]
    fn source_span_to_span_clamps_large_values() {
        let big = u32::MAX as usize + 100;
        let ss = SourceSpan::new(big, big, big, big, big, big);
        let span = span_from_source_span(ss);

        assert_eq!(span.start, u32::MAX);
        assert_eq!(span.end, u32::MAX);
        assert_eq!(span.line, Some(u32::MAX));
        assert_eq!(span.column, Some(u32::MAX));
    }

    #[test]
    fn source_span_to_span_minimal() {
        let ss = SourceSpan::new(0, 0, 1, 1, 1, 3);
        let span = span_from_source_span(ss);

        assert_eq!(span.start, 0_u32);
        assert_eq!(span.end, 0_u32);
        assert!(span.is_empty());
        assert_eq!(span.line, Some(1_u32));
        assert_eq!(span.column, Some(1_u32));
    }

    // --- SourceMark → Span ---

    #[test]
    fn source_mark_available_produces_line_col() {
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
        let mark = SourceMark::unavailable();
        let span: Span = mark.into();

        assert_eq!(span.start, 0_u32);
        assert_eq!(span.end, 0_u32);
        assert_eq!(span.line, None);
        assert_eq!(span.column, None);
    }

    #[test]
    fn source_mark_available_with_large_values_clamped() {
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
    fn source_mark_unavailable_ignores_line_col_values() {
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
    fn bridge_conversions_never_panic() {
        // Exercise extreme values.
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

    #[test]
    fn source_mark_from_parser_span_preserves_data() {
        // B89: SourceMark::from_parser_span preserves index, line, col
        let parser_span = saphyr_parser::Span::empty(saphyr_parser::Marker::new(5, 3, 42));
        let mark = SourceMark::from_parser_span(parser_span);

        assert_eq!(mark.index, 5);
        // end_index matches index for an empty span
        assert_eq!(mark.line, 3);
        assert_eq!(mark.column, 42);
        assert!(mark.available);
    }

    #[test]
    fn source_mark_from_parser_span_always_sets_available_true() {
        // B90: from_parser_span always sets available: true
        let parser_span = saphyr_parser::Span::empty(saphyr_parser::Marker::new(0, 1, 1));
        let mark = SourceMark::from_parser_span(parser_span);

        assert!(mark.available);
    }

    #[test]
    fn source_mark_unavailable_has_all_fields_zero() {
        // B91: SourceMark::unavailable() has available: false, all fields zero
        let mark = SourceMark::unavailable();

        assert_eq!(mark.index, 0);
        assert_eq!(mark.end_index, 0);
        assert_eq!(mark.line, 0);
        assert_eq!(mark.column, 0);
        assert!(!mark.available);
    }

    #[test]
    fn source_mark_unavailable_converts_to_zero_span() {
        // B91: unavailable mark produces Span::ZERO-equivalent
        let mark = SourceMark::unavailable();
        let span: Span = mark.into();

        assert_eq!(span.start, 0);
        assert_eq!(span.end, 0);
        assert_eq!(span.line, None);
        assert_eq!(span.column, None);
    }

    #[test]
    fn clamp_u32_identity_across_full_range() {
        // B77 extended: test many values within u32 range
        let test_values: &[usize] = &[0, 1, 42, 255, 65535, u32::MAX as usize];
        for &val in test_values {
            assert_eq!(
                clamp_u32(val),
                val as u32,
                "clamp_u32({val}) must be identity within u32 range"
            );
        }
    }

    #[test]
    fn clamp_u32_saturates_above_u32_max() {
        // B79: values above u32::MAX saturate to u32::MAX
        assert_eq!(clamp_u32(u32::MAX as usize + 1), u32::MAX);
        assert_eq!(clamp_u32(u32::MAX as usize + 100_000), u32::MAX);
        assert_eq!(clamp_u32(usize::MAX), u32::MAX);
    }

    #[test]
    fn span_from_source_span_pairs_line_and_column() {
        // B83: line and column always Some in output
        let ss = SourceSpan::new(0, 10, 5, 8, 5, 15);
        let span = span_from_source_span(ss);

        assert!(span.line.is_some(), "line must be Some");
        assert!(span.column.is_some(), "column must be Some");
        assert_eq!(span.line.is_some(), span.column.is_some());
    }
}
