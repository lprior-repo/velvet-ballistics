#![forbid(unsafe_code)]
//! IPC server module.
//!
//! Submodules:
//! - impl_: IpcServer implementation
//! - helpers: buffer and frame helpers

#![allow(unused_imports)]

use vb_core::action::{ActionFailure, ActionFailureCode, ActionTicket};
use vb_core::ids::{ActionId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::CompiledWorkflow;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{AskAnswer, AskTicket};
use vb_runtime::trace::TraceEvent;

pub mod dispatch;
pub mod error;
pub mod handlers;
pub(crate) mod helpers;
pub mod impl_;
pub mod ticket;
pub mod trace;

#[cfg(test)]
mod dispatch_tests;
#[cfg(test)]
mod impl_tests;

use crate::{
    IPC_HEADER_LEN, IpcActionOutputPayload, IpcCommand, IpcError, IpcFrameHeader, IpcPayload,
    IpcTraceEvent, IpcTraceEventKind, MaxPayloadBytes, SubmitRunPayload,
};
pub use error::IpcServerError;
use handlers::{
    handle_answer_ask, handle_cancel_run, handle_complete_action, handle_fail_action,
    handle_health, handle_inspect_run, handle_list_events, handle_shutdown, handle_submit_run,
};

pub const MAX_CLIENTS: usize = 256;

/// IPC server serving commands over a Unix domain socket.
pub struct IpcServer {
    poll: mio::Poll,
    listener: mio::net::UnixListener,
    events: mio::Events,
    clients: [Option<ClientConnection>; MAX_CLIENTS],
    #[cfg(test)]
    test_poll_result: Option<Result<bool, IpcServerError>>,
}

struct ClientConnection {
    stream: mio::net::UnixStream,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
    /// Magic validation state — starts as AwaitingMagic, transitions to MagicValidated.
    magic_state: helpers::MagicValidationState,
}

/// Response payload sent back to IPC clients after command processing.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum IpcResponse {
    /// Command accepted and dispatched with a run identifier acknowledgement.
    AcceptedRun { run_id: u64 },
    /// Health check succeeded.
    Healthy,
    /// Shutdown acknowledged.
    ShuttingDown,
    /// Command completed with trace event count.
    TraceCount { count: u32 },
    /// Command completed with typed run events.
    Events { events: Vec<IpcTraceEvent> },
    /// Run inspection acknowledged.
    Inspected { run_id: u64 },
    /// Payload decode failed.
    BadRequest,
    /// Typed IPC payload error.
    PayloadError { diagnostic: u16, message: String },
    /// The request payload variant did not match the frame command.
    CommandPayloadMismatch,
    /// The IPC layer needs a workflow resolver before it can submit the run.
    WorkflowResolutionRequired,
    /// The resolver could not provide a supported workflow artifact.
    WorkflowResolutionUnsupported,
    /// Resolved workflow did not match the request digest.
    WorkflowDigestMismatch,
    /// A runtime count exceeded the response field width.
    CountOutOfRange { actual: usize, limit: u32 },
    /// Frame decode failed before command dispatch.
    FrameError { message: String },
    /// Runtime rejected the command.
    RuntimeError { message: String },
}

/// Resolves compiled workflows for IPC submit commands.
pub trait WorkflowResolver {
    /// Returns the compiled workflow for an already-validated digest.
    fn resolve_workflow(
        &mut self,
        digest: vb_core::WorkflowDigest,
    ) -> Result<CompiledWorkflow, WorkflowResolutionError>;
}

/// Workflow resolution failed before runtime submission.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum WorkflowResolutionError {
    /// No resolver is wired into this IPC surface.
    #[error("workflow resolution required")]
    Required,
    /// The requested workflow digest is unknown.
    #[error("workflow not found")]
    NotFound,
    /// Resolver rejected the compiled workflow artifact.
    #[error("workflow artifact invalid")]
    InvalidArtifact,
}

pub(crate) use helpers::{
    append_read_bytes, borrow_workflow_resolver, extract_payload, frame_error_response,
    frame_total_len, read_buffer_header, send_response,
};

/// Serves one IPC polling turn on an existing server.
pub fn serve_ipc(
    server: &mut IpcServer,
    runtime: &mut Runtime,
    timeout: Option<std::time::Duration>,
) -> Result<bool, IpcServerError> {
    server.poll_once(runtime, timeout)
}

#[cfg(test)]
impl IpcServer {
    pub(crate) fn client_count(&self) -> usize {
        self.clients.iter().filter(|c| c.is_some()).count()
    }

    pub(crate) fn client_stream_mut(
        &mut self,
        token_index: usize,
    ) -> Option<&mut mio::net::UnixStream> {
        let index = token_index.checked_sub(1)?;
        self.clients.get_mut(index)?.as_mut().map(|c| &mut c.stream)
    }

    pub(crate) fn client_write_buffer_mut(&mut self, token_index: usize) -> Option<&mut Vec<u8>> {
        let index = token_index.checked_sub(1)?;
        self.clients
            .get_mut(index)?
            .as_mut()
            .map(|c| &mut c.write_buffer)
    }

    pub(crate) fn set_test_poll_once_result(&mut self, result: Result<bool, IpcServerError>) {
        self.test_poll_result = Some(result);
    }

    pub(crate) fn reregister_client(
        &mut self,
        token_index: usize,
        interest: mio::Interest,
    ) -> Result<(), IpcServerError> {
        let token = mio::Token(token_index);
        let index = token_index
            .checked_sub(1)
            .ok_or_else(|| IpcServerError::PollFailed {
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "client not found"),
            })?;
        let client = self
            .clients
            .get_mut(index)
            .and_then(|c| c.as_mut())
            .ok_or_else(|| IpcServerError::PollFailed {
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "client not found"),
            })?;
        self.poll
            .registry()
            .reregister(&mut client.stream, token, interest)
            .map_err(|source| IpcServerError::PollFailed { source })
    }
}
