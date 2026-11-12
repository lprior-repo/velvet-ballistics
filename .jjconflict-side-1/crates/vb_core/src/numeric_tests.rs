#![forbid(unsafe_code)]
//! Behavioral tests for the shared numeric promotion kernel.

use crate::errors::CoreError;
use crate::ids::{ListId, SymbolId};
use crate::numeric::{NumericError, add, compare, div, mul, sub};
use crate::value::{FiniteF64, SlotValue};
use core::cmp::Ordering;

fn f64_value(raw: f64) -> Result<SlotValue, CoreError> {
    FiniteF64::new(raw).map(SlotValue::F64)
}

fn run(
    op: fn(SlotValue, SlotValue) -> Result<SlotValue, NumericError>,
    left: SlotValue,
    right: SlotValue,
) -> Result<SlotValue, CoreError> {
    op(left, right).map_err(|_| CoreError::InternalInvariantViolation {
        reason: "numeric op failed".into(),
    })
}

fn run_cmp(left: SlotValue, right: SlotValue) -> Result<core::cmp::Ordering, CoreError> {
    compare(left, right).map_err(|_| CoreError::InternalInvariantViolation {
        reason: "numeric compare failed".into(),
    })
}

fn not_numeric_found(left: SlotValue, right: SlotValue) -> Result<&'static str, CoreError> {
    match add(left, right) {
        Err(NumericError::NotNumeric { found }) => Ok(found),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "expected NotNumeric".into(),
        }),
    }
}

#[test]
fn i64_add_preserves_checked_semantics() -> Result<(), CoreError> {
    if run(add, SlotValue::I64(19), SlotValue::I64(23))? != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "19 + 23 should be 42".into(),
        });
    }
    if add(SlotValue::I64(i64::MAX), SlotValue::I64(1)) != Err(NumericError::Overflow) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "i64::MAX + 1 must overflow".into(),
        });
    }
    Ok(())
}

#[test]
fn i64_sub_mul_preserve_checked_semantics() -> Result<(), CoreError> {
    if sub(SlotValue::I64(i64::MIN), SlotValue::I64(1)) != Err(NumericError::Overflow) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "i64::MIN - 1 must overflow".into(),
        });
    }
    if mul(SlotValue::I64(i64::MAX), SlotValue::I64(2)) != Err(NumericError::Overflow) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "i64::MAX * 2 must overflow".into(),
        });
    }
    if run(mul, SlotValue::I64(6), SlotValue::I64(7))? != SlotValue::I64(42) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "6 * 7 should be 42".into(),
        });
    }
    Ok(())
}

#[test]
fn i64_div_preserves_truncation_and_zero_check() -> Result<(), CoreError> {
    if run(div, SlotValue::I64(-10), SlotValue::I64(3))? != SlotValue::I64(-3) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "-10 / 3 must truncate toward zero".into(),
        });
    }
    if div(SlotValue::I64(10), SlotValue::I64(0)) != Err(NumericError::DivisionByZero) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "division by zero must fail".into(),
        });
    }
    if div(SlotValue::I64(i64::MIN), SlotValue::I64(-1)) != Err(NumericError::Overflow) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "i64::MIN / -1 must overflow".into(),
        });
    }
    Ok(())
}

#[test]
fn f64_add_overflow_is_non_finite() -> Result<(), CoreError> {
    let large = f64_value(f64::MAX)?;
    if add(large, large) != Err(NumericError::NonFinite) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "f64::MAX + f64::MAX must be non-finite".into(),
        });
    }
    Ok(())
}

#[test]
fn f64_div_by_zero_is_non_finite_not_division_by_zero() -> Result<(), CoreError> {
    let one = f64_value(1.0)?;
    let zero = f64_value(0.0)?;
    if div(one, zero) != Err(NumericError::NonFinite) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "1.0 / 0.0 must be non-finite".into(),
        });
    }
    if div(zero, zero) != Err(NumericError::NonFinite) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "0.0 / 0.0 must be non-finite".into(),
        });
    }
    Ok(())
}

#[test]
fn mixed_add_promotes_to_f64() -> Result<(), CoreError> {
    let result = run(add, SlotValue::I64(3), f64_value(0.5)?)?;
    if result != f64_value(3.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(3) + F64(0.5) should be F64(3.5)".into(),
        });
    }
    let flipped = run(add, f64_value(1.5)?, SlotValue::I64(2))?;
    if flipped != f64_value(3.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(1.5) + I64(2) should be F64(3.5)".into(),
        });
    }
    Ok(())
}

#[test]
fn mixed_sub_mul_div_promote_to_f64() -> Result<(), CoreError> {
    if run(sub, SlotValue::I64(10), f64_value(0.5)?)? != f64_value(9.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(10) - F64(0.5) should be F64(9.5)".into(),
        });
    }
    if run(mul, f64_value(2.5)?, SlotValue::I64(4))? != f64_value(10.0)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(2.5) * I64(4) should be F64(10.0)".into(),
        });
    }
    if run(div, SlotValue::I64(1), f64_value(2.0)?)? != f64_value(0.5)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(1) / F64(2.0) should be F64(0.5)".into(),
        });
    }
    if run(div, f64_value(7.5)?, SlotValue::I64(2))? != f64_value(3.75)? {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(7.5) / I64(2) should be F64(3.75)".into(),
        });
    }
    Ok(())
}

#[test]
fn mixed_div_by_zero_is_non_finite() -> Result<(), CoreError> {
    if div(SlotValue::I64(1), f64_value(0.0)?) != Err(NumericError::NonFinite) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(1) / F64(0.0) must be non-finite".into(),
        });
    }
    if div(f64_value(1.0)?, SlotValue::I64(0)) != Err(NumericError::NonFinite) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(1.0) / I64(0) must be non-finite".into(),
        });
    }
    Ok(())
}

#[test]
fn mixed_comparison_promotes_operands() -> Result<(), CoreError> {
    if run_cmp(SlotValue::I64(3), f64_value(3.5)?)? != Ordering::Less {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(3) < F64(3.5) should order Less".into(),
        });
    }
    if run_cmp(f64_value(2.0)?, SlotValue::I64(2))? != Ordering::Equal {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(2.0) == I64(2) should order Equal".into(),
        });
    }
    if run_cmp(SlotValue::I64(10), f64_value(9.5)?)? != Ordering::Greater {
        return Err(CoreError::InternalInvariantViolation {
            reason: "I64(10) > F64(9.5) should order Greater".into(),
        });
    }
    if run_cmp(f64_value(-0.5)?, SlotValue::I64(0))? != Ordering::Less {
        return Err(CoreError::InternalInvariantViolation {
            reason: "F64(-0.5) < I64(0) should order Less".into(),
        });
    }
    Ok(())
}

#[test]
fn non_numeric_reports_offending_side() -> Result<(), CoreError> {
    let left_found = not_numeric_found(SlotValue::Bool(true), SlotValue::I64(1))?;
    if left_found != "boolean" {
        return Err(CoreError::InternalInvariantViolation {
            reason: "non-numeric left must report its type".into(),
        });
    }
    let right_found = not_numeric_found(SlotValue::I64(1), SlotValue::Null)?;
    if right_found != "null" {
        return Err(CoreError::InternalInvariantViolation {
            reason: "non-numeric right must report its type".into(),
        });
    }
    let symbol_found = not_numeric_found(
        SlotValue::Symbol(SymbolId::new(0)),
        SlotValue::F64(FiniteF64::new(1.0)?),
    )?;
    if symbol_found != "symbol" {
        return Err(CoreError::InternalInvariantViolation {
            reason: "symbol left must report its type".into(),
        });
    }
    let list_found = not_numeric_found(
        SlotValue::F64(FiniteF64::new(1.0)?),
        SlotValue::List(ListId::new(0)),
    )?;
    if list_found != "list" {
        return Err(CoreError::InternalInvariantViolation {
            reason: "list right must report its type".into(),
        });
    }
    Ok(())
}

fn promoted_i64(value: i64) -> Result<f64, CoreError> {
    match run(
        add,
        SlotValue::I64(value),
        SlotValue::F64(FiniteF64::new(0.0)?),
    )? {
        SlotValue::F64(promoted) => Ok(promoted.get()),
        _ => Err(CoreError::InternalInvariantViolation {
            reason: "mixed add must promote to F64".into(),
        }),
    }
}

fn decimal_oracle(value: i64) -> Result<f64, CoreError> {
    let text = format!("{value}");
    text.parse::<f64>()
        .map_err(|_| CoreError::InternalInvariantViolation {
            reason: "decimal integer text must parse as f64".into(),
        })
}

#[test]
fn promotion_matches_decimal_oracle_at_edges() -> Result<(), CoreError> {
    let edges: [i64; 20] = [
        0,
        1,
        -1,
        2_147_483_647,
        2_147_483_648,
        -2_147_483_648,
        -2_147_483_649,
        4_294_967_295,
        4_294_967_296,
        9_007_199_254_740_991,
        9_007_199_254_740_992,
        9_007_199_254_740_993,
        9_007_199_254_740_994,
        -9_007_199_254_740_993,
        123_456_789_012_345_678,
        -98_765_432_109_876_543,
        i64::MAX,
        i64::MAX - 1,
        i64::MIN,
        i64::MIN + 1,
    ];
    for value in edges {
        if promoted_i64(value)? != decimal_oracle(value)? {
            return Err(CoreError::InternalInvariantViolation {
                reason: "promotion must match decimal oracle".into(),
            });
        }
    }
    Ok(())
}

#[test]
fn promotion_matches_decimal_oracle_on_pseudorandom_sweep() -> Result<(), CoreError> {
    let mut state: u64 = 0x1234_5678_9ABC_DEF1;
    let mut index = 0usize;
    while index < 50_000 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let value = i64::from_ne_bytes(state.to_ne_bytes());
        if promoted_i64(value)? != decimal_oracle(value)? {
            return Err(CoreError::InternalInvariantViolation {
                reason: "promotion must match decimal oracle on sweep".into(),
            });
        }
        index = index
            .checked_add(1)
            .ok_or(CoreError::InternalInvariantViolation {
                reason: "sweep index overflow".into(),
            })?;
    }
    Ok(())
}
