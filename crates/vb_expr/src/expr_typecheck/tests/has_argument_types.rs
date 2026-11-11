#![forbid(unsafe_code)]
//! has() helper type inference and arity error tests.

use crate::parser::{ExprAst, ExprHelper as H};
use crate::typecheck::{ExprType, TypeContext, typecheck_expr};
use crate::{ExprError, ExprResult};

fn check(source: &str) -> ExprResult<ExprType> {
    let tokens = crate::lexer::lex_expr(source)?;
    let ast = crate::parser::parse_expr(&tokens)?;
    typecheck_expr(&ast, &TypeContext::new())
}

fn check_with(source: &str, ctx: &TypeContext) -> ExprResult<ExprType> {
    let tokens = crate::lexer::lex_expr(source)?;
    let ast = crate::parser::parse_expr(&tokens)?;
    typecheck_expr(&ast, ctx)
}

fn text(value: &str) -> ExprAst {
    ExprAst::Literal(crate::parser::ExprLiteral::Text(Box::from(value)))
}

#[test]
fn has_unknown_receiver_text_key_returns_bool() -> ExprResult<()> {
    let result = check("has($obj, \"key\")")?;
    assert_eq!(result, ExprType::Bool);
    Ok(())
}

#[test]
fn has_object_receiver_unknown_key_returns_bool() -> ExprResult<()> {
    let mut ctx = TypeContext::new();
    ctx.add_variable(Box::from("$obj"), ExprType::Object);
    let result = check_with("has($obj, $key)", &ctx)?;
    assert_eq!(result, ExprType::Bool);
    Ok(())
}

#[test]
fn has_unknown_both_returns_bool() -> ExprResult<()> {
    let result = check("has($x, $y)")?;
    assert_eq!(result, ExprType::Bool);
    Ok(())
}

#[test]
fn has_zero_args_rejected() -> ExprResult<()> {
    let ast = ExprAst::Helper {
        name: H::Has,
        args: vec![].into_boxed_slice(),
    };
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::HelperArityMismatch {
        helper,
        expected,
        actual,
    }) = result
    else {
        return Err(ExprError::UnexpectedToken {
            token: "expected HelperArityMismatch for has()".into(),
        });
    };
    assert_eq!(helper, "has");
    assert_eq!(expected, 2);
    assert_eq!(actual, 0);
    Ok(())
}

#[test]
fn has_three_args_rejected() -> ExprResult<()> {
    let ast = ExprAst::Helper {
        name: H::Has,
        args: vec![text("a"), text("b"), text("c")].into_boxed_slice(),
    };
    let result = typecheck_expr(&ast, &TypeContext::new());
    let Err(ExprError::HelperArityMismatch {
        helper,
        expected,
        actual,
    }) = result
    else {
        return Err(ExprError::UnexpectedToken {
            token: "expected HelperArityMismatch for has(a,b,c)".into(),
        });
    };
    assert_eq!(helper, "has");
    assert_eq!(expected, 2);
    assert_eq!(actual, 3);
    Ok(())
}

#[test]
fn has_both_wrong_types_report_receiver_first() {
    assert_eq!(
        check("has(true,42)"),
        Err(ExprError::TypeMismatch {
            expected: "object".into(),
            found: "boolean".into()
        })
    );
}
