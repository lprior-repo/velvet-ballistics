#![forbid(unsafe_code)]
use super::parse_step_fields::value_field;
use super::types::{AstExpression, AstValue};
use crate::{CompileError, expression};
use saphyr::Yaml;
use vb_core::SlotIdx;

pub(super) fn parse_expression(node: &Yaml<'_>) -> Result<AstExpression, CompileError> {
    if let Some(value) = node.as_integer() {
        return parse_slot_expr(value);
    }
    Ok(match node.as_str() {
        Some(value) => parse_source_expression(value)?,
        _ => AstExpression::Literal(parse_value(node)?),
    })
}

fn parse_source_expression(value: &str) -> Result<AstExpression, CompileError> {
    expression::parse_expression(value).map(|parsed| AstExpression::Parsed(Box::new(parsed)))
}

pub(super) fn parse_slot_expr(value: i64) -> Result<AstExpression, CompileError> {
    let raw = u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
    Ok(AstExpression::Slot(SlotIdx::new(raw)))
}

pub(super) fn parse_value(node: &Yaml<'_>) -> Result<AstValue, CompileError> {
    if node.is_null() {
        Ok(AstValue::Null)
    } else if let Some(value) = node.as_bool() {
        Ok(AstValue::Bool(value))
    } else if let Some(value) = node.as_integer() {
        Ok(AstValue::I64(value))
    } else {
        parse_non_scalar_value(node)
    }
}

fn parse_non_scalar_value(node: &Yaml<'_>) -> Result<AstValue, CompileError> {
    if let Some(value) = node.as_str() {
        Ok(text_or_ref(value))
    } else if let Some(sequence) = node.as_sequence() {
        let mut values = Vec::with_capacity(sequence.len());
        for value in sequence {
            values.push(parse_value(value)?);
        }
        Ok(AstValue::Sequence(values))
    } else if let Some(mapping) = node.as_mapping() {
        let mut entries = Vec::with_capacity(mapping.len());
        for (key, value) in mapping {
            entries.push(value_field(key, value)?);
        }
        Ok(AstValue::Mapping(entries))
    } else {
        Err(CompileError::BadValue)
    }
}

fn text_or_ref(value: &str) -> AstValue {
    if value.starts_with('$') {
        AstValue::Reference(value.into())
    } else {
        AstValue::Text(value.into())
    }
}
