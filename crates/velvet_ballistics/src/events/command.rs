use std::path::Path;
use std::process::ExitCode;

use crate::args::{EventStatus, OutputFormat};
use crate::commands_journal;
use crate::exit_code::CliExitCode;
use crate::file_io::{check_journal_dir, parse_run_id, report_storage_open_error};
use crate::output::{json_error, json_out_exit};
use vb_storage::{FjallJournal, JournalEvent};

pub(super) fn read(
    run_id: &str,
    db: &Path,
    output: OutputFormat,
) -> Result<Vec<JournalEvent>, ExitCode> {
    let run = parse_run_id(run_id, output)?;
    check_journal_dir(db, output)?;
    let journal = open(run_id, db, output)?;
    let events = journal
        .events_for_run(run)
        .map_err(|error| read_error(run_id, error, output))?;
    if events.is_empty() {
        return Err(no_events(run_id, output));
    }
    Ok(events)
}

fn open(run_id: &str, db: &Path, output: OutputFormat) -> Result<FjallJournal, ExitCode> {
    match FjallJournal::open(db, None) {
        Ok(journal) => Ok(journal),
        Err(vb_storage::JournalError::ProcessLockHeld { .. }) => Err(
            crate::replay::write_locked_read_surface("events", run_id, output),
        ),
        Err(error) => {
            report_storage_open_error(&error, db, output);
            Err(CliExitCode::StorageError.into())
        }
    }
}

fn read_error(run_id: &str, error: vb_storage::JournalError, output: OutputFormat) -> ExitCode {
    let message = format!("error reading events for run {run_id}: {error}");
    if output == OutputFormat::Text {
        errln!("{message}");
    } else {
        json_error(
            &serde_json::json!({"success":false,"error":message}),
            output,
        );
    }
    CliExitCode::StorageError.into()
}

fn no_events(run_id: &str, output: OutputFormat) -> ExitCode {
    let message = format!("run {run_id}: no events found");
    if output == OutputFormat::Text {
        errln!("{message}");
    } else {
        json_error(
            &serde_json::json!({"success":false,"run_id":run_id,"status":"not_found","events":[],"total":0,"error":message}),
            output,
        );
    }
    CliExitCode::ValidationFailed.into()
}

pub(super) fn select(
    events: Vec<JournalEvent>,
    status: Option<EventStatus>,
    limit: Option<i64>,
) -> Vec<JournalEvent> {
    let events = select_status(events, status).into_iter();
    match limit.and_then(|value| usize::try_from(value).ok()) {
        Some(limit) => events.take(limit).collect(),
        None => events.collect(),
    }
}

fn select_status(events: Vec<JournalEvent>, status: Option<EventStatus>) -> Vec<JournalEvent> {
    let Some(status) = status else {
        return events;
    };
    let expected = status.as_str();
    let trace = commands_journal::build_trace(&events);
    events
        .into_iter()
        .zip(trace)
        .filter_map(|(event, entry)| {
            entry
                .status
                .filter(|status| status.as_str() == expected)
                .map(|_| event)
        })
        .collect()
}

pub(super) fn render(run_id: &str, events: Vec<JournalEvent>, output: OutputFormat) -> ExitCode {
    if output == OutputFormat::Text {
        events.iter().for_each(super::print_event);
        outln!("{} event(s) total", events.len());
        crate::inspect::write_vb_kyyf_trace("events", run_id, events.len());
        return ExitCode::SUCCESS;
    }
    let projected: Vec<_> = events.iter().map(crate::replay::event_to_json).collect();
    json_out_exit(
        &serde_json::json!({"schema_version":crate::cli_envelope::SCHEMA_VERSION,"kind":"events_report","run_id":run_id,"events":projected,"total":events.len()}),
        output,
    )
}
