use super::*;
use crate::mod_compile_errors::CompileError;
use crate::mod_compile_errors::non_string_key_error;
use saphyr::Yaml;

pub(super) fn validate_choose_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    let mapping = primitive_body_mapping(body, index, "choose")?;
    if is_canonical_choose_shape(mapping, index)? {
        return validate_canonical_choose_shape(body, index);
    }
    validate_legacy_choose_shape(body, index)
}

fn validate_legacy_choose_shape(body: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    reject_unknown_primitive_fields(body, index, "choose", &["condition", "on_true", "on_false"])?;
    required_step_field(body, index, "condition")?;
    required_branch_target(body, index, "on_true")?;
    required_branch_target(body, index, "on_false")?;
    Ok(())
}

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
        &["variable", "input", "at_once", "steps"],
    )?;
    required_primitive_string_field(body, index, "variable", "for_each.variable")?;
    required_primitive_string_field(body, index, "input", "for_each.input")?;
    validate_optional_u32_field(body, index, "for_each", "at_once")?;
    validate_optional_steps_sequence(body, index)?;
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
        &["variable", "source", "pages", "items", "steps"],
    )?;
    required_primitive_string_field(body, index, "variable", "collect.variable")?;
    required_primitive_string_field(body, index, "source", "collect.source")?;
    validate_optional_u32_field(body, index, "collect", "pages")?;
    validate_optional_u32_field(body, index, "collect", "items")?;
    validate_optional_steps_sequence(body, index)?;
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
        &["variable", "input", "initial", "steps"],
    )?;
    required_primitive_string_field(body, index, "variable", "reduce.variable")?;
    required_primitive_string_field(body, index, "input", "reduce.input")?;
    required_primitive_string_field(body, index, "initial", "reduce.initial")?;
    validate_optional_steps_sequence(body, index)?;
    Ok(())
}

pub(super) fn validate_repeat_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "repeat", &["max_attempts", "steps"])?;
    required_u16_field(body, index, "repeat", "max_attempts")?;
    validate_optional_steps_sequence(body, index)?;
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

fn validate_optional_u32_field(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    field: &'static str,
) -> Result<(), CompileError> {
    if body.as_mapping_get(field).is_some() {
        required_u32_field(body, step, primitive, field).map(|_| ())
    } else {
        Ok(())
    }
}

fn validate_optional_steps_sequence(body: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    let Some(node) = body.as_mapping_get("steps") else {
        return Ok(());
    };
    if node.as_sequence().is_some() {
        Ok(())
    } else {
        Err(CompileError::StepFieldShape {
            step,
            field: "steps",
            expected: "a sequence",
        })
    }
}

fn validate_together_branches(body: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    let branches = required_step_field(body, step, "branches")?
        .as_sequence()
        .ok_or(CompileError::StepFieldShape {
            step,
            field: "branches",
            expected: "a sequence",
        })?;
    for branch in branches {
        validate_together_branch(branch, step)?;
    }
    Ok(())
}

fn validate_together_branch(branch: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    let mapping = branch.as_mapping().ok_or(CompileError::StepFieldShape {
        step,
        field: "together.branches[]",
        expected: "a mapping",
    })?;
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            return Err(CompileError::StepShape { step });
        };
        if !matches!(field, "label" | "steps") {
            return Err(CompileError::UnknownStepPrimitiveField {
                step,
                primitive: "together",
                field: Box::<str>::from(field),
            });
        }
    }
    required_primitive_string_field(branch, step, "label", "together.branches[].label")?;
    validate_optional_steps_sequence(branch, step)
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

pub(super) fn validate_optional_top_level_shapes(doc: &Yaml<'_>) -> Result<(), CompileError> {
    optional_inputs_mapping(doc)?;
    optional_vars_mapping(doc)?;
    optional_secret_mapping(doc)?;
    optional_examples_sequence(doc)
}

pub(super) fn optional_inputs_mapping(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("inputs") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "inputs",
        expected: "a mapping",
    })?;
    for (key, _) in mapping {
        let Some(name) = key.as_str() else {
            return Err(non_string_key_error());
        };
        validate_public_name("inputs", name)?;
    }
    Ok(())
}

pub(super) fn optional_vars_mapping(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("vars") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "vars",
        expected: "a mapping",
    })?;
    for (key, value) in mapping {
        let Some(name) = key.as_str() else {
            return Err(non_string_key_error());
        };
        validate_public_name("vars", name)?;
        slot_value(value, 0)?;
    }
    Ok(())
}

pub(super) fn optional_secret_mapping(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("secrets") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "secrets",
        expected: "a mapping",
    })?;
    for (key, value) in mapping {
        let Some(name) = key.as_str() else {
            return Err(non_string_key_error());
        };
        validate_public_name("secrets", name)?;
        if value.as_str().is_none() {
            return Err(CompileError::FieldShape {
                field: "secrets",
                expected: "a mapping of secret names to environment variable names",
            });
        }
    }
    Ok(())
}

pub(super) fn optional_examples_sequence(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("examples") else {
        return Ok(());
    };
    let examples = node.as_sequence().ok_or(CompileError::FieldShape {
        field: "examples",
        expected: "a sequence",
    })?;
    for example in examples {
        if !example.is_mapping() {
            return Err(CompileError::FieldShape {
                field: "examples",
                expected: "a sequence of mappings",
            });
        }
        let name = required_example_name(example)?;
        validate_public_name("examples", name)?;
    }
    Ok(())
}

pub(super) fn required_example_name<'a>(example: &'a Yaml<'a>) -> Result<&'a str, CompileError> {
    let name = example
        .as_mapping_get("name")
        .ok_or(CompileError::MissingField {
            field: "examples.name",
        })?;
    name.as_str().ok_or(CompileError::FieldShape {
        field: "examples.name",
        expected: "a string",
    })
}
