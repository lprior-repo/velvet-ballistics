//! IPC server module.
//!
//! Submodules:
//! - impl_: IpcServer implementation
//! - helpers: buffer and frame helpers

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
pub mod helpers;
pub mod impl_;
pub mod ticket;
pub mod trace;

use crate::{
    IPC_HEADER_LEN, IpcActionOutputPayload, IpcCommand, IpcError, IpcFrameHeader, IpcPayload,
    IpcTraceEvent, IpcTraceEventKind, MaxPayloadBytes, RunSummary, SubmitRunPayload,
};
pub use error::IpcServerError;
use handlers::{
    handle_answer_ask, handle_cancel_run, handle_complete_action, handle_fail_action,
    handle_health, handle_inspect_run, handle_list_events, handle_shutdown, handle_submit_run,
};

const SERVER_TOKEN: mio::Token = mio::Token(0);
const FIRST_CLIENT_TOKEN: usize = 1;
const MAX_CLIENTS: usize = 256;
const READ_CHUNK_BYTES: usize = 4096;

/// IPC server serving commands over a Unix domain socket.
pub struct IpcServer {
    poll: mio::Poll,
    listener: mio::net::UnixListener,
    events: mio::Events,
    clients: std::collections::HashMap<usize, ClientConnection>,
    next_token: usize,
}

struct ClientConnection {
    stream: mio::net::UnixStream,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
}

/// Response payload sent back to IPC clients after command processing.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// List of run summaries.
    RunList { runs: Vec<RunSummary> },
    /// Runtime metrics snapshot.
    Metrics(crate::RuntimeMetrics),
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

pub use helpers::{
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
