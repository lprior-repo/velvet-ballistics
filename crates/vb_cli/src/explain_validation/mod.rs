#![forbid(unsafe_code)]
//! Validation error explanation and failure formatting.
//!
//! Splits the original monolithic `explain_validation.rs` into two
//! domain-scoped sub-modules so callers import exactly the error
//! explanation path they need.

pub mod validation;
pub mod verification;

pub(crate) use validation::explain_validation_error;
pub(crate) use verification::explain_verification_failure;
