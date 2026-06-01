#![forbid(unsafe_code)]
//! Run replay command.

use std::process::ExitCode;
use std::io::{self, Write};
use crate::args::{ActionRegistryMode, Command, OutputFormat, ParseError, StepTarget};
use crate::exit_code::CliExitCode;
use crate::output::{json_error, json_out, output_error_exit, write_stdout_line, write_stderr_line, write_failure_message};
use crate::output_utils::*;
use crate::file_io::{read_file, parse_run_id, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::cli_envelope;

pub(crate) fn cmd_replay(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(vb_storage::JournalError::ProcessLockHeld { .. }) => {
            return write_locked_read_surface("replay", run_id, output);
        }
        Err(error) => {
            report_storage_open_error(&error, db, output);
            return CliExitCode::StorageError.into();
        }
    };

    let mut tracker = vb_storage::recovery::ActionReplayTracker::new();
    match vb_storage::recovery::recover_full_journal(&journal, rid, &mut tracker, &[], &[]) {
        Ok(events) => {
            let terminal_name = vb_storage::recovery::extract_terminal(&events)
                .map(|e| commands_diff::event_name(e).to_string());

            match output {
                OutputFormat::Yaml | OutputFormat::Postcard => {
                    let event_list: Vec<serde_json::Value> =
                        events.iter().map(event_to_json).collect();
                    emit_json_or_return!(
                        &serde_json::json!({
                            "schema_version": cli_envelope::SCHEMA_VERSION,
                            "kind": "replay_report",
                            "run_id": run_id,
                            "recovered": events.len(),
                            "events": event_list,
                            "terminal": terminal_name
                        }),
                        output,
                    );
                }
                OutputFormat::Text => {
                    outln!("recovered {} event(s) for run {run_id}", events.len());
                    for event in &events {
                        print_event(event);
                    }
                    match vb_storage::recovery::extract_terminal(&events) {
                        Some(terminal) => {
                            outln!("terminal: {}", commands_diff::event_name(terminal));
                        }
                        None => {
                            outln!("terminal: none");
                        }
                    }
                    write_vb_kyyf_trace("replay", run_id, events.len());
                }
            }
        }
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error replaying run {run_id}: {e}")
                    }),
                    output,
                );
            } else {
                errln!("error replaying run {run_id}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    }

    ExitCode::SUCCESS
}

pub(crate) fn write_locked_read_surface(
    command: &'static str,
    run_id: &str,
    output: OutputFormat,
) -> ExitCode {
    match output {
        OutputFormat::Text => {
            outln!(
                "{command} run {run_id}: storage is held by an active writer; public CLI surface is available"
            );
            write_vb_kyyf_trace(command, run_id, 0);
            ExitCode::SUCCESS
        }
        OutputFormat::Yaml | OutputFormat::Postcard => json_out_exit(
            &serde_json::json!({
                "run_id": run_id,
                "command": command,
                "status": "writer_lock_held",
                "surface": "available"
            }),
            output,
        ),
    }
}

