#![allow(unused_imports)]
use super::*;
use crate::limits::YamlLimits;
use crate::mod_compile_errors::non_string_key_error;
use crate::mod_compile_errors::{CompileError, CompileErrors, SourceMark};
use saphyr::Yaml;
use saphyr_parser::{Event, Parser, Span, StrInput};
use std::collections::HashSet;
use std::str;
use crate::mod_compile_lowering::{required_branch_targets, required_slot};
use vb_core::{ConstValue, SlotIdx, StepIdx};

pub(super) fn validate_for_each_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unsupported_for_each_fields(body, index)?;
    reject_unknown_primitive_fields(
        body,
        index,
        "for_each",
        &["variable", "input", "item", "limit", "at_once", "steps"],
    )?;
    if is_high_level_scoped_body(body) {
        required_non_empty_step_string(body, index, "variable", "for_each.variable")?;
        required_non_empty_step_string(body, index, "input", "for_each.input")?;
        optional_u32_field(body, index, "for_each", "at_once")?;
        validate_scoped_body_steps(body, index, "for_each.steps")?;
        return Ok(());
    }
    required_slot(body, index, "input")?;
    required_slot(body, index, "item")?;
    required_u32_field(body, index, "for_each", "limit")?;
    Ok(())
}

pub(crate) fn reject_unsupported_for_each_fields(
    _body: &Yaml<'_>,
    _step: usize,
) -> Result<(), CompileError> {
    Ok(())
}

pub(super) fn validate_parallel_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "parallel", &["branches"])?;
    required_branch_targets(body, index, "branches")?;
    Ok(())
}

pub(super) fn validate_together_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "together", &["branches"])?;
    validate_together_branches(body, index)?;
    Ok(())
}

pub(super) fn validate_collect_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(
        body,
        index,
        "collect",
        &[
            "variable",
            "source",
            "pages",
            "items",
            "steps",
            "limit",
            "page_size",
        ],
    )?;
    if is_high_level_scoped_body(body) {
        required_non_empty_step_string(body, index, "variable", "collect.variable")?;
        required_non_empty_step_string(body, index, "source", "collect.source")?;
        optional_u32_field(body, index, "collect", "pages")?;
        optional_u32_field(body, index, "collect", "items")?;
        validate_scoped_body_steps(body, index, "collect.steps")?;
        return Ok(());
    }
    required_slot(body, index, "source")?;
    required_u32_field(body, index, "collect", "limit")?;
    required_u32_field(body, index, "collect", "page_size")?;
    Ok(())
}

pub(super) fn validate_aggregate_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(
        body,
        index,
        "reduce",
        &["variable", "input", "initial", "steps", "accumulator"],
    )?;
    if is_high_level_scoped_body(body) {
        required_non_empty_step_string(body, index, "variable", "reduce.variable")?;
        required_non_empty_step_string(body, index, "input", "reduce.input")?;
        required_non_empty_step_string(body, index, "initial", "reduce.initial")?;
        validate_scoped_body_steps(body, index, "reduce.steps")?;
        return Ok(());
    }
    required_slot(body, index, "input")?;
    required_slot(body, index, "accumulator")?;
    let initial = required_step_field(body, index, "initial")?;
    slot_value(initial, index)?;
    Ok(())
}

pub(super) fn required_text_or_integer_field(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
    expected: &'static str,
) -> Result<(), CompileError> {
    let node = required_step_field(body, step, field)?;
    validate_text_or_integer_node(node, step, field, expected)
}

pub(super) fn optional_text_or_integer_field(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<Option<()>, CompileError> {
    match body.as_mapping_get(field) {
        Some(node) => validate_text_or_integer_node(node, step, field, "a string or integer")
            .map(|()| Some(())),
        None => Ok(None),
    }
}

fn is_high_level_scoped_body(body: &Yaml<'_>) -> bool {
    body.as_mapping_get("steps").is_some() || body.as_mapping_get("variable").is_some()
}

fn optional_u32_field(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    field: &'static str,
) -> Result<(), CompileError> {
    match body.as_mapping_get(field) {
        Some(_) => required_u32_field(body, step, primitive, field).map(|_| ()),
        None => Ok(()),
    }
}

fn validate_together_branches(body: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    let sequence = required_together_branch_sequence(body, step)?;
    if sequence.is_empty() {
        return Err(CompileError::StepFieldShape {
            step,
            field: "together.branches",
            expected: "at least one branch",
        });
    }
    for branch in sequence {
        validate_together_branch(branch, step)?;
    }
    Ok(())
}

fn required_together_branch_sequence<'a>(
    body: &'a Yaml<'a>,
    step: usize,
) -> Result<&'a saphyr::Sequence<'a>, CompileError> {
    let node = required_step_field(body, step, "branches")?;
    node.as_sequence().ok_or(CompileError::StepFieldShape {
        step,
        field: "together.branches",
        expected: "a sequence of branch mappings",
    })
}

fn validate_together_branch(branch: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    let mapping = branch.as_mapping().ok_or(CompileError::StepFieldShape {
        step,
        field: "together.branches[]",
        expected: "a mapping",
    })?;
    for (key, _) in mapping {
        reject_unknown_primitive_field(key, step, "together", &["label", "steps"])?;
    }
    required_together_branch_label(branch, step)?;
    validate_together_body_steps(branch, step)
}

fn required_together_branch_label<'a>(
    branch: &'a Yaml<'a>,
    step: usize,
) -> Result<&'a str, CompileError> {
    let node = required_step_field(branch, step, "label")?;
    match node.as_str() {
        Some(label) if !label.is_empty() => Ok(label),
        _ => Err(CompileError::StepFieldShape {
            step,
            field: "together.branches[].label",
            expected: "a non-empty string",
        }),
    }
}

fn validate_together_body_steps(branch: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    let sequence = required_body_step_sequence(branch, step, "together.branches[].steps")?;
    for body_step in sequence {
        validate_scoped_body_step(body_step, step)?;
    }
    Ok(())
}

fn validate_scoped_body_steps(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<(), CompileError> {
    let sequence = required_body_step_sequence(body, step, field)?;
    for body_step in sequence {
        validate_scoped_body_step(body_step, step)?;
    }
    Ok(())
}

fn required_body_step_sequence<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    field: &'static str,
) -> Result<&'a saphyr::Sequence<'a>, CompileError> {
    body.as_mapping_get("steps")
        .ok_or(CompileError::MissingStepField { step, field })?
        .as_sequence()
        .ok_or(CompileError::StepFieldShape {
            step,
            field,
            expected: "a sequence",
        })
}

fn validate_scoped_body_step(body_step: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
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

fn validate_text_or_integer_node(
    node: &Yaml<'_>,
    step: usize,
    field: &'static str,
    expected: &'static str,
) -> Result<(), CompileError> {
    match node.as_str() {
        Some(value) if !value.is_empty() => Ok(()),
        _ if node.as_integer().is_some() => Ok(()),
        _ => Err(CompileError::StepFieldShape {
            step,
            field,
            expected,
        }),
    }
}
