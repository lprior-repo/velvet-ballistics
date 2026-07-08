use super::*;
use crate::mod_compile_errors::CompileError;
use saphyr::Yaml;

fn has_any_field(body: &Yaml<'_>, fields: &[&str]) -> bool {
    for field in fields {
        if body.as_mapping_get(field).is_some() {
            return true;
        }
    }
    false
}

pub(super) fn validate_for_each_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unsupported_for_each_fields(body, index)?;
    if has_any_field(body, &["variable", "steps"]) {
        return validate_canonical_for_each_shape(body, index);
    }
    reject_unknown_primitive_fields(
        body,
        index,
        "for_each",
        &["input", "item", "limit", "at_once"],
    )?;
    required_slot(body, index, "input")?;
    required_slot(body, index, "item")?;
    required_u32_field(body, index, "for_each", "limit")?;
    Ok(())
}

fn validate_canonical_for_each_shape(body: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    reject_unknown_primitive_fields(
        body,
        index,
        "for_each",
        &["variable", "input", "at_once", "steps"],
    )?;
    required_non_empty_string_field(body, index, "variable", "for_each.variable")?;
    required_non_empty_string_field(body, index, "input", "for_each.input")?;
    optional_u32_field(body, index, "for_each", "at_once")?;
    optional_steps_field(body, index, "for_each.steps")
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
    validate_parallel_branches(body, index)?;
    Ok(())
}

fn validate_parallel_branches(body: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    let node = required_step_field(body, index, "branches")?;
    let sequence = node.as_sequence().ok_or(CompileError::StepFieldShape {
        step: index,
        field: "branches",
        expected: "a sequence",
    })?;
    let mut items = sequence.iter();
    let Some(first) = items.next() else {
        required_branch_targets(body, index, "branches")?;
        return Ok(());
    };
    if first.as_mapping().is_some() {
        validate_parallel_branch(first, index)?;
        for branch in items {
            validate_parallel_branch(branch, index)?;
        }
        return Ok(());
    }
    required_branch_targets(body, index, "branches")?;
    Ok(())
}

fn validate_parallel_branch(branch: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    let mapping = branch.as_mapping().ok_or(CompileError::StepFieldShape {
        step: index,
        field: "together.branches[]",
        expected: "a mapping",
    })?;
    for (key, _) in mapping {
        reject_unknown_primitive_field(key, index, "parallel", &["label", "steps"])?;
    }
    validate_parallel_branch_label(branch, index)?;
    validate_parallel_branch_steps(branch, index)
}

fn validate_parallel_branch_label(branch: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    let node = branch
        .as_mapping_get("label")
        .ok_or(CompileError::MissingStepField {
            step: index,
            field: "together.branches[].label",
        })?;
    match node.as_str() {
        Some(label) if !label.is_empty() => Ok(()),
        _ => Err(CompileError::StepFieldShape {
            step: index,
            field: "together.branches[].label",
            expected: "a non-empty string",
        }),
    }
}

fn validate_parallel_branch_steps(branch: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    let Some(steps) = branch.as_mapping_get("steps") else {
        return Ok(());
    };
    steps
        .as_sequence()
        .map(|_| ())
        .ok_or(CompileError::StepFieldShape {
            step: index,
            field: "together.branches[].steps",
            expected: "a sequence",
        })
}

pub(super) fn validate_collect_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    if has_any_field(body, &["variable", "pages", "items", "steps"]) {
        return validate_canonical_collect_shape(body, index);
    }
    reject_unknown_primitive_fields(body, index, "collect", &["source", "limit", "page_size"])?;
    required_slot(body, index, "source")?;
    required_u32_field(body, index, "collect", "limit")?;
    required_u32_field(body, index, "collect", "page_size")?;
    Ok(())
}

fn validate_canonical_collect_shape(body: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    reject_unknown_primitive_fields(
        body,
        index,
        "collect",
        &["variable", "source", "pages", "items", "steps"],
    )?;
    required_non_empty_string_field(body, index, "variable", "collect.variable")?;
    required_non_empty_string_field(body, index, "source", "collect.source")?;
    optional_u32_field(body, index, "collect", "pages")?;
    optional_u32_field(body, index, "collect", "items")?;
    optional_steps_field(body, index, "collect.steps")
}

pub(super) fn validate_aggregate_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    if has_any_field(body, &["variable", "steps"]) {
        return validate_canonical_aggregate_shape(body, index);
    }
    reject_unknown_primitive_fields(
        body,
        index,
        "aggregate",
        &["input", "accumulator", "initial"],
    )?;
    required_slot(body, index, "input")?;
    required_slot(body, index, "accumulator")?;
    let initial = required_step_field(body, index, "initial")?;
    slot_value(initial, index)?;
    Ok(())
}

fn validate_canonical_aggregate_shape(body: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    reject_unknown_primitive_fields(
        body,
        index,
        "aggregate",
        &["variable", "input", "initial", "steps"],
    )?;
    required_non_empty_string_field(body, index, "variable", "aggregate.variable")?;
    required_non_empty_string_field(body, index, "input", "aggregate.input")?;
    required_non_empty_string_field(body, index, "initial", "aggregate.initial")?;
    optional_steps_field(body, index, "aggregate.steps")
}

fn required_non_empty_string_field(
    body: &Yaml<'_>,
    index: usize,
    key: &'static str,
    field: &'static str,
) -> Result<(), CompileError> {
    let node = required_step_field(body, index, key)?;
    match node.as_str() {
        Some(value) if !value.is_empty() => Ok(()),
        _ => Err(CompileError::StepFieldShape {
            step: index,
            field,
            expected: "a non-empty string",
        }),
    }
}

fn optional_u32_field(
    body: &Yaml<'_>,
    index: usize,
    primitive: &'static str,
    field: &'static str,
) -> Result<(), CompileError> {
    if body.as_mapping_get(field).is_some() {
        required_u32_field(body, index, primitive, field)?;
    }
    Ok(())
}

fn optional_steps_field(
    body: &Yaml<'_>,
    index: usize,
    field: &'static str,
) -> Result<(), CompileError> {
    let Some(steps) = body.as_mapping_get("steps") else {
        return Ok(());
    };
    steps
        .as_sequence()
        .map(|_| ())
        .ok_or(CompileError::StepFieldShape {
            step: index,
            field,
            expected: "a sequence",
        })
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
