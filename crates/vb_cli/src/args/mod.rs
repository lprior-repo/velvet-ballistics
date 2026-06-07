//! Argument parsing for velvet_ballistics.
#![forbid(unsafe_code)]

pub(crate) mod action;
pub(crate) mod error;
pub(crate) mod flag_spec;
pub(crate) mod other;
pub(crate) mod run_ops;
pub(crate) mod shared;
pub(crate) mod status;
pub(crate) mod trace;
pub(crate) mod types;
pub(crate) mod workflow;

mod tests;

// Re-export public types used throughout the crate
pub(crate) use error::ParseError;
pub(crate) use shared::parse_args;
pub(crate) use types::{
    ActionRegistryMode, Command, DiffMode, DurabilityMode, EmitTarget, EventStatus, OutputFormat,
    StatusOptions, StepTarget, SystemStatusOptions, VALID_COMMANDS, VerifyProfile,
};
pub(crate) use vb_core::action::ActionName;
