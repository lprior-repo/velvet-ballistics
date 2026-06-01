//! Velvet Ballastics binary entrypoint.
#![forbid(unsafe_code)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]

mod action;
mod action_specs;
mod agent_context;
mod agent_io;
mod app_impl;
mod args;
mod bench_run;
mod cli_envelope;
mod cli_postcard;
mod commands_ai_context;
mod commands_diff;
mod commands_incident;
mod commands_journal;
mod commands_status;
mod commands_system_status;
mod commands_verify;
mod commands_workflow;
mod compile;
mod constants;
mod deliver_sink;
mod dispatcher;
mod doctor;
mod doctor_helpers;
mod events;
mod exit_code;
mod explain;
mod explain_errors;
mod explain_reports;
mod explain_repair;
mod explain_validation;
mod explain_validation2;
mod file_io;
mod graph;
mod incident_diff;
mod inspect;
mod io_helpers;
mod ipc_serve;
#[cfg(test)]
mod mode_error;
mod output;
mod output_utils;
mod replay;
mod run;
mod run_compiled;
mod run_compiled_runtime;
mod run_ops;
mod run_step;
mod simulate;
mod step_helpers;
mod submit;
mod trace;
mod validate;
mod verify;

pub(crate) use output::{OutputError, json_out, write_stdout_line, write_stdout_line_checked};

fn main() -> std::process::ExitCode {
    app_impl::run_from_env()
}
