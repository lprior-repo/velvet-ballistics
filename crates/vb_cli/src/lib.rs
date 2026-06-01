#![forbid(unsafe_code)]
//! Velvet Ballistics is the CLI runtime for bead lifecycle management.


#[macro_export]
macro_rules! outln {
    ($($arg:tt)*) => {{
        crate::output::write_stdout_line(format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! errln {
    ($($arg:tt)*) => {{
        crate::output::write_stderr_line(format_args!($($arg)*));
    }};
}

pub(crate) mod agent_context;
pub mod cli_postcard;
pub mod commands_diff;
pub mod commands_incident;
pub mod lifecycle;
pub mod naming_scan;
pub mod status;

// Re-enabled modules (files verified present with content).
// Kept commented: run_resume, explain_validation3, explain_validation4 — files do not exist.
// explain_repair was previously BROKEN; uncomment if syntax is fixed.
pub mod action;
pub mod action_specs;
pub mod agent_io;
pub mod bench_run;
pub mod compile;
pub mod constants;
pub mod dispatcher;
pub mod doctor;
pub mod doctor_helpers;
pub mod events;
pub mod explain;
pub mod explain_errors;
pub mod explain_reports;
pub mod explain_validation;
pub mod explain_validation2;
pub mod file_io;
pub mod graph;
pub mod incident_diff;
pub mod incident_ops;
pub mod inspect;
pub mod io_helpers;
pub mod ipc_serve;
pub mod output;
pub mod output_utils;
pub mod replay;
pub mod run;
pub mod run_cancel;
pub mod run_compiled;
pub mod run_compiled_runtime;
pub mod run_ops;
pub mod run_step;
pub mod simulate;
pub mod step_helpers;
pub mod submit;
pub mod trace;
pub mod verify;

// Newly declared modules (previously missing from lib.rs).
pub mod args;
pub mod cli_envelope;
pub mod cli_error;
pub mod commands_ai_context;
pub mod commands_journal;
pub mod commands_status;
pub mod commands_system_status;
pub mod commands_verify;
pub mod commands_workflow;
pub mod deliver_sink;
pub mod exit_code;
pub mod explain_compile;
pub mod explain_repair;
pub mod mode_error;
pub mod storage;
pub mod validate;
pub mod workflow;

