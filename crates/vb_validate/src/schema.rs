#![forbid(unsafe_code)]
//! Schema validation for workflow documents.
//!
//! Validates required/unknown fields, version strings, trigger declarations,
//! ID grammar rules, step field shapes, and single-primitive constraints.

use crate::{ValidationError, ValidationResult};

mod trigger;
mod types;

pub use trigger::validate_trigger;
pub use types::{FieldValue, StepDoc, WorkflowDoc};

const CANONICAL_VERSION: &str = "velvet-ballistics/v1";

const REQUIRED_TOP_LEVEL_FIELDS: &[&str] = &["version", "name", "when", "steps"];

const ALLOWED_TOP_LEVEL_FIELDS: &[&str] = &[
    "version", "name", "when", "inputs", "vars", "secrets", "result", "examples", "steps",
];

const ALLOWED_STEP_FIELDS: &[&str] = &[
    "id",
    "name",
    "if",
    "with",
    "then",
    "set",
    "choose",
    "for_each",
    "together",
    "collect",
    "reduce",
    "repeat",
    "wait",
    "ask",
    "finish",
    "do",
    "on_error",
    "try_again",
];

const STEP_PRIMITIVES: &[&str] = &[
    "set", "do", "choose", "for_each", "together", "collect", "reduce", "repeat", "wait", "ask",
    "finish",
];

const RESERVED_IDS: &[&str] = &[
    "now",
    "random",
    "runtime",
    "null",
    "true",
    "false",
    "input",
    "inputs",
    "vars",
    "secrets",
    "steps",
    "error",
    "attempt",
    "total",
    "result",
    "when",
    "item",
    "do",
    "set",
    "choose",
    "for_each",
    "collect",
    "repeat",
    "wait",
    "ask",
    "try_again",
    "on_error",
    "then",
    "finish",
];

/// Validates a workflow document against the v1 schema.
///
/// Checks required fields, unknown fields, version, trigger, IDs,
/// step fields, and single-primitive rules.
pub fn validate_workflow_schema(doc: &WorkflowDoc) -> ValidationResult<()> {
    validate_duplicate_fields(doc)?;
    validate_required_fields(doc)?;
    validate_unknown_fields(doc)?;
    validate_version(doc)?;
    validate_trigger(doc)?;
    validate_ids(doc)?;
    validate_step_fields(doc)?;
    Ok(())
}

fn validate_duplicate_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    validate_no_duplicate_names(&doc.fields)?;
    let Some(steps) = doc.get_sequence("steps") else {
        return Ok(());
    };
    for step in steps {
        validate_no_duplicate_names(&step.fields)?;
    }
    Ok(())
}

fn validate_no_duplicate_names(fields: &[(String, FieldValue)]) -> ValidationResult<()> {
    let mut seen: Vec<&str> = Vec::with_capacity(fields.len());
    for (name, _) in fields {
        if seen.contains(&name.as_str()) {
            return Err(ValidationError::DuplicateKey);
        }
        seen.push(name.as_str());
    }
    Ok(())
}

/// Checks that the version field matches the canonical version string.
pub fn validate_version(doc: &WorkflowDoc) -> ValidationResult<()> {
    match doc.get_string("version") {
        Some(version) if version == CANONICAL_VERSION => Ok(()),
        Some(version) => Err(ValidationError::InvalidVersion {
            version: version.to_owned(),
        }),
        None => Err(ValidationError::MissingRequiredField {
            field: "version".to_owned(),
        }),
    }
}

/// Validates all step and top-level IDs against grammar, reserved words, and duplicates.
pub fn validate_ids(doc: &WorkflowDoc) -> ValidationResult<()> {
    let name = doc
        .get_string("name")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "name".to_owned(),
        })?;
    validate_id("name", name)?;
    let steps = doc
        .get_sequence("steps")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
        })?;
    if steps.is_empty() {
        return Err(ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
        });
    }
    let mut seen: Vec<&str> = Vec::with_capacity(steps.len());
    for step in steps {
        let id = step
            .get_string("id")
            .ok_or_else(|| ValidationError::MissingRequiredField {
                field: "step id".to_owned(),
            })?;
        validate_single_id(id, &seen)?;
        seen.push(id);
    }
    Ok(())
}

/// Validates step field shapes and the single-primitive constraint.
pub fn validate_step_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    let steps = doc
        .get_sequence("steps")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
        })?;
    for step in steps {
        validate_step_unknown_fields(step)?;
        validate_single_primitive(step)?;
    }
    Ok(())
}

fn validate_required_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    for field in REQUIRED_TOP_LEVEL_FIELDS {
        if !doc.has_field(field) {
            return Err(ValidationError::MissingRequiredField {
                field: (*field).to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_unknown_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    for field in doc.field_names() {
        if !ALLOWED_TOP_LEVEL_FIELDS.contains(&field) {
            return Err(ValidationError::UnknownTopLevelField);
        }
    }
    Ok(())
}

fn validate_step_unknown_fields(step: &StepDoc) -> ValidationResult<()> {
    for field in step.field_names() {
        if !ALLOWED_STEP_FIELDS.contains(&field) {
            return Err(ValidationError::UnknownStepField);
        }
    }
    Ok(())
}

/// Ensures exactly one primitive field per step.
pub fn validate_single_primitive(step: &StepDoc) -> ValidationResult<()> {
    let mut count = 0_usize;
    for (field, _) in &step.fields {
        if STEP_PRIMITIVES.contains(&field.as_str()) {
            count = count.saturating_add(1);
        }
    }
    if count == 0 {
        return Err(ValidationError::MissingStepPrimitive);
    }
    if count > 1 {
        return Err(ValidationError::MultipleStepPrimitives);
    }
    Ok(())
}

fn validate_id(field: &str, id: &str) -> ValidationResult<()> {
    if !is_valid_id(id) {
        return Err(ValidationError::InvalidId {
            id: format!("{field}: {id}"),
        });
    }
    if is_reserved_id(id) {
        return Err(ValidationError::ReservedId {
            id: format!("{field}: {id}"),
        });
    }
    Ok(())
}

fn validate_single_id(id: &str, seen: &[&str]) -> ValidationResult<()> {
    if !is_valid_id(id) {
        return Err(ValidationError::InvalidId { id: id.to_owned() });
    }
    if is_reserved_id(id) {
        return Err(ValidationError::ReservedId { id: id.to_owned() });
    }
    if seen.contains(&id) {
        return Err(ValidationError::DuplicateId { id: id.to_owned() });
    }
    Ok(())
}

fn is_valid_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 64 {
        return false;
    }
    let first = id.as_bytes().first();
    let Some(&byte) = first else {
        return false;
    };
    if !byte.is_ascii_lowercase() {
        return false;
    }
    for byte in id.as_bytes() {
        if !byte.is_ascii_lowercase() && !byte.is_ascii_digit() && *byte != b'_' {
            return false;
        }
    }
    true
}

fn is_reserved_id(id: &str) -> bool {
    RESERVED_IDS.contains(&id)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "schema/tests.rs"]
mod tests;
