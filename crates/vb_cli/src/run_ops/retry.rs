#![forbid(unsafe_code)]
//! Retry operation: re-run a failed/partially-executed run from the point of failure.

use crate::args::OutputFormat;
use crate::exit_code::CliExitCode;
use crate::file_io::{parse_run_id, read_journal_events, report_storage_open_error};
use crate::lifecycle;
use crate::output::json_error;
use crate::emit_json_or_return;
use std::process::ExitCode;

pub(crate) fn cmd_retry(
    run_id: &str,
    step: Option<&u16>,
    db: &std::path::Path,
    output: OutputFormat,
) -> ExitCode {
    let events = match read_journal_events(run_id, db, output) {
        Ok(ev) => ev,
        Err(code) => return code,
    };
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
                CliExitCode::ValidationFailed,
                output,
            );
        } else {
            crate::errln!("run {run_id}: no events found");
        }
        return CliExitCode::ValidationFailed.into();
    }
    let analysis = crate::commands_journal::analyze_retry(&events);
    if !analysis.can_retry {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({ "success": false, "error": format!("run {run_id} {}", analysis.reason) }),
                CliExitCode::ValidationFailed,
                output,
            );
        } else {
            crate::errln!("run {run_id} {}", analysis.reason);
        }
        return CliExitCode::ValidationFailed.into();
    }
    let resume_step = match step {
        Some(s) => *s,
        None => analysis
            .last_successful_step
            .map(|s| s.saturating_add(1))
            .unwrap_or(0),
    };
    // Analysis data is reported only on text stdout — it is intentionally
    // omitted from the structured JSON/YAML output so the lifecycle result is the
    // sole mapping at the document root (no duplicate `run_id` key). For
    // structured formats, the analysis goes to stderr (best-effort diagnostic)
    // to keep stdout parseable.
    if output == OutputFormat::Text {
        match (analysis.failed_at_step, analysis.last_successful_step) {
            (Some(fail), Some(last)) => {
                crate::outln!("Run {run_id} failed at step {fail}. Last successful: step {last}.")
            }
            (Some(fail), None) => {
                crate::outln!("Run {run_id} failed at step {fail}. No successful steps recorded.")
            }
            (None, Some(last)) => {
                crate::outln!("Run {run_id} failed. Last successful: step {last}.")
            }
            (None, None) => crate::outln!("Run {run_id} failed. No step progress recorded."),
        }
        crate::outln!("Retry will resume from step {resume_step} with recovered state.");
    } else {
        match (analysis.failed_at_step, analysis.last_successful_step) {
            (Some(fail), Some(last)) => {
                crate::errln!("Run {run_id} failed at step {fail}. Last successful: step {last}.")
            }
            (Some(fail), None) => {
                crate::errln!("Run {run_id} failed at step {fail}. No successful steps recorded.")
            }
            (None, Some(last)) => {
                crate::errln!("Run {run_id} failed. Last successful: step {last}.")
            }
            (None, None) => crate::errln!("Run {run_id} failed. No step progress recorded."),
        }
        crate::errln!("Retry will resume from step {resume_step} with recovered state.");
    }

    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            report_storage_open_error(&e, db, output);
            return CliExitCode::StorageError.into();
        }
    };

    if let Err(e) = lifecycle::retry(rid, &journal) {
        let message = format!("error retrying run {run_id}: {e}");
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({ "success": false, "error": message }),
                CliExitCode::RuntimeFailed,
                output,
            );
        } else {
            crate::errln!("{message}");
        }
        return CliExitCode::RuntimeFailed.into();
    }

    // Single, deterministic user-visible summary that merges the analysis
    // and the success status. The prior implementation emitted two payloads
    // (analysis then success) which produced duplicate `run_id` keys in the
    // structured YAML/JSON output and broke downstream parsers.
    if output != OutputFormat::Text {
        emit_json_or_return!(
            &serde_json::json!({
                "success": true,
                "run_id": run_id,
                "status": "retrying",
                "failed_at_step": analysis.failed_at_step,
                "last_successful_step": analysis.last_successful_step,
                "resume_from_step": resume_step,
                "events": events.len()
            }),
            output,
        );
    } else {
        crate::outln!("Run {run_id} retry event written.");
    }
    ExitCode::SUCCESS
}
