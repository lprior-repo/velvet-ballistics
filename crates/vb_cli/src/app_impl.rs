//! Velvet Ballastics CLI application implementation.
//!
//! Holzman Rust: thin imperative shell over functional core.
//! This module declares the extraction modules and re-exports public items.
#![forbid(unsafe_code)]

#![allow(clippy::too_many_arguments, clippy::too_many_lines)]
#![allow(clippy::match_single_binding, clippy::match_wildcard_for_single_variants)]

use std::io::Write;
use std::process::ExitCode;

use crate::output::{json_out, output_error_exit, write_stderr_line, write_stdout_line};
use crate::output_utils::cli_exit_code_name;

pub(crate) use crate::dispatcher::run_from_env;

pub(crate) use crate::exit_code::CliExitCode;
pub(crate) use crate::args::{ActionRegistryMode, Command, DurabilityMode, OutputFormat, ParseError, StepTarget};
pub(crate) use crate::commands_ai_context::{RunStatus, redacted_slot_value, suggested_ai_commands};

pub(crate) use crate::output::{OutputError, json_out, write_stdout_line, write_stdout_line_checked};

#[path = "app_impl_tests.rs"]
mod tests;
