//! AI context handler and run-id parsing.
//!
//! The handler is the sole public entry point. It orchestrates:
//! 1. Run-ID parsing and validation
//! 2. Journal opening
//! 3. Event / header / snapshot fetch
//! 4. Assembly of the AI-context payload
//! 5. Serialised output

#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::process::ExitCode;

use serde_json::Value;

use crate::args::OutputFormat;
use crate::cli_envelope;
use crate::exit_code::CliExitCode;
use crate::output::json_error;

use super::error_reporting::*;
use super::run_status::{RunStatus, run_status_from_events, suggested_ai_commands};
use super::snapshot::latest_snapshot_for_run;
use super::workflow::{ai_workflow_summary, workflow_digest_from_events};

/// Handle the `ai-context` command for the given run.
pub(crate) fn handle(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };
    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(journal) => journal,
        Err(e) => {
            report_storage_open_error(&e, db, output);
            return CliExitCode::StorageError.into();
        }
    };
    let events = match journal.events_for_run(rid) {
        Ok(events) if !events.is_empty() => events,
        Ok(_) => return report_run_not_found(run_id, output),
        Err(e) => {
            report_journal_read_error("events", run_id, &e, output);
            return CliExitCode::StorageError.into();
        }
    };
    let header = match journal.run_header(rid) {
        Ok(header) => header,
        Err(e) => {
            report_journal_read_error("run header", run_id, &e, output);
            return CliExitCode::StorageError.into();
        }
    };
    let digest = header
        .as_ref()
        .map(|header| header.compiled_digest)
        .or_else(|| workflow_digest_from_events(&events));
    let latest_snapshot = match latest_snapshot_for_run(&journal, rid, &events) {
        Ok(snapshot) => snapshot,
        Err(e) => {
            report_journal_read_error("snapshot", run_id, &e, output);
            return CliExitCode::StorageError.into();
        }
    };
    let workflow = ai_workflow_summary(&journal, digest);
    let status = run_status_from_events(&events);
    let payload = serde_json::json!({
        "run_id": rid.get(),
        "workflow": workflow,
        "journal_event_trail": super::events::ai_journal_events(&events, latest_snapshot.as_ref()),
        "action_contracts": super::action_contracts::ai_action_contracts(&events, workflow.get("referenced_actions")),
        "trace_ring_snapshot": super::run_status::trace_ring_snapshot(),
        "suggested_next_cli_commands": suggested_ai_commands(run_id, db, status),
    });
    let envelope = crate::cli_envelope::serialize_with_version(
        &payload,
        crate::cli_envelope::Kind::AiContextPacket,
    );
    match crate::json_out(&envelope, output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            write_stderr_line(format_args!("output failed: {error}"));
            CliExitCode::StorageError.into()
        }
    }
}

fn parse_run_id(raw: &str, output: OutputFormat) -> Result<vb_core::RunId, ExitCode> {
    match raw.parse::<u64>() {
        Ok(0) => {
            write_run_id_error(raw, "run_id must be non-zero", output);
            Err(CliExitCode::ValidationFailed.into())
        }
        Ok(id) => Ok(vb_core::RunId::new(id)),
        Err(e) => {
            write_run_id_error(raw, &e.to_string(), output);
            Err(CliExitCode::ValidationFailed.into())
        }
    }
}

fn write_run_id_error(raw: &str, reason: &str, output: OutputFormat) {
    let message = format!("invalid run_id '{raw}': {reason}");
    if output == OutputFormat::Text {
        write_stderr_line(format_args!("{message}"));
    } else {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": message,
            }),
            CliExitCode::ValidationFailed,
            output,
        );
    }
}

fn write_stderr_line(args: std::fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if let Err(error) = handle
        .write_fmt(args)
        .and_then(|()| handle.write_all(b"\n"))
    {
        write_stderr_best_effort(format_args!("stderr write failed: {error}"));
    }
}
