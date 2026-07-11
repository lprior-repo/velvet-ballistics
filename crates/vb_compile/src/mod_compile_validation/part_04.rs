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

pub(super) fn validate_choose_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "choose", &["condition", "on_true", "on_false"])?;
    required_step_field(body, index, "condition")?;
    required_branch_target(body, index, "on_true")?;
    required_branch_target(body, index, "on_false")?;
    Ok(())
}

pub(super) fn validate_canonical_choose_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "choose", &["branches", "otherwise"])?;
    validate_choose_branches(body, index)?;
    validate_choose_otherwise(body, index)?;
    Ok(())
}

fn validate_choose_branches(body: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    let Some(node) = body.as_mapping_get("branches") else {
        return Ok(());
    };
    let sequence = node.as_sequence().ok_or(CompileError::StepFieldShape {
        step,
        field: "choose.branches",
        expected: "a sequence",
    })?;
    if sequence.len() > 64 {
        return Err(CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "choose",
            field: "branches",
            value: sequence.len(),
            limit: 64,
        });
    }
    for branch in sequence {
        validate_choose_branch(branch, step)?;
    }
    Ok(())
}

fn validate_choose_branch(branch: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    let mapping = branch.as_mapping().ok_or(CompileError::StepFieldShape {
        step,
        field: "choose.branches[]",
        expected: "a mapping",
    })?;
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            return Err(CompileError::StepShape { step });
        };
        if !["when", "steps"].contains(&field) {
            return Err(CompileError::UnknownStepPrimitiveField {
                step,
                primitive: "choose",
                field: Box::<str>::from(field),
            });
        }
    }
    required_non_empty_step_string(branch, step, "when", "choose.branches[].when")?;
    validate_choose_body_steps(branch, step)
}

fn validate_choose_body_steps(branch: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    let Some(node) = branch.as_mapping_get("steps") else {
        return Ok(());
    };
    let sequence = node.as_sequence().ok_or(CompileError::StepFieldShape {
        step,
        field: "choose.branches[].steps",
        expected: "a sequence",
    })?;
    for body_step in sequence {
        validate_choose_body_step(body_step, step)?;
    }
    Ok(())
}

fn validate_choose_body_step(body_step: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    let id = required_step_id(body_step, step)?;
    validate_public_name("step id", id)?;
    let StepSpec { primitive, body } = step_spec(body_step, step)?;
    match primitive {
        StepPrimitive::Set | StepPrimitive::Save => validate_choose_body_set(body, step),
        StepPrimitive::Run | StepPrimitive::Do => {
            validate_choose_body_do(body, step, primitive.as_str())
        }
        _ => Err(CompileError::UnsupportedStepPrimitive {
            step,
            primitive: primitive.as_str(),
        }),
    }
}

pub(super) fn validate_choose_body_set(body: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    reject_unknown_primitive_fields(body, step, "set", &["output", "value"])?;
    required_non_empty_step_string(body, step, "output", "choose.body.set.output")?;
    required_non_empty_step_string(body, step, "value", "choose.body.set.value")?;
    Ok(())
}

pub(super) fn validate_choose_body_do(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
) -> Result<(), CompileError> {
    reject_unknown_primitive_fields(body, step, primitive, &["action", "input"])?;
    required_non_empty_step_string(body, step, "action", "choose.body.do.action")?;
    required_non_empty_step_string(body, step, "input", "choose.body.do.input")?;
    Ok(())
}

fn validate_choose_otherwise(body: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    let Some(node) = body.as_mapping_get("otherwise") else {
        return Ok(());
    };
    let Some(value) = node.as_str() else {
        return Err(CompileError::StepFieldShape {
            step,
            field: "choose.otherwise",
            expected: "a non-empty string",
        });
    };
    if value.is_empty() {
        return Err(CompileError::StepFieldShape {
            step,
            field: "choose.otherwise",
            expected: "a non-empty string",
        });
    }
    validate_public_name("choose.otherwise", value)
}

pub(super) fn required_non_empty_step_string(
    node: &Yaml<'_>,
    step: usize,
    field: &'static str,
    diagnostic: &'static str,
) -> Result<(), CompileError> {
    let value = node
        .as_mapping_get(field)
        .ok_or(CompileError::MissingStepField {
            step,
            field: diagnostic,
        })?;
    match value.as_str() {
        Some(text) if !text.is_empty() => Ok(()),
        _ => Err(CompileError::StepFieldShape {
            step,
            field: diagnostic,
            expected: "a non-empty string",
        }),
    }
}

pub(super) fn validate_repeat_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "repeat", &["max_attempts", "steps"])?;
    required_u16_field(body, index, "repeat", "max_attempts")?;
    Ok(())
}

pub(super) fn validate_finish_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    if index != last_step {
        return Err(CompileError::StepFieldShape {
            step: index,
            field: "finish",
            expected: "the last step",
        });
    }
    reject_unknown_primitive_fields(body, index, "finish", &["result"])?;
    required_step_field(body, index, "result")?;
    Ok(())
}

pub(super) fn validate_phase_zero_result(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("result") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "result",
        expected: "a mapping",
    })?;
    if mapping.is_empty() {
        Ok(())
    } else {
        Err(CompileError::UnsupportedTopLevelResult)
    }
}
