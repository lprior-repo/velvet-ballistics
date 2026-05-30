//! Velvet Ballastics binary entrypoint.
#![forbid(unsafe_code)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]

mod agent_context;
mod app_impl;
mod args;
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
mod deliver_sink;
mod exit_code;
#[cfg(test)]
mod mode_error;

pub(crate) use app_impl::{OutputError, json_out, write_stdout_line, write_stdout_line_checked};
#[cfg(test)]
pub(crate) use exit_code::CliExitCode;

fn main() -> std::process::ExitCode {
    app_impl::run_from_env()
}
