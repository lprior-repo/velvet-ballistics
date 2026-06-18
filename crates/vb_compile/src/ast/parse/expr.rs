#![forbid(unsafe_code)]
//! Expression and index parsing for step bodies.
//!
//! Handles slot indices, step indices, action IDs, u32/u16 numeric fields,
//! and the top-level expression parser.

use crate::ast::types::AstExpression;
use crate::{CompileError, expression};
use saphyr::Yaml;
use vb_core::{ActionId, SlotIdx, StepIdx};

use super::field::{parse_value, step_field};

/// Parse a step's `action` integer field into an `ActionId`.
pub(crate) fn parse_action_idx(node: &Yaml<'_>, step: usize) -> Result<ActionId, CompileError> {
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field: "action",
        expected: "an integer action id",
    })?;
    let raw = u16::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive: "run",
        field: "action",
        value: integer_error_value(value),
        limit: usize::from(u16::MAX),
    })?;
    Ok(ActionId::new(raw))
}

/// Parse a step's integer field into a `StepIdx`.
pub(crate) fn parse_step_idx(node: &Yaml<'_>) -> Result<StepIdx, CompileError> {
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step: 0,
        field: "branch target",
        expected: "an integer step index",
    })?;
    let raw = u16::try_from(value).map_err(|_| CompileError::BranchTargetOutOfRange { value })?;
    Ok(StepIdx::new(raw))
}

/// Parse a sequence of integer step indexes.
pub(crate) fn parse_step_idx_sequence(
    node: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<Vec<StepIdx>, CompileError> {
    let sequence = node.as_sequence().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "a sequence of integer step indexes",
    })?;
    let mut targets = Vec::with_capacity(sequence.len());
    for item in sequence {
        targets.push(parse_step_idx(item)?);
    }
    Ok(targets)
}

/// Parse a step body u32 integer field with overflow checking.
pub(crate) fn parse_u32_field(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<u32, CompileError> {
    let value = step_field(body, step, field)?.as_integer().ok_or({
        CompileError::StepFieldShape {
            step,
            field,
            expected: "a non-negative u32 integer",
        }
    })?;
    u32::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive: field,
        field,
        value: integer_error_value(value),
        limit: usize::try_from(u32::MAX).map_or(usize::MAX, |limit| limit),
    })
}

/// Parse a step body u16 integer field with overflow checking.
pub(crate) fn parse_u16_field(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<u16, CompileError> {
    let value = step_field(body, step, field)?.as_integer().ok_or({
        CompileError::StepFieldShape {
            step,
            field,
            expected: "a non-negative u16 integer",
        }
    })?;
    u16::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive: field,
        field,
        value: integer_error_value(value),
        limit: usize::from(u16::MAX),
    })
}

/// Convert an error-range integer to a safe display value.
pub(crate) fn integer_error_value(value: i64) -> usize {
    match usize::try_from(value) {
        Ok(value) => value,
        Err(_) => usize::MAX,
    }
}

/// Top-level expression parser.
///
/// Integer nodes become slot expressions; strings become source
/// expressions; everything else becomes a literal value expression.
pub(crate) fn parse_expression(node: &Yaml<'_>) -> Result<AstExpression, CompileError> {
    if let Some(value) = node.as_integer() {
        return parse_slot_expr(value);
    }
    Ok(match node.as_str() {
        Some(value) => parse_source_expression(value)?,
        _ => AstExpression::Literal(parse_value(node)?),
    })
}

/// Parse a source-language expression string into a `ParsedExpression`.
pub(crate) fn parse_source_expression(value: &str) -> Result<AstExpression, CompileError> {
    expression::parse_expression(value).map(|parsed| AstExpression::Parsed(Box::new(parsed)))
}

/// Parse an integer node as a slot expression.
pub(crate) fn parse_slot_expr(value: i64) -> Result<AstExpression, CompileError> {
    let raw = u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
    Ok(AstExpression::Slot(SlotIdx::new(raw)))
}

/// Parse an integer node into a `SlotIdx` with overflow checking.
pub(crate) fn parse_slot_idx(
    node: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<SlotIdx, CompileError> {
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "an integer slot index",
    })?;
    let raw = u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
    Ok(SlotIdx::new(raw))
}
