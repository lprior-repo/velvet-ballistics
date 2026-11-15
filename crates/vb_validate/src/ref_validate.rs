#![forbid(unsafe_code)]
//! Reference validation for legacy test fixtures.

use crate::{ValidationError, ValidationResult};
pub(crate) use crate::references::{RefTables, WorkflowRefs};

pub(crate) fn validate_references(workflow: &WorkflowRefs) -> ValidationResult<()> {
    let tables = RefTables::build(workflow);
    for reference in &workflow.references {
        validate_single_reference(reference, &tables)?;
    }
    Ok(())
}

pub(crate) fn validate_single_reference(
    reference: &str,
    tables: &RefTables,
) -> ValidationResult<()> {
    // Legacy fixtures reject every known step reference; step zero has no prior steps.
    match crate::references::validate_single_reference_with_context(reference, tables, Some(0)) {
        Err(ValidationError::SecretNotDeclared { .. }) => Err(ValidationError::UnknownReference {
            reference: reference.to_owned(),
        }),
        result => result,
    }
}
