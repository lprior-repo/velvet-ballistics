#![forbid(unsafe_code)]
//! Validation functions for workflow schema.

use crate::{ValidationError, ValidationResult};
use super::doc::{FieldValue, StepDoc, WorkflowDoc};
use vb_core::span::Span;

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
    "parallel",
    "collect",
    "aggregate",
    "repeat",
    "wait",
    "ask",
    "finish",
    "do",
    "on_error",
    "try_again",
];

const STEP_PRIMITIVES: &[&str] = &[
    "set", "do", "choose", "for_each", "parallel", "collect", "aggregate", "repeat", "wait", "ask",
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
    "parallel",
    "collect",
    "aggregate",
    "repeat",
    "wait",
    "ask",
    "try_again",
    "on_error",
    "then",
    "finish",
];

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
            return Err(ValidationError::DuplicateKey { span: Span::ZERO });
        }
        seen.push(name.as_str());
    }
    Ok(())
}

pub fn validate_version(doc: &WorkflowDoc) -> ValidationResult<()> {
    match doc.get_string("version") {
        Some(version) if version == CANONICAL_VERSION => Ok(()),
        Some(version) => Err(ValidationError::InvalidVersion {
            version: version.to_owned(),
         span: Span::ZERO}),
        None => Err(ValidationError::MissingRequiredField {
            field: "version".to_owned(),
         span: Span::ZERO}),
    }
}

pub fn validate_trigger(doc: &WorkflowDoc) -> ValidationResult<()> {
    let trigger = doc
        .get_mapping("when")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "when".to_owned(),
         span: Span::ZERO})?;
    if trigger.is_empty() {
        return Err(ValidationError::MissingRequiredField {
            field: "when".to_owned(),
         span: Span::ZERO});
    }
    if trigger.len() > 1 {
        return Err(ValidationError::UnsupportedTrigger {
            trigger: "multiple triggers".to_owned(),
         span: Span::ZERO});
    }
    let (kind, body) = trigger
        .first()
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "when".to_owned(),
         span: Span::ZERO})?;
    match kind.as_str() {
        "manual" | "webhook" => validate_empty_trigger(kind, body),
        "schedule" => validate_named_string_trigger(kind, body, "cron"),
        "event" => validate_named_string_trigger(kind, body, "type"),
        "http" => Err(ValidationError::HttpTriggerOutOfCore { span: Span::ZERO }),
        other => Err(ValidationError::UnsupportedTrigger {
            trigger: other.to_owned(),
         span: Span::ZERO}),
    }
}

fn validate_empty_trigger(kind: &str, body: &FieldValue) -> ValidationResult<()> {
    match body {
        FieldValue::Empty => Ok(()),
        FieldValue::Mapping(entries) if entries.is_empty() => Ok(()),
        _ => Err(ValidationError::UnsupportedTrigger {
            trigger: kind.to_owned(),
         span: Span::ZERO}),
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
         span: Span::ZERO});
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
         span: Span::ZERO})
    }
}

pub fn validate_ids(doc: &WorkflowDoc) -> ValidationResult<()> {
    let name = doc
        .get_string("name")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "name".to_owned(),
         span: Span::ZERO})?;
    validate_id("name", name)?;
    let mut seen: Vec<&str> = vec![name];
    let Some(steps) = doc.get_sequence("steps") else {
        return Ok(());
    };
    for (index, step) in steps.iter().enumerate() {
        if let Some(id) = step.get_string("id") {
            validate_single_id(id, &seen)?;
            seen.push(id);
        } else {
            return Err(ValidationError::MissingRequiredField {
                field: format!("steps[{index}].id"),
             span: Span::ZERO});
        }
    }
    Ok(())
}

fn validate_id(field: &str, id: &str) -> ValidationResult<()> {
    if !is_valid_id(id) {
        return Err(ValidationError::InvalidId {
            id: format!("{field}: {id}"),
         span: Span::ZERO});
    }
    if is_reserved_id(id) {
        return Err(ValidationError::ReservedId {
            id: format!("{field}: {id}"),
         span: Span::ZERO});
    }
    Ok(())
}

fn validate_single_id(id: &str, seen: &[&str]) -> ValidationResult<()> {
    if !is_valid_id(id) {
        return Err(ValidationError::InvalidId { id: id.to_owned() , span: Span::ZERO});
    }
    if is_reserved_id(id) {
        return Err(ValidationError::ReservedId { id: id.to_owned() , span: Span::ZERO});
    }
    if seen.contains(&id) {
        return Err(ValidationError::DuplicateId { id: id.to_owned() , span: Span::ZERO});
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

pub fn validate_step_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    let steps = doc
        .get_sequence("steps")
        .ok_or_else(|| ValidationError::MissingRequiredField {
            field: "steps".to_owned(),
         span: Span::ZERO})?;
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
             span: Span::ZERO});
        }
    }
    Ok(())
}

fn validate_unknown_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    for field in doc.field_names() {
        if !ALLOWED_TOP_LEVEL_FIELDS.contains(&field) {
            return Err(ValidationError::UnknownTopLevelField { span: Span::ZERO });
        }
    }
    Ok(())
}

fn validate_step_unknown_fields(step: &StepDoc) -> ValidationResult<()> {
    for field in step.field_names() {
        if !ALLOWED_STEP_FIELDS.contains(&field) {
            return Err(ValidationError::UnknownStepField { span: Span::ZERO });
        }
    }
    Ok(())
}

pub fn validate_single_primitive(step: &StepDoc) -> ValidationResult<()> {
    let mut count = 0_usize;
    for (field, _) in &step.fields {
        if STEP_PRIMITIVES.contains(&field.as_str()) {
            count = count.saturating_add(1);
        }
    }
    if count == 0 {
        return Err(ValidationError::MissingStepPrimitive { span: Span::ZERO });
    }
    if count > 1 {
        return Err(ValidationError::MultipleStepPrimitives { span: Span::ZERO });
    }
    Ok(())
}
