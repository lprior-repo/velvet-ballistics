mod tests {
    use crate::{Located, SourceMap, Span, SpanError, Spanned as SpannedAlias};

    #[test]
    fn zero_span_is_empty() {
        assert!(Span::ZERO.is_empty());
        assert_eq!(Span::ZERO, Span::new(0, 0));
    }

    #[test]
    fn span_preserves_offsets() {
        let span = Span::new(2, 5);

        assert_eq!(span.start, 2);
        assert_eq!(span.end, 5);
        assert!(!span.is_empty());
    }

    #[test]
    fn located_and_spanned_hold_value_and_span() {
        let located = Located::new(42_u32, Span::ZERO);
        let spanned: SpannedAlias<u32> = located.clone();

        assert_eq!(located.value, 42);
        assert_eq!(spanned.span, Span::ZERO);
    }

    #[test]
    fn source_map_placeholder_is_constructible() {
        let map = SourceMap::new();

        assert_eq!(map, SourceMap::default());
    }

    // =========================================================================
    // Additional edge-case tests — Span construction, equality, located values
    // =========================================================================

    #[test]
    fn span_default_is_zero() {
        assert_eq!(Span::default(), Span::ZERO);
        assert!(Span::default().is_empty());
    }

    #[test]
    fn span_new_at_max_offsets() {
        let span = Span::new(u32::MAX, u32::MAX);
        assert!(span.is_empty());
        assert_eq!(span.start, u32::MAX);
        assert_eq!(span.end, u32::MAX);
    }

    #[test]
    fn span_new_with_start_equal_end_is_empty() {
        let span = Span::new(100, 100);
        assert!(span.is_empty());
    }

    #[test]
    fn span_new_with_start_less_than_end_is_not_empty() {
        let span = Span::new(5, 10);
        assert!(!span.is_empty());
    }

    #[test]
    fn span_equality_same_offsets() {
        assert_eq!(Span::new(10, 20), Span::new(10, 20));
    }

    #[test]
    fn span_inequality_different_start() {
        assert_ne!(Span::new(0, 10), Span::new(1, 10));
    }

    #[test]
    fn span_inequality_different_end() {
        assert_ne!(Span::new(0, 10), Span::new(0, 20));
    }

    #[test]
    fn span_copy_preserves_equality() {
        let a = Span::new(5, 15);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn span_clone_preserves_equality() {
        let a = Span::new(5, 15);
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn span_debug_format_contains_offsets() {
        let span = Span::new(10, 20);
        let debug = format!("{span:?}");
        assert!(debug.contains("Span"), "Debug must contain 'Span'");
    }

    #[test]
    fn located_new_preserves_value_and_span() {
        let span = Span::new(1, 5);
        let located = Located::new(42_u32, span);
        assert_eq!(located.value, 42);
        assert_eq!(located.span, span);
    }

    #[test]
    fn spanned_alias_works_same_as_located() {
        let span = Span::new(3, 7);
        let spanned: SpannedAlias<i64> = SpannedAlias::new(-1, span);
        assert_eq!(spanned.value, -1);
        assert_eq!(spanned.span, span);
    }

    #[test]
    fn located_clone_preserves_equality() {
        let span = Span::new(0, 10);
        let a = Located::new(String::from("test"), span);
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn source_map_equality() {
        assert_eq!(SourceMap::new(), SourceMap::new());
        assert_eq!(SourceMap::new(), SourceMap::default());
    }

    #[test]
    fn source_map_debug_format() {
        let map = SourceMap::new();
        let debug = format!("{map:?}");
        assert!(
            debug.contains("SourceMap"),
            "Debug must contain 'SourceMap'"
        );
    }

    #[test]
    fn span_single_byte_span() {
        let span = Span::new(5, 6);
        assert!(!span.is_empty());
        assert_eq!(span.start, 5);
        assert_eq!(span.end, 6);
    }

    #[test]
    fn span_large_span() {
        let span = Span::new(0, u32::MAX);
        assert!(!span.is_empty());
    }

    // =========================================================================
    // Span::try_new (CV-106) — checked constructor + SpanError behaviour
    // =========================================================================

    #[test]
    fn try_new_accepts_start_less_than_end() {
        let span = Span::try_new(2, 5).expect("start < end must be Ok");
        assert_eq!(span, Span::new(2, 5));
        assert!(!span.is_empty());
    }

    #[test]
    fn try_new_accepts_start_equal_end() {
        let span = Span::try_new(7, 7).expect("start == end must be Ok");
        assert!(span.is_empty());
        assert_eq!(span, Span::new(7, 7));
    }

    #[test]
    fn try_new_accepts_zero_zero() {
        let span = Span::try_new(0, 0).expect("0,0 must be Ok");
        assert_eq!(span, Span::ZERO);
        assert!(span.is_empty());
    }

    #[test]
    fn try_new_accepts_zero_to_max() {
        let span = Span::try_new(0, u32::MAX).expect("0,MAX must be Ok");
        assert!(!span.is_empty());
        assert_eq!(span.start, 0);
        assert_eq!(span.end, u32::MAX);
    }

    #[test]
    fn try_new_accepts_max_to_max() {
        let span = Span::try_new(u32::MAX, u32::MAX).expect("MAX,MAX must be Ok");
        assert!(span.is_empty());
        assert_eq!(span.start, u32::MAX);
        assert_eq!(span.end, u32::MAX);
    }

    #[test]
    fn try_new_rejects_start_greater_than_end() {
        let err = Span::try_new(5, 3).expect_err("start > end must be Err");
        assert_eq!(err, SpanError::StartGreaterThanEnd { start: 5, end: 3 });
    }

    #[test]
    fn try_new_rejects_one_above_boundary() {
        // The smallest possible inversion: end = start - 1.
        let err = Span::try_new(10, 9).expect_err("start - 1 must be Err");
        assert_eq!(err, SpanError::StartGreaterThanEnd { start: 10, end: 9 });
    }

    #[test]
    fn try_new_rejects_max_zero_pair() {
        // The largest possible inversion: start = MAX, end = 0.
        let err = Span::try_new(u32::MAX, 0).expect_err("MAX,0 must be Err");
        assert_eq!(
            err,
            SpanError::StartGreaterThanEnd {
                start: u32::MAX,
                end: 0,
            }
        );
    }

    #[test]
    fn try_new_error_carries_offending_operands() {
        // The error must preserve the exact operands for diagnostics.
        let err = Span::try_new(42, 17).expect_err("must be Err");
        match err {
            SpanError::StartGreaterThanEnd { start, end } => {
                assert_eq!(start, 42);
                assert_eq!(end, 17);
            }
        }
    }

    #[test]
    fn try_new_preserves_existing_new_semantics() {
        // `new` must remain unchecked and accept the same inputs as
        // before; `try_new` is strictly additive.
        let inverted = Span::new(7, 3);
        assert_eq!(inverted.start, 7);
        assert_eq!(inverted.end, 3);

        // The same input via `try_new` is rejected.
        assert!(Span::try_new(7, 3).is_err());
    }

    #[test]
    fn span_error_display_is_human_readable() {
        let err = SpanError::StartGreaterThanEnd { start: 9, end: 4 };
        let rendered = format!("{err}");
        assert!(
            rendered.contains('9'),
            "display must include start: {rendered}"
        );
        assert!(
            rendered.contains('4'),
            "display must include end: {rendered}"
        );
    }
}
