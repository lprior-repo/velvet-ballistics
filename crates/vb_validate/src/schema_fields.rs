//! Field-level validation helpers for schema validation.

use crate::{ValidationError, ValidationResult, schema_types::FieldValue};

use crate::schema_constants::ALLOWED_STEP_FIELDS;
use crate::schema_types::{StepDoc, WorkflowDoc};

/// Validates no duplicate fields exist at the top level or within steps.
pub fn validate_duplicate_fields(doc: &WorkflowDoc) -> ValidationResult<()> {
    validate_no_duplicate_names(&doc.fields)?;
    let Some(steps) = doc.get_sequence("steps") else {
        return Ok(());
    };
    for step in steps {
        validate_no_duplicate_names(&step.fields)?;
    }
    Ok(())
}

/// Checks for duplicate field names within a single field list.
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

/// Validates that a step has no unknown fields.
pub fn validate_step_unknown_fields(step: &StepDoc) -> ValidationResult<()> {
    for field in step.field_names() {
        if !ALLOWED_STEP_FIELDS.contains(&field) {
            return Err(ValidationError::UnknownStepField);
        }
    }
    Ok(())
}
