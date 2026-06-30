#![forbid(unsafe_code)]
//! Velvet Ballistics is the CLI runtime for bead lifecycle management.

pub(crate) mod agent_context;
pub mod cli_postcard;
pub mod commands_diff;
pub mod commands_incident;
pub mod lifecycle;
pub mod naming_scan;
pub mod status;

// The remaining CLI implementation modules are binary-only because they depend
// on local output macros, process exit handling, and argument parsing types.
