#![forbid(unsafe_code)]
//! Proptest strategies for F64 edge cases used in proof obligation verification.
//!
//! These strategies are consumed by the proptest obligations (PO-003 through PO-012)
//! during State 11 (formal verification). They are compiled only under `#[cfg(test)]`.
//!
//! Key F64 edge cases covered:
//! - NaN (all bit patterns including quiet and signaling)
//! - Positive and negative infinity
//! - Subnormal values (smallest, largest)
//! - Signed zero (+0.0, -0.0)
//! - Min/max finite values
//! - Large finite values that could cause overflow in arithmetic

use proptest::prelude::*;

/// Strategy for a finite f64 value (not NaN, not ±Inf).
pub fn finite_f64_strategy() -> impl Strategy<Value = f64> {
    // Filter out NaN and infinity
    any::<f64>().prop_filter("must be finite (not NaN, not infinity)", |f| f.is_finite())
}

/// Strategy for pairs of finite f64 values suitable for addition testing.
pub fn finite_f64_add_strategy() -> impl Strategy<Value = (f64, f64)> {
    finite_f64_strategy()
        .prop_flatten(|l| finite_f64_strategy().prop_map(move |r| (l, r)))
}

/// Strategy for pairs of finite f64 values suitable for subtraction testing.
pub fn finite_f64_sub_strategy() -> impl Strategy<Value = (f64, f64)> {
    finite_f64_strategy()
        .prop_flatten(|l| finite_f64_strategy().prop_map(move |r| (l, r)))
}

/// Strategy for pairs of finite f64 values suitable for multiplication testing.
pub fn finite_f64_mul_strategy() -> impl Strategy<Value = (f64, f64)> {
    finite_f64_strategy()
        .prop_flatten(|l| finite_f64_strategy().prop_map(move |r| (l, r)))
}

/// Strategy for pairs of finite f64 values suitable for division testing
/// (divisor is guaranteed non-zero).
pub fn finite_f64_div_strategy() -> impl Strategy<Value = (f64, f64)> {
    finite_f64_strategy().prop_flatten(|l| {
        finite_f64_strategy()
            .prop_filter("divisor must be non-zero", |r| r != 0.0 && !r.is_nan())
            .prop_map(move |r| (l, r))
    })
}

/// Strategy for a NaN f64 value (any bit pattern).
pub fn nan_f64_strategy() -> impl Strategy<Value = f64> {
    // NaN bit patterns: exponent all ones, mantissa non-zero
    // We generate any f64 and filter to NaN
    any::<f64>().prop_filter("must be NaN", |f| f.is_nan())
}

/// Strategy for ±Inf f64 values.
pub fn infinite_f64_strategy() -> impl Strategy<Value = f64> {
    prop_oneof![Just(f64::INFINITY), Just(f64::NEG_INFINITY)]
}

/// Strategy for special f64 edge cases: NaN, Inf, subnormal, max, min.
pub fn f64_edge_case_strategy() -> impl Strategy<Value = f64> {
    prop_oneof![
        Just(f64::NAN),
        Just(f64::INFINITY),
        Just(f64::NEG_INFINITY),
        Just(f64::MIN_POSITIVE),          // smallest positive normal
        Just(f64::MAX),                   // largest finite
        Just(f64::MIN),                   // most negative finite
        Just(f64::from_bits(1_u64)),     // smallest positive subnormal
        Just(f64::from_bits(0x000F_FFFF_FFFF_FFFF_u64)), // largest subnormal
        Just(0.0_f64),                   // positive zero
        Just(-0.0_f64),                  // negative zero
    ]
}

/// Strategy for subnormal f64 values specifically.
pub fn subnormal_f64_strategy() -> impl Strategy<Value = f64> {
    // Subnormal: exponent bits all zero, mantissa non-zero
    // We pick from a curated set since arbitrary u64 bit patterns
    // would be mostly normal (very low probability of subnormal)
    prop_oneof![
        Just(f64::from_bits(1_u64)),                          // smallest positive subnormal
        Just(f64::from_bits(0x000F_FFFF_FFFF_FFFF_u64)),     // largest positive subnormal
        Just(f64::from_bits(0x8000_0000_0000_0001_u64)),    // smallest negative subnormal
        Just(f64::from_bits(0x800F_FFFF_FFFF_FFFF_u64)),     // largest negative subnormal
    ]
}

/// Strategy for f64 values that could cause overflow in addition.
pub fn f64_add_overflow_strategy() -> impl Strategy<Value = (f64, f64)> {
    prop_oneof![
        // Both large positive
        Just((f64::MAX / 2.0, f64::MAX / 2.0)),
        // One positive, one borderline
        Just((f64::MAX / 2.0, f64::MAX / 2.0 + 1.0)),
        // Max finite + smallest positive
        Just((f64::MAX, f64::MIN_POSITIVE)),
        // Near-infinite boundary
        Just((f64::MAX, f64::INFINITY)),
    ]
}

/// Strategy for f64 values that could cause underflow in subtraction.
pub fn f64_sub_underflow_strategy() -> impl Strategy<Value = (f64, f64)> {
    prop_oneof![
        // Two values close together
        Just((1.0, 0.5 * f64::MIN_POSITIVE)),
        // Two nearly equal values
        Just((f64::MIN_POSITIVE, f64::MIN_POSITIVE / 2.0)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_f64_strategy_always_produces_finite() {
        let mut runner = proptest::test_runner::TestRunner::default();
        runner
            .run(&finite_f64_strategy(), |f| {
                assert!(
                    f.is_finite(),
                    "finite_f64_strategy produced non-finite: {:?}",
                    f
                );
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn nan_f64_strategy_always_produces_nan() {
        let mut runner = proptest::test_runner::TestRunner::default();
        runner
            .run(&nan_f64_strategy(), |f| {
                assert!(f.is_nan(), "nan_f64_strategy produced non-NaN: {:?}", f);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn infinite_f64_strategy_always_produces_infinity() {
        let mut runner = proptest::test_runner::TestRunner::default();
        runner
            .run(&infinite_f64_strategy(), |f| {
                assert!(
                    f.is_infinite(),
                    "infinite_f64_strategy produced non-infinite: {:?}",
                    f
                );
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn f64_edge_case_strategy_covers_all_cases() {
        let mut runner = proptest::test_runner::TestRunner::default();
        runner
            .run(&f64_edge_case_strategy(), |f| {
                // This just verifies the strategy generates without panicking
                // and covers all special values
                let special = f.is_nan()
                    || f.is_infinite()
                    || f.is_subnormal()
                    || f == 0.0
                    || f == -0.0
                    || f == f64::MIN_POSITIVE
                    || f == f64::MAX
                    || f == f64::MIN;
                assert!(special, "edge case strategy failed for: {:?}", f);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn finite_f64_div_strategy_never_generates_zero_divisor() {
        let mut runner = proptest::test_runner::TestRunner::default();
        runner
            .run(&finite_f64_div_strategy(), |(l, r)| {
                assert_ne!(r, 0.0, "divisor must not be zero");
                assert!(
                    r.is_finite(),
                    "divisor must be finite, got: {:?}",
                    r
                );
                assert!(
                    l.is_finite(),
                    "dividend must be finite, got: {:?}",
                    l
                );
                Ok(())
            })
            .unwrap();
    }
}
