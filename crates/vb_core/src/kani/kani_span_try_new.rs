//! CV-106: Kani harnesses for `Span::try_new` and `Span::new`.
//!
//! These harnesses prove, at the bit level, the correctness contract
//! of the CV-106 fix:
//!
//! - PO1: `Span::try_new(s, e)` returns `Ok(_)` iff `s <= e`.
//! - PO2: On the `Err` path, the returned error carries the exact
//!   `start` and `end` operands.
//! - PO3: `Span::new(s, e)` is unchanged: always returns
//!   `Span { start: s, end: e }`, including inverted inputs.
//!
//! The harnesses use `kani::any()` so the proof is over every
//! `u32 × u32` pair Kani can represent. Kani operates at the bit
//! level, so the per-input state space is bounded by the bit width
//! (32 + 32 = 64 symbolic bits), not the value space.

#![forbid(unsafe_code)]

use crate::span::{Span, SpanError};

#[cfg(kani)]
mod harnesses {
    use super::{Span, SpanError};

    /// PO1 + PO2: For any `s, e: u32`, `Span::try_new(s, e)` returns
    /// `Ok` iff `s <= e`, and on the `Err` path the operands are
    /// preserved verbatim.
    #[kani::proof]
    fn kani_span_try_new_returns_ok_or_err() {
        let s: u32 = kani::any();
        let e: u32 = kani::any();

        let result = Span::try_new(s, e);

        if s <= e {
            assert!(
                result.is_ok(),
                "try_new must return Ok when start <= end: \
                 got {:?} for ({}, {})",
                result,
                s,
                e,
            );
            let span = result.expect("Ok branch above");
            assert_eq!(span.start, s, "Ok branch must preserve start");
            assert_eq!(span.end, e, "Ok branch must preserve end");
        } else {
            assert!(
                result.is_err(),
                "try_new must return Err when start > end: \
                 got {:?} for ({}, {})",
                result,
                s,
                e,
            );
            let err = result.expect_err("Err branch above");
            match err {
                SpanError::StartGreaterThanEnd { start, end } => {
                    assert_eq!(start, s, "Err must carry the original start");
                    assert_eq!(end, e, "Err must carry the original end");
                }
            }
        }
    }

    /// PO2 (focused): For any inverted pair `s > e`, the `Err`
    /// variant matches `SpanError::StartGreaterThanEnd { start: s, end: e }`
    /// exactly. Uses `kani::assume(s > e)` to scope the proof to the
    /// rejection branch.
    #[kani::proof]
    fn kani_span_try_new_error_carries_operands() {
        let s: u32 = kani::any();
        let e: u32 = kani::any();
        kani::assume(s > e);

        let result = Span::try_new(s, e);
        let err = result.expect_err("inverted pair must produce Err");

        assert_eq!(
            err,
            SpanError::StartGreaterThanEnd { start: s, end: e },
            "Err variant must carry the exact operands",
        );
    }

    /// PO3: `Span::new` is unchanged. It must accept any
    /// `u32 × u32` pair, including inverted inputs, and return the
    /// offsets verbatim. This is a regression check: the fix must
    /// not change the existing unchecked constructor's behaviour.
    #[kani::proof]
    fn kani_span_new_unchanged() {
        let s: u32 = kani::any();
        let e: u32 = kani::any();

        let span = Span::new(s, e);
        assert_eq!(span.start, s, "new must preserve start");
        assert_eq!(span.end, e, "new must preserve end");

        // The constructed Span must compare equal to a direct struct
        // literal — guards against any accidental field reordering or
        // transformation inside `new`.
        let literal = Span { start: s, end: e };
        assert_eq!(span, literal, "new must equal a direct struct literal");
    }

    /// PO1 (positive branch only): for any `s <= e`, the returned
    /// `Span` has `is_empty()` true exactly when `s == e`. This
    /// binds the `try_new` post-state to the `is_empty()` predicate.
    #[kani::proof]
    fn kani_span_try_new_is_empty_consistent() {
        let s: u32 = kani::any();
        let e: u32 = kani::any();
        kani::assume(s <= e);

        let span = Span::try_new(s, e).expect("try_new must accept start <= end");
        assert_eq!(
            span.is_empty(),
            s == e,
            "is_empty must agree with start == end",
        );
    }
}
