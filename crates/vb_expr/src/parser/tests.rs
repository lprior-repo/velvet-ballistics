//! BDD parser tests.

#[allow(unused_imports)]
use crate::ExprError;
use crate::lexer::{BinaryOp, UnaryOp, lex_expr};
#[allow(unused_imports)]
use crate::parser::{ExprAst, ExprHelper, ExprLiteral, parse_expr};

mod adversarial;

#[allow(dead_code)]
fn parse(source: &str) -> crate::ExprResult<ExprAst> {
    let tokens = lex_expr(source)?;
    parse_expr(&tokens)
}

#[test]
fn parses_addition_with_multiplication_precedence() -> crate::ExprResult<()> {
    let expr = parse("1 + 2 * 3")?;
    let (op, _, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Add);
    let (right_op, _, _) = as_binary(right)?;
    assert_eq!(right_op, BinaryOp::Mul);
    Ok(())
}

#[test]
fn parses_left_associative_subtraction() -> crate::ExprResult<()> {
    let expr = parse("1 - 2 - 3")?;
    let (op, left, _) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Sub);
    let (left_op, _, _) = as_binary(left)?;
    assert_eq!(left_op, BinaryOp::Sub);
    Ok(())
}

#[test]
fn parses_not_and_or_precedence() -> crate::ExprResult<()> {
    let expr = parse("not $a and $b or $c")?;
    let (op, left, _) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Or);
    let (left_op, not_expr, _) = as_binary(left)?;
    assert_eq!(left_op, BinaryOp::And);
    let (not_op, _) = as_unary(not_expr)?;
    assert_eq!(not_op, UnaryOp::Not);
    Ok(())
}

#[test]
fn parses_helper_call() -> crate::ExprResult<()> {
    let expr = parse("contains($tags, \"urgent\")")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Contains);
    assert_eq!(args.len(), 2);
    Ok(())
}

#[test]
fn rejects_unknown_helper() {
    let result = parse("unknown_func(1)");
    assert!(matches!(result, Err(ExprError::UnknownHelper { .. })));
}

#[test]
fn rejects_wrong_arity() {
    let result = parse("contains(1)");
    assert!(matches!(result, Err(ExprError::HelperArityMismatch { .. })));
}

#[test]
fn rejects_parse_depth() {
    let open = "(".repeat(usize::from(crate::parser::MAX_DEPTH).saturating_add(2));
    let close = ")".repeat(usize::from(crate::parser::MAX_DEPTH).saturating_add(2));
    let source = format!("{open}true{close}");
    let result = parse(&source);
    assert!(matches!(result, Err(ExprError::ParseDepthExceeded { .. })));
}

// --- BDD parser tests ---

#[test]
fn parse_expr_parses_simple_addition() -> crate::ExprResult<()> {
    let expr = parse("5 + 3")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Add);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(5)));
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(3)));
    Ok(())
}

#[test]
fn parse_expr_parses_operator_precedence_correctly() -> crate::ExprResult<()> {
    let expr = parse("1 + 2 * 3")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Add);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(1)));
    let (inner_op, _, _) = as_binary(right)?;
    assert_eq!(inner_op, BinaryOp::Mul);
    Ok(())
}

#[test]
fn parse_expr_parses_parenthesized_grouping() -> crate::ExprResult<()> {
    let expr = parse("(1 + 2) * 3")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Mul);
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(3)));
    let (inner_op, _, _) = as_binary(left)?;
    assert_eq!(inner_op, BinaryOp::Add);
    Ok(())
}

#[test]
fn parse_expr_parses_unary_negation() -> crate::ExprResult<()> {
    let expr = parse("-5")?;
    let (op, inner) = as_unary(&expr)?;
    assert_eq!(op, UnaryOp::Neg);
    assert_eq!(*inner, ExprAst::Literal(ExprLiteral::I64(5)));
    Ok(())
}

#[test]
fn parse_expr_parses_boolean_not() -> crate::ExprResult<()> {
    let expr = parse("not true")?;
    let (op, inner) = as_unary(&expr)?;
    assert_eq!(op, UnaryOp::Not);
    assert_eq!(*inner, ExprAst::Literal(ExprLiteral::Bool(true)));
    Ok(())
}

#[test]
fn parse_expr_parses_comparison_operators() -> crate::ExprResult<()> {
    let expr = parse("5 == 5")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Eq);
    assert_eq!(*left, ExprAst::Literal(ExprLiteral::I64(5)));
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::I64(5)));

    let expr_ne = parse("5 != 3")?;
    let (op_ne, _, _) = as_binary(&expr_ne)?;
    assert_eq!(op_ne, BinaryOp::NotEq);

    let expr_lt = parse("1 < 2")?;
    let (op_lt, _, _) = as_binary(&expr_lt)?;
    assert_eq!(op_lt, BinaryOp::Lt);

    let expr_gt = parse("2 > 1")?;
    let (op_gt, _, _) = as_binary(&expr_gt)?;
    assert_eq!(op_gt, BinaryOp::Gt);

    let expr_lte = parse("1 <= 2")?;
    let (op_lte, _, _) = as_binary(&expr_lte)?;
    assert_eq!(op_lte, BinaryOp::Lte);

    let expr_gte = parse("2 >= 1")?;
    let (op_gte, _, _) = as_binary(&expr_gte)?;
    assert_eq!(op_gte, BinaryOp::Gte);
    Ok(())
}

#[test]
fn parse_expr_parses_logical_and_or() -> crate::ExprResult<()> {
    let expr = parse("true and false or true")?;
    let (op, left, right) = as_binary(&expr)?;
    assert_eq!(op, BinaryOp::Or);
    assert_eq!(*right, ExprAst::Literal(ExprLiteral::Bool(true)));
    let (left_op, _, _) = as_binary(left)?;
    assert_eq!(left_op, BinaryOp::And);
    Ok(())
}

#[test]
fn parse_expr_parses_helper_call_with_arguments() -> crate::ExprResult<()> {
    let expr = parse("contains($x, $y)")?;
    let (name, args) = as_helper(&expr)?;
    assert_eq!(name, ExprHelper::Contains);
    assert_eq!(args.len(), 2);
    assert_eq!(args.first(), Some(&ExprAst::Reference(Box::from("$x"))));
    assert_eq!(args.get(1), Some(&ExprAst::Reference(Box::from("$y"))));
    Ok(())
}

#[test]
fn parse_expr_parses_variable_reference() -> crate::ExprResult<()> {
    let expr = parse("$data.field")?;
    assert_eq!(expr, ExprAst::Reference(Box::from("$data.field")));
    Ok(())
}

#[test]
fn parse_expr_returns_error_for_empty_input() -> crate::ExprResult<()> {
    let tokens = lex_expr("")?;
    let result = parse_expr(&tokens);
    let Err(ExprError::UnexpectedToken { token }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected UnexpectedToken".into(),
        });
    };
    assert!(
        token.contains("End"),
        "token should contain 'End', got: {token}"
    );
    Ok(())
}

#[test]
fn parse_expr_returns_unknown_helper_for_bad_helper() -> crate::ExprResult<()> {
    let result = parse("bogus_func(1)");
    let Err(ExprError::UnknownHelper { helper }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected UnknownHelper".into(),
        });
    };
    assert_eq!(helper, "bogus_func");
    Ok(())
}

#[test]
fn parse_expr_returns_wrong_arity_error_for_contains_with_one_arg() -> crate::ExprResult<()> {
    let result = parse("contains(1)");
    let Err(ExprError::HelperArityMismatch {
        helper,
        expected,
        actual,
    }) = result
    else {
        return Err(ExprError::UnexpectedToken {
            token: "expected HelperArityMismatch".into(),
        });
    };
    assert_eq!(helper, "contains");
    assert_eq!(expected, 2);
    assert_eq!(actual, 1);
    Ok(())
}

#[test]
fn parse_expr_returns_error_for_missing_right_paren() -> crate::ExprResult<()> {
    let result = parse("(1 + 2");
    let Err(ExprError::UnexpectedToken { token }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected UnexpectedToken".into(),
        });
    };
    assert!(
        token.contains("right parenthesis"),
        "token should mention right parenthesis, got: {token}"
    );
    Ok(())
}

#[allow(dead_code)]
fn as_binary(expr: &ExprAst) -> crate::ExprResult<(BinaryOp, &ExprAst, &ExprAst)> {
    match expr {
        ExprAst::Binary { op, left, right } => Ok((*op, left, right)),
        other => Err(ExprError::UnexpectedToken {
            token: format!("expected binary, got {other:?}"),
        }),
    }
}

#[allow(dead_code)]
fn as_unary(expr: &ExprAst) -> crate::ExprResult<(UnaryOp, &ExprAst)> {
    match expr {
        ExprAst::Unary { op, expr } => Ok((*op, expr)),
        other => Err(ExprError::UnexpectedToken {
            token: format!("expected unary, got {other:?}"),
        }),
    }
}

#[allow(dead_code)]
fn as_helper(expr: &ExprAst) -> crate::ExprResult<(ExprHelper, &[ExprAst])> {
    match expr {
        ExprAst::Helper { name, args } => Ok((*name, args)),
        other => Err(ExprError::UnexpectedToken {
            token: format!("expected helper, got {other:?}"),
        }),
    }
}
