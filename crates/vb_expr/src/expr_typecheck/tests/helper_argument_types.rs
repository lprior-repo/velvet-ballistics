#![forbid(unsafe_code)]
//! Original helper-contract cases, sharing assertions rather than dropping cases.

use crate::parser::{ExprAst, ExprHelper as H, ExprLiteral};
use crate::typecheck::ExprType::{Bool, I64, List, Object};
use crate::typecheck::{ExprType, TypeContext, typecheck_expr};
use crate::{ExprError, ExprResult};

fn check(source: &str) -> ExprResult<ExprType> {
    let tokens = crate::lexer::lex_expr(source)?;
    let ast = crate::parser::parse_expr(&tokens)?;
    typecheck_expr(&ast, &TypeContext::new())
}

fn known(source: &str, name: &str, ty: ExprType) -> ExprResult<ExprType> {
    let mut ctx = TypeContext::new();
    ctx.add_variable(Box::from(name), ty);
    let tokens = crate::lexer::lex_expr(source)?;
    let ast = crate::parser::parse_expr(&tokens)?;
    typecheck_expr(&ast, &ctx)
}

fn returns(result: ExprResult<ExprType>, expected: ExprType) -> ExprResult<()> {
    assert_eq!(result?, expected);
    Ok(())
}

fn mismatch(result: ExprResult<ExprType>, want: (&str, &str)) -> ExprResult<()> {
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch".into(),
        });
    };
    assert_eq!(expected, want.0);
    assert_eq!(found, want.1);
    Ok(())
}

fn arity(helper: H, args: Vec<ExprAst>, want: (&str, usize, usize)) -> ExprResult<()> {
    let ast = ExprAst::Helper {
        name: helper,
        args: args.into_boxed_slice(),
    };
    let result = typecheck_expr(&ast, &TypeContext::new());
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
    assert_eq!(helper, want.0);
    assert_eq!(expected, want.1);
    assert_eq!(actual, want.2);
    Ok(())
}

fn integer(value: i64) -> ExprAst {
    ExprAst::Literal(ExprLiteral::I64(value))
}

fn text(value: &str) -> ExprAst {
    ExprAst::Literal(ExprLiteral::Text(Box::from(value)))
}

#[test]
fn sum_list_returns_i64() -> ExprResult<()> {
    returns(known("sum($items)", "$items", List), I64)
}

#[test]
fn exists_any_returns_bool() -> ExprResult<()> {
    returns(check("exists(42)"), Bool)?;
    returns(check("exists(true)"), Bool)?;
    returns(check("exists(null)"), Bool)?;
    returns(check("exists(\"text\")"), Bool)?;
    returns(known("exists($x)", "$x", List), Bool)?;
    returns(known("exists($x)", "$x", Object), Bool)
}

#[test]
fn starts_with_text_text_returns_bool() -> ExprResult<()> {
    returns(check("starts_with(\"abc\", \"a\")"), Bool)
}

#[test]
fn ends_with_text_text_returns_bool() -> ExprResult<()> {
    returns(check("ends_with(\"abc\", \"c\")"), Bool)
}

#[test]
fn has_object_text_returns_bool() -> ExprResult<()> {
    returns(known("has($obj, \"key\")", "$obj", Object), Bool)
}

#[test]
fn append_list_any_returns_list() -> ExprResult<()> {
    returns(known("append($items, 42)", "$items", List), List)?;
    returns(known("append($items, \"text\")", "$items", List), List)?;
    returns(known("append($items, null)", "$items", List), List)
}

#[test]
fn append_if_list_any_bool_returns_list() -> ExprResult<()> {
    returns(known("append_if($items, 42, true)", "$items", List), List)
}

#[test]
fn merge_object_object_returns_object() -> ExprResult<()> {
    returns(known("merge($a, $b)", "$a", Object), Object)
}

#[test]
fn unique_list_returns_list() -> ExprResult<()> {
    returns(known("unique($items)", "$items", List), List)
}

#[test]
fn sum_non_list_rejected() -> ExprResult<()> {
    mismatch(check("sum(42)"), ("list", "i64"))?;
    mismatch(check("sum(\"text\")"), ("list", "text"))?;
    mismatch(check("sum(true)"), ("list", "boolean"))?;
    mismatch(check("sum(null)"), ("list", "null"))?;
    mismatch(known("sum($x)", "$x", Object), ("list", "object"))
}

#[test]
fn starts_with_wrong_types_rejected() -> ExprResult<()> {
    mismatch(check("starts_with(42, \"a\")"), ("text", "i64"))?;
    mismatch(check("starts_with(\"hello\", 42)"), ("text", "i64"))?;
    mismatch(check("starts_with(\"hello\", true)"), ("text", "boolean"))
}

#[test]
fn ends_with_wrong_types_rejected() -> ExprResult<()> {
    mismatch(check("ends_with(42, \"a\")"), ("text", "i64"))?;
    mismatch(check("ends_with(\"hello\", 42)"), ("text", "i64"))
}

#[test]
fn has_wrong_types_rejected() -> ExprResult<()> {
    mismatch(check("has(42, \"key\")"), ("object", "i64"))?;
    mismatch(check("has(\"not-obj\", \"key\")"), ("object", "text"))?;
    mismatch(check("has($obj, 42)"), ("text", "i64"))?;
    mismatch(check("has($obj, true)"), ("text", "boolean"))
}

#[test]
fn append_first_non_list_rejected() -> ExprResult<()> {
    mismatch(check("append(42, 1)"), ("list", "i64"))?;
    mismatch(check("append(\"not-list\", 1)"), ("list", "text"))?;
    returns(known("append($items, null)", "$items", List), List)
}

#[test]
fn append_if_wrong_types_rejected() -> ExprResult<()> {
    mismatch(check("append_if(42, 1, true)"), ("list", "i64"))?;
    mismatch(
        check("append_if($items, 1, \"not-bool\")"),
        ("boolean", "text"),
    )?;
    mismatch(check("append_if($items, 1, 42)"), ("boolean", "i64"))
}

#[test]
fn merge_wrong_types_rejected() -> ExprResult<()> {
    mismatch(check("merge(42, $b)"), ("object", "i64"))?;
    mismatch(check("merge(\"not-obj\", $b)"), ("object", "text"))?;
    mismatch(check("merge($a, 42)"), ("object", "i64"))?;
    mismatch(check("merge($a, \"not-obj\")"), ("object", "text"))
}

#[test]
fn unique_non_list_rejected() -> ExprResult<()> {
    mismatch(check("unique(42)"), ("list", "i64"))?;
    mismatch(check("unique(\"not-list\")"), ("list", "text"))
}

#[test]
fn unknown_first_does_not_mask_later_type_error() -> ExprResult<()> {
    mismatch(check("starts_with($x, 42)"), ("text", "i64"))?;
    mismatch(check("merge($x, \"not-obj\")"), ("object", "text"))?;
    mismatch(check("append_if($x, 1, \"not-bool\")"), ("boolean", "text"))
}

#[test]
fn unknown_later_does_not_mask_earlier_type_error() -> ExprResult<()> {
    mismatch(check("starts_with(42, $x)"), ("text", "i64"))?;
    mismatch(check("has(\"not-obj\", $x)"), ("object", "text"))
}

#[test]
fn arity_error_starts_with_missing_arg() -> ExprResult<()> {
    arity(H::StartsWith, vec![text("a")], ("starts_with", 2, 1))
}

#[test]
fn arity_error_starts_with_extra_arg() -> ExprResult<()> {
    arity(
        H::StartsWith,
        vec![text("a"), text("b"), text("c")],
        ("starts_with", 2, 3),
    )
}

#[test]
fn arity_error_ends_with_missing_arg() -> ExprResult<()> {
    arity(H::EndsWith, vec![text("a")], ("ends_with", 2, 1))
}

#[test]
fn arity_error_has_missing_arg() -> ExprResult<()> {
    arity(H::Has, vec![text("a")], ("has", 2, 1))
}

#[test]
fn arity_error_append_missing_arg() -> ExprResult<()> {
    arity(H::Append, vec![integer(1)], ("append", 2, 1))
}

#[test]
fn arity_error_append_if_missing_args() -> ExprResult<()> {
    arity(H::AppendIf, vec![integer(1)], ("append_if", 3, 1))
}

#[test]
fn arity_error_merge_missing_arg() -> ExprResult<()> {
    arity(H::Merge, vec![integer(1)], ("merge", 2, 1))
}

#[test]
fn arity_error_sum_missing_args() -> ExprResult<()> {
    arity(H::Sum, vec![], ("sum", 1, 0))
}

#[test]
fn arity_error_unique_missing_args() -> ExprResult<()> {
    arity(H::Unique, vec![], ("unique", 1, 0))
}

#[test]
fn arity_error_exists_extra_args() -> ExprResult<()> {
    arity(H::Exists, vec![integer(1), integer(2)], ("exists", 1, 2))
}

#[test]
fn arity_error_length_missing_args() -> ExprResult<()> {
    arity(H::Length, vec![], ("length", 1, 0))
}

#[test]
fn arity_error_count_missing_args() -> ExprResult<()> {
    arity(H::Count, vec![], ("count", 1, 0))
}

#[test]
fn arity_error_empty_missing_args() -> ExprResult<()> {
    arity(H::Empty, vec![], ("empty", 1, 0))
}

#[test]
fn arity_error_contains_missing_args() -> ExprResult<()> {
    arity(H::Contains, vec![], ("contains", 2, 0))
}

#[test]
fn nested_arithmetic_error_precedes_helper_type_error() -> ExprResult<()> {
    mismatch(check("starts_with(1, true + 1)"), ("number", "boolean"))
}

#[test]
fn no_nested_error_returns_helper_type_error() -> ExprResult<()> {
    mismatch(check("starts_with(1 + 2, \"a\")"), ("text", "i64"))
}

#[test]
fn deferred_length_passes_unknown() -> ExprResult<()> {
    returns(check("length($x)"), I64)
}

#[test]
fn deferred_count_passes_unknown() -> ExprResult<()> {
    returns(check("count($x)"), I64)
}

#[test]
fn deferred_empty_passes_unknown() -> ExprResult<()> {
    returns(check("empty($x)"), Bool)
}

#[test]
fn deferred_contains_passes_unknown() -> ExprResult<()> {
    returns(check("contains($x, $y)"), Bool)
}
