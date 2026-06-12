#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! Velvet Ballistics is the CLI runtime for bead lifecycle management.

pub(crate) mod action_specs;
pub(crate) mod agent_context;
pub(crate) mod agent_io;
pub mod args;
pub mod bench;
pub(crate) mod bench_run;
pub mod cli_envelope;
pub mod cli_error;
pub mod cli_postcard;
pub mod commands;
pub mod commands_ai_context;
pub mod commands_diff;
pub mod commands_incident;
pub mod commands_journal;
pub mod commands_status;
pub mod commands_system_status;
pub mod commands_verify;
pub mod commands_workflow;
pub mod compile;
pub mod constants;
pub mod deliver_sink;
pub mod dispatcher;
pub mod doctor;
pub mod doctor_helpers;
pub mod events;
pub mod exit_code;
pub mod explain;
pub mod explain_compile;
pub mod explain_errors;
pub mod explain_plan;
pub mod explain_repair;
pub mod explain_reports;
pub mod explain_validation;
pub mod explain_validation2;
pub mod file_io;
pub mod graph;
pub mod incident_diff;
pub mod incident_ops;
pub mod inspect;
pub mod io;
pub mod io_helpers;
pub mod ipc_serve;
#[cfg(kani)]
pub mod kani_lifecycle;
pub mod lifecycle;
pub mod mode_error;
pub mod naming_scan;
pub mod output;
pub mod output_utils;
pub mod replay;
pub mod run;
pub mod run_compiled;
pub mod run_compiled_runtime;
pub mod run_compiled_runtime_trace;
pub mod run_ops;
pub mod run_step;
pub mod semantic_diff;
pub mod simulate;
pub mod status;
pub mod step_helpers;
pub mod storage;
pub mod storage_event_format;
pub mod submit;
pub mod trace;
pub mod validate;
pub mod verify;
#[cfg(verus)]
pub mod verus_lifecycle;
pub mod workflow;

pub use crate::dispatcher::run_from_env;

// Re-enabled modules (files verified present with content).
// Kept commented: run_resume, explain_validation3, explain_validation4 — files do not exist.

// Re-exports for convenience — items expected at crate root by many files.
pub(crate) use crate::args::{DiffMode, OutputFormat};
pub(crate) use crate::exit_code::CliExitCode;
pub(crate) use crate::file_io::write_failure_message;
pub(crate) use crate::output::OutputError;
pub(crate) use crate::output::write_contract_error_json;
pub(crate) use crate::output::{
    json_error, json_out, write_stderr_line, write_stdout_line, write_stdout_line_checked,
};
