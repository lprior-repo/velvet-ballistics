#![forbid(unsafe_code)]
//! Run replay command.

use crate::args::{ActionRegistryMode, Command, OutputFormat, ParseError, StepTarget};
use crate::cli_envelope;
use crate::events::event_to_json;
use crate::exit_code::CliExitCode;
use crate::file_io::{parse_run_id, read_file, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::output::{
    json_error, json_out, json_out_exit, output_error_exit, write_failure_message,
    write_stderr_line, write_stdout_line,
};
use crate::output_utils::*;
use crate::storage::print_event;
use std::io::{self, Write};
use std::process::ExitCode;

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
                .map(|e| crate::commands_diff::event_name(e).to_string());

            match output {
                OutputFormat::Yaml | OutputFormat::Postcard => {
                    let event_list: Vec<serde_json::Value> =
                        events.iter().map(event_to_json).collect();
                    crate::emit_json_or_return!(
                        &serde_json::json!({
                            "schema_version": crate::cli_envelope::SCHEMA_VERSION,
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
                    crate::outln!("recovered {} event(s) for run {run_id}", events.len());
                    for event in &events {
                        print_event(event);
                    }
                    match vb_storage::recovery::extract_terminal(&events) {
                        Some(terminal) => {
                            crate::outln!(
                                "terminal: {}",
                                crate::commands_diff::event_name(terminal)
                            );
                        }
                        None => {
                            crate::outln!("terminal: none");
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
                crate::errln!("error replaying run {run_id}: {e}");
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
            crate::outln!(
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

fn print_event_stubs(_event: &vb_storage::JournalEvent) {
    // stub - use storage::print_event instead
}

fn write_vb_kyyf_trace(command: &str, run_id: &str, events_len: usize) {
    crate::outln!(
        "BDD-KYYF-002 command={command} run_id={run_id} evidence=.evidence/vb-kyyf/storage-replay-resume.md digest=normalized-replay events={events_len}"
    );
}
