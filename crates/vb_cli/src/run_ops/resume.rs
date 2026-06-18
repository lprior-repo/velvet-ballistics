#![forbid(unsafe_code)]
//! Resume operation: continue a run that was suspended at a checkpoint.

use crate::args::OutputFormat;
use crate::exit_code::CliExitCode;
use crate::file_io::{parse_run_id, read_journal_events, report_storage_open_error};
use crate::lifecycle;
use crate::output::json_error;
use crate::emit_json_or_return;
use std::process::ExitCode;

pub(crate) fn cmd_resume(
    run_id: &str,
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
    let analysis = crate::commands_journal::analyze_resume(&events);
    if !analysis.can_resume {
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
    let resume_step = analysis.suspended_at_step;
    if output != OutputFormat::Text {
        emit_json_or_return!(
            &serde_json::json!({
                "run_id": run_id, "suspended_at_step": analysis.suspended_at_step,
                "status": "suspended", "resume_from_step": resume_step, "events": events.len()
            }),
            output,
        );
    } else {
        match resume_step {
            Some(step) => crate::outln!(
                "Run {run_id} suspended at step {step}. Resume would continue from step {step} with recovered state."
            ),
            None => crate::outln!(
                "Run {run_id} is active but no explicit suspension point found. Resume would continue from current state."
            ),
        }
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

    if let Err(e) = lifecycle::resume(rid, &journal) {
        let message = format!("error resuming run {run_id}: {e}");
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

    if output != OutputFormat::Text {
        emit_json_or_return!(
            &serde_json::json!({ "success": true, "run_id": run_id, "status": "resumed" }),
            output,
        );
    } else {
        crate::outln!("Run {run_id} resume event written.");
    }
    ExitCode::SUCCESS
}
