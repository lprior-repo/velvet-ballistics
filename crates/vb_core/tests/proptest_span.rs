// Proptest: Span constructor properties
// PO-P01: Span constructor for-all properties (C1.1-C1.3)
//
// Properties:
//  1. Span::with_location(start,end,line,col) → line==Some(line) && col==Some(col)
//  2. Span::new(start,end) → line==None && col==None
//  3. Span::ZERO has line==None, col==None, start==0, end==0
//  4. Paired invariant: line.is_some() == col.is_some()
// Strategy: u32::ANY for all fields, filter start <= end

use proptest::prelude::*;
use vb_core::span::Span;

proptest! {
    #[test]
    fn span_with_location_preserves_all_fields(
        start in 0u32..=u32::MAX,
        end in 0u32..=u32::MAX,
        line in 1u32..=u32::MAX,
        col in 1u32..=u32::MAX,
    ) {
        prop_assume!(start <= end);

        let span = Span::with_location(start, end, line, col);

        prop_assert_eq!(span.start, start);
        prop_assert_eq!(span.end, end);
        prop_assert_eq!(span.line, Some(line));
        prop_assert_eq!(span.column, Some(col));
        prop_assert_eq!(span.line.is_some(), span.column.is_some());
    }

    #[test]
    fn span_new_has_no_location(
        start in 0u32..=u32::MAX,
        end in 0u32..=u32::MAX,
    ) {
        prop_assume!(start <= end);

        let span = Span::new(start, end);

        prop_assert_eq!(span.start, start);
        prop_assert_eq!(span.end, end);
        prop_assert!(span.line.is_none());
        prop_assert!(span.column.is_none());
        prop_assert_eq!(span.line.is_some(), span.column.is_some());
    }

    #[test]
    fn span_paired_invariant_holds(
        start in 0u32..=u32::MAX,
        end in 0u32..=u32::MAX,
    ) {
        prop_assume!(start <= end);

        // new() always produces paired None
        let s1 = Span::new(start, end);
        prop_assert_eq!(s1.line.is_some(), s1.column.is_some());
    }

    #[test]
    fn span_with_location_paired_invariant(
        start in 0u32..=u32::MAX,
        end in 0u32..=u32::MAX,
        line in 1u32..=u32::MAX,
        col in 1u32..=u32::MAX,
    ) {
        prop_assume!(start <= end);

        let s2 = Span::with_location(start, end, line, col);
        prop_assert_eq!(s2.line.is_some(), s2.column.is_some());

        // Location returns the pair
        let loc = s2.location();
        prop_assert_eq!(loc, Some((line, col)));
    }

    #[test]
    fn span_is_empty_when_start_equals_end(
        offset in 0u32..=u32::MAX,
    ) {
        let span = Span::new(offset, offset);
        prop_assert!(span.is_empty());
    }

    #[test]
    fn span_is_not_empty_when_start_less_than_end(
        start in 0u32..=(u32::MAX - 1),
    ) {
        let end = start + 1;
        let span = Span::new(start, end);
        prop_assert!(!span.is_empty());
    }
}

/// Zero-parameter tests moved outside proptest! block for compatibility
/// with proptest 1.11.0 (which requires at least one strategy parameter).

#[test]
fn span_zero_is_canonical_empty() {
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
