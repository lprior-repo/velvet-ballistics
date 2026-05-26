#![forbid(unsafe_code)]
//! Velvet Ballastics is the CLI runtime for bead lifecycle management.

pub(crate) mod agent_context;
pub mod commands_diff;
pub mod commands_incident;
pub mod lifecycle;

pub mod status;
