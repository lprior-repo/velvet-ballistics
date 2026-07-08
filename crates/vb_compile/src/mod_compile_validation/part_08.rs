use super::*;
use crate::mod_compile_errors::CompileError;
use saphyr::Yaml;

pub(crate) fn is_canonical_choose_shape(
    mapping: &saphyr::Mapping<'_>,
    step: usize,
) -> Result<bool, CompileError> {
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            return Err(CompileError::StepShape { step });
        };
        if matches!(field, "branches" | "otherwise") {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn validate_canonical_choose_shape(
    body: &Yaml<'_>,
    index: usize,
) -> Result<(), CompileError> {
    reject_unknown_primitive_fields(body, index, "choose", &["branches", "otherwise"])?;
    validate_optional_primitive_string_field(body, index, "otherwise", "a string step id")?;
    if let Some(branches) = body.as_mapping_get("branches") {
        validate_canonical_choose_branches(branches, index)?;
    }
    Ok(())
}

fn validate_canonical_choose_branches(node: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    let branches = node.as_sequence().ok_or(CompileError::StepFieldShape {
        step: index,
        field: "choose.branches",
        expected: "a sequence",
    })?;
    if branches.len() > 64 {
        return Err(CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "choose",
            field: "branches",
            value: branches.len(),
            limit: 64,
        });
    }
    for branch in branches {
        validate_canonical_choose_branch(branch, index)?;
    }
    Ok(())
}

fn validate_canonical_choose_branch(branch: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    let mapping = branch.as_mapping().ok_or(CompileError::StepFieldShape {
        step: index,
        field: "choose.branches[]",
        expected: "a mapping",
    })?;
    for (key, _) in mapping {
        reject_unknown_choose_branch_field(key, index)?;
    }
    required_primitive_string_field(branch, index, "when", "choose.branches[].when")?;
    if let Some(steps) = branch.as_mapping_get("steps") {
        validate_choose_body_steps(steps, index)?;
    }
    Ok(())
}

fn reject_unknown_choose_branch_field(key: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    let Some(field) = key.as_str() else {
        return Err(CompileError::StepShape { step });
    };
    if matches!(field, "when" | "steps") {
        Ok(())
    } else {
        Err(CompileError::UnknownStepPrimitiveField {
            step,
            primitive: "choose",
            field: Box::<str>::from(field),
        })
    }
}

fn validate_choose_body_steps(node: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    let steps = node.as_sequence().ok_or(CompileError::StepFieldShape {
        step: index,
        field: "choose.branches[].steps",
        expected: "a sequence",
    })?;
    for step in steps {
        validate_choose_body_step(step, index)?;
    }
    Ok(())
}

fn validate_choose_body_step(step: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    let id = required_step_id(step, index)?;
    validate_public_name("step id", id)?;
    let StepSpec { primitive, body } = step_spec(step, index)?;
    match primitive {
        StepPrimitive::Set | StepPrimitive::Save => {
            validate_choose_body_set(body, index, primitive.as_str())
        }
        StepPrimitive::Run | StepPrimitive::Do => {
            validate_choose_body_do(body, index, primitive.as_str())
        }
        other => Err(CompileError::UnsupportedStepPrimitive {
            step: index,
            primitive: other.as_str(),
        }),
    }
}

fn validate_choose_body_set(
    body: &Yaml<'_>,
    index: usize,
    primitive: &'static str,
) -> Result<(), CompileError> {
    reject_unknown_primitive_fields(body, index, primitive, &["output", "value"])?;
    let output_field = primitive_output_field(primitive);
    let output = required_primitive_string_field(body, index, "output", output_field)?;
    if primitive == "save" && output.is_empty() {
        return Err(CompileError::StepFieldShape {
            step: index,
            field: output_field,
            expected: "non-empty string",
        });
    }
    let value_field = primitive_value_field(primitive);
    required_primitive_string_field(body, index, "value", value_field)?;
    Ok(())
}

fn validate_choose_body_do(
    body: &Yaml<'_>,
    index: usize,
    primitive: &'static str,
) -> Result<(), CompileError> {
    reject_unknown_primitive_fields(body, index, primitive, &["action", "input"])?;
    let action_field = primitive_action_field(primitive);
    let action = required_primitive_string_field(body, index, "action", action_field)?;
    if primitive == "run" && action.is_empty() {
        return Err(CompileError::StepFieldShape {
            step: index,
            field: action_field,
            expected: "non-empty string",
        });
    }
    let input_field = primitive_input_field(primitive);
    required_primitive_string_field(body, index, "input", input_field)?;
    Ok(())
}

fn primitive_output_field(primitive: &'static str) -> &'static str {
    if primitive == "save" {
        "save.output"
    } else {
        "set.output"
    }
}

fn primitive_value_field(primitive: &'static str) -> &'static str {
    if primitive == "save" {
        "save.value"
    } else {
        "set.value"
    }
}

fn primitive_action_field(primitive: &'static str) -> &'static str {
    if primitive == "run" {
        "run.action"
    } else {
        "do.action"
    }
}

fn primitive_input_field(primitive: &'static str) -> &'static str {
    if primitive == "run" {
        "run.input"
    } else {
        "do.input"
    }
}

pub(crate) fn validate_optional_primitive_string_field(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
    expected: &'static str,
) -> Result<bool, CompileError> {
    let Some(node) = body.as_mapping_get(field) else {
        return Ok(false);
    };
    if node.as_str().is_some() {
        Ok(true)
    } else {
        Err(CompileError::StepFieldShape {
            step,
            field,
            expected,
        })
    }
}

pub(crate) fn required_primitive_string_field<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    field: &'static str,
    diagnostic_field: &'static str,
) -> Result<&'a str, CompileError> {
    let node = required_step_field(body, step, field)?;
    node.as_str().ok_or(CompileError::StepFieldShape {
        step,
        field: diagnostic_field,
        expected: "a string",
    })
}
