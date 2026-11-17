//! CLI answer command transport.
#![forbid(unsafe_code)]

use crate::app_impl::prelude::*;
use crate::args::AnswerSelector;
use response::{answer_error, answer_response_with_identity};
use std::path::Path;
use vb_core::RunId;
use vb_ipc::server::IpcResponse;

const ANSWER_COMMAND: IpcCommand = IpcCommand::AnswerAsk;
const ANSWER_CORRELATION: u64 = 0;

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
        .send_command(ANSWER_COMMAND, ANSWER_CORRELATION, &payload)
        .map_err(|error| format!("error sending answer: {error}"))
}

fn receive_answer(client: &mut IpcClient, run_id: RunId, output: OutputFormat) -> ExitCode {
    match client.recv_response(vb_ipc::MaxPayloadBytes::DEFAULT) {
        Ok((header, response)) => answer_response_with_identity(
            header.command,
            header.correlation,
            response,
            run_id,
            output,
        ),
        Err(error) => answer_error(
            &format!("error receiving response: {error}"),
            output,
            CliExitCode::IpcError,
        ),
    }
}

mod response;

#[cfg(test)]
mod tests {
    use super::response::answer_response;
    include!("answer_tests.rs");
}
