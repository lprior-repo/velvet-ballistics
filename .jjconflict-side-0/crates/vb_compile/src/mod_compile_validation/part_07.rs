#![allow(unused_imports)]
use super::*;
use crate::limits::YamlLimits;
use crate::mod_compile_errors::non_string_key_error;
use crate::mod_compile_errors::{CompileError, CompileErrors, SourceMark};
use saphyr::Yaml;
use saphyr_parser::{Event, Parser, Span, StrInput};
use std::collections::HashSet;
use std::str;
use vb_core::{ConstValue, SlotIdx, StepIdx};

pub(crate) fn required_u16_field(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    field: &'static str,
) -> Result<u16, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "a non-negative u16 integer",
    })?;
    u16::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive,
        field,
        value: integer_error_value(value),
        limit: usize::from(u16::MAX),
    })
}

pub(crate) fn integer_error_value(value: i64) -> usize {
    match usize::try_from(value) {
        Ok(value) => value,
        Err(_) => usize::MAX,
    }
}

pub(crate) fn required_branch_targets(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<Vec<StepIdx>, CompileError> {
    let node = required_step_field(body, step, field)?;
    let sequence = node.as_sequence().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "a sequence of integer step indexes",
    })?;
    if sequence.is_empty() {
        return Err(CompileError::StepFieldShape {
            step,
            field,
            expected: "at least one integer step index",
        });
    }
    let mut targets = Vec::with_capacity(sequence.len());
    let mut index = 0usize;
    while index < sequence.len() {
        let Some(node) = sequence.get(index) else {
            return Err(CompileError::StepIndexOutOfRange { value: index });
        };
        let value = node.as_integer().ok_or(CompileError::StepFieldShape {
            step,
            field,
            expected: "a sequence of integer step indexes",
        })?;
        let value =
            u16::try_from(value).map_err(|_| CompileError::BranchTargetOutOfRange { value })?;
        targets.push(StepIdx::new(value));
        index = index
            .checked_add(1)
            .ok_or(CompileError::StepIndexOutOfRange { value: index })?;
    }
    Ok(targets)
}

#[allow(dead_code)]
pub(crate) fn checked_step_offset(
    id: StepIdx,
    offset: u16,
    primitive: &'static str,
    field: &'static str,
) -> Result<StepIdx, CompileError> {
    id.checked_add(offset)
        .ok_or(CompileError::PrimitiveLoweringLimitExceeded {
            primitive,
            field,
            value: id.as_usize(),
            limit: usize::from(u16::MAX),
        })
}

#[allow(dead_code)]
pub(crate) fn required_action(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
) -> Result<vb_core::ActionId, CompileError> {
    let node = required_step_field(body, step, "action")?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field: "action",
        expected: "an integer action id",
    })?;
    let raw = u16::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive,
        field: "action",
        value: integer_error_value(value),
        limit: usize::from(u16::MAX),
    })?;
    Ok(vb_core::ActionId::new(raw))
}

#[allow(dead_code)]
pub(crate) fn required_branch_target(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<StepIdx, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "an integer step index",
    })?;
    let value = u16::try_from(value).map_err(|_| CompileError::BranchTargetOutOfRange { value })?;
    Ok(StepIdx::new(value))
}

pub(crate) fn slot_value(node: &Yaml<'_>, step: usize) -> Result<ConstValue, CompileError> {
    match node {
        Yaml::Value(saphyr::Scalar::Null) => Ok(ConstValue::Null),
        Yaml::Value(saphyr::Scalar::Boolean(value)) => Ok(ConstValue::Bool(*value)),
        Yaml::Value(saphyr::Scalar::Integer(value)) => Ok(ConstValue::I64(*value)),
        Yaml::Value(saphyr::Scalar::String(value)) | Yaml::Representation(value, _, None) => {
            text_slot_value(value.as_ref(), step)
        }
        Yaml::Sequence(sequence) => list_slot_value(sequence, step),
        Yaml::Mapping(mapping) => object_slot_value(mapping, step),
        _ => Err(CompileError::UnsupportedConstantValue { step }),
    }
}

pub(super) fn text_slot_value(_value: &str, step: usize) -> Result<ConstValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

pub(super) fn list_slot_value(
    _sequence: &saphyr::Sequence<'_>,
    step: usize,
) -> Result<ConstValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

pub(super) fn object_slot_value(
    _mapping: &saphyr::Mapping<'_>,
    step: usize,
) -> Result<ConstValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}
