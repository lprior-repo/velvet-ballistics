//! Velvet Ballastics CLI application implementation.
//!
//! Holzman Rust: thin imperative shell over functional core.
//! This module declares the extraction modules and re-exports public items.
#![forbid(unsafe_code)]

#![allow(clippy::too_many_arguments, clippy::too_many_lines)]
#![allow(clippy::match_single_binding, clippy::match_wildcard_for_single_variants)]

mod constants;
mod file_io;
mod agent_io;
mod action;
mod action_specs;
mod verify;
mod validate;
mod compile;
mod run;
mod submit;
mod run_step;
mod step_helpers;
mod run_compiled;
mod run_compiled_runtime;
mod ipc_serve;
mod inspect;
mod events;
mod replay;
mod trace;
mod run_ops;
mod incident_diff;
mod explain;
mod explain_reports;
mod explain_errors;
mod explain_repair;
mod explain_validation;
mod explain_validation2;
mod graph;
mod simulate;
mod bench_run;
mod doctor;
mod doctor_helpers;
mod io_helpers;
mod output_utils;
mod output;
mod dispatcher;

use std::io::Write;
use std::process::ExitCode;

use crate::output::{json_out, output_error_exit, write_stderr_line, write_stdout_line};
use crate::output_utils::cli_exit_code_name;

/// Macro for writing to stdout with trailing newline.};
}

/// Macro for writing to stderr with trailing newline.};
}

/// Macro for emitting JSON output or returning an error exit code.
    }};
}

pub(crate) use crate::dispatcher::run_from_env;

pub(crate) use crate::exit_code::CliExitCode;
pub(crate) use crate::args::{ActionRegistryMode, Command, DurabilityMode, OutputFormat, ParseError, StepTarget};
pub(crate) use crate::commands_ai_context::{RunStatus, redacted_slot_value, suggested_ai_commands};

pub(crate) use crate::output::{OutputError, json_out, write_stdout_line, write_stdout_line_checked};

#[path = "app_impl_tests.rs"]
mod tests;
