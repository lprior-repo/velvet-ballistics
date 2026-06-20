#![forbid(unsafe_code)]
//! Type and taint validation for workflow documents.
//!
//! Tracks input/action/result types through workflow steps, propagates secret
//! taint facts, and validates resource contract bounds against protocol hard
//! limits. `Finish` results carrying `Secret` or `DerivedFromSecret` taint
//! produce `SECRET_RESULT_LEAK` when `allows_secret_results` is `false`.
//!
//! # Module Layout
//!
//! - [`types`] — Value types, taint lattice, and value facts.
//! - model — Workflow input model (decls, limits, steps).
//! - limits — Resource contract bound checking.
//! - facts — Fact table construction and reference resolution.
//! - `step` — Step-level type and taint validation.

pub mod types;

mod facts;
mod limits;
mod model;
mod step;

// ---------------------------------------------------------------------------
// Public re-exports (type_taint API surface)
// ---------------------------------------------------------------------------

pub use self::limits::validate_resource_limits;
pub use self::model::*;
pub use self::step::{validate_taint, validate_types};
pub use self::types::{Taint, ValueFact, ValueType};

#[cfg(test)]
#[path = "type_taint_tests.rs"]
mod tests;
