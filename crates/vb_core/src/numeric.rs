#![forbid(unsafe_code)]
//! Shared numeric promotion kernel for expression evaluation.
//!
//! Single implementation of mixed `I64`/`F64` coercion used by the forward
//! evaluator (`vb_expr`) and deterministic replay (`vb_core::replay`).
//! Mirrors the typechecker's `coerce_numeric`: any `F64` operand promotes the
//! operation to `F64`; pure `I64` operations keep checked integer semantics.
//! Non-numeric operands always fail; each consumer maps [`NumericError`] to
//! its own error type.

use core::cmp::Ordering;

use crate::value::{FiniteF64, SlotValue};

/// Failure modes for shared numeric evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericError {
    /// At least one operand is not numeric; carries the offending type name.
    NotNumeric {
        /// Runtime type name of the offending operand.
        found: &'static str,
    },
    /// A checked integer operation overflowed.
    Overflow,
    /// Integer division by zero.
    DivisionByZero,
    /// A floating-point result was non-finite.
    NonFinite,
}

/// Adds two numeric slot values with mixed promotion.
pub fn add(left: SlotValue, right: SlotValue) -> Result<SlotValue, NumericError> {
    match coerce_pair(left, right)? {
        NumericPair::I64(left, right) => left
            .checked_add(right)
            .map(SlotValue::I64)
            .ok_or(NumericError::Overflow),
        NumericPair::F64(left, right) => finite(left + right).map(SlotValue::F64),
    }
}

/// Subtracts two numeric slot values with mixed promotion.
pub fn sub(left: SlotValue, right: SlotValue) -> Result<SlotValue, NumericError> {
    match coerce_pair(left, right)? {
        NumericPair::I64(left, right) => left
            .checked_sub(right)
            .map(SlotValue::I64)
            .ok_or(NumericError::Overflow),
        NumericPair::F64(left, right) => finite(left - right).map(SlotValue::F64),
    }
}

/// Multiplies two numeric slot values with mixed promotion.
pub fn mul(left: SlotValue, right: SlotValue) -> Result<SlotValue, NumericError> {
    match coerce_pair(left, right)? {
        NumericPair::I64(left, right) => left
            .checked_mul(right)
            .map(SlotValue::I64)
            .ok_or(NumericError::Overflow),
        NumericPair::F64(left, right) => finite(left * right).map(SlotValue::F64),
    }
}

/// Divides two numeric slot values with mixed promotion.
///
/// Integer division truncates toward zero; dividing by zero fails.
/// Floating-point division never reports division by zero: a zero divisor
/// yields an infinite or NaN quotient, which fails as non-finite.
pub fn div(left: SlotValue, right: SlotValue) -> Result<SlotValue, NumericError> {
    match coerce_pair(left, right)? {
        NumericPair::I64(left, right) => {
            if right == 0 {
                return Err(NumericError::DivisionByZero);
            }
            left.checked_div(right)
                .map(SlotValue::I64)
                .ok_or(NumericError::Overflow)
        }
        NumericPair::F64(left, right) => finite(left / right).map(SlotValue::F64),
    }
}

/// Orders two numeric slot values with mixed promotion.
///
/// Mixed pairs promote to `F64` before comparing, matching the typechecker's
/// numeric coercion: the comparison sees the promoted values.
pub fn compare(left: SlotValue, right: SlotValue) -> Result<Ordering, NumericError> {
    match coerce_pair(left, right)? {
        NumericPair::I64(left, right) => Ok(left.cmp(&right)),
        NumericPair::F64(left, right) => left.partial_cmp(&right).ok_or(NumericError::NonFinite),
    }
}

enum NumericPair {
    I64(i64, i64),
    F64(f64, f64),
}

fn is_numeric(value: SlotValue) -> bool {
    matches!(value, SlotValue::I64(_) | SlotValue::F64(_))
}

fn coerce_pair(left: SlotValue, right: SlotValue) -> Result<NumericPair, NumericError> {
    match (left, right) {
        (SlotValue::I64(left), SlotValue::I64(right)) => Ok(NumericPair::I64(left, right)),
        (SlotValue::F64(left), SlotValue::F64(right)) => {
            Ok(NumericPair::F64(left.get(), right.get()))
        }
        (SlotValue::I64(number), SlotValue::F64(float)) => {
            Ok(NumericPair::F64(promote_i64(number)?, float.get()))
        }
        (SlotValue::F64(float), SlotValue::I64(number)) => {
            Ok(NumericPair::F64(float.get(), promote_i64(number)?))
        }
        (other_left, other_right) => {
            if is_numeric(other_left) {
                Err(NumericError::NotNumeric {
                    found: other_right.type_name(),
                })
            } else {
                Err(NumericError::NotNumeric {
                    found: other_left.type_name(),
                })
            }
        }
    }
}

fn finite(value: f64) -> Result<FiniteF64, NumericError> {
    FiniteF64::new(value).map_err(|_| NumericError::NonFinite)
}

/// Exactly-rounded `i64` to `f64` conversion without `as` casts.
///
/// Splits the value into high and low 32-bit halves (both exactly
/// representable) and combines them with a single rounding via `mul_add`,
/// which computes `high * 2^32 + low` with an infinite-precision product and
/// one final rounding. The decomposition is total: an arithmetic right shift
/// by 32 always lands in `i32` range, and masking keeps the low 32 bits in
/// `u32` range, so the checked conversions below cannot fail.
fn promote_i64(value: i64) -> Result<f64, NumericError> {
    let high = value.checked_shr(32).ok_or(NumericError::Overflow)?;
    let high = i32::try_from(high).map_err(|_| NumericError::Overflow)?;
    let low = u32::try_from(value & 0xFFFF_FFFF).map_err(|_| NumericError::Overflow)?;
    Ok(f64::from(high).mul_add(4_294_967_296.0, f64::from(low)))
}

#[cfg(test)]
#[path = "numeric_tests.rs"]
mod tests;
