use super::canonical_types::validate_canonical_references_scoped;
use super::{CanonicalScope, resolve_slot_reference_scoped};
use crate::expression::{BinaryOp, ParsedExpression, parse_expression};
use crate::{CompileError, CompileErrors};
use vb_expr::expr_typecheck::ExprType;

pub(in crate::mod_compile_lowering) fn validate_for_each_expression(
    value: &str,
    scope: CanonicalScope<'_>,
) -> Result<(), CompileErrors> {
    let expr = parse_expression(value).map_err(|e| CompileErrors(vec![e]))?;
    validate_canonical_references_scoped(&expr, scope).map_err(|e| CompileErrors(vec![e]))?;
    validate_operands(&expr, scope, true).map_err(|e| CompileErrors(vec![e]))
}

fn reject_unknown_reference(name: &str, scope: CanonicalScope<'_>) -> Result<(), CompileError> {
    let slot = resolve_slot_reference_scoped(name, scope)?;
    let ty = scope.get_type(slot).unwrap_or(ExprType::Unknown);
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

fn validate_operands(
    expr: &ParsedExpression,
    scope: CanonicalScope<'_>,
    allow_unknown: bool,
) -> Result<(), CompileError> {
    match expr {
        ParsedExpression::Reference(_) if allow_unknown => Ok(()),
        ParsedExpression::Reference(name) => reject_unknown_reference(name, scope),
        ParsedExpression::Literal(_) => Ok(()),
        ParsedExpression::Unary { expr: inner, .. } => validate_operands(inner, scope, false),
        ParsedExpression::Binary { op, left, right } => {
            let allow_unknown = matches!(op, BinaryOp::Eq | BinaryOp::NotEq);
            validate_operands(left, scope, allow_unknown)?;
            validate_operands(right, scope, allow_unknown)
        }
        ParsedExpression::HelperCall { args, .. } => {
            for arg in args {
                validate_operands(arg, scope, true)?;
            }
            Ok(())
        }
    }
}
