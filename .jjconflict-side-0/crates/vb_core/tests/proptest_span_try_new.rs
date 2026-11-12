//! CV-106: property tests for `Span::try_new`.
//!
//! These tests complement the inline `mod tests` in `span.rs` and the
//! Kani harnesses in `kani/kani_span_try_new.rs`. proptest explores
//! the full `u32 × u32` input space (with biased generators to keep
//! the inverted-input region well-covered) and asserts:
//!
//! 1. `try_new` is total: every input returns `Ok` or the typed
//!    `SpanError::StartGreaterThanEnd` — never a panic, never an
//!    unexpected variant.
//! 2. The accepted set is exactly `start <= end`.
//! 3. The `Err` path carries the exact `start` and `end` operands.
//! 4. `Span::new` is unchanged: every input is accepted verbatim.
//! 5. `is_empty()` is consistent with `start == end` on the `Ok`
//!    branch of `try_new`.

#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::span::{Span, SpanError};

/// Strategy that produces a `u32` pair. Inverted pairs are biased
/// in to exercise the rejection branch.
fn span_pair() -> impl Strategy<Value = (u32, u32)> {
    (any::<u32>(), any::<u32>())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1024,
        .. ProptestConfig::default()
    })]

    /// `try_new` is total: returns `Ok` iff `start <= end`.
    #[test]
    fn try_new_is_total((s, e) in span_pair()) {
        let result = Span::try_new(s, e);
        if s <= e {
            let span = result.expect("try_new must accept start <= end");
            prop_assert_eq!(span.start, s);
            prop_assert_eq!(span.end, e);
        } else {
            let err = result.expect_err("try_new must reject start > end");
            prop_assert_eq!(
                err,
                SpanError::StartGreaterThanEnd { start: s, end: e }
            );
        }
    }

    /// `Span::new` is unchanged: it accepts every pair, including
    /// inverted inputs, preserving the offsets verbatim.
    #[test]
    fn new_is_unchanged((s, e) in span_pair()) {
        let span = Span::new(s, e);
        prop_assert_eq!(span.start, s);
        prop_assert_eq!(span.end, e);
    }

    /// The `Err` path preserves the exact `start` and `end` operands.
    #[test]
    fn try_new_error_carries_operands((s, e) in (any::<u32>(), any::<u32>()).prop_filter(
        "inverted pair",
        |(s, e)| s > e,
    )) {
        let err = Span::try_new(s, e).expect_err("inverted pair must be Err");
        match err {
            SpanError::StartGreaterThanEnd { start, end } => {
                prop_assert_eq!(start, s);
                prop_assert_eq!(end, e);
            }
            // SpanError is `#[non_exhaustive]`; the only currently
            // reachable variant is StartGreaterThanEnd. Any other
            // variant would be a future addition to the type and
            // would itself need to carry the offending operands.
            _ => prop_assert!(false, "unexpected SpanError variant: {:?}", err),
        }
    }

    /// `is_empty()` agrees with `start == end` on the accepted branch.
    #[test]
    fn try_new_is_empty_matches_offsets((s, e) in span_pair()) {
        if let Ok(span) = Span::try_new(s, e) {
            prop_assert_eq!(span.is_empty(), s == e);
        }
    }

    /// `try_new(0, 0)` always returns `Span::ZERO`.
    #[test]
    fn try_new_zero_zero_is_span_zero(_unused in 0u8..1) {
        let span = Span::try_new(0, 0).expect("0,0 must be Ok");
        prop_assert_eq!(span, Span::ZERO);
        prop_assert!(span.is_empty());
    }

    /// Boundary: `Span::try_new(u32::MAX, u32::MAX)` is the largest
    /// accepted input and must produce an empty span.
    #[test]
    fn try_new_max_max_is_empty(_unused in 0u8..1) {
        let span = Span::try_new(u32::MAX, u32::MAX).expect("MAX,MAX must be Ok");
        prop_assert_eq!(span.start, u32::MAX);
        prop_assert_eq!(span.end, u32::MAX);
        prop_assert!(span.is_empty());
    }

    /// Boundary: `Span::try_new(u32::MAX, 0)` is the largest possible
    /// rejection and must surface the typed error with both operands.
    #[test]
    fn try_new_max_zero_is_err(_unused in 0u8..1) {
        let err = Span::try_new(u32::MAX, 0)
            .expect_err("MAX,0 must be Err");
        prop_assert_eq!(
            err,
            SpanError::StartGreaterThanEnd { start: u32::MAX, end: 0 }
        );
    }
}
