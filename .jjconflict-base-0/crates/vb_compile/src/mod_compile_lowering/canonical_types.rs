#![forbid(unsafe_code)]
//! Canonical expression type inference using shared vb_expr type checker.
//!
//! Converts canonical ParsedExpression to vb_expr ExprAst for shared type
//! checking. Builds a TypeContext by resolving each reference in the
//! expression through the canonical resolver and mapping to slot types.

use crate::CompileError;
use crate::expression::{BinaryOp, ExpressionHelper, ExpressionLiteral, ParsedExpression, UnaryOp};
use std::collections::HashMap;
use vb_core::SlotIdx;
use vb_expr::expr_parser::types::{ExprAst, ExprHelper, ExprLiteral};
use vb_expr::expr_typecheck::{ExprType, TypeContext, typecheck_expr};

use super::canonical_references::StepReferenceResolver;
use super::canonical_scope::CanonicalScope;

/// Type-checks a canonical expression and returns its inferred type.
/// Known static type errors (e.g., bool + number) are rejected. Dynamic
/// action outputs remain Unknown.
#[allow(dead_code)]
pub(super) fn infer_canonical_expr_type(
    expr: &ParsedExpression,
    slot_types: &HashMap<SlotIdx, ExprType>,
    resolver: &StepReferenceResolver<'_>,
) -> Result<ExprType, CompileError> {
    let ast = to_vb_expr_ast(expr)?;
    let context = build_type_context(expr, slot_types, resolver)?;
    typecheck_expr(&ast, &context).map_err(vb_expr_error_to_compile)
}

/// Scoped variant using borrowed `CanonicalScope` chain for type look-ups.
pub(super) fn infer_canonical_expr_type_scoped(
    expr: &ParsedExpression,
    scope: CanonicalScope<'_>,
    resolver: &StepReferenceResolver<'_>,
) -> Result<ExprType, CompileError> {
    let ast = to_vb_expr_ast(expr)?;
    let context = build_type_context_scoped(expr, scope, resolver)?;
    typecheck_expr(&ast, &context).map_err(vb_expr_error_to_compile)
}

/// Validates that all references resolve to known slots.
#[allow(dead_code)]
pub(super) fn validate_canonical_references(
    expr: &ParsedExpression,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
) -> Result<(), CompileError> {
    let resolver = StepReferenceResolver::from_maps(outputs, step_outputs);
    validate_canonical_references_with_resolver(expr, &resolver)
}

pub(super) fn validate_canonical_references_scoped(
    expr: &ParsedExpression,
    scope: CanonicalScope<'_>,
) -> Result<(), CompileError> {
    let resolver = StepReferenceResolver::new(scope);
    validate_canonical_references_with_resolver(expr, &resolver)
}

pub(super) fn validate_canonical_references_with_resolver(
    expr: &ParsedExpression,
    resolver: &StepReferenceResolver<'_>,
) -> Result<(), CompileError> {
    for (reference, _) in collect_references(expr) {
        resolver.resolve_slot(&reference)?;
    }
    Ok(())
}

/// Collects all reference names from an expression tree.
fn collect_references(expr: &ParsedExpression) -> Vec<(String, bool)> {
    let mut refs = Vec::new();
    collect_refs_inner(expr, &mut refs);
    refs
}

fn collect_refs_inner(expr: &ParsedExpression, refs: &mut Vec<(String, bool)>) {
    match expr {
        ParsedExpression::Reference(name) => {
            let is_explicit_step = name.contains(".step.") || name.contains(".steps.");
            refs.push((name.to_string(), is_explicit_step));
        }
        ParsedExpression::Unary { expr: inner, .. } => collect_refs_inner(inner, refs),
        ParsedExpression::Binary { left, right, .. } => {
            collect_refs_inner(left, refs);
            collect_refs_inner(right, refs);
        }
        ParsedExpression::HelperCall { args, .. } => {
            for arg in args {
                collect_refs_inner(arg, refs);
            }
        }
        ParsedExpression::Literal(_) => {}
    }
}

/// Builds a TypeContext by resolving each reference in the expression
/// through the canonical resolver and mapping to slot types.
#[allow(dead_code)]
fn build_type_context(
    expr: &ParsedExpression,
    slot_types: &HashMap<SlotIdx, ExprType>,
    resolver: &StepReferenceResolver<'_>,
) -> Result<TypeContext, CompileError> {
    let mut context = TypeContext::new();
    for (reference, _) in collect_references(expr) {
        let slot = resolver.resolve_slot(&reference)?;
        let ty = slot_types.get(&slot).copied().unwrap_or(ExprType::Unknown);
        context.add_variable(reference.clone().into(), ty);
    }
    Ok(context)
}

fn build_type_context_scoped(
    expr: &ParsedExpression,
    scope: CanonicalScope<'_>,
    resolver: &StepReferenceResolver<'_>,
) -> Result<TypeContext, CompileError> {
    let mut context = TypeContext::new();
    for (reference, _) in collect_references(expr) {
        let slot = resolver.resolve_slot(&reference)?;
        let ty = scope.get_type(slot).unwrap_or(ExprType::Unknown);
        context.add_variable(reference.clone().into(), ty);
    }
    Ok(context)
}

/// Converts a vb_expr type-checking error to a CompileError variant.
fn vb_expr_error_to_compile(e: vb_expr::ExprError) -> CompileError {
    match e {
        vb_expr::ExprError::TypeMismatch { expected, found } => CompileError::TypeMismatch {
            field: "set.value",
            expected: type_name_static(&expected),
            found: type_name_static(&found),
        },
        vb_expr::ExprError::StackUnderflow
        | vb_expr::ExprError::DivisionByZero
        | vb_expr::ExprError::IntegerOverflow => CompileError::TypeMismatch {
            field: "set.value",
            expected: "valid expression",
            found: "invalid runtime value",
        },
        vb_expr::ExprError::ExpressionTooLong { max, .. } => {
            CompileError::ExpressionLimitExceeded {
                expression: Box::from("expression"),
                limit: "token count",
                max,
            }
        }
        vb_expr::ExprError::HelperArityMismatch {
            helper,
            expected,
            actual,
        } => CompileError::ExpressionHelperArity {
            helper: helper_name_static(&helper),
            expected,
            actual,
        },
        vb_expr::ExprError::UnknownHelper { helper } => CompileError::ExpressionUnexpectedToken {
            expression: Box::from(format!("unknown helper: {helper}")),
            index: 0,
            expected: "valid expression",
        },
        _ => CompileError::ExpressionUnexpectedToken {
            expression: Box::from(format!("{e}")),
            index: 0,
            expected: "valid expression",
        },
    }
}

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
        ParsedExpression::Literal(lit) => Ok(ExprAst::Literal(to_vb_expr_literal(lit)?)),
        ParsedExpression::Reference(name) => Ok(ExprAst::Reference(name.clone())),
        ParsedExpression::Unary { op, expr: inner } => {
            let vb_op = match op {
                UnaryOp::Not => vb_expr::lexer::UnaryOp::Not,
                UnaryOp::Neg => vb_expr::lexer::UnaryOp::Neg,
            };
            Ok(ExprAst::Unary {
                op: vb_op,
                expr: Box::new(to_vb_expr_ast(inner)?),
            })
        }
        ParsedExpression::Binary { op, left, right } => to_vb_expr_binary(op, left, right),
        ParsedExpression::HelperCall { name, args } => to_vb_expr_helper(name, args),
    }
}

fn to_vb_expr_binary(
    op: &BinaryOp,
    left: &ParsedExpression,
    right: &ParsedExpression,
) -> Result<ExprAst, CompileError> {
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
    Ok(ExprAst::Binary {
        op: vb_op,
        left: Box::new(to_vb_expr_ast(left)?),
        right: Box::new(to_vb_expr_ast(right)?),
    })
}

fn to_vb_expr_helper(
    name: &ExpressionHelper,
    args: &[ParsedExpression],
) -> Result<ExprAst, CompileError> {
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

fn to_vb_expr_literal(lit: &ExpressionLiteral) -> Result<ExprLiteral, CompileError> {
    match lit {
        ExpressionLiteral::Null => Ok(ExprLiteral::Null),
        ExpressionLiteral::Bool(v) => Ok(ExprLiteral::Bool(*v)),
        ExpressionLiteral::I64(v) => Ok(ExprLiteral::I64(*v)),
        ExpressionLiteral::F64(v) => Ok(ExprLiteral::F64(*v)),
        ExpressionLiteral::Text(v) => Ok(ExprLiteral::Text(v.clone())),
    }
}
