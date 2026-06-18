//! Error-reporting helpers for the AI-context command.
//!
//! Each function formats diagnostics in the requested `OutputFormat` (text vs JSON).

#![forbid(unsafe_code)]

use std::io::{self, Write};

use crate::args::OutputFormat;
use crate::exit_code::CliExitCode;
use crate::output::json_error;

pub(super) fn report_storage_open_error(
    e: &vb_storage::JournalError,
    db: &std::path::Path,
    output: OutputFormat,
) {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": format!("error opening journal at {}: {e}", db.display())
            }),
            CliExitCode::StorageError,
            output,
        );
    } else {
        write_stderr_line(format_args!(
            "error opening journal at {}: {e}",
            db.display()
        ));
    }
}

pub(super) fn report_run_not_found(run_id: &str, output: OutputFormat) -> std::process::ExitCode {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "code": "RUN_NOT_FOUND",
                "run_id": run_id,
            }),
            CliExitCode::ValidationFailed,
            output,
        );
    } else {
        write_stderr_line(format_args!("RUN_NOT_FOUND: run {run_id}"));
    }
    CliExitCode::ValidationFailed.into()
}

pub(super) fn report_journal_read_error(
    area: &str,
    run_id: &str,
    e: &vb_storage::JournalError,
    output: OutputFormat,
) {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": format!("error reading {area} for run {run_id}: {e}")
            }),
            CliExitCode::StorageError,
            output,
        );
    } else {
        write_stderr_line(format_args!("error reading {area} for run {run_id}: {e}"));
    }
}

pub(super) fn write_stderr_line(args: std::fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if let Err(error) = handle
        .write_fmt(args)
        .and_then(|()| handle.write_all(b"\n"))
    {
        write_stderr_best_effort(format_args!("stderr write failed: {error}"));
    }
}

pub(super) fn write_stderr_best_effort(args: std::fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if let Err(_write_error) = handle
        .write_fmt(args)
        .and_then(|()| handle.write_all(b"\n"))
    {}
}
