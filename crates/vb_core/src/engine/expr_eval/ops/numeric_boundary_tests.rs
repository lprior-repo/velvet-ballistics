// Numeric operator boundary tests: comparisons, typed errors, promotion, taint.
// Uses validated EvalExpr->Finish drive through real workflow.

use crate::errors::EngineError;
use crate::value::{ConstValue, FiniteF64, SlotValue, Taint};
use crate::workflow::ExprOp;

use super::f64_const;

fn slot_f64(value: f64) -> Result<SlotValue, String> {
    FiniteF64::new(value)
        .map(SlotValue::F64)
        .map_err(|error| error.to_string())
}

fn to_const(value: SlotValue) -> Result<ConstValue, String> {
    match value {
        SlotValue::I64(n) => Ok(ConstValue::I64(n)),
        SlotValue::F64(f) => Ok(ConstValue::F64(f)),
        other => Err(format!("expected numeric fixture, got {other:?}")),
    }
}

fn left_as_f64(value: &SlotValue) -> Result<f64, String> {
    match value {
        SlotValue::I64(n) => {
            let n_i32 = i32::try_from(*n).map_err(|_| "i64 too large for test")?;
            Ok(f64::from(n_i32))
        }
        SlotValue::F64(f) => Ok(f.get()),
        other => Err(format!("unexpected {other:?}")),
    }
}

fn right_as_f64(value: &SlotValue) -> Result<f64, String> {
    match value {
        SlotValue::I64(n) => {
            let n_i32 = i32::try_from(*n).map_err(|_| "i64 too large for test")?;
            Ok(f64::from(n_i32))
        }
        SlotValue::F64(f) => Ok(f.get()),
        other => Err(format!("unexpected {other:?}")),
    }
}

type ComparisonCase = (&'static str, ExprOp, SlotValue, SlotValue, bool);
type ComparisonPairing = (&'static str, bool, bool);

include!("numeric_comparison_tests.rs");
include!("numeric_error_tests.rs");
include!("numeric_rounding_tests.rs");
include!("numeric_taint_tests.rs");

#[test]
fn comparison_outcomes_all_pairings() -> Result<(), String> {
    for case in COMPARISON_CASES {
        for pairing in COMPARISON_PAIRINGS {
            check_comparison_case(case, pairing)?;
        }
    }
    Ok(())
}

#[test]
fn exact_typed_errors() -> Result<(), String> {
    check_arithmetic_overflows()?;
    check_division_overflow()?;
    check_type_mismatches()?;
    check_integer_division_zero()?;
    check_float_division_zero()?;
    let f64_max = f64_const(f64::MAX)?;
    check_f64_max_add(&f64_max)?;
    check_f64_max_mul(f64_max)?;
    Ok(())
}

#[test]
fn exact_i64_promotion_rounding() -> Result<(), String> {
    for case in ROUNDING_CASES {
        check_promotion_case(case)?;
    }
    let two_53 = f64_const(9007199254740992.0)?;
    check_rounded_gt(&two_53)?;
    check_rounded_gte(two_53)?;
    Ok(())
}

#[test]
fn taint_all_ops_pairings() -> Result<(), String> {
    let pairings = taint_type_pairings()?;
    for case in NUMERIC_TAINT_CASES {
        for pairing in pairings {
            for side in TAINT_SIDES {
                check_taint_case(case, pairing, side)?;
            }
        }
    }
    Ok(())
}
