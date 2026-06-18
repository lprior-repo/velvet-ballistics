//! Runtime status command output.
//!
//! This module provides:
//! - `build_status` — assembles a `CliStatus` snapshot from a transient shard
//!   with optional journal overlay
//! - `print_status` — renders the snapshot as text, YAML, or JSON

#![forbid(unsafe_code)]

mod build;
mod types;

#[cfg(test)]
mod tests;

// Re-export public API at module level.
pub(crate) use build::build_status;
pub(crate) use types::CliStatus;
pub(crate) use types::DbProbeStatus;

// `print_status` is used internally by the dispatcher; it is re-exported
// here for the same visibility the original flat module provided.
pub(crate) use output::print_status;

mod output;
