#![forbid(unsafe_code)]
//! Diagnostic modules for error code constants and rendering.
//!
//! Public modules:
//! - `codes` - Stable diagnostic error codes (Section 16 of the master contract)
//! - `render` - Diagnostic rendering for validation errors
//!
//! Test-only modules:
//! - `convert` - Diagnostic collection and test helpers
//! - `tests` - BDD exact-assertion tests for diagnostic conversion

pub mod diag_codes;
pub mod diag_render;

#[cfg(test)]
mod diag_convert;

#[cfg(test)]
mod diag_tests;
