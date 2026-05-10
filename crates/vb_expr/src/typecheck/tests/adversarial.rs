#![forbid(unsafe_code)]
//! Adversarial typecheck tests.

#![allow(dead_code, unused_imports, clippy::panic_in_result_fn)]

use crate::ExprError;
use crate::lexer::lex_expr;
use crate::parser::parse_expr;
use crate::typecheck::{TypeContext, typecheck_expr};

fn check(source: &str) -> crate::ExprResult<crate::typecheck::ExprType> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    typecheck_expr(&ast, &TypeContext::new())
}

#[test]
fn typecheck_expr_rejects_null_in_arithmetic() -> crate::ExprResult<()> {
    let result = check("null + 1");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for null + 1".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "null");
    Ok(())
}

#[test]
fn typecheck_expr_rejects_text_in_arithmetic() -> crate::ExprResult<()> {
    let result = check("\"hello\" - 1");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for text - 1".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "text");
    Ok(())
}

#[test]
fn typecheck_expr_rejects_null_in_comparison() -> crate::ExprResult<()> {
    let result = check("null < 1");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for null < 1".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "null");
    Ok(())
}

#[test]
fn typecheck_expr_rejects_number_in_and() -> crate::ExprResult<()> {
    let result = check("1 and 2");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for 1 and 2".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "i64");
    Ok(())
}

#[test]
fn typecheck_expr_rejects_null_in_and() -> crate::ExprResult<()> {
    let result = check("null and true");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for null and true".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "null");
    Ok(())
}

#[test]
fn typecheck_expr_rejects_negation_on_null() -> crate::ExprResult<()> {
    let result = check("-null");
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for -null".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "null");
    Ok(())
}

#[test]
fn typecheck_expr_allows_eq_on_mixed_types() -> crate::ExprResult<()> {
    let ty = check("null == 1")?;
    assert_eq!(ty, crate::typecheck::ExprType::Bool);
    Ok(())
}

#[test]
fn typecheck_expr_allows_not_eq_on_incompatible_types() -> crate::ExprResult<()> {
    let ty = check("true != null")?;
    assert_eq!(ty, crate::typecheck::ExprType::Bool);
    Ok(())
}

// =========================================================================
// BLACKHAT security regression tests -- typecheck
// =========================================================================

/// BH-TC-001: Typecheck rejects null in all arithmetic operations.
#[test]
fn blackhat_tc_001a_null_rejected_in_add() -> crate::ExprResult<()> {
    let result = check("null + 1");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-001: null + 1 should be TypeMismatch"
    );
    Ok(())
}

#[test]
fn blackhat_tc_001b_null_rejected_in_subtract() -> crate::ExprResult<()> {
    let result = check("null - 1");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-001: null - 1 should be TypeMismatch"
    );
    Ok(())
}

#[test]
fn blackhat_tc_001c_null_rejected_in_multiply() -> crate::ExprResult<()> {
    let result = check("null * 1");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-001: null * 1 should be TypeMismatch"
    );
    Ok(())
}

#[test]
fn blackhat_tc_001d_null_rejected_in_divide() -> crate::ExprResult<()> {
    let result = check("null / 1");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-001: null / 1 should be TypeMismatch"
    );
    Ok(())
}

/// BH-TC-002: Typecheck rejects null in all comparison operations.
#[test]
fn blackhat_tc_002a_null_rejected_in_lt() -> crate::ExprResult<()> {
    let result = check("null < 1");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-002: null < 1 should be TypeMismatch"
    );
    Ok(())
}

#[test]
fn blackhat_tc_002b_null_rejected_in_lte() -> crate::ExprResult<()> {
    let result = check("null <= 1");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-002: null <= 1 should be TypeMismatch"
    );
    Ok(())
}

#[test]
fn blackhat_tc_002c_null_rejected_in_gt() -> crate::ExprResult<()> {
    let result = check("null > 1");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-002: null > 1 should be TypeMismatch"
    );
    Ok(())
}

#[test]
fn blackhat_tc_002d_null_rejected_in_gte() -> crate::ExprResult<()> {
    let result = check("null >= 1");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-002: null >= 1 should be TypeMismatch"
    );
    Ok(())
}

/// BH-TC-003: Typecheck rejects non-boolean in logical operators.
#[test]
fn blackhat_tc_003_non_bool_rejected_in_logical() -> crate::ExprResult<()> {
    // number and number
    let result = check("1 and 2");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { expected, .. }) if expected == "boolean"),
        "BH-TC-003a: 1 and 2 should fail"
    );
    // null and true
    let result = check("null and true");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-003b: null and true should fail"
    );
    // 1 or 0
    let result = check("1 or 0");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-003c: 1 or 0 should fail"
    );
    Ok(())
}

/// BH-TC-004: Equality operators allow mixed types (by design).
///
/// `==` and `!=` are polymorphic -- they can compare any two types.
/// This is by design for null-safety checks.
#[test]
fn blackhat_tc_004_equality_allows_mixed_types() -> crate::ExprResult<()> {
    assert_eq!(check("null == 1")?, crate::typecheck::ExprType::Bool);
    assert_eq!(check("true != null")?, crate::typecheck::ExprType::Bool);
    assert_eq!(check("null == null")?, crate::typecheck::ExprType::Bool);
    Ok(())
}

/// BH-TC-005: Negation rejects null and boolean.
#[test]
fn blackhat_tc_005_negation_rejects_non_numeric() -> crate::ExprResult<()> {
    let result = check("-null");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-005a: -null should fail"
    );
    let result = check("-true");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-005b: -true should fail"
    );
    Ok(())
}

/// BH-TC-006: Not rejects non-boolean.
#[test]
fn blackhat_tc_006_not_rejects_non_bool() -> crate::ExprResult<()> {
    let result = check("not 1");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-006a: not 1 should fail"
    );
    let result = check("not null");
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "BH-TC-006b: not null should fail"
    );
    Ok(())
}

/// BH-TC-007: Unknown type allowed through (deferred to runtime).
///
/// Unresolved references produce `Unknown` type which passes through all
/// operators. This is by design -- the eval layer catches type errors.
#[test]
fn blackhat_tc_007_unknown_deferred_to_runtime() -> crate::ExprResult<()> {
    assert_eq!(check("$x + 1")?, crate::typecheck::ExprType::I64);
    assert_eq!(check("not $x")?, crate::typecheck::ExprType::Bool);
    assert_eq!(check("$x and $y")?, crate::typecheck::ExprType::Bool);
    Ok(())
}
