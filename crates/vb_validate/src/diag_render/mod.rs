#![forbid(unsafe_code)]
//! Diagnostic construction from validation errors.
//!
//! Converts [`ValidationError`](crate::ValidationError) variants into stable
//! [`Diagnostic`](vb_core::diagnostic::Diagnostic) records with error codes
//! matching the master contract (Section 16).
//!
//! This module provides the public API surface for diagnostic emission:
//! - [`diagnostic_from_error`] — full diagnostic record
//! - [`error_code`] — numeric code extraction only

#![allow(unreachable_pub)]

pub mod mapping;

mod fallback;
pub use fallback::diagnostic_fallback_symbolic;

mod construction;
pub use construction::{diagnostic_from_error, error_code};

#[cfg(test)]
mod tests;
