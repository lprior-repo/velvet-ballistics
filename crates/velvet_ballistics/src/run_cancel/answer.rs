//! CLI answer command transport.
#![forbid(unsafe_code)]

use crate::app_impl::prelude::*;
use crate::args::AnswerSelector;
use std::path::Path;
use vb_core::RunId;
use vb_ipc::server::IpcResponse;

pub(crate) fn cmd_answer(
    run_id: &str,
    selector: AnswerSelector,
    value_file: &Path,
    db: &Path,
    output: OutputFormat,
) -> ExitCode {
    let request = match prepare_answer(run_id, selector, value_file) {
        Ok(request) => request,
        Err(error) => return answer_error(&error, output, CliExitCode::ValidationFailed),
    };
    let mut client = match connect_answer(db) {
        Ok(client) => client,
        Err(error) => return answer_error(&error, output, CliExitCode::IpcError),
    };
    if let Err(error) = send_answer(&mut client, request.run_id, request.ticket, request.bytes) {
        return answer_error(&error, output, CliExitCode::IpcError);
    }
    receive_answer(&mut client, request.run_id, output)
}

struct PreparedAnswer {
    run_id: RunId,
    ticket: u64,
    bytes: Vec<u8>,
}

fn prepare_answer(
    run_id: &str,
    selector: AnswerSelector,
    value_file: &Path,
) -> Result<PreparedAnswer, String> {
    let run_id = parse_answer_run(run_id)?;
    let ticket = selector
        .wire_ticket()
        .ok_or_else(|| String::from("answer selector encoding overflow"))?;
    let bytes = read_answer_file(value_file)?;
    Ok(PreparedAnswer {
        run_id,
        ticket,
        bytes,
    })
}

fn connect_answer(db: &Path) -> Result<IpcClient, String> {
    let socket = db.with_extension("sock");
    IpcClient::connect(&socket).map_err(|error| {
        format!(
            "error connecting to IPC server at {}: {error}",
            socket.display()
        )
    })
}

fn parse_answer_run(run_id: &str) -> Result<RunId, String> {
    run_id
        .parse::<u64>()
        .map(RunId::new)
        .map_err(|error| format!("invalid run_id '{run_id}': {error}"))
}

fn read_answer_file(value_file: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(value_file)
        .map_err(|error| format!("error reading value file {}: {error}", value_file.display()))
}

fn send_answer(
    client: &mut IpcClient,
    run_id: RunId,
    ticket: u64,
    answer: Vec<u8>,
) -> Result<(), String> {
    let payload = IpcPayload::AnswerAsk {
        run_id,
        ticket,
        answer,
        taint: None,
    };
    client
        .send_command(IpcCommand::AnswerAsk, 0, &payload)
        .map_err(|error| format!("error sending answer: {error}"))
}

fn receive_answer(client: &mut IpcClient, run_id: RunId, output: OutputFormat) -> ExitCode {
    match client.recv_response(vb_ipc::MaxPayloadBytes::DEFAULT) {
        Ok((_header, response)) => answer_response(response, run_id, output),
        Err(error) => answer_error(
            &format!("error receiving response: {error}"),
            output,
            CliExitCode::IpcError,
        ),
    }
}

fn answer_response(response: IpcResponse, run_id: RunId, output: OutputFormat) -> ExitCode {
    match response {
        IpcResponse::AcceptedRun {
            run_id: accepted_run,
        } if accepted_run == run_id.get() => answer_accepted(run_id, output),
        IpcResponse::RuntimeError { message } => answer_prefixed_error(
            "runtime error",
            &message,
            output,
            CliExitCode::RuntimeFailed,
        ),
        IpcResponse::PayloadError { message, .. } => answer_prefixed_error(
            "payload error",
            &message,
            output,
            CliExitCode::ValidationFailed,
        ),
        other => answer_error(
            &format!("unexpected response: {other:?}"),
            output,
            CliExitCode::RuntimeFailed,
        ),
    }
}

fn answer_accepted(run_id: RunId, output: OutputFormat) -> ExitCode {
    if output != OutputFormat::Text {
        emit_json_or_return!(
            &serde_json::json!({ "success": true, "run_id": run_id.get() }),
            output
        );
    } else {
        outln!("answer accepted for run {}", run_id.get());
    }
    ExitCode::SUCCESS
}

fn answer_error(message: &str, output: OutputFormat, code: CliExitCode) -> ExitCode {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({ "success": false, "error": message }),
            output,
        );
    } else {
        errln!("{message}");
    }
    code.into()
}

fn answer_prefixed_error(
    prefix: &str,
    message: &str,
    output: OutputFormat,
    code: CliExitCode,
) -> ExitCode {
    if output == OutputFormat::Text {
        errln!("{prefix}: {message}");
        return code.into();
    }
    answer_error(message, output, code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_answer_run_identity_succeeds() {
        for id in [0, 1, u64::MAX] {
            assert_eq!(
                answer_response(
                    IpcResponse::AcceptedRun { run_id: id },
                    RunId::new(id),
                    OutputFormat::Text,
                ),
                ExitCode::SUCCESS,
            );
        }
    }

    #[test]
    fn mismatched_answer_run_identity_is_runtime_failure() {
        for (requested, accepted) in [(1, 2), (0, u64::MAX), (u64::MAX, 0)] {
            assert_eq!(
                answer_response(
                    IpcResponse::AcceptedRun { run_id: accepted },
                    RunId::new(requested),
                    OutputFormat::Text,
                ),
                ExitCode::from(CliExitCode::RuntimeFailed),
            );
        }
    }

    #[test]
    fn runtime_error_response_preserves_exit_status() {
        assert_eq!(
            answer_response(
                IpcResponse::RuntimeError {
                    message: String::from("runtime rejected answer"),
                },
                RunId::new(1),
                OutputFormat::Text,
            ),
            ExitCode::from(CliExitCode::RuntimeFailed),
        );
    }

    #[test]
    fn payload_error_response_preserves_exit_status() {
        assert_eq!(
            answer_response(
                IpcResponse::PayloadError {
                    diagnostic: 1,
                    message: String::from("invalid answer"),
                },
                RunId::new(1),
                OutputFormat::Text,
            ),
            ExitCode::from(CliExitCode::ValidationFailed),
        );
    }

    #[test]
    fn unrelated_response_preserves_runtime_failure() {
        assert_eq!(
            answer_response(IpcResponse::Healthy, RunId::new(1), OutputFormat::Text),
            ExitCode::from(CliExitCode::RuntimeFailed),
        );
    }
}
