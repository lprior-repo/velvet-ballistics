#![forbid(unsafe_code)]
//! Diagnostic conversion for validation errors.
//!
//! Converts `ValidationError` variants into stable `Diagnostic` records with
//! error codes matching the master contract (Section 16).

mod mapping;
#[cfg(test)]
mod tests;

pub use mapping::{diagnostic_from_error, error_code};
