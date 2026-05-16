//! Schema validation for workflow documents.

pub mod doc;
pub mod validation;

pub use doc::{FieldValue, StepDoc, WorkflowDoc};
pub use validation::{
    validate_ids, validate_single_primitive, validate_step_fields, validate_trigger,
    validate_version, validate_workflow_schema,
};
