//! Module: replay
#![forbid(unsafe_code)]

mod event_to_json;
#[cfg(test)]
mod event_to_json_tests;
mod pristine_resume;
#[cfg(test)]
mod pristine_resume_tests;

use crate::app_impl::prelude::*;
use crate::events::print_event;
use crate::inspect::write_vb_kyyf_trace;
use pristine_resume::{pristine_resume_candidate, run_pristine_resume, select_pristine_resume};

pub(crate) use event_to_json::event_to_json;

pub(crate) fn cmd_replay(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };

    if let Err(code) = check_journal_dir(db, output) {
        return code;
    }

    let journal = match open_replay_journal(run_id, db, output) {
        Ok(journal) => journal,
        Err(code) => return code,
    };
    replay_journal(&journal, rid, run_id, output)
}

fn open_replay_journal(
    run_id: &str,
    db: &std::path::Path,
    output: OutputFormat,
) -> Result<vb_storage::FjallJournal, ExitCode> {
    match vb_storage::FjallJournal::open(db, None) {
        Ok(journal) => Ok(journal),
        Err(vb_storage::JournalError::ProcessLockHeld { .. }) => {
            Err(write_locked_read_surface("replay", run_id, output))
        }
        Err(error) => {
            report_storage_open_error(&error, db, output);
            Err(CliExitCode::StorageError.into())
        }
    }
}

fn replay_journal(
    journal: &vb_storage::FjallJournal,
    rid: vb_core::RunId,
    run_id: &str,
    output: OutputFormat,
) -> ExitCode {
    let mut tracker = vb_storage::recovery::ActionReplayTracker::new();
    match vb_storage::recovery::recover_full_journal(journal, rid, &mut tracker, &[], &[]) {
        Ok(events) => write_replay_success(run_id, &events, output),
        Err(error) => write_replay_error(run_id, &error, output),
    }
}

fn write_replay_success(
    run_id: &str,
    events: &[vb_storage::JournalEvent],
    output: OutputFormat,
) -> ExitCode {
    match output {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            crate::output::json_out_exit(&replay_report(run_id, events), output)
        }
        OutputFormat::Text => write_replay_text(run_id, events),
    }
}

fn replay_report(run_id: &str, events: &[vb_storage::JournalEvent]) -> serde_json::Value {
    let terminal_name = vb_storage::recovery::extract_terminal(events)
        .map(|event| commands_diff::event_name(event).to_string());
    let event_list: Vec<serde_json::Value> = events.iter().map(event_to_json).collect();
    serde_json::json!({
        "schema_version": cli_envelope::SCHEMA_VERSION,
        "kind": "replay_report",
        "run_id": run_id,
        "recovered": events.len(),
        "events": event_list,
        "terminal": terminal_name
    })
}

fn write_replay_text(run_id: &str, events: &[vb_storage::JournalEvent]) -> ExitCode {
    outln!("recovered {} event(s) for run {run_id}", events.len());
    events.iter().for_each(print_event);
    let terminal_name =
        vb_storage::recovery::extract_terminal(events).map_or("none", commands_diff::event_name);
    outln!("terminal: {terminal_name}");
    write_vb_kyyf_trace("replay", run_id, events.len());
    ExitCode::SUCCESS
}

fn write_replay_error(
    run_id: &str,
    error: &vb_storage::recovery::RecoveryError,
    output: OutputFormat,
) -> ExitCode {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": format!("error replaying run {run_id}: {error}")
            }),
            output,
        );
    } else {
        errln!("error replaying run {run_id}: {error}");
    }
    replay_error_exit_code(error).into()
}

fn replay_error_exit_code(error: &vb_storage::recovery::RecoveryError) -> CliExitCode {
    match error {
        vb_storage::recovery::RecoveryError::ReplayDivergence { .. } => {
            CliExitCode::ReplayDivergence
        }
        _ => CliExitCode::StorageError,
    }
}

pub(crate) fn cmd_resume(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };

    if let Err(code) = check_journal_dir(db, output) {
        return code;
    }

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(journal) => journal,
        Err(error) => {
            report_storage_open_error(&error, db, output);
            return CliExitCode::StorageError.into();
        }
    };

    match velvet_ballistics::lifecycle::resume(rid, &journal) {
        Ok(()) => match output {
            OutputFormat::Text => {
                outln!("resumed run {run_id}");
                ExitCode::SUCCESS
            }
            OutputFormat::Yaml | OutputFormat::Postcard => json_out_exit(
                &serde_json::json!({
                    "success": true,
                    "run_id": run_id,
                    "status": "resumed"
                }),
                output,
            ),
        },
        Err(error) => {
            let error = if pristine_resume_candidate(&error) {
                match select_pristine_resume(rid, error, journal.events_for_run_full(rid)) {
                    Ok(()) => return run_pristine_resume(run_id, rid, journal, output),
                    Err(original) => original,
                }
            } else {
                error
            };
            let message = format!("error resuming run {run_id}: {error}");
            if output == OutputFormat::Text {
                errln!("{message}");
            } else {
                write_failure_message(&message, output, CliExitCode::LifecycleError);
            }
            CliExitCode::LifecycleError.into()
        }
    }
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
