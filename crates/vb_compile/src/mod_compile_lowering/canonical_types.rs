#![forbid(unsafe_code)]
//! Canonical expression type inference using shared vb_expr type checker.
//!
//! Converts canonical ParsedExpression to vb_expr ExprAst for shared type
//! checking. Uses slot-based type identity: types are stored by SlotIdx and
//! resolved through the canonical reference resolver to ensure consistent
//! types for known aliases and step forms.

use crate::CompileError;
use crate::expression::{
    BinaryOp, ExpressionHelper, ExpressionLiteral, ParsedExpression, UnaryOp,
};
use std::collections::HashMap;
use vb_core::SlotIdx;
use vb_expr::expr_parser::types::{ExprAst, ExprHelper, ExprLiteral};
use vb_expr::expr_typecheck::{ExprType, TypeContext, typecheck_expr};

/// Type-checks a canonical expression and returns its inferred type.
/// Unknown future references type as Unknown (dynamic), but known static type
/// errors (e.g., bool + number) are rejected.
///
/// Types are resolved by SlotIdx: references are resolved through the canonical
/// resolver to their actual slot, and the type is looked up from the slot
/// type map. This ensures that known aliases and step forms for the same
/// slot produce consistent types.
pub(super) fn infer_canonical_expr_type(
    expr: &ParsedExpression,
    slot_types: &HashMap<SlotIdx, ExprType>,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
) -> Result<ExprType, CompileError> {
    let ast = to_vb_expr_ast(expr)?;
    let context = build_type_context(slot_types, outputs, step_outputs)?;
    typecheck_expr(&ast, &context).map_err(vb_expr_error_to_compile)
}

/// Validates that all references in the expression resolve to known slots.
/// Returns an error for unknown or ambiguous references before type checking,
/// so that name errors take precedence over misleading type errors.
pub(super) fn validate_canonical_references(
    expr: &ParsedExpression,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
) -> Result<(), CompileError> {
    for (reference, _) in collect_references(expr) {
        let body = reference.strip_prefix('$').unwrap_or(&reference);
        if body.starts_with("input.")
            || body.starts_with("vars.")
            || body.starts_with("secrets.")
        {
            continue;
        }
        let _slot = resolve_reference_slot(body, &reference, outputs, step_outputs)?;
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
        ParsedExpression::Unary { op: _, expr: inner } => {
            collect_refs_inner(inner, refs);
        }
        ParsedExpression::Binary {
            op: _,
            left,
            right,
        } => {
            collect_refs_inner(left, refs);
            collect_refs_inner(right, refs);
        }
        ParsedExpression::HelperCall { name: _, args } => {
            for arg in args {
                collect_refs_inner(arg, refs);
            }
        }
        ParsedExpression::Literal(_) => {}
    }
}

/// Resolves a reference body to its slot using the canonical resolver.
fn resolve_reference_slot(
    body: &str,
    reference: &str,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
) -> Result<SlotIdx, CompileError> {
    if let Some((root, tail)) = body.split_once('.') {
        if root == "step" || root == "steps" {
            return resolve_explicit_step_id(tail, reference, step_outputs);
        }
    }
    resolve_direct_name(body, reference, outputs, step_outputs)
}

fn resolve_explicit_step_id(
    step_id: &str,
    reference: &str,
    step_outputs: &HashMap<String, SlotIdx>,
) -> Result<SlotIdx, CompileError> {
    step_outputs.get(step_id).copied().ok_or(
        CompileError::UnknownReferenceName {
            kind: "step",
            reference: Box::from(reference),
            name: Box::from(step_id),
        },
    )
}

fn resolve_direct_name(
    name: &str,
    reference: &str,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
) -> Result<SlotIdx, CompileError> {
    let in_outputs = outputs.get(name).copied();
    let in_step_outputs = step_outputs.get(name).copied();

    match (in_outputs, in_step_outputs) {
        (Some(slot), Some(same_slot)) if slot == same_slot => Ok(slot),
        (Some(slot), None) => Ok(slot),
        (None, Some(slot)) => Ok(slot),
        (Some(_), Some(_)) => Err(CompileError::ExpressionLoweringUnsupported {
            feature: format!(
                "ambiguous reference '{}' found in both output names and step IDs with different slots",
                name
            ).into(),
        }),
        (None, None) => Err(CompileError::UnknownReferenceName {
            kind: "output",
            reference: Box::from(reference),
            name: Box::from(name),
        }),
    }
}

/// Builds a TypeContext by resolving all known references to their slots
/// and mapping them to their types via the slot type map.
fn build_type_context(
    slot_types: &HashMap<SlotIdx, ExprType>,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
) -> Result<TypeContext, CompileError> {
    let mut context = TypeContext::new();

    // Register all output names
    for (name, slot) in outputs {
        if let Some(ty) = slot_types.get(slot) {
            let canonical = format!("${name}");
            context.add_variable(canonical.into(), *ty);
        }
    }

    // Register all step IDs
    for (step_id, slot) in step_outputs {
        if let Some(ty) = slot_types.get(slot) {
            // Register as $step.<id> and $steps.<id>
            let step_ref = format!("$step.{step_id}");
            let steps_ref = format!("$steps.{step_id}");
            context.add_variable(step_ref.into(), *ty);
            context.add_variable(steps_ref.into(), *ty);
        }
    }

    Ok(context)
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
