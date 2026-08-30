#![forbid(unsafe_code)]
//! Schema support modules for document model and validation helpers.
//!
//! These modules provide a lightweight document model for schema validation
//! and are used exclusively for testing the main schema module.
//!
//! Modules:
//! - `schema_doc` - Lightweight document model types (WorkflowDoc, StepDoc, FieldValue)
//! - `schema_id` - ID validation helpers
//! - `schema_fields` - Field and document structure validation
//! - `schema_tests` - Tests for schema validation

#[cfg(test)]
pub mod schema_doc;

#[cfg(test)]
pub mod schema_id;

#[cfg(test)]
pub mod schema_fields;

#[cfg(test)]
pub mod schema_tests;
