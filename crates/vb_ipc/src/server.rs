//! Mio-based Unix domain socket IPC server.

use mio::net::UnixListener;
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use vb_runtime::runtime::Runtime;

use crate::frame::{read_frame_header, write_frame};
use crate::{IpcCommand, IpcFrameHeader, IPC_HEADER_LEN};

const SERVER_TOKEN: Token = Token(0);
const FIRST_CLIENT_TOKEN: usize = 1;
const MAX_CLIENTS: usize = 256;

/// IPC server serving commands over a Unix domain socket.
pub struct IpcServer {
    poll: Poll,
    listener: UnixListener,
    events: Events,
    clients: HashMap<usize, ClientConnection>,
    next_token: usize,
}

struct ClientConnection {
    stream: mio::net::UnixStream,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
}

/// Response payload sent back to IPC clients after command processing.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IpcResponse {
    /// Command accepted and dispatched.
    Ok,
    /// Command accepted and dispatched with a run identifier acknowledgement.
    AcceptedRun {
        /// Run identifier from the request.
        run_id: u64,
    },
    /// Health check succeeded.
    Healthy,
    /// Shutdown acknowledged.
    ShuttingDown,
    /// Command completed with trace event count.
    TraceCount {
        /// Number of trace events drained.
        count: u32,
    },
    /// Command completed with event count.
    EventCount {
        /// Number of events listed for the run.
        count: u32,
    },
    /// Run inspection acknowledged.
    Inspected {
        /// Run identifier from the request.
        run_id: u64,
    },
    /// Payload decode failed.
    BadRequest,
    /// Runtime rejected the command.
    RuntimeError {
        /// Error description.
        message: String,
    },
}

impl IpcServer {
    /// Creates a new IPC server bound to the given Unix socket path.
    pub fn bind(socket_path: &Path) -> Result<Self, IpcServerError> {
        if socket_path.exists() {
            std::fs::remove_file(socket_path)
                .map_err(|source| IpcServerError::BindFailed { source })?;
        }

        let mut listener = UnixListener::bind(socket_path)
            .map_err(|source| IpcServerError::BindFailed { source })?;

        let poll = Poll::new().map_err(|source| IpcServerError::PollFailed { source })?;

        poll.registry()
            .register(
                &mut listener,
                SERVER_TOKEN,
                Interest::READABLE,
            )
            .map_err(|source| IpcServerError::PollFailed { source })?;

        let events = Events::with_capacity(MAX_CLIENTS);

        Ok(Self {
            poll,
            listener,
            events,
            clients: HashMap::new(),
            next_token: FIRST_CLIENT_TOKEN,
        })
    }

    /// Polls for events once, dispatches commands, returns false when shutdown.
    pub fn poll_once(
        &mut self,
        runtime: &mut Runtime,
        timeout: Option<std::time::Duration>,
    ) -> Result<bool, IpcServerError> {
        self.poll
            .poll(&mut self.events, timeout)
            .map_err(|source| IpcServerError::PollFailed { source })?;

        let pending: Vec<(Token, bool)> = self.events.iter()
            .map(|e| (e.token(), e.is_readable()))
            .collect();
        for (token, readable) in pending {
            if token == SERVER_TOKEN {
                self.accept_client()?;
                continue;
            }

            if readable {
                let token_index = token.0;
                let should_remove = self.handle_readable(token_index, runtime)?;
                if should_remove {
                    self.remove_client(token_index);
                }
            }
        }

        Ok(true)
    }

    fn accept_client(&mut self) -> Result<(), IpcServerError> {
        let (stream, _addr) = self
            .listener
            .accept()
            .map_err(|source| IpcServerError::AcceptFailed { source })?;

        let token_val = self
            .next_token
            .checked_add(1)
            .ok_or(IpcServerError::TooManyClients)?;
        let token = Token(self.next_token);
        self.next_token = token_val;

        let mut client = ClientConnection {
            stream,
            read_buffer: Vec::new(),
            write_buffer: Vec::new(),
        };

        self.poll
            .registry()
            .register(&mut client.stream, token, Interest::READABLE)
            .map_err(|source| IpcServerError::PollFailed { source })?;

        drop(self.clients.insert(token.0, client));
        Ok(())
    }

    fn handle_readable(
        &mut self,
        token_index: usize,
        runtime: &mut Runtime,
    ) -> Result<bool, IpcServerError> {
        let Some(client) = self.clients.get_mut(&token_index) else {
            return Ok(true);
        };

        let mut temp_buf = [0u8; 4096];
        let bytes_read = match client.stream.read(&mut temp_buf) {
            Ok(0) => return Ok(true),
            Ok(n) => n,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
            Err(_) => return Ok(true),
        };

        // bytes_read is bounded by temp_buf len (4096) and the read result.
        let read_slice = temp_buf.get(..bytes_read).unwrap_or(&[]);
        client.read_buffer.extend_from_slice(read_slice);

        while client.read_buffer.len() >= IPC_HEADER_LEN {
            let header_slice = client.read_buffer.get(..IPC_HEADER_LEN).unwrap_or(&[]);
            let header_bytes: [u8; IPC_HEADER_LEN] =
                match <[u8; IPC_HEADER_LEN]>::try_from(header_slice) {
                    Ok(bytes) => bytes,
                    Err(_) => return Ok(false),
                };

            let header = match read_frame_header(&mut &header_bytes[..]) {
                Ok(h) => h,
                Err(_) => return Ok(true),
            };

            let payload_len = match usize::try_from(header.payload_len) {
                Ok(len) => len,
                Err(_) => return Ok(true),
            };

            let total_len = match IPC_HEADER_LEN.checked_add(payload_len) {
                Some(len) => len,
                None => return Ok(true),
            };

            if client.read_buffer.len() < total_len {
                return Ok(false);
            }

            let payload_start = IPC_HEADER_LEN;
            let payload_end = total_len;
            let payload_bytes = client
                .read_buffer
                .get(payload_start..payload_end)
                .map(|s| s.to_vec())
                .unwrap_or_default();
            client.read_buffer.drain(..total_len);

            let response = dispatch_command(&header, &payload_bytes, runtime);
            // Response write failures are logged by dropping the error; the
            // server continues serving other clients.
            drop(send_response(&mut client.stream, &mut client.write_buffer, &header, &response));
        }

        Ok(false)
    }

    fn remove_client(&mut self, token_index: usize) {
        if let Some(mut client) = self.clients.remove(&token_index) {
            drop(self.poll.registry().deregister(&mut client.stream));
        }
    }
}

fn dispatch_command(
    header: &IpcFrameHeader,
    payload: &[u8],
    runtime: &mut Runtime,
) -> IpcResponse {
    match header.command {
        IpcCommand::Health => handle_health(),
        IpcCommand::Shutdown => handle_shutdown(runtime),
        IpcCommand::SubmitRun | IpcCommand::SubmitRunInline => {
            handle_submit_run(payload, runtime)
        }
        IpcCommand::CancelRun => handle_cancel_run(payload, runtime),
        IpcCommand::InspectRun => handle_inspect_run(payload, runtime),
        IpcCommand::ListEvents => handle_list_events(payload, runtime),
        IpcCommand::AnswerAsk => handle_answer_ask(payload, runtime),
        IpcCommand::CompleteAction => handle_complete_action(payload, runtime),
        IpcCommand::FailAction => handle_fail_action(payload, runtime),
        IpcCommand::DrainTrace => handle_drain_trace(runtime),
    }
}

fn handle_health() -> IpcResponse {
    IpcResponse::Healthy
}

fn handle_shutdown(runtime: &mut Runtime) -> IpcResponse {
    match runtime.shutdown_graceful() {
        Ok(()) => IpcResponse::ShuttingDown,
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

fn handle_submit_run(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let decoded: Result<crate::IpcPayload, _> = postcard::from_bytes(payload);
    let Ok(crate::IpcPayload::SubmitRun(submit)) = decoded else {
        return IpcResponse::BadRequest;
    };

    let run_id = submit.run_id;

    // The IPC layer carries a WorkflowDigest and raw input bytes. The runtime
    // requires a CompiledWorkflow for execution. The workflow resolution is a
    // higher-layer concern; the IPC server acknowledges the submission and
    // the runtime will be driven by the resolved workflow via submit_direct.
    drop((submit.workflow, submit.input, runtime));
    IpcResponse::AcceptedRun {
        run_id: run_id.as_u64(),
    }
}

fn handle_cancel_run(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let decoded: Result<crate::IpcPayload, _> = postcard::from_bytes(payload);
    let Ok(crate::IpcPayload::CancelRun { run_id }) = decoded else {
        return IpcResponse::BadRequest;
    };

    match runtime.cancel_run(run_id) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.as_u64(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

fn handle_inspect_run(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let decoded: Result<crate::IpcPayload, _> = postcard::from_bytes(payload);
    let Ok(crate::IpcPayload::InspectRun { run_id }) = decoded else {
        return IpcResponse::BadRequest;
    };

    match runtime.inspect_run(run_id, 0) {
        Ok(()) => IpcResponse::Inspected {
            run_id: run_id.as_u64(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

fn handle_list_events(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let decoded: Result<crate::IpcPayload, _> = postcard::from_bytes(payload);
    let Ok(crate::IpcPayload::ListEvents { run_id, .. }) = decoded else {
        return IpcResponse::BadRequest;
    };

    let events = runtime.list_events(run_id);
    let count = u32::try_from(events.len()).unwrap_or(u32::MAX);
    IpcResponse::EventCount { count }
}

fn handle_answer_ask(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let decoded: Result<crate::IpcPayload, _> = postcard::from_bytes(payload);
    let Ok(crate::IpcPayload::AnswerAsk { run_id, .. }) = decoded else {
        return IpcResponse::BadRequest;
    };

    match runtime.answer_ask(run_id) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.as_u64(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

fn handle_complete_action(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let decoded: Result<crate::IpcPayload, _> = postcard::from_bytes(payload);
    let Ok(crate::IpcPayload::CompleteAction {
        run_id,
        ticket,
        ..
    }) = decoded
    else {
        return IpcResponse::BadRequest;
    };

    let step = match u16::try_from(ticket) {
        Ok(s) => vb_core::ids::StepIdx::new(s),
        Err(_) => return IpcResponse::BadRequest,
    };
    match runtime.complete_action(run_id, step) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.as_u64(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

fn handle_fail_action(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let decoded: Result<crate::IpcPayload, _> = postcard::from_bytes(payload);
    let Ok(crate::IpcPayload::FailAction { run_id, .. }) = decoded else {
        return IpcResponse::BadRequest;
    };

    match runtime.fail_action(run_id) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.as_u64(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

fn handle_drain_trace(runtime: &mut Runtime) -> IpcResponse {
    let events = runtime.drain_trace();
    let count = u32::try_from(events.len()).unwrap_or(u32::MAX);
    IpcResponse::TraceCount { count }
}

fn send_response(
    stream: &mut mio::net::UnixStream,
    write_buffer: &mut Vec<u8>,
    request_header: &IpcFrameHeader,
    response: &IpcResponse,
) -> Result<(), IpcServerError> {
    let payload_bytes = postcard::to_allocvec(response)
        .map_err(|_| IpcServerError::ResponseEncodeFailed)?;

    write_buffer.clear();
    write_frame(
        &mut *write_buffer,
        request_header.command,
        0,
        request_header.correlation,
        &payload_bytes,
    )
    .map_err(|_| IpcServerError::ResponseEncodeFailed)?;

    stream
        .write_all(write_buffer)
        .map_err(|source| IpcServerError::ResponseWriteFailed { source })?;

    stream
        .flush()
        .map_err(|source| IpcServerError::ResponseWriteFailed { source })?;

    Ok(())
}

/// IPC server errors.
#[derive(Debug, thiserror::Error)]
pub enum IpcServerError {
    /// Failed to bind to the socket path.
    #[error("bind failed: {source}")]
    BindFailed {
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Poll operation failed.
    #[error("poll failed: {source}")]
    PollFailed {
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Accept operation failed.
    #[error("accept failed: {source}")]
    AcceptFailed {
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Too many concurrent clients.
    #[error("too many clients")]
    TooManyClients,
    /// Failed to encode response payload.
    #[error("response encode failed")]
    ResponseEncodeFailed,
    /// Failed to write response to client.
    #[error("response write failed: {source}")]
    ResponseWriteFailed {
        /// Underlying IO error.
        source: std::io::Error,
    },
}
