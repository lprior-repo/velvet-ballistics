//! Module: run_ops

use crate::app_impl::prelude::*;

pub(crate) fn cmd_retry(
    run_id: &str,
    requested_step: u16,
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
                &serde_json::json!({ "success": false, "error": format!("run {run_id} not found") }),
                output,
            );
        } else {
            errln!("run {run_id}: no events found");
        }
        return CliExitCode::StorageError.into();
    }
    let analysis = commands_journal::analyze_retry(&events);
    if !analysis.can_retry {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({ "success": false, "error": format!("run {run_id} {}", analysis.reason) }),
                output,
            );
        } else {
            errln!("run {run_id} {}", analysis.reason);
        }
        return CliExitCode::LifecycleError.into();
    }
    if analysis.failed_at_step != Some(requested_step) {
        let message = format!(
            "run {run_id} failed at observed step {:?}; requested step {requested_step} is stale",
            analysis.failed_at_step
        );
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": message,
                    "observed_failed_step": analysis.failed_at_step,
                    "requested_step": requested_step
                }),
                output,
            );
        } else {
            errln!("{message}");
        }
        return CliExitCode::LifecycleError.into();
    }
    let message = format!(
        "run {run_id} failed at step {requested_step}, but durable retry execution is unavailable"
    );
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "error": message,
                "failed_at_step": analysis.failed_at_step,
                "last_successful_step": analysis.last_successful_step,
                "events": events.len()
            }),
            output,
        );
    } else {
        errln!("{message}");
    }
    CliExitCode::LifecycleError.into()
}
