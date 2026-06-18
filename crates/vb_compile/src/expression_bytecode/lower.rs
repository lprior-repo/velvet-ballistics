//! Expression tree lowering: recursive descent into bytecode operations.

use crate::CompileError;
use crate::expression::{BinaryOp, ExpressionHelper, ExpressionLiteral, ParsedExpression, UnaryOp};
use crate::expression_bytecode::{
    binary_op, helper_op, validate_helper_arity,
};
use vb_core::{ConstValue, ExprOp, FiniteF64};

use super::resolver::ExpressionReferenceResolver;

/// Lowers a parsed expression tree into bytecode operations.
///
/// Dispatches to the appropriate lowering function based on expression variant.
pub(crate) fn lower_expr(
    expression: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<(), CompileError> {
    match expression {
        ParsedExpression::Literal(literal) => lower_literal(literal, constants, ops),
        ParsedExpression::Unary { op, expr } => {
            lower_unary(*op, expr, constants, ops, resolver)
        }
        ParsedExpression::Binary { op, left, right } => {
            lower_binary(*op, left, right, constants, ops, resolver)
        }
        ParsedExpression::HelperCall { name, args } => {
            lower_helper(*name, args, constants, ops, resolver)
        }
        ParsedExpression::Reference(reference) => {
            lower_reference(reference, ops, resolver)
        }
    }
}

fn lower_reference(
    reference: &str,
    ops: &mut Vec<ExprOp>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<(), CompileError> {
    ops.push(resolver.resolve_reference(reference)?);
    Ok(())
}

fn lower_literal(
    literal: &ExpressionLiteral,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
) -> Result<(), CompileError> {
    let value = match literal {
        ExpressionLiteral::Null => ConstValue::Null,
        ExpressionLiteral::Bool(value) => ConstValue::Bool(*value),
        ExpressionLiteral::I64(value) => ConstValue::I64(*value),
        ExpressionLiteral::F64(value) => ConstValue::F64(*value),
        ExpressionLiteral::Text(_) => {
            return Err(CompileError::ExpressionLoweringUnsupported {
                feature: "text constants".into(),
            });
        }
    };
    let constant = push_expression_constant(value, constants)?;
    ops.push(ExprOp::LoadConst(constant));
    Ok(())
}

fn lower_unary(
    op: UnaryOp,
    expr: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<(), CompileError> {
    match op {
        UnaryOp::Not => {
            lower_expr(expr, constants, ops, resolver)?;
            ops.push(ExprOp::Not);
            Ok(())
        }
        UnaryOp::Neg => lower_numeric_negation(expr, constants, ops, resolver),
    }
}

fn lower_numeric_negation(
    expr: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<(), CompileError> {
    // Optimize: when the operand is a constant literal, load the absolute
    // value and apply `Neg` directly. This produces a shorter bytecode
    // sequence (`LoadConst + Neg`) compared to the legacy `0 - expr`
    // pattern (`LoadConst(0) + LoadConst(v) + Sub`).
    if let ParsedExpression::Literal(lit) = expr {
        match lit {
            ExpressionLiteral::I64(v) => {
                // Negation of i64::MIN overflows, so we use the absolute
                // value of the operand (which is always safe for non-MIN
                // values) and apply Neg. For i64::MIN the runtime `eval_neg`
                // will correctly return an error.
                let abs_value = v.checked_abs().unwrap_or(*v);
                let idx = push_expression_constant(ConstValue::I64(abs_value), constants)?;
                ops.push(ExprOp::LoadConst(idx));
                ops.push(ExprOp::Neg);
                return Ok(());
            }
            ExpressionLiteral::F64(f) => {
                // Negate the raw f64 value; if the result is non-finite, the
                // runtime `eval_neg` will return NonFiniteNumber.
                let raw = -f.get();
                // For -0.0, abs(-0.0) == 0.0, so we store the positive form.
                let abs_raw = raw.abs();
                let abs_f = FiniteF64::new(abs_raw).map_err(|_| {
                    CompileError::ExpressionLoweringUnsupported {
                        feature: "non-finite absolute value constant".into(),
                    }
                })?;
                let idx = push_expression_constant(ConstValue::F64(abs_f), constants)?;
                ops.push(ExprOp::LoadConst(idx));
                ops.push(ExprOp::Neg);
                return Ok(());
            }
            _ => {}
        }
    }

    // General case: lower the operand, then apply `Neg`.
    // The runtime `eval_neg` handles both I64 and F64 types correctly,
    // so we don't need the legacy `0 - expr` type-safety dance.
    lower_expr(expr, constants, ops, resolver)?;
    ops.push(ExprOp::Neg);
    Ok(())
}

fn lower_binary(
    op: BinaryOp,
    left: &ParsedExpression,
    right: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<(), CompileError> {
    lower_expr(left, constants, ops, resolver)?;
    lower_expr(right, constants, ops, resolver)?;
    ops.push(binary_op(op));
    Ok(())
}

fn lower_helper(
    name: ExpressionHelper,
    args: &[ParsedExpression],
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<(), CompileError> {
    validate_helper_arity(name, args.len())?;
    for arg in args {
        lower_expr(arg, constants, ops, resolver)?;
    }
    ops.push(helper_op(name));
    Ok(())
}

/// Pushes a constant value into the constants pool and returns its index.
pub(crate) fn push_expression_constant(
    value: ConstValue,
    constants: &mut Vec<ConstValue>,
) -> Result<vb_core::ConstIdx, CompileError> {
    let index = u16::try_from(constants.len()).map_err(|_| {
        CompileError::Workflow(vb_core::WorkflowError::ConstOutOfBounds {
            constant: vb_core::ConstIdx::new(u16::MAX),
        })
    })?;
    constants.push(value);
    Ok(vb_core::ConstIdx::new(index))
}
