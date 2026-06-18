//! Verify command module — CLI-layer static-analysis orchestration.
//!
//! This module owns the CLI command functions that drive verification:
//! reading workflow files, invoking the pipeline, and emitting results.
//!
//! ## Module layout
//!
//! - [`command`] — entry points (`cmd_verify`, `cmd_verify_with_durability`)
//! - [`report`] — JSON report builders and completion messages
//! - [`error`] — error emission, formatting, and exit-code helpers
//! - [`output`] — machine-readable stdout helpers
//! - [`io`] — workflow file reading
//!
//! ## Re-exports
//!
//! The following items are re-exported at the module root for convenience:
//! - [`command::cmd_verify`]
//! - [`command::cmd_verify_with_durability`]
//! - [`command::uses_verify_human_text`]
//! - [`error::deferred_gate_message`]
//! - [`error::verify_error_message`]
//! - [`error::cli_exit_code_number`]
//! - [`report::verify_success_report`]
//! - [`report::verify_deferred_report`]
//! - [`report::durability_block`]
//! - [`report::verification_completion_message`]

#![forbid(unsafe_code)]

mod command;
mod error;
mod io;
mod output;
mod report;

#[cfg(test)]
mod tests;

// Re-export the public command entry points.
pub(crate) use command::{cmd_verify, cmd_verify_with_durability, uses_verify_human_text};

// Re-export error formatting helpers used by other CLI modules.
pub(crate) use error::{cli_exit_code_number, deferred_gate_message, verify_error_message};

// Re-export report builders used by the explain command.
pub(crate) use report::{
    durability_block, verification_completion_message, verify_deferred_report,
    verify_success_report,
};
