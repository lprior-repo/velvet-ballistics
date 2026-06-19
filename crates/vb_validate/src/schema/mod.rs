#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code, unused_imports))]
//! Schema validation for workflow documents.
//!
//! Validates required/unknown fields, version strings, trigger declarations,
//! ID grammar rules, step field shapes, and single-primitive constraints.

pub mod types;
pub mod validation;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use types::{FieldValue, StepDoc, WorkflowDoc};
pub use validation::{
    is_reserved_id, is_valid_id, validate_id, validate_ids, validate_single_id,
    validate_single_primitive, validate_step_fields, validate_trigger, validate_version,
    validate_workflow_schema,
};
