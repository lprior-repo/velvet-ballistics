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

pub(super) fn reject_unknown_trigger_fields(
    trigger: &'static str,
    mapping: &saphyr::Mapping<'_>,
    allowed: &[&str],
) -> Result<(), CompileError> {
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            return Err(non_string_key_error());
        };
        if !allowed.contains(&field) {
            return Err(CompileError::UnknownTriggerField {
                trigger,
                field: Box::<str>::from(field),
            });
        }
    }
    Ok(())
}

pub(super) fn required_trigger_string_field<'a>(
    node: &'a Yaml<'a>,
    trigger: &'static str,
    field: &'static str,
) -> Result<&'a str, CompileError> {
    let value = node
        .as_mapping_get(field)
        .ok_or(CompileError::MissingTriggerField { trigger, field })?;
    value.as_str().ok_or(CompileError::InvalidTriggerField {
        trigger,
        field,
        expected: "a string",
    })
}

pub(super) fn optional_trigger_string_field(
    node: &Yaml<'_>,
    trigger: &'static str,
    field: &'static str,
) -> Result<(), CompileError> {
    match node.as_mapping_get(field) {
        Some(value) if value.as_str().is_none() => Err(CompileError::InvalidTriggerField {
            trigger,
            field,
            expected: "a string",
        }),
        _ => Ok(()),
    }
}

pub(crate) fn required_string_field<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a str, CompileError> {
    let node = doc
        .as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?;
    node.as_str().ok_or(CompileError::FieldShape {
        field,
        expected: "a string",
    })
}

pub(crate) fn required_sequence_field<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a saphyr::Sequence<'a>, CompileError> {
    let node = doc
        .as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?;
    node.as_sequence().ok_or(CompileError::FieldShape {
        field,
        expected: "a sequence",
    })
}

pub(super) fn required_mapping_field<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a saphyr::Mapping<'a>, CompileError> {
    let node = doc
        .as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?;
    node.as_mapping().ok_or(CompileError::FieldShape {
        field,
        expected: "a mapping",
    })
}

pub(crate) fn reject_last_non_finish(index: usize, last_step: usize) -> Result<(), CompileError> {
    if index == last_step {
        Err(CompileError::LastStepMustFinish)
    } else {
        Ok(())
    }
}

pub(crate) fn required_step_field<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    field: &'static str,
) -> Result<&'a Yaml<'a>, CompileError> {
    body.as_mapping_get(field)
        .ok_or(CompileError::MissingStepField { step, field })
}

#[allow(dead_code)]
pub(super) fn required_next_step(
    next: Option<StepIdx>,
    index: usize,
) -> Result<StepIdx, CompileError> {
    next.ok_or(CompileError::StepIndexOutOfRange { value: index })
}

#[allow(dead_code)]
pub(crate) fn reject_unknown_primitive_fields(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    allowed: &[&str],
) -> Result<(), CompileError> {
    let mapping = primitive_body_mapping(body, step, primitive)?;
    for (key, _) in mapping {
        reject_unknown_primitive_field(key, step, primitive, allowed)?;
    }
    Ok(())
}

pub(super) fn primitive_body_mapping<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    primitive: &'static str,
) -> Result<&'a saphyr::Mapping<'a>, CompileError> {
    body.as_mapping().ok_or(CompileError::StepFieldShape {
        step,
        field: primitive,
        expected: "a mapping",
    })
}

pub(super) fn reject_unknown_primitive_field(
    key: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    allowed: &[&str],
) -> Result<(), CompileError> {
    let Some(field) = key.as_str() else {
        return Err(CompileError::StepShape { step });
    };
    if allowed.contains(&field) {
        Ok(())
    } else {
        Err(CompileError::UnknownStepPrimitiveField {
            step,
            primitive,
            field: Box::<str>::from(field),
        })
    }
}

pub(crate) fn required_slot(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<SlotIdx, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "an integer slot index",
    })?;
    let value = u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
    Ok(SlotIdx::new(value))
}

pub(crate) fn required_u32_field(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    field: &'static str,
) -> Result<u32, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "a non-negative u32 integer",
    })?;
    u32::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive,
        field,
        value: integer_error_value(value),
        limit: usize::try_from(u32::MAX).map_or(usize::MAX, |limit| limit),
    })
}
