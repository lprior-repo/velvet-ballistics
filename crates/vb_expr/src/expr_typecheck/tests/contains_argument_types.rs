use crate::parser::{ExprAst, ExprHelper as H};
use crate::typecheck::{ExprType, TypeContext, typecheck_expr};
use crate::{ExprError, ExprResult};

fn check(source: &str) -> ExprResult<ExprType> {
    let tokens = crate::lexer::lex_expr(source)?;
    let ast = crate::parser::parse_expr(&tokens)?;
    typecheck_expr(&ast, &TypeContext::new())
}

fn check_with(source: &str, context: &TypeContext) -> ExprResult<ExprType> {
    let tokens = crate::lexer::lex_expr(source)?;
    let ast = crate::parser::parse_expr(&tokens)?;
    typecheck_expr(&ast, context)
}

fn contains_ast(args: Vec<ExprAst>) -> ExprAst {
    ExprAst::Helper {
        name: H::Contains,
        args: args.into_boxed_slice(),
    }
}

#[test]
fn contains_list_receiver_accepts_bool_i64_f64_and_null_needles() -> ExprResult<()> {
    let mut context = TypeContext::new();
    context.add_variable(Box::from("$items"), ExprType::List);
    for source in [
        "contains($items, true)",
        "contains($items, 7)",
        "contains($items, 3.5)",
        "contains($items, null)",
    ] {
        assert_eq!(check_with(source, &context)?, ExprType::Bool);
    }
    Ok(())
}

#[test]
fn contains_list_receiver_accepts_text_list_object_needles() -> ExprResult<()> {
    for needle_type in [ExprType::Text, ExprType::List, ExprType::Object] {
        let mut ctx = TypeContext::new();
        ctx.add_variable(Box::from("$items"), ExprType::List);
        ctx.add_variable(Box::from("$needle"), needle_type);
        assert_eq!(
            check_with("contains($items, $needle)", &ctx)?,
            ExprType::Bool
        );
    }
    Ok(())
}

#[test]
fn contains_unknown_receiver_accepts_bool_i64_f64_and_null_needles() -> ExprResult<()> {
    for source in [
        "contains($items, true)",
        "contains($items, 7)",
        "contains($items, 3.5)",
        "contains($items, null)",
    ] {
        assert_eq!(check(source)?, ExprType::Bool);
    }
    Ok(())
}

#[test]
fn contains_rejects_each_known_non_list_receiver_with_exact_mismatch() -> ExprResult<()> {
    let mut context = TypeContext::new();
    context.add_variable(Box::from("$object"), ExprType::Object);
    for (receiver, found) in [
        ("true", "boolean"),
        ("7", "i64"),
        ("3.5", "f64"),
        ("null", "null"),
        ("\"text\"", "text"),
        ("$object", "object"),
    ] {
        assert_eq!(
            check_with(&format!("contains({receiver}, 1)"), &context),
            Err(ExprError::TypeMismatch {
                expected: "list".into(),
                found: found.into(),
            })
        );
    }
    Ok(())
}

#[test]
fn contains_typecheck_reports_exact_zero_one_and_three_argument_errors() -> ExprResult<()> {
    let reference = ExprAst::Reference(Box::from("$items"));
    let cases = [
        (Vec::new(), 0),
        (vec![reference.clone()], 1),
        (vec![reference.clone(), reference.clone(), reference], 3),
    ];
    for (args, actual) in cases {
        assert_eq!(
            typecheck_expr(&contains_ast(args), &TypeContext::new()),
            Err(ExprError::HelperArityMismatch {
                helper: "contains".into(),
                expected: 2,
                actual,
            })
        );
    }
    Ok(())
}
