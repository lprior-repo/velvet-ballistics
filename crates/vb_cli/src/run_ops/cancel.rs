#![forbid(unsafe_code)]
//! Cancel operation: idempotently write a RunCancelled journal event.
//!
//! Also provides `run_is_terminal` and `format_cancel_output` helpers used
//! by the cancel workflow.

use crate::args::OutputFormat;
use crate::exit_code::CliExitCode;
use crate::output::json_error;
use crate::emit_json_or_return;
use std::process::ExitCode;

/// Return true if the event stream contains a terminal run marker.
pub(crate) fn run_is_terminal(events: &[vb_storage::JournalEvent]) -> bool {
    events.iter().any(|e| {
        matches!(
            e,
            vb_storage::JournalEvent::RunFinished { .. }
                | vb_storage::JournalEvent::RunFailedEvent { .. }
                | vb_storage::JournalEvent::RunCancelled { .. }
        )
    })
}

/// Emit a JSON or text cancellation confirmation.
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
        crate::outln!("Run {run_id} cancelled{detail} ({note})");
        ExitCode::SUCCESS
    }
}

/// Append a RunCancelled event to the journal.
pub(crate) fn write_cancel_event(
    journal: &vb_storage::FjallJournal,
    rid: vb_core::RunId,
    reason: Option<String>,
    events: &[vb_storage::JournalEvent],
) -> Result<(), vb_storage::JournalError> {
    let next_seq = match events.last() {
        Some(e) => e.seq().get().saturating_add(1),
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
    db: &std::path::Path,
    reason: Option<String>,
    output: OutputFormat,
) -> ExitCode {
    let rid = match crate::file_io::parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            crate::file_io::report_storage_open_error(&e, db, output);
            return CliExitCode::StorageError.into();
        }
    };

    let events = match journal.events_for_run(rid) {
        Ok(ev) => ev,
        Err(e) => {
            let message = format!("error reading run {run_id}: {e}");
            if output != OutputFormat::Text {
                crate::file_io::write_failure_message(&message, output, CliExitCode::StorageError);
            } else {
                crate::errln!("{message}");
            }
            return CliExitCode::StorageError.into();
        }
    };

    // Idempotent: no events means run never existed.
    if events.is_empty() {
        return format_cancel_output(
            run_id,
            reason.as_deref(),
            "run not found, idempotent",
            output,
        );
    }

    // Idempotent: already terminal.
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
                CliExitCode::StorageError,
                output,
            );
        } else {
            crate::errln!("error writing cancel event: {e}");
        }
        return CliExitCode::StorageError.into();
    }

    format_cancel_output(run_id, reason.as_deref(), "cancelled", output)
}
