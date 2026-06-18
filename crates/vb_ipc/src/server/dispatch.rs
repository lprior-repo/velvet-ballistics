#![forbid(unsafe_code)]
//! IPC command dispatch.

use vb_runtime::runtime::Runtime;

use super::error::IpcServerError;
use crate::server::{IpcServer, WorkflowResolver};

/// Serves one IPC polling turn on an existing server.
pub fn serve_ipc(
    server: &mut IpcServer,
    runtime: &mut Runtime,
    timeout: Option<std::time::Duration>,
) -> Result<bool, IpcServerError> {
    server.poll_once(runtime, timeout)
}

/// Serves one IPC polling turn with workflow resolution for submit commands.
pub fn serve_ipc_with_resolver(
    server: &mut IpcServer,
    runtime: &mut Runtime,
    timeout: Option<std::time::Duration>,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> Result<bool, IpcServerError> {
    server.poll_once_with_resolver(runtime, timeout, resolver)
}

use super::handlers::SubmitCommand;
use super::handlers::{
    handle_answer_ask, handle_cancel_run, handle_complete_action, handle_fail_action,
    handle_health, handle_inspect_run, handle_list_events, handle_shutdown, handle_submit_run,
    handle_submit_run_inline,
};
use super::trace::handle_drain_trace;
use crate::IpcCommand;
use crate::server::IpcResponse;

#[cfg(test)]
pub fn dispatch_command(
    header: &crate::IpcFrameHeader,
    payload: &[u8],
    runtime: &mut Runtime,
) -> IpcResponse {
    dispatch_command_with_resolver(header, payload, runtime, None)
}

pub fn dispatch_command_with_resolver(
    header: &crate::IpcFrameHeader,
    payload: &[u8],
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    match header.command {
        IpcCommand::Health => handle_health(),
        IpcCommand::Shutdown => handle_shutdown(runtime),
        IpcCommand::SubmitRun => {
            handle_submit_run(SubmitCommand::SubmitRun, header, payload, runtime, resolver)
        }
        IpcCommand::SubmitRunInline => handle_submit_run_inline(payload, runtime, resolver),
        IpcCommand::CancelRun => handle_cancel_run(payload, runtime),
        IpcCommand::InspectRun => handle_inspect_run(payload, runtime),
        IpcCommand::ListEvents => handle_list_events(payload, runtime),
        IpcCommand::AnswerAsk => handle_answer_ask(payload, runtime),
        IpcCommand::CompleteAction => handle_complete_action(payload, runtime),
        IpcCommand::FailAction => handle_fail_action(payload, runtime),
        IpcCommand::DrainTrace => handle_drain_trace(payload, runtime),
        IpcCommand::UnknownCommand(_) => match unknown_command_static_response(header.command) {
            Some(response) => response.into_ipc_response(),
            None => IpcResponse::BadRequest,
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StaticIpcResponse {
    BadRequest,
}

impl StaticIpcResponse {
    fn into_ipc_response(self) -> IpcResponse {
        match self {
            Self::BadRequest => IpcResponse::BadRequest,
        }
    }
}

pub(crate) fn unknown_command_static_response(command: IpcCommand) -> Option<StaticIpcResponse> {
    match command {
        IpcCommand::UnknownCommand(_) => Some(StaticIpcResponse::BadRequest),
        _ => None,
    }
}
