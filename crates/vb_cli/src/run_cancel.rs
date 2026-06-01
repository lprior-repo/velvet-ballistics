use crate::args::OutputFormat;
use crate::exit_code::CliExitCode;
use crate::output::{json_error, output_error_exit};
use crate::output_utils::{infer_legacy_json_error_code, legacy_json_error_message, write_diagnostic_message_stderr};
use crate::file_io::{parse_run_id, read_journal_events, report_storage_open_error};
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use vb_core::RunId;
use vb_runtime::journal::RuntimeJournalConfig;
use vb_ipc::{IpcCommand, IpcPayload};
use vb_ipc::client::IpcClient;

fn cmd_answer(
    run_id: &str,
    step: u16,
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
                    output,
                );
            } else {
                crate::errln!("invalid run_id '{run_id}': {e}");
            }
            return CliExitCode::ValidationFailed.into();
        }
    };

    // Read value_file as bytes (expected to be postcard-encoded SlotValue)
    let answer_bytes = match std::fs::read(value_file) {
        Ok(bytes) => bytes,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error reading value file {}: {e}", value_file.display())
                    }),
                    output,
                );
            } else {
                crate::errln!("error reading value file {}: {e}", value_file.display());
            }
            return CliExitCode::ValidationFailed.into();
        }
    };

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
        ticket: step.into(),
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
                    output,
                );
            } else {
                crate::errln!("error receiving response: {e}");
            }
            CliExitCode::IpcError.into()
        }
    }
}

fn run_is_terminal(events: &[vb_storage::JournalEvent]) -> bool {
    events.iter().any(|e| {
        matches!(
            e,
            vb_storage::JournalEvent::RunFinished { .. }
                | vb_storage::JournalEvent::RunFailedEvent { .. }
                | vb_storage::JournalEvent::RunCancelled { .. }
        )
    })
}

fn format_cancel_output(
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

fn write_cancel_event(
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

