//! Schema validation for workflow documents.
//!
//! This module provides cold-path validation of YAML workflow documents
//! against the velvet-ballastics/v1 schema.

// Re-exports for backwards compatibility - modules are declared in lib.rs
pub use crate::schema_constants::{is_reserved_id, is_valid_id, validate_id};
pub use crate::schema_types::{FieldValue, StepDoc, WorkflowDoc};
pub use crate::schema_validate::{
    validate_ids, validate_single_id, validate_single_primitive, validate_step_fields,
    validate_trigger, validate_version, validate_workflow_schema,
};
