#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(unused_imports))]
//! Reference validation for workflow documents.
//!
//! Builds reference tables from declared inputs, vars, secrets, and step IDs,
//! then validates that all `$input.*`, `$vars.*`, `$secrets.*`, `$steps.*`,
//! direct `$step_id.*`, and loop-variable references resolve to declared names.
//! Rejects `$runtime.*`, `$now`, and `$random`.
//!
//! The [`RefTables`] type and [`validate_single_reference`] function are public
//! so that `vb_compile` can build tables from its AST and share the core
//! reference validation logic without duplication (DRIFT-5).

mod parse;
mod tables;
mod validate;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

// ---------------------------------------------------------------------------
// Re-exports (public API surface)
// ---------------------------------------------------------------------------

// Data model
pub use self::tables::{RefTables, WorkflowRefs};

// Validation entry points
pub use self::validate::{
    validate_references, validate_single_reference, validate_single_reference_in_on_error,
    validate_single_reference_in_repeat, validate_single_reference_with_context,
    validate_step_references,
};

// Parsing helpers (public for test access)
pub use self::parse::{OUTPUT_FIELD_SYMBOL, parse_step_reference};

// Re-export StepIdx for the validate module's public API
pub use vb_core::ids::StepIdx;

// Internal helper that tests reach via `super::validate_rooted_reference`.
pub(crate) use self::validate::validate_rooted_reference;
