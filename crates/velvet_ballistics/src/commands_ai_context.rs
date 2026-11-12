//! AI context CLI command.
#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::process::ExitCode;

use serde_json::Value;

use crate::args::OutputFormat;
use crate::cli_envelope;
use crate::exit_code::CliExitCode;
use crate::file_io::check_journal_dir;

use self::journal_events::{
    ai_action_contracts, ai_journal_events, run_status_from_events, trace_ring_snapshot,
};
use self::workflow_summary::{ai_workflow_summary, workflow_digest_from_events};

mod journal_events;
mod workflow_summary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunStatus {
    Running,
    Finished,
    Failed,
    Cancelled,
}

pub(crate) fn handle(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };
    if let Err(code) = check_journal_dir(db, output) {
        return code;
    }
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
        "journal_event_trail": ai_journal_events(&events, latest_snapshot.as_ref()),
        "action_contracts": ai_action_contracts(&events, workflow.get("referenced_actions")),
        "trace_ring_snapshot": trace_ring_snapshot(),
        "suggested_next_cli_commands": suggested_ai_commands(run_id, db, status),
    });
    let envelope =
        cli_envelope::serialize_with_version(&payload, cli_envelope::Kind::AiContextPacket);
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
            output,
        );
    }
}

fn report_storage_open_error(
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
            output,
        );
    } else {
        write_stderr_line(format_args!(
            "error opening journal at {}: {e}",
            db.display()
        ));
    }
}

fn report_run_not_found(run_id: &str, output: OutputFormat) -> ExitCode {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "code": "RUN_NOT_FOUND",
                "run_id": run_id,
            }),
            output,
        );
    } else {
        write_stderr_line(format_args!("RUN_NOT_FOUND: run {run_id}"));
    }
    CliExitCode::ValidationFailed.into()
}

fn report_journal_read_error(
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
            output,
        );
    } else {
        write_stderr_line(format_args!("error reading {area} for run {run_id}: {e}"));
    }
}

fn latest_snapshot_for_run(
    journal: &vb_storage::FjallJournal,
    run: vb_core::RunId,
    events: &[vb_storage::JournalEvent],
) -> Result<Option<vb_storage::RunSnapshot>, vb_storage::JournalError> {
    latest_snapshot_from_events(events, |seq| journal.snapshot(run, seq))
}

fn latest_snapshot_from_events(
    events: &[vb_storage::JournalEvent],
    mut snapshot_at: impl FnMut(
        vb_storage::EventSeq,
    )
        -> Result<Option<vb_storage::RunSnapshot>, vb_storage::JournalError>,
) -> Result<Option<vb_storage::RunSnapshot>, vb_storage::JournalError> {
    events.iter().rev().try_fold(None, |found, event| {
        if found.is_some() {
            Ok(found)
        } else {
            snapshot_at(event.seq())
        }
    })
}

fn push_unique_u32(mut values: Vec<u32>, value: u32) -> Vec<u32> {
    if !values.contains(&value) {
        values.push(value);
    }
    values
}

pub(crate) fn suggested_ai_commands(
    run_id: &str,
    db: &std::path::Path,
    status: RunStatus,
) -> Vec<String> {
    let db_arg = db.display();
    let base = vec![
        format!("velvet-ballistics inspect {run_id} --db {db_arg} --emit yaml"),
        format!("velvet-ballistics events {run_id} --db {db_arg} --emit yaml"),
    ];
    match status {
        RunStatus::Failed | RunStatus::Cancelled => base
            .into_iter()
            .chain([
                format!("velvet-ballistics incident {run_id} --db {db_arg} --emit yaml"),
                format!("velvet-ballistics retry {run_id} --db {db_arg} --emit yaml"),
            ])
            .collect(),
        RunStatus::Running => base
            .into_iter()
            .chain([
                format!("velvet-ballistics trace {run_id} --db {db_arg} --emit yaml"),
                format!("velvet-ballistics resume {run_id} --db {db_arg} --emit yaml"),
            ])
            .collect(),
        RunStatus::Finished => base
            .into_iter()
            .chain([format!(
                "velvet-ballistics replay {run_id} --db {db_arg} --emit yaml"
            )])
            .collect(),
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

fn write_stderr_best_effort(args: std::fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if let Err(_write_error) = handle
        .write_fmt(args)
        .and_then(|()| handle.write_all(b"\n"))
    {}
}

fn json_error(value: &Value, format: OutputFormat) {
    match format {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            if let Err(error) = crate::output::write_structured_stderr(value, format) {
                write_stderr_best_effort(format_args!("stderr write failed: {error}"));
            }
        }
        OutputFormat::Text => write_stderr_line(format_args!("{value}")),
    }
}

#[cfg(test)]
#[path = "commands_ai_context/tests.rs"]
mod tests;
