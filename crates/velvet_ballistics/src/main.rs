//! Velvet Ballastics binary entrypoint.
#![forbid(unsafe_code)]

macro_rules! outln {
    ($($arg:tt)*) => {{
        $crate::output::write_stdout_line(format_args!($($arg)*));
    }};
}

macro_rules! errln {
    ($($arg:tt)*) => {{
        $crate::output::write_stderr_line(format_args!($($arg)*));
    }};
}

macro_rules! emit_json_or_return {
    ($value:expr, $format:expr $(,)?) => {{
        if let Err(error) = $crate::output::json_out($value, $format) {
            return $crate::output::output_error_exit(&error);
        }
    }};
}

mod action;
mod action_specs;
mod agent_context;
mod agent_io;
#[cfg(test)]
#[path = "agent_context/tests/answer_metadata.rs"]
mod answer_metadata_tests;
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
mod execution_plan;
mod exit_code;
mod explain;
mod file_io;
mod graph;
#[cfg(test)]
#[path = "agent_context/tests/implemented_metadata.rs"]
mod implemented_metadata_tests;
mod incident_diff;
mod incident_ops;
mod inspect;
mod io_helpers;
mod ipc_serve;
#[cfg(test)]
mod mode_error;
mod output;
mod output_utils;
mod replay;
mod run;
mod run_cancel;
mod run_cancel_ops;
mod run_compiled;
mod run_compiled_runtime;
mod run_ops;
mod run_step;
mod semantic_diff;
mod semantic_diff_baseline;
mod semantic_diff_lines;
mod simulate;
mod step_helpers;
mod step_helpers_display;
mod submit;
mod trace;
mod validate;
mod verify;
mod workflow_examples;

pub(crate) use output::{OutputError, json_out, write_stdout_line, write_stdout_line_checked};

fn main() -> std::process::ExitCode {
    app_impl::run_from_env()
}
