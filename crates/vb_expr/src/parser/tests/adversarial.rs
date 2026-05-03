//! Adversarial parser tests.

use crate::lexer::{lex_expr, BinaryOp, UnaryOp};
use crate::parser::{parse_expr, ExprAst, ExprHelper, ExprLiteral};
use crate::ExprError;

fn parse(source: &str) -> crate::ExprResult<ExprAst> {
    let tokens = lex_expr(source)?;
    parse_expr(&tokens)
}

fn as_binary(expr: &ExprAst) -> crate::ExprResult<(BinaryOp, &ExprAst, &ExprAst)> {
    match expr {
        ExprAst::Binary { op, left, right } => Ok((*op, left, right)),
        other => Err(ExprError::UnexpectedToken {
            token: format!("expected binary, got {other:?}"),
        }),
    }
}

fn as_unary(expr: &ExprAst) -> crate::ExprResult<(UnaryOp, &ExprAst)> {
    match expr {
        ExprAst::Unary { op, expr } => Ok((*op, expr)),
        other => Err(ExprError::UnexpectedToken {
            token: format!("expected unary, got {other:?}"),
        }),
    }
}

#[test]
fn parse_expr_chained_unary_not_true() -> crate::ExprResult<()> {
    let expr = parse("not not not not true")?;
    let (op1, inner1) = as_unary(&expr)?;
    assert_eq!(op1, UnaryOp::Not);
    let (op2, inner2) = as_unary(inner1)?;
    assert_eq!(op2, UnaryOp::Not);
    let (op3, inner3) = as_unary(inner2)?;
    assert_eq!(op3, UnaryOp::Not);
    let (op4, inner4) = as_unary(inner3)?;
    assert_eq!(op4, UnaryOp::Not);
    assert_eq!(*inner4, ExprAst::Literal(ExprLiteral::Bool(true)));
    Ok(())
}

#[test]
fn parse_expr_double_negation_parses_correctly() -> crate::ExprResult<()> {
    let expr = parse("--5")?;
    let (op1, inner1) = as_unary(&expr)?;
    assert_eq!(op1, UnaryOp::Neg);
    let (op2, inner2) = as_unary(inner1)?;
    assert_eq!(op2, UnaryOp::Neg);
    assert_eq!(*inner2, ExprAst::Literal(ExprLiteral::I64(5)));
    Ok(())
}

#[test]
fn parse_expr_rejects_trailing_operator() -> crate::ExprResult<()> {
    let result = parse("1 +");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "trailing operator should produce UnexpectedToken"
    );
    Ok(())
}

#[test]
fn parse_expr_rejects_double_operator() -> crate::ExprResult<()> {
    let result = parse("1 + * 2");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "double operator should produce UnexpectedToken"
    );
    Ok(())
}

#[test]
fn parse_expr_deeply_nested_parentheses_within_limit() -> crate::ExprResult<()> {
    let expr = parse("(((((((1 + 2)))))))")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Add);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(1)));
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(2)));
    Ok(())
}

#[test]
fn parse_expr_rejects_empty_parentheses() -> crate::ExprResult<()> {
    let result = parse("()");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "empty parentheses should produce UnexpectedToken"
    );
    Ok(())
}

#[test]
fn parse_expr_rejects_extra_right_paren() -> crate::ExprResult<()> {
    let result = parse("1)");
    assert!(
        matches!(result, Err(ExprError::UnexpectedToken { .. })),
        "trailing right paren should produce UnexpectedToken"
    );
    Ok(())
}

#[test]
fn parse_expr_rejects_unknown_identifier_without_paren() -> crate::ExprResult<()> {
    let result = parse("foo");
    let Err(ExprError::UnexpectedToken { token }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected UnexpectedToken".into(),
        });
    };
    assert!(
        token.contains("unknown identifier"),
        "token should mention unknown identifier, got: {token}"
    );
    Ok(())
}

#[test]
fn parse_expr_null_equality_parses_as_binary_eq() -> crate::ExprResult<()> {
    let expr = parse("null == null")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Eq);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::Null));
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::Null));
    Ok(())
}

#[test]
fn parse_expr_rejects_helper_with_too_many_args() -> crate::ExprResult<()> {
    let result = parse("contains(1, 2, 3, 4, 5, 6, 7, 8, 9)");
    assert!(
        matches!(result, Err(ExprError::TooManyHelperArgs { len: 9, max: 8 })),
        "9 helper args should exceed the 8-arg limit"
    );
    Ok(())
}
