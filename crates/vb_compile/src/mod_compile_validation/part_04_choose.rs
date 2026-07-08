use super::*;
use crate::mod_compile_errors::CompileError;
use saphyr::Yaml;

pub(super) fn validate_choose_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    let has_canonical = has_any_field(body, &["branches", "otherwise"]);
    let has_legacy = has_any_field(body, &["condition", "on_true", "on_false"]);
    match (has_canonical, has_legacy) {
        (true, true) => Err(CompileError::StepFieldShape {
            step: index,
            field: "choose",
            expected: "either branches/otherwise or condition/on_true/on_false",
        }),
        (true, false) => validate_canonical_choose_shape(body, index),
        (false, _) => validate_legacy_choose_shape(body, index),
    }
}

fn has_any_field(body: &Yaml<'_>, fields: &[&str]) -> bool {
    for field in fields {
        if body.as_mapping_get(field).is_some() {
            return true;
        }
    }
    false
}

fn validate_legacy_choose_shape(body: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    reject_unknown_primitive_fields(body, index, "choose", &["condition", "on_true", "on_false"])?;
    required_step_field(body, index, "condition")?;
    required_branch_target(body, index, "on_true")?;
    required_branch_target(body, index, "on_false")?;
    Ok(())
}

fn validate_canonical_choose_shape(body: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    reject_unknown_primitive_fields(body, index, "choose", &["branches", "otherwise"])?;
    validate_choose_otherwise(body, index)?;
    validate_choose_branches(body, index)
}

fn validate_choose_otherwise(body: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    let Some(otherwise) = body.as_mapping_get("otherwise") else {
        return Ok(());
    };
    match otherwise.as_str() {
        Some(label) if !label.is_empty() => Ok(()),
        _ => Err(CompileError::StepFieldShape {
            step: index,
            field: "choose.otherwise",
            expected: "a non-empty string",
        }),
    }
}

fn validate_choose_branches(body: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    let Some(branches) = body.as_mapping_get("branches") else {
        return Ok(());
    };
    let sequence = branches.as_sequence().ok_or(CompileError::StepFieldShape {
        step: index,
        field: "choose.branches",
        expected: "a sequence",
    })?;
    for branch in sequence {
        validate_choose_branch(branch, index)?;
    }
    Ok(())
}

fn validate_choose_branch(branch: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    let mapping = branch.as_mapping().ok_or(CompileError::StepFieldShape {
        step: index,
        field: "choose.branches[]",
        expected: "a mapping",
    })?;
    for (key, _) in mapping {
        reject_unknown_primitive_field(key, index, "choose", &["when", "steps"])?;
    }
    validate_choose_branch_when(branch, index)?;
    validate_choose_branch_steps(branch, index)
}

fn validate_choose_branch_when(branch: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    let node = branch
        .as_mapping_get("when")
        .ok_or(CompileError::MissingStepField {
            step: index,
            field: "choose.branches[].when",
        })?;
    match node.as_str() {
        Some(value) if !value.is_empty() => Ok(()),
        _ => Err(CompileError::StepFieldShape {
            step: index,
            field: "choose.branches[].when",
            expected: "a non-empty string",
        }),
    }
}

fn validate_choose_branch_steps(branch: &Yaml<'_>, index: usize) -> Result<(), CompileError> {
    let Some(steps) = branch.as_mapping_get("steps") else {
        return Ok(());
    };
    steps
        .as_sequence()
        .map(|_| ())
        .ok_or(CompileError::StepFieldShape {
            step: index,
            field: "choose.branches[].steps",
            expected: "a sequence",
        })
}
