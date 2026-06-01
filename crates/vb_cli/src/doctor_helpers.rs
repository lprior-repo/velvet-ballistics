#![forbid(unsafe_code)]
//! Helper functions for the doctor command.

use crate::args::{ActionRegistryMode, Command, OutputFormat, ParseError, StepTarget};
use crate::exit_code::CliExitCode;
use crate::file_io::read_file;
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::output::{
    json_error, json_out, output_error_exit, write_failure_message, write_stderr_line,
    write_stdout_line,
};
use crate::output_utils::*;
use std::io::{self, Write};
use std::process::ExitCode;

pub(crate) fn cmd_doctor_without_db(output: OutputFormat) -> ExitCode {
    let remediation = "rerun with `doctor --db <path>` to verify Fjall journal storage";
    let checks = vec![serde_json::json!({
        "check": "database_path",
        "status": "skip",
        "category": "missing_db",
        "message": "no --db <path> provided; persistent journal checks skipped",
        "remediation": remediation
    })];

    if output != OutputFormat::Text {
        crate::emit_json_or_return!(
            &serde_json::json!({
                "success": true,
                "mode": "stateless",
                "category": "missing_db",
                "checks": checks,
                "remediation": remediation
            }),
            output,
        );
    } else {
        crate::outln!("doctor: no --db <path> provided; persistent journal checks skipped");
        crate::outln!("doctor: {remediation}");
    }

    ExitCode::SUCCESS
}
