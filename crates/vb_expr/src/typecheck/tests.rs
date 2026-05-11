#![forbid(unsafe_code)]
//! BDD typecheck tests.

mod adversarial;

#[allow(unused_imports)]
use crate::ExprError;
use crate::lexer::lex_expr;
use crate::parser::parse_expr;
use crate::typecheck::{ExprType, TypeContext, typecheck_expr};

#[allow(dead_code)]
fn check(source: &str) -> crate::ExprResult<ExprType> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    typecheck_expr(&ast, &TypeContext::new())
}

#[test]
fn infers_literal_types() -> crate::ExprResult<()> {
    assert_eq!(check("42")?, ExprType::I64);
    assert_eq!(check("true")?, ExprType::Bool);
    assert_eq!(check("null")?, ExprType::Null);
    assert_eq!(check("\"hello\"")?, ExprType::Text);
    assert_eq!(check("3.14")?, ExprType::F64);
    Ok(())
}

#[test]
fn infers_arithmetic_result() -> crate::ExprResult<()> {
    assert_eq!(check("1 + 2")?, ExprType::I64);
    Ok(())
}

#[test]
fn infers_comparison_result() -> crate::ExprResult<()> {
    assert_eq!(check("1 < 2")?, ExprType::Bool);
    assert_eq!(check("1 == 2")?, ExprType::Bool);
    Ok(())
}

#[test]
fn infers_logical_result() -> crate::ExprResult<()> {
    assert_eq!(check("true and false")?, ExprType::Bool);
    assert_eq!(check("true or false")?, ExprType::Bool);
    Ok(())
}

#[test]
fn infers_helper_result() -> crate::ExprResult<()> {
    assert_eq!(check("length($x)")?, ExprType::I64);
    assert_eq!(check("empty($x)")?, ExprType::Bool);
    assert_eq!(check("contains($x, $y)")?, ExprType::Bool);
    Ok(())
}

#[test]
fn infers_unary_not() -> crate::ExprResult<()> {
    assert_eq!(check("not true")?, ExprType::Bool);
    Ok(())
}

#[test]
fn infers_negation_preserves_type() -> crate::ExprResult<()> {
    assert_eq!(check("-42")?, ExprType::I64);
    Ok(())
}

#[test]
fn unknown_type_for_unresolved_reference() -> crate::ExprResult<()> {
    assert_eq!(check("$unknown")?, ExprType::Unknown);
    Ok(())
}

#[test]
fn context_resolves_known_variables() -> crate::ExprResult<()> {
    let mut ctx = TypeContext::new();
    ctx.add_variable(Box::from("$x"), ExprType::I64);
    let tokens = lex_expr("$x + 1")?;
    let ast = parse_expr(&tokens)?;
    let ty = typecheck_expr(&ast, &ctx)?;
    assert_eq!(ty, ExprType::I64);
    Ok(())
}

// --- BDD typecheck tests ---

#[test]
fn typecheck_expr_validates_numeric_operands() -> crate::ExprResult<()> {
    let tokens = lex_expr("1 + 2")?;
    let ast = parse_expr(&tokens)?;
    let ty = typecheck_expr(&ast, &TypeContext::new())?;
    assert_eq!(ty, ExprType::I64);
    Ok(())
}

#[test]
fn typecheck_expr_rejects_string_in_arithmetic() -> crate::ExprResult<()> {
    let tokens = lex_expr("\"hello\" + 1")?;
    let ast = parse_expr(&tokens)?;
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "text");
    Ok(())
}

#[test]
fn typecheck_expr_validates_boolean_operands_for_logical_ops() -> crate::ExprResult<()> {
    let tokens = lex_expr("true and false")?;
    let ast = parse_expr(&tokens)?;
    let ty = typecheck_expr(&ast, &TypeContext::new())?;
    assert_eq!(ty, ExprType::Bool);
    Ok(())
}

#[test]
fn typecheck_expr_rejects_number_in_logical_op() -> crate::ExprResult<()> {
    let tokens = lex_expr("1 and 2")?;
    let ast = parse_expr(&tokens)?;
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "i64");
    Ok(())
}

#[test]
fn infix_binding_power_returns_correct_precedence_for_operators() {
    let (or_bp, _) = crate::lexer::infix_binding_power(crate::lexer::BinaryOp::Or);
    let (and_bp, _) = crate::lexer::infix_binding_power(crate::lexer::BinaryOp::And);
    let (add_bp, _) = crate::lexer::infix_binding_power(crate::lexer::BinaryOp::Add);
    let (mul_bp, _) = crate::lexer::infix_binding_power(crate::lexer::BinaryOp::Mul);
    assert!(
        or_bp < and_bp,
        "or bp ({or_bp}) should be less than and bp ({and_bp})"
    );
    assert!(
        and_bp < add_bp,
        "and bp ({and_bp}) should be less than add bp ({add_bp})"
    );
    assert!(
        add_bp < mul_bp,
        "add bp ({add_bp}) should be less than mul bp ({mul_bp})"
    );
}

#[test]
fn typecheck_expr_validates_negation_on_number() -> crate::ExprResult<()> {
    let tokens = lex_expr("-42")?;
    let ast = parse_expr(&tokens)?;
    let ty = typecheck_expr(&ast, &TypeContext::new())?;
    assert_eq!(ty, ExprType::I64);
    Ok(())
}

#[test]
fn typecheck_expr_rejects_negation_on_boolean() -> crate::ExprResult<()> {
    let tokens = lex_expr("-true")?;
    let ast = parse_expr(&tokens)?;
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "boolean");
    Ok(())
}

#[test]
fn typecheck_expr_infers_helper_return_types() -> crate::ExprResult<()> {
    let ty_len = check("length($x)")?;
    assert_eq!(ty_len, ExprType::I64);

    let ty_empty = check("empty($x)")?;
    assert_eq!(ty_empty, ExprType::Bool);

    let ty_contains = check("contains($x, $y)")?;
    assert_eq!(ty_contains, ExprType::Bool);

    let ty_sum = check("sum($x)")?;
    assert_eq!(ty_sum, ExprType::I64);

    let ty_unique = check("unique($x)")?;
    assert_eq!(ty_unique, ExprType::List);
    Ok(())
}

#[test]
fn typecheck_expr_allows_unknown_in_arithmetic_left() -> crate::ExprResult<()> {
    let ty = check("$x + 1")?;
    assert_eq!(ty, ExprType::I64);
    Ok(())
}
