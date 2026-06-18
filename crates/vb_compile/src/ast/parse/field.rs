#![forbid(unsafe_code)]
//! Field extractors and value parsing.
//!
//! Provides low-level helpers for extracting typed values from YAML nodes
//! and converting them into AST value types.

use super::expr::parse_slot_idx;
use crate::ast::types::{AstMapEntry, AstValue};
use crate::CompileError;
use saphyr::Yaml;
use vb_core::SlotIdx;

/// Extract a required string field from a mapping node.
pub(crate) fn required_str<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a str, CompileError> {
    doc.as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?
        .as_str()
        .ok_or(CompileError::FieldShape {
            field,
            expected: "a string",
        })
}

/// Extract a required mapping field from a mapping node.
pub(crate) fn required_mapping<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a saphyr::Mapping<'a>, CompileError> {
    doc.as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?
        .as_mapping()
        .ok_or(CompileError::FieldShape {
            field,
            expected: "a mapping",
        })
}

/// Extract a required sequence field from a mapping node.
pub(crate) fn required_sequence<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a saphyr::Sequence<'a>, CompileError> {
    doc.as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?
        .as_sequence()
        .ok_or(CompileError::FieldShape {
            field,
            expected: "a sequence",
        })
}

/// Extract an optional string field from a mapping node.
pub(crate) fn optional_str<'a>(value: &'a Yaml<'a>, field: &str) -> Option<&'a str> {
    value.as_mapping_get(field).and_then(Yaml::as_str)
}

/// Extract a required field from a step body node (returns raw Yaml).
pub(crate) fn step_field<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    field: &'static str,
) -> Result<&'a Yaml<'a>, CompileError> {
    body.as_mapping_get(field)
        .ok_or(CompileError::MissingStepField { step, field })
}

/// Extract a required string field from a step body node.
pub(crate) fn step_str<'a>(
    step: &'a Yaml<'a>,
    index: usize,
    field: &'static str,
) -> Result<&'a str, CompileError> {
    step.as_mapping_get(field)
        .ok_or(CompileError::MissingStepField { step: index, field })?
        .as_str()
        .ok_or(CompileError::StepFieldShape {
            step: index,
            field,
            expected: "a string",
        })
}

/// Optionally parse a slot index from a field.
pub(crate) fn optional_slot(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<Option<SlotIdx>, CompileError> {
    match body.as_mapping_get(field) {
        Some(node) => parse_slot_idx(node, step, field).map(Some),
        None => Ok(None),
    }
}

/// Parse a YAML node into an `AstValue`.
pub(crate) fn parse_value(node: &Yaml<'_>) -> Result<AstValue, CompileError> {
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

/// Parse non-scalar YAML nodes (string, sequence, mapping) into `AstValue`.
pub(crate) fn parse_non_scalar_value(node: &Yaml<'_>) -> Result<AstValue, CompileError> {
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

/// Convert a text string into an `AstValue`, distinguishing references from literals.
pub(crate) fn text_or_ref(value: &str) -> AstValue {
    if value.starts_with('$') {
        AstValue::Reference(value.into())
    } else {
        AstValue::Text(value.into())
    }
}

/// Parse a save-body mapping into `AstMapEntry<AstValue>` fields.
pub(crate) fn parse_value_fields(
    body: &Yaml<'_>,
) -> Result<Vec<AstMapEntry<AstValue>>, CompileError> {
    let mapping = body.as_mapping().ok_or(CompileError::StepFieldShape {
        step: 0,
        field: "save",
        expected: "an object",
    })?;
    let mut fields = Vec::with_capacity(mapping.len());
    for (key, value) in mapping {
        fields.push(value_field(key, value)?);
    }
    Ok(fields)
}

/// Parse a single key/value pair into an `AstMapEntry<AstValue>`.
pub(crate) fn value_field(
    key: &Yaml<'_>,
    value: &Yaml<'_>,
) -> Result<AstMapEntry<AstValue>, CompileError> {
    let name = key.as_str().ok_or_else(crate::non_string_key_error)?;
    Ok(AstMapEntry {
        name: name.into(),
        value: parse_value(value)?,
        mark: None,
    })
}
