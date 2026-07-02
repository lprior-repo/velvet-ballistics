//! Cancel operations: formatting, event writing, and idempotent cancellation.

use std::path::Path;
use std::process::ExitCode;

use crate::app_impl::prelude::*;

pub(crate) fn format_cancel_output(
    run_id: &str,
    reason: Option<&str>,
    note: &str,
    output: OutputFormat,
) -> ExitCode {
    if output != OutputFormat::Text {
        emit_json_or_return!(
            &serde_json::json!({
                "success": true,
                "run_id": run_id,
                "status": "cancelled",
                "reason": reason,
                "note": note,
            }),
            output,
        );
        ExitCode::SUCCESS
    } else {
        let detail = match reason {
            Some(r) => format!(" (reason: {r})"),
            None => String::new(),
        };
        outln!("Run {run_id} cancelled{detail} ({note})");
        ExitCode::SUCCESS
    }
}

pub(crate) fn write_cancel_event(
    journal: &vb_storage::FjallJournal,
    rid: vb_core::RunId,
    reason: Option<String>,
    events: &[vb_storage::JournalEvent],
) -> Result<(), vb_storage::JournalError> {
    let next_seq = match events.last() {
        Some(event) => event.seq().get().saturating_add(1),
        None => 0,
    };
    let event = vb_storage::JournalEvent::RunCancelled {
        run: rid,
        seq: vb_storage::EventSeq::new(next_seq),
        attempt: 1,
        reason,
    };
    journal.append_journaled(&event)
}

pub(crate) fn cmd_cancel(
    run_id: &str,
    db: &Path,
    reason: Option<String>,
    output: OutputFormat,
) -> ExitCode {
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

    let events = match journal.events_for_run(rid) {
        Ok(ev) => ev,
        Err(e) => {
            let message = format!("error reading run {run_id}: {e}");
            if output != OutputFormat::Text {
                write_failure_message(&message, output, CliExitCode::StorageError);
            } else {
                errln!("{message}");
            }
            return CliExitCode::StorageError.into();
        }
    };

    if events.is_empty() {
        return format_cancel_output(
            run_id,
            reason.as_deref(),
            "run not found, idempotent",
            output,
        );
    }

    if run_is_terminal(&events) {
        return format_cancel_output(
            run_id,
            reason.as_deref(),
            "already terminal, idempotent",
            output,
        );
    }

    if let Err(e) = write_cancel_event(&journal, rid, reason.clone(), &events) {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("error writing cancel event: {e}")
                }),
                output,
            );
        } else {
            errln!("error writing cancel event: {e}");
        }
        return CliExitCode::StorageError.into();
    }

    format_cancel_output(run_id, reason.as_deref(), "cancelled", output)
}
