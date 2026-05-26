// Kani proof: Span paired invariant and constructor correctness
// PO-K01: Span-Enrich invariants (C1.1-C1.3)
//
// Proves against the enriched Span type (line: Option<u32>, column: Option<u32>):
//  1. Span::with_location(l, c) produces line==Some(l) && column==Some(c)
//  2. Span::new(s, e) produces line==None && column==None
//  3. Span::ZERO has line==None && column==None
//  4. The paired invariant: line.is_some() == column.is_some()
//  5. location() returns Some((l,c)) iff both fields are Some
// Assumes: u32 values are bounded to [0, u32::MAX] (true by type)

#![forbid(unsafe_code)]

use crate::span::Span;

#[kani::proof]
#[kani::unwind(3)]
fn span_with_location_produces_paired_invariant() {
    let start: u32 = kani::any();
    let end: u32 = kani::any();
    let line: u32 = kani::any();
    let col: u32 = kani::any();

    // Precondition: start <= end, line >= 1, col >= 1
    // Note: caller must guarantee these; Span::with_location accepts any u32
    // but semantically valid only with line/col >= 1.
    let span = Span::with_location(start, end, line, col);

    // Core invariant: line and column are always paired
    assert_eq!(span.line.is_some(), span.column.is_some());

    // Exact value preservation
    assert_eq!(span.line, Some(line));
    assert_eq!(span.column, Some(col));
    assert_eq!(span.start, start);
    assert_eq!(span.end, end);

    // location() returns the pair
    assert_eq!(span.location(), Some((line, col)));
}

#[kani::proof]
fn span_new_produces_no_location() {
    let start: u32 = kani::any();
    let end: u32 = kani::any();

    let span = Span::new(start, end);

    // No line/column on byte-offset-only spans
    assert!(span.line.is_none());
    assert!(span.column.is_none());
    assert_eq!(span.location(), None);
    assert_eq!(span.start, start);
    assert_eq!(span.end, end);
}

#[kani::proof]
fn span_zero_has_no_location() {
    let span = Span::ZERO;

    // ZERO is the canonical empty/unknown span
    assert!(span.line.is_none());
    assert!(span.column.is_none());
    assert_eq!(span.location(), None);
    assert_eq!(span.start, 0);
    assert_eq!(span.end, 0);
    assert!(span.is_empty());
}

#[kani::proof]
fn span_default_equals_zero() {
    let span = Span::default();

    assert_eq!(span, Span::ZERO);
    assert!(span.line.is_none());
    assert!(span.column.is_none());
    assert_eq!(span.location(), None);
}

/// Master harness proving paired invariant for all constructors.
#[kani::proof]
fn span_paired_invariant_proof() {
    // Verify Span::new always produces paired None fields
    let s1: u32 = kani::any();
    let e1: u32 = kani::any();
    let sp1 = Span::new(s1, e1);
    assert_eq!(sp1.line.is_some(), sp1.column.is_some());

    // Verify Span::with_location always produces paired Some fields
    let s2: u32 = kani::any();
    let e2: u32 = kani::any();
    let l2: u32 = kani::any();
    let c2: u32 = kani::any();
    let sp2 = Span::with_location(s2, e2, l2, c2);
    assert_eq!(sp2.line.is_some(), sp2.column.is_some());

    // Verify Span::ZERO has paired None fields
    let sp3 = Span::ZERO;
    assert_eq!(sp3.line.is_some(), sp3.column.is_some());

    // Verify Span::default() has paired None fields
    let sp4 = Span::default();
    assert_eq!(sp4.line.is_some(), sp4.column.is_some());
}
