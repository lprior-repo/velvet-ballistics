#![forbid(unsafe_code)]
//! Typed initial parsing for reduce accumulators.
//!
//! The accumulator initial is a literal only: anything that can become a
//! [`ConstValue`] (null, boolean, integer, finite float). Malformed shapes
//! are rejected at the parse boundary with the lowering's `aggregate.initial`
//! diagnostic, so the lowering core never sees unchecked text.

use crate::expression::{ExpressionLiteral, ParsedExpression, UnaryOp, parse_expression};
use crate::mod_compile_errors::{CompileError, CompileErrors};
use vb_core::{ConstValue, FiniteF64};
use vb_expr::expr_typecheck::ExprType;

pub(in crate::mod_compile_lowering) fn parse_typed_initial(
    text: &str,
    index: usize,
) -> Result<(ConstValue, ExprType), CompileErrors> {
    if let Ok(value) = text.parse::<i64>() {
        return Ok((ConstValue::I64(value), ExprType::I64));
    }
    let parsed = parse_expression(text).map_err(|e| map_initial_error(e, index))?;
    literal_constant(&parsed, index)
}

fn map_initial_error(error: CompileError, index: usize) -> CompileErrors {
    if is_expression_error(&error) {
        initial_error(index)
    } else {
        CompileErrors(vec![error])
    }
}

fn is_expression_error(error: &CompileError) -> bool {
    matches!(
        error,
        CompileError::ExpressionUnexpectedChar { .. }
            | CompileError::ExpressionUnterminatedString { .. }
            | CompileError::ExpressionIntegerOutOfRange { .. }
            | CompileError::ExpressionFloatOutOfRange { .. }
            | CompileError::ExpressionLimitExceeded { .. }
            | CompileError::ExpressionUnexpectedToken { .. }
            | CompileError::ExpressionUnknownIdentifier { .. }
            | CompileError::ExpressionLoweringUnsupported { .. }
            | CompileError::ExpressionHelperArity { .. }
    )
}

fn literal_constant(
    expr: &ParsedExpression,
    index: usize,
) -> Result<(ConstValue, ExprType), CompileErrors> {
    match expr {
        ParsedExpression::Literal(literal) => literal_value(literal, index),
        ParsedExpression::Unary {
            op: UnaryOp::Neg,
            expr,
        } => negate_literal(expr, index),
        _ => Err(initial_error(index)),
    }
}

fn literal_value(
    literal: &ExpressionLiteral,
    index: usize,
) -> Result<(ConstValue, ExprType), CompileErrors> {
    match literal {
        ExpressionLiteral::Null => Ok((ConstValue::Null, ExprType::Null)),
        ExpressionLiteral::Bool(value) => Ok((ConstValue::Bool(*value), ExprType::Bool)),
        ExpressionLiteral::I64(value) => Ok((ConstValue::I64(*value), ExprType::I64)),
        ExpressionLiteral::F64(value) => Ok((ConstValue::F64(*value), ExprType::F64)),
        ExpressionLiteral::Text(_) => Err(initial_error(index)),
    }
}

fn negate_literal(
    expr: &ParsedExpression,
    index: usize,
) -> Result<(ConstValue, ExprType), CompileErrors> {
    match expr {
        ParsedExpression::Literal(ExpressionLiteral::I64(value)) => value
            .checked_neg()
            .map(|negated| (ConstValue::I64(negated), ExprType::I64))
            .ok_or_else(|| initial_error(index)),
        ParsedExpression::Literal(ExpressionLiteral::F64(value)) => FiniteF64::new(-value.get())
            .map(|negated| (ConstValue::F64(negated), ExprType::F64))
            .map_err(|_| initial_error(index)),
        _ => Err(initial_error(index)),
    }
}

fn initial_error(index: usize) -> CompileErrors {
    CompileErrors(vec![CompileError::StepFieldShape {
        step: index,
        field: "aggregate.initial",
        expected: "a literal constant: null, boolean, integer, or finite float",
    }])
}
