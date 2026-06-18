//! Schema validation functions.
//!
//! Validates required/unknown fields, version strings, trigger declarations,
//! ID grammar rules, step field shapes, and single-primitive constraints.

use super::types::{FieldValue, STEP_PRIMITIVES, StepDoc, WorkflowDoc};
use crate::{ValidationError, ValidationResult};

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
pub(crate) fn validate_workflow_schema(doc: &WorkflowDoc) -> ValidationResult<()> {
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
pub(crate) fn validate_version(doc: &WorkflowDoc) -> ValidationResult<()> {
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

/// Validates the trigger (when) block accepts canonical v1 triggers and rejects HTTP.
pub(crate) fn validate_trigger(doc: &WorkflowDoc) -> ValidationResult<()> {
    let trigger = doc
        .get_mapping("when")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "when".to_owned(),
        })?;
    if trigger.is_empty() {
        return Err(ValidationError::MissingRequiredField {
            field: "when".to_owned(),
        });
    }
    if trigger.len() > 1 {
        return Err(ValidationError::UnsupportedTrigger {
            trigger: "multiple triggers".to_owned(),
        });
    }
    let (kind, body) = trigger
        .first()
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "when".to_owned(),
        })?;
    match kind.as_str() {
        "manual" | "webhook" => validate_empty_trigger(kind, body),
        "schedule" => validate_named_string_trigger(kind, body, "cron"),
        "event" => validate_named_string_trigger(kind, body, "type"),
        "http" => Err(ValidationError::HttpTriggerOutOfCore),
        other => Err(ValidationError::UnsupportedTrigger {
            trigger: other.to_owned(),
        }),
    }
}

fn validate_empty_trigger(kind: &str, body: &FieldValue) -> ValidationResult<()> {
    match body {
        FieldValue::Empty => Ok(()),
        FieldValue::Mapping(entries) if entries.is_empty() => Ok(()),
        _ => Err(ValidationError::UnsupportedTrigger {
            trigger: kind.to_owned(),
        }),
    }
}

fn validate_named_string_trigger(
    kind: &str,
    body: &FieldValue,
    required_field: &str,
) -> ValidationResult<()> {
    let FieldValue::Mapping(entries) = body else {
        return Err(ValidationError::UnsupportedTrigger {
            trigger: kind.to_owned(),
        });
    };
    let valid = entries.iter().any(|(field, value)| match value {
        FieldValue::String(text) => field == required_field && !text.is_empty(),
        _ => false,
    });
    if valid {
        Ok(())
    } else {
        Err(ValidationError::UnsupportedTrigger {
            trigger: kind.to_owned(),
        })
    }
}

/// Validates all step and top-level IDs against grammar, reserved words, and duplicates.
pub(crate) fn validate_ids(doc: &WorkflowDoc) -> ValidationResult<()> {
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
pub(crate) fn validate_step_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    let steps = doc
        .get_sequence("steps")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
        })?;
    for step in steps {
        validate_step_unknown_fields(step)?;
        validate_single_primitive(step)?;
        validate_step_body(step)?;
    }
    Ok(())
}

/// Validates that a step's primitive body is structurally sound.
fn validate_step_body(step: &StepDoc) -> ValidationResult<()> {
    let Some((primitive, value)) = step.primitive_value() else {
        return Ok(());
    };
    if *value == FieldValue::Empty {
        return Ok(());
    }
    match primitive {
        "choose" => {
            if let FieldValue::Mapping(entries) = value {
                let has_branches = entries.iter().any(|(k, _)| k == "branches");
                let has_default = entries.iter().any(|(k, _)| k == "default");
                if !has_branches && !has_default {
                    return Err(ValidationError::InvalidChoose);
                }
            }
        }
        "for_each" => {
            if let FieldValue::Mapping(entries) = value {
                let has_body = entries
                    .iter()
                    .any(|(k, v)| k == "for_each" && !matches!(v, FieldValue::Empty));
                if !has_body {
                    return Err(ValidationError::InvalidForEach);
                }
            }
        }
        "together" => {
            if let FieldValue::Mapping(entries) = value {
                let has_body = entries
                    .iter()
                    .any(|(k, v)| k == "together" && matches!(v, FieldValue::Sequence(_)));
                if !has_body {
                    return Err(ValidationError::InvalidTogether);
                }
            }
        }
        "collect" => {
            if let FieldValue::Mapping(entries) = value {
                let has_of = entries.iter().any(|(k, _)| k == "of");
                let has_reduce = entries.iter().any(|(k, _)| k == "reduce");
                if !has_of || !has_reduce {
                    return Err(ValidationError::InvalidCollect);
                }
            }
        }
        "reduce" => {
            if let FieldValue::Mapping(entries) = value {
                let has_body = entries
                    .iter()
                    .any(|(k, v)| k == "reduce" && !matches!(v, FieldValue::Empty));
                if !has_body {
                    return Err(ValidationError::InvalidReduce);
                }
            }
        }
        "repeat" => {
            if let FieldValue::Mapping(entries) = value {
                let has_body = entries
                    .iter()
                    .any(|(k, v)| k == "repeat" && !matches!(v, FieldValue::Empty));
                if !has_body {
                    return Err(ValidationError::InvalidRepeat);
                }
            }
        }
        "wait" => {
            if let FieldValue::Mapping(entries) = value
                && entries.is_empty()
            {
                return Err(ValidationError::InvalidWait);
            }
        }
        "ask" => {
            if let FieldValue::Mapping(entries) = value
                && entries.is_empty()
            {
                return Err(ValidationError::InvalidAsk);
            }
        }
        "finish" => {
            if let FieldValue::Mapping(entries) = value {
                let has_result = entries.iter().any(|(k, _)| k == "result");
                if !has_result {
                    return Err(ValidationError::InvalidFinish);
                }
            }
        }
        "retry" | "try_again" => {
            if let FieldValue::Mapping(entries) = value
                && entries.is_empty()
            {
                return Err(ValidationError::InvalidRetry);
            }
        }
        "on_error" => {
            if let FieldValue::Mapping(entries) = value {
                let has_body = entries
                    .iter()
                    .any(|(k, v)| k == "on_error" && !matches!(v, FieldValue::Empty));
                if !has_body {
                    return Err(ValidationError::InvalidOnError);
                }
            }
        }
        _ => {}
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

pub(crate) fn validate_id(field: &str, id: &str) -> ValidationResult<()> {
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

pub(crate) fn validate_single_id(id: &str, seen: &[&str]) -> ValidationResult<()> {
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

pub(crate) fn is_valid_id(id: &str) -> bool {
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

pub(crate) fn is_reserved_id(id: &str) -> bool {
    RESERVED_IDS.contains(&id)
}
