#![forbid(unsafe_code)]
//! Run operations: retry, resume, answer, cancel.

use crate::args::{
    ActionRegistryMode, Command, DurabilityMode, OutputFormat, ParseError, StepTarget,
};
use crate::commands_journal;
use crate::exit_code::CliExitCode;
use crate::file_io::{parse_run_id, read_file, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::output::{
    json_error, json_out, output_error_exit, write_contract_error_json, write_failure_message,
    write_stderr_line, write_stdout_line,
};
use crate::output_utils::*;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use vb_ipc::client::IpcClient;
use vb_ipc::{IpcCommand, IpcPayload};

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
    // If --step was provided, use it; otherwise calculate from analysis
    let resume_step = match step {
        Some(s) => *s,
        None => analysis
            .last_successful_step
            .map(|s| s.saturating_add(1))
            .unwrap_or(0),
    };
    if output != OutputFormat::Text {
        crate::emit_json_or_return!(
            &serde_json::json!({
                "run_id": run_id, "failed_at_step": analysis.failed_at_step,
                "last_successful_step": analysis.last_successful_step,
                "resume_from_step": resume_step, "events": events.len()
            }),
            output,
        );
    } else {
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
    }
    ExitCode::SUCCESS
}

pub(crate) fn cmd_resume(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
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
        crate::emit_json_or_return!(
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
    ExitCode::SUCCESS
}

pub(crate) fn cmd_answer(
    run_id: &str,
    slot: u16,
    value_file: &std::path::Path,
    db: &std::path::Path,
    output: OutputFormat,
) -> ExitCode {
    // Parse run_id
    let rid = match run_id.parse::<u64>() {
        Ok(id) => vb_core::RunId::new(id),
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("invalid run_id '{run_id}': {e}")
                    }),
                    CliExitCode::ValidationFailed,
                    output,
                );
            } else {
                crate::errln!("invalid run_id '{run_id}': {e}");
            }
            return CliExitCode::ValidationFailed.into();
        }
    };

    // Read value file as bytes (expected to be postcard-encoded SlotValue)
    let answer_bytes = match std::fs::read(value_file) {
        Ok(bytes) => bytes,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error reading value file {}: {e}", value_file.display())
                    }),
                    CliExitCode::ValidationFailed,
                    output,
                );
            } else {
                crate::errln!("error reading value file {}: {e}", value_file.display());
            }
            return CliExitCode::ValidationFailed.into();
        }
    };
    if postcard::from_bytes::<vb_core::value::SlotValue>(&answer_bytes).is_err() {
        let message = "answer value file must contain postcard-encoded SlotValue";
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": message
                }),
                CliExitCode::ValidationFailed,
                output,
            );
        } else {
            crate::errln!("{message}");
        }
        return CliExitCode::ValidationFailed.into();
    }

    // Derive IPC socket path from db path: <db_parent>/<db_stem>.sock
    // e.g., /var/lib/vb/run.db -> /var/lib/vb/run.sock
    let socket_path = db.with_extension("sock");

    // Connect to the IPC server
    let mut client = match IpcClient::connect(&socket_path) {
        Ok(c) => c,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error connecting to IPC server at {}: {e}", socket_path.display())
                    }),
                    CliExitCode::IpcError,
                    output,
                );
            } else {
                crate::errln!(
                    "error connecting to IPC server at {}: {e}",
                    socket_path.display()
                );
            }
            return CliExitCode::IpcError.into();
        }
    };

    // Construct the IPC payload
    let payload = IpcPayload::AnswerAsk {
        run_id: rid,
        answer_slot: vb_core::ids::SlotIdx::new(slot),
        answer: answer_bytes,
        taint: None,
    };

    // Send the command
    if let Err(e) = client.send_command(IpcCommand::AnswerAsk, 0, &payload) {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("error sending answer: {e}")
                }),
                CliExitCode::IpcError,
                output,
            );
        } else {
            crate::errln!("error sending answer: {e}");
        }
        return CliExitCode::IpcError.into();
    }

    // Receive and process the response
    match client.recv_response(vb_ipc::MaxPayloadBytes::DEFAULT) {
        Ok((_header, response)) => match response {
            vb_ipc::server::IpcResponse::AcceptedRun { run_id: _ } => {
                if output != OutputFormat::Text {
                    crate::emit_json_or_return!(
                        &serde_json::json!({
                            "success": true,
                            "run_id": rid.get()
                        }),
                        output,
                    );
                } else {
                    crate::outln!("answer accepted for run {}", rid.get());
                }
                ExitCode::SUCCESS
            }
            vb_ipc::server::IpcResponse::RuntimeError { message } => {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": message
                        }),
                        CliExitCode::RuntimeFailed,
                        output,
                    );
                } else {
                    crate::errln!("runtime error: {message}");
                }
                CliExitCode::RuntimeFailed.into()
            }
            vb_ipc::server::IpcResponse::PayloadError {
                diagnostic: _,
                message,
            } => {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": message
                        }),
                        CliExitCode::ValidationFailed,
                        output,
                    );
                } else {
                    crate::errln!("payload error: {message}");
                }
                CliExitCode::ValidationFailed.into()
            }
            other => {
                if output != OutputFormat::Text {
                    json_error(
                        &serde_json::json!({
                            "success": false,
                            "error": format!("unexpected response: {other:?}")
                        }),
                        CliExitCode::RuntimeFailed,
                        output,
                    );
                } else {
                    crate::errln!("unexpected response: {other:?}");
                }
                CliExitCode::RuntimeFailed.into()
            }
        },
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error receiving response: {e}")
                    }),
                    CliExitCode::IpcError,
                    output,
                );
            } else {
                crate::errln!("error receiving response: {e}");
            }
            CliExitCode::IpcError.into()
        }
    }
}

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

pub(crate) fn format_cancel_output(
    run_id: &str,
    reason: Option<&str>,
    note: &str,
    output: OutputFormat,
) -> ExitCode {
    if output != OutputFormat::Text {
        crate::emit_json_or_return!(
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
