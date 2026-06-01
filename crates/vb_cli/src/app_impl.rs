//! Velvet Ballastics CLI application implementation.
//!
//! Holzman Rust: thin imperative shell over functional core.
//! Each submodule is bounded to 300 lines per architectural drift policy.
//!
#![forbid(unsafe_code)]

#![allow(clippy::too_many_arguments, clippy::too_many_lines)]
#![allow(clippy::match_single_binding, clippy::match_wildcard_for_single_variants)]

// Submodules removed - were never created (architectural drift cleanup)

use std::process::ExitCode;

// Re-exports for binary and tests - items that exist in the crate
pub(crate) use crate::exit_code::CliExitCode;
pub(crate) use crate::args::{ActionRegistryMode, Command, DurabilityMode, OutputFormat, ParseError, StepTarget};
pub(crate) use crate::commands_ai_context::{RunStatus, redacted_slot_value, suggested_ai_commands};

// Module stubs re-exported for binary compatibility
pub mod explain_repair {
    // Stub module - implementation pending
}

pub(crate) fn run_from_env() -> ExitCode {
    // STUB: Full CLI implementation requires submodules that were never created.
    // This stub allows the binary to compile while the full implementation
    // is pending submodule creation.
    eprintln!("error: CLI implementation incomplete (submodules not created)");
    ExitCode::from(1)
}
