//! Adversarial typecheck tests.

use crate::lexer::lex_expr;
use crate::parser::parse_expr;
use crate::typecheck::{typecheck_expr, TypeContext};
use crate::ExprError;

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
