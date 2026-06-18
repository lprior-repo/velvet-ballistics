//! Reference validation for compiled workflow ASTs.
//!
//! Delegates core reference validation to `vb_validate::references` to avoid
//! duplicating validation logic. Handles compile-specific references (slot
//! accessors) locally since those are not part of the standalone validator's
//! surface.

mod errors;
mod tables;
mod validate;
mod walk;

pub(crate) use walk::validate_workflow_ast;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
