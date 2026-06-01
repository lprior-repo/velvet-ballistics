#![forbid(unsafe_code)]
//! Velvet Ballistics is the CLI runtime for bead lifecycle management.

pub(crate) mod agent_context;
pub mod cli_postcard;
pub mod commands_diff;
pub mod commands_incident;
pub mod lifecycle;
pub mod naming_scan;
pub mod status;

// pub mod constants; // MISSING
// pub mod dispatcher; // MISSING
// pub mod file_io; // MISSING
// pub mod agent_io; // MISSING
// pub mod action; // MISSING
// pub mod action_specs; // MISSING
// pub mod verify; // MISSING
// pub mod compile; // MISSING
// pub mod run; // MISSING
// pub mod submit; // MISSING
// pub mod run_step; // MISSING
// pub mod step_helpers; // MISSING
// pub mod run_compiled; // MISSING
// pub mod run_compiled_runtime; // MISSING
// pub mod ipc_serve; // MISSING
// pub mod inspect; // MISSING
// pub mod events; // MISSING
// pub mod replay; // MISSING
// pub mod trace; // MISSING
// pub mod run_ops; // MISSING
// pub mod run_resume; // MISSING
// pub mod run_cancel; // MISSING
// pub mod incident_diff; // MISSING
// pub mod incident_ops; // MISSING
// pub mod explain; // MISSING
// pub mod explain_reports; // MISSING
// pub mod explain_errors; // MISSING
// pub mod explain_repair; // BROKEN - syntax error
// pub mod explain_validation; // MISSING
// pub mod explain_validation2; // MISSING
// pub mod explain_validation3; // MISSING
// pub mod explain_validation4; // MISSING
// pub mod graph; // MISSING
// pub mod simulate; // MISSING
// pub mod bench_run; // MISSING
// pub mod doctor; // MISSING
// pub mod doctor_helpers; // MISSING
// pub mod io_helpers; // MISSING
// pub mod output_utils; // MISSING
// pub mod output; // MISSING

