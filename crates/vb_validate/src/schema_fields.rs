//! Field and document structure validation for schema validation.

use crate::{ValidationError, ValidationResult};
use crate::schema_doc::{FieldValue, StepDoc, WorkflowDoc};
use crate::schema_id::{is_reserved_id, is_valid_id, validate_single_id};

const CANONICAL_VERSION: &str = "velvet-ballastics/v1";
const REQUIRED_TOP_LEVEL_FIELDS: &[&str] = &["version", "name", "when", "steps"];
const ALLOWED_TOP_LEVEL_FIELDS: &[&str] = &[
    "version", "name", "when", "inputs", "vars", "secrets", "result", "examples", "steps",
];
const ALLOWED_STEP_FIELDS: &[&str] = &[
    "id", "name", "if", "with", "then", "set", "choose", "for_each", "together",
    "collect", "reduce", "repeat", "wait", "ask", "finish", "do", "on_error", "try_again",
];
const STEP_PRIMITIVES: &[&str] = &[
    "set", "do", "choose", "for_each", "together", "collect", "reduce", "repeat", "wait", "ask", "finish",
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
    if let Some(steps) = doc.get_sequence("steps") {
        for step in steps { validate_no_duplicate_names(&step.fields)?; }
    }
    Ok(())
}

fn validate_no_duplicate_names(fields: &[(String, FieldValue)]) -> ValidationResult<()> {
    let mut seen: Vec<&str> = Vec::with_capacity(fields.len());
    for (name, _) in fields {
        if seen.contains(&name.as_str()) { return Err(ValidationError::DuplicateKey); }
        seen.push(name.as_str());
    }
    Ok(())
}

pub fn validate_version(doc: &WorkflowDoc) -> ValidationResult<()> {
    match doc.get_string("version") {
        Some(v) if v == CANONICAL_VERSION => Ok(()),
        Some(v) => Err(ValidationError::InvalidVersion { version: v.to_owned() }),
        None => Err(ValidationError::MissingRequiredField { field: "version".to_owned() }),
    }
}

pub fn validate_trigger(doc: &WorkflowDoc) -> ValidationResult<()> {
    let trigger = doc.get_mapping("when").ok_or_else(|| ValidationError::MissingRequiredField { field: "when".to_owned() })?;
    if trigger.is_empty() { return Err(ValidationError::MissingRequiredField { field: "when".to_owned() }); }
    if trigger.len() > 1 { return Err(ValidationError::UnsupportedTrigger { trigger: "multiple triggers".to_owned() }); }
    let (kind, _body) = trigger.first().ok_or_else(|| ValidationError::MissingRequiredField { field: "when".to_owned() })?;
    match kind.as_str() {
        "manual" | "ipc" => Ok(()),
        "http" => Err(ValidationError::HttpTriggerOutOfCore),
        other => Err(ValidationError::UnsupportedTrigger { trigger: other.to_owned() }),
    }
}

pub fn validate_ids(doc: &WorkflowDoc) -> ValidationResult<()> {
    let name = doc.get_string("name").ok_or_else(|| ValidationError::MissingRequiredField { field: "name".to_owned() })?;
    validate_id("name", name)?;
    let steps = doc.get_sequence("steps").ok_or_else(|| ValidationError::MissingRequiredField { field: "steps".to_owned() })?;
    if steps.is_empty() { return Err(ValidationError::MissingRequiredField { field: "steps".to_owned() }); }
    let mut seen: Vec<&str> = Vec::with_capacity(steps.len());
    for step in steps {
        let id = step.get_string("id").ok_or_else(|| ValidationError::MissingRequiredField { field: "step id".to_owned() })?;
        validate_single_id(id, &seen)?;
        seen.push(id);
    }
    Ok(())
}

pub fn validate_step_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    let steps = doc.get_sequence("steps").ok_or_else(|| ValidationError::MissingRequiredField { field: "steps".to_owned() })?;
    for step in steps {
        validate_step_unknown_fields(step)?;
        validate_single_primitive(step)?;
    }
    Ok(())
}

fn validate_required_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    for field in REQUIRED_TOP_LEVEL_FIELDS {
        if !doc.has_field(field) { return Err(ValidationError::MissingRequiredField { field: (*field).to_owned() }); }
    }
    Ok(())
}

fn validate_unknown_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    for field in doc.field_names() {
        if !ALLOWED_TOP_LEVEL_FIELDS.contains(&field) { return Err(ValidationError::UnknownTopLevelField); }
    }
    Ok(())
}

fn validate_step_unknown_fields(step: &StepDoc) -> ValidationResult<()> {
    for field in step.field_names() {
        if !ALLOWED_STEP_FIELDS.contains(&field) { return Err(ValidationError::UnknownStepField); }
    }
    Ok(())
}

pub fn validate_single_primitive(step: &StepDoc) -> ValidationResult<()> {
    let mut count = 0_usize;
    for (field, _) in &step.fields {
        if STEP_PRIMITIVES.contains(&field.as_str()) { count = count.saturating_add(1); }
    }
    if count == 0 { return Err(ValidationError::MissingStepPrimitive); }
    if count > 1 { return Err(ValidationError::MultipleStepPrimitives); }
    Ok(())
}

fn validate_id(field: &str, id: &str) -> ValidationResult<()> {
    if !is_valid_id(id) { return Err(ValidationError::InvalidId { id: format!("{field}: {id}") }); }
    if is_reserved_id(id) { return Err(ValidationError::ReservedId { id: format!("{field}: {id}") }); }
    Ok(())
}
