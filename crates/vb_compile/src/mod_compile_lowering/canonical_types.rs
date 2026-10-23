#![forbid(unsafe_code)]
//! Canonical expression type inference using shared vb_expr type checker.
//!
//! Converts canonical ParsedExpression to vb_expr ExprAst for shared type
//! checking. Maintains per-Set step type context for reference resolution.

use crate::CompileError;
use crate::expression::{
    BinaryOp, ExpressionHelper, ExpressionLiteral, ParsedExpression, UnaryOp,
};
use std::collections::HashMap;
use vb_expr::expr_parser::types::{ExprAst, ExprLiteral, ExprHelper};
use vb_expr::expr_typecheck::{TypeContext, ExprType, typecheck_expr};

/// Type-checks a canonical expression and returns its inferred type.
/// Unknown future references type as Unknown (dynamic), but known static type
/// errors (e.g., bool + number) are rejected.
///
/// Prior types are mapped using canonical reference names ($prefix) to match
/// how ParsedExpression::Reference stores names, ensuring proper TypeContext
/// lookup for known types (bool, i64, f64, etc.).
pub(super) fn infer_canonical_expr_type(
    expr: &ParsedExpression,
    prior_types: &HashMap<String, ExprType>,
) -> Result<ExprType, CompileError> {
    let ast = to_vb_expr_ast(expr)?;
    let mut context = TypeContext::new();
    // Prior types stored as bare names (e.g., "valid"); TypeContext lookup
    // requires canonical reference names with $ prefix to match Reference.
    for (name, ty) in prior_types {
        let canonical = format!("${name}");
        context.add_variable(canonical.into(), *ty);
    }
    typecheck_expr(&ast, &context).map_err(vb_expr_error_to_compile)
}

/// Converts a vb_expr type-checking error to a CompileError variant.
fn vb_expr_error_to_compile(e: vb_expr::ExprError) -> CompileError {
    match e {
        vb_expr::ExprError::TypeMismatch { expected, found } => {
            CompileError::TypeMismatch {
                field: "set.value",
                expected: type_name_static(&expected),
                found: type_name_static(&found),
            }
        }
        vb_expr::ExprError::StackUnderflow
        | vb_expr::ExprError::DivisionByZero
        | vb_expr::ExprError::IntegerOverflow => {
            CompileError::TypeMismatch {
                field: "set.value",
                expected: "valid expression",
                found: "invalid runtime value",
            }
        }
        vb_expr::ExprError::ExpressionTooLong { max, .. } => {
            CompileError::ExpressionLimitExceeded {
                expression: Box::from("expression"),
                limit: "token count",
                max,
            }
        }
        vb_expr::ExprError::HelperArityMismatch {
            helper, expected, actual,
        } => CompileError::ExpressionHelperArity {
            helper: helper_name_static(&helper),
            expected,
            actual,
        },
        vb_expr::ExprError::UnknownHelper { helper } => {
            CompileError::ExpressionUnexpectedToken {
                expression: Box::from(format!("unknown helper: {helper}")),
                index: 0,
                expected: "valid expression",
            }
        }
        _ => CompileError::ExpressionUnexpectedToken {
            expression: Box::from(format!("{e}")),
            index: 0,
            expected: "valid expression",
        },
    }
}

/// Maps a dynamic vb_expr type name string to a static &'static str.
fn type_name_static(name: &str) -> &'static str {
    match name {
        "null" => "null",
        "boolean" => "boolean",
        "i64" => "i64",
        "f64" => "f64",
        "text" => "text",
        "list" => "list",
        "object" => "object",
        "number" => "number",
        "unknown" => "unknown",
        _ => "unknown",
    }
}

/// Maps a dynamic helper name to a static &'static str.
fn helper_name_static(name: &str) -> &'static str {
    match name {
        "contains" => "contains",
        "starts_with" => "starts_with",
        "ends_with" => "ends_with",
        "has" => "has",
        "exists" => "exists",
        "length" => "length",
        "empty" => "empty",
        "append" => "append",
        "append_if" => "append_if",
        "merge" => "merge",
        "sum" => "sum",
        "count" => "count",
        "unique" => "unique",
        _ => "unknown helper",
    }
}

/// Converts canonical ParsedExpression to vb_expr ExprAst for shared type
/// checking. Reference names are passed through with $ prefix intact.
fn to_vb_expr_ast(expr: &ParsedExpression) -> Result<ExprAst, CompileError> {
    match expr {
        ParsedExpression::Literal(lit) => {
            Ok(ExprAst::Literal(to_vb_expr_literal(lit)?))
        }
        ParsedExpression::Reference(name) => {
            Ok(ExprAst::Reference(name.clone()))
        }
        ParsedExpression::Unary { op, expr: inner } => {
            let vb_op = match op {
                UnaryOp::Not => vb_expr::lexer::UnaryOp::Not,
                UnaryOp::Neg => vb_expr::lexer::UnaryOp::Neg,
            };
            let vb_inner = to_vb_expr_ast(inner)?;
            Ok(ExprAst::Unary {
                op: vb_op,
                expr: Box::new(vb_inner),
            })
        }
        ParsedExpression::Binary { op, left, right } => {
            let vb_op = match op {
                BinaryOp::Or => vb_expr::lexer::BinaryOp::Or,
                BinaryOp::And => vb_expr::lexer::BinaryOp::And,
                BinaryOp::Eq => vb_expr::lexer::BinaryOp::Eq,
                BinaryOp::NotEq => vb_expr::lexer::BinaryOp::NotEq,
                BinaryOp::Lt => vb_expr::lexer::BinaryOp::Lt,
                BinaryOp::Lte => vb_expr::lexer::BinaryOp::Lte,
                BinaryOp::Gt => vb_expr::lexer::BinaryOp::Gt,
                BinaryOp::Gte => vb_expr::lexer::BinaryOp::Gte,
                BinaryOp::Add => vb_expr::lexer::BinaryOp::Add,
                BinaryOp::Sub => vb_expr::lexer::BinaryOp::Sub,
                BinaryOp::Mul => vb_expr::lexer::BinaryOp::Mul,
                BinaryOp::Div => vb_expr::lexer::BinaryOp::Div,
            };
            let vb_left = to_vb_expr_ast(left)?;
            let vb_right = to_vb_expr_ast(right)?;
            Ok(ExprAst::Binary {
                op: vb_op,
                left: Box::new(vb_left),
                right: Box::new(vb_right),
            })
        }
        ParsedExpression::HelperCall { name, args } => {
            let vb_helper = match name {
                ExpressionHelper::Contains => ExprHelper::Contains,
                ExpressionHelper::StartsWith => ExprHelper::StartsWith,
                ExpressionHelper::EndsWith => ExprHelper::EndsWith,
                ExpressionHelper::Has => ExprHelper::Has,
                ExpressionHelper::Exists => ExprHelper::Exists,
                ExpressionHelper::Length => ExprHelper::Length,
                ExpressionHelper::Empty => ExprHelper::Empty,
                ExpressionHelper::Append => ExprHelper::Append,
                ExpressionHelper::AppendIf => ExprHelper::AppendIf,
                ExpressionHelper::Merge => ExprHelper::Merge,
                ExpressionHelper::Sum => ExprHelper::Sum,
                ExpressionHelper::Count => ExprHelper::Count,
                ExpressionHelper::Unique => ExprHelper::Unique,
            };
            let vb_args = args
                .iter()
                .map(to_vb_expr_ast)
                .collect::<Result<Vec<ExprAst>, CompileError>>()?;
            Ok(ExprAst::Helper {
                name: vb_helper,
                args: vb_args.into_boxed_slice(),
            })
        }
    }
}

fn to_vb_expr_literal(lit: &ExpressionLiteral) -> Result<ExprLiteral, CompileError> {
    match lit {
        ExpressionLiteral::Null => Ok(ExprLiteral::Null),
        ExpressionLiteral::Bool(v) => Ok(ExprLiteral::Bool(*v)),
        ExpressionLiteral::I64(v) => Ok(ExprLiteral::I64(*v)),
        ExpressionLiteral::F64(v) => Ok(ExprLiteral::F64(*v)),
        ExpressionLiteral::Text(v) => Ok(ExprLiteral::Text(v.clone())),
    }
}
