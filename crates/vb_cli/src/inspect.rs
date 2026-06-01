#![forbid(unsafe_code)]
//! Run inspection command.

use std::process::ExitCode;
use crate::args::{ActionRegistryMode, Command, OutputFormat, ParseError, StepTarget};
use crate::exit_code::CliExitCode;
use crate::output::{json_error, json_out, output_error_exit, write_stdout_line, write_stderr_line, write_failure_message};
use crate::output_utils::*;
use crate::file_io::{read_file, parse_run_id, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};

pub(crate) fn cmd_inspect(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(vb_storage::JournalError::ProcessLockHeld { .. }) => {
            return write_locked_read_surface("inspect", run_id, output);
        }
        Err(error) => {
            report_storage_open_error(&error, db, output);
            return CliExitCode::StorageError.into();
        }
    };

    match journal.events_for_run(rid) {
        Ok(events) => {
            if events.is_empty() {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "run_id": run_id,
                            "status": "not_found",
                            "events": 0,
                            "error": format!("run {run_id}: no events found")
                        }),
                        output,
                    );
                } else {
                    crate::errln!("run {run_id}: no events found");
                }
                return CliExitCode::ValidationFailed.into();
            } else {
                let state = vb_storage::derive_lifecycle_state_from_events(&events);
                let status = vb_storage::lifecycle_state_to_inspect_status(state);
                if output != OutputFormat::Text {
                    crate::emit_json_or_return!(
                        &serde_json::json!({
                            "run_id": run_id,
                            "status": status,
                            "events": events.len()
                        }),
                        output,
                    );
                } else {
                    crate::outln!("run {run_id}: status={status}, events={}", events.len());
                    write_vb_kyyf_trace("inspect", run_id, events.len());
                }
            }
        }
        Err(e) => {
            let message = format!("error reading run {run_id}: {e}");
            if output != OutputFormat::Text {
                write_failure_message(&message, output, CliExitCode::StorageError);
            } else {
                crate::errln!("{message}");
            }
            return CliExitCode::StorageError.into();
        }
    }

    ExitCode::SUCCESS
}

pub(crate) fn write_vb_kyyf_trace(command: &str, run_id: &str, events_len: usize) {
    crate::outln!(
        "BDD-KYYF-002 command={command} run_id={run_id} evidence=.evidence/vb-kyyf/storage-replay-resume.md digest=normalized-replay events={events_len}"
    );
}


fn write_locked_read_surface(_operation: &str, _run_id: &str, _output: crate::args::OutputFormat) -> std::process::ExitCode {
    crate::errln!("locked read surface not implemented");
    std::process::ExitCode::FAILURE
}
