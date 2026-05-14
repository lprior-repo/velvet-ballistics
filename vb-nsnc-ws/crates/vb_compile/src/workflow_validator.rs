#![forbid(unsafe_code)]
//! Workflow document shape validation.
//!
//! Validates YAML workflow documents against the Phase 0 schema.

mod workflow_validators;
mod workflow_trigger_validators;

pub use workflow_validators::{
    non_string_key_error, validate_public_name, validate_workflow_document_shape,
};
pub use workflow_trigger_validators::validate_workflow_trigger;
