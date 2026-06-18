#![forbid(unsafe_code)]
//! Answer operation: submit a postcard-encoded SlotValue to a running run via IPC.

use crate::args::OutputFormat;
use crate::exit_code::CliExitCode;
use crate::output::json_error;
use crate::emit_json_or_return;
use std::path::Path;
use std::process::ExitCode;
use vb_ipc::client::IpcClient;
use vb_ipc::{IpcCommand, IpcPayload};

pub(crate) fn cmd_answer(
    run_id: &str,
    slot: u16,
    value_file: &Path,
    db: &Path,
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
                    emit_json_or_return!(
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
