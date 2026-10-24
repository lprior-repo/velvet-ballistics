use super::canonical_types::validate_canonical_references;
use super::{CanonicalScope, resolve_slot_reference};
use crate::expression::{ParsedExpression, parse_expression};
use crate::{CompileError, CompileErrors};
use vb_expr::expr_typecheck::ExprType;

pub(in crate::mod_compile_lowering) fn validate_for_each_expression(
    value: &str,
    scope: CanonicalScope<'_>,
) -> Result<(), CompileErrors> {
    let expr = parse_expression(value).map_err(|e| CompileErrors(vec![e]))?;
    validate_canonical_references(&expr, scope.outputs, scope.step_outputs)
        .map_err(|e| CompileErrors(vec![e]))?;
    match &expr {
        ParsedExpression::Literal(_) | ParsedExpression::Reference(_) => Ok(()),
        _ => reject_unknown(&expr, scope).map_err(|e| CompileErrors(vec![e])),
    }
}

fn reject_unknown_reference(name: &str, scope: CanonicalScope<'_>) -> Result<(), CompileError> {
    let slot = resolve_slot_reference(name, scope.outputs, scope.step_outputs)?;
    let ty = scope
        .slot_types
        .get(&slot)
        .copied()
        .unwrap_or(ExprType::Unknown);
    if ty == ExprType::Unknown {
        Err(CompileError::TypeMismatch {
            field: "set.value",
            expected: "known static operand type",
            found: "unknown",
        })
    } else {
        Ok(())
    }
}

fn reject_unknown(expr: &ParsedExpression, scope: CanonicalScope<'_>) -> Result<(), CompileError> {
    match expr {
        ParsedExpression::Reference(name) => reject_unknown_reference(name, scope),
        ParsedExpression::Literal(_) => Ok(()),
        ParsedExpression::Unary { expr: inner, .. } => reject_unknown(inner, scope),
        ParsedExpression::Binary { left, right, .. } => {
            reject_unknown(left, scope)?;
            reject_unknown(right, scope)
        }
        ParsedExpression::HelperCall { args, .. } => {
            for arg in args {
                reject_unknown(arg, scope)?;
            }
            Ok(())
        }
    }
}
