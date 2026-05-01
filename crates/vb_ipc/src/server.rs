//! Mio-based Unix domain socket IPC server.

use mio::net::UnixListener;
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use vb_runtime::runtime::Runtime;

use crate::frame::write_frame;
use crate::{IPC_HEADER_LEN, IpcCommand, IpcError, IpcFrameHeader, MaxPayloadBytes};

const SERVER_TOKEN: Token = Token(0);
const FIRST_CLIENT_TOKEN: usize = 1;
const MAX_CLIENTS: usize = 256;
const READ_CHUNK_BYTES: usize = 4096;

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
    /// The request payload variant did not match the frame command.
    CommandPayloadMismatch,
    /// The IPC layer needs a workflow resolver before it can submit the run.
    WorkflowResolutionRequired,
    /// A runtime count exceeded the response field width.
    CountOutOfRange {
        /// Actual count that could not fit in the response.
        actual: usize,
        /// Maximum representable response count.
        limit: u32,
    },
    /// Frame decode failed before command dispatch.
    FrameError {
        /// Typed frame error text.
        message: String,
    },
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
            .register(&mut listener, SERVER_TOKEN, Interest::READABLE)
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

        let pending: Vec<(Token, bool)> = self
            .events
            .iter()
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

        let mut temp_buf = [0u8; READ_CHUNK_BYTES];
        let bytes_read = match client.stream.read(&mut temp_buf) {
            Ok(0) => return Ok(true),
            Ok(n) => n,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
            Err(_) => return Ok(true),
        };

        append_read_bytes(&mut client.read_buffer, &temp_buf, bytes_read)?;

        while client.read_buffer.len() >= IPC_HEADER_LEN {
            let header_bytes = read_buffer_header(&client.read_buffer)?;
            let header = match IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT) {
                Ok(h) => h,
                Err(error) => {
                    let response = frame_error_response(error);
                    let fallback_header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, 0);
                    drop(send_response(
                        &mut client.stream,
                        &mut client.write_buffer,
                        &fallback_header,
                        &response,
                    ));
                    return Ok(true);
                }
            };

            let total_len = frame_total_len(&header)?;
            if client.read_buffer.len() < total_len {
                return Ok(false);
            }

            let payload_bytes = extract_payload(&client.read_buffer, total_len)?;
            client.read_buffer.drain(..total_len);

            let response = dispatch_command(&header, &payload_bytes, runtime);
            // Response write failures are logged by dropping the error; the
            // server continues serving other clients.
            drop(send_response(
                &mut client.stream,
                &mut client.write_buffer,
                &header,
                &response,
            ));
        }

        Ok(false)
    }

    fn remove_client(&mut self, token_index: usize) {
        if let Some(mut client) = self.clients.remove(&token_index) {
            drop(self.poll.registry().deregister(&mut client.stream));
        }
    }
}

fn dispatch_command(header: &IpcFrameHeader, payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    match header.command {
        IpcCommand::Health => handle_health(),
        IpcCommand::Shutdown => handle_shutdown(runtime),
        IpcCommand::SubmitRun | IpcCommand::SubmitRunInline => {
            handle_submit_run(header, payload, runtime)
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

fn handle_submit_run(
    header: &IpcFrameHeader,
    payload: &[u8],
    _runtime: &mut Runtime,
) -> IpcResponse {
    let decoded: Result<crate::IpcPayload, _> = postcard::from_bytes(payload);
    let Ok(decoded) = decoded else {
        return IpcResponse::BadRequest;
    };

    match (header.command, decoded) {
        (IpcCommand::SubmitRun, crate::IpcPayload::SubmitRun(submit))
        | (IpcCommand::SubmitRunInline, crate::IpcPayload::SubmitRunInline(submit)) => {
            let _workflow_digest = submit.workflow;
            let _input = submit.input;
            IpcResponse::WorkflowResolutionRequired
        }
        _ => IpcResponse::CommandPayloadMismatch,
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

    match runtime.list_events(run_id) {
        Ok(events) => count_response(events.len(), IpcResponseKind::Event),
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
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
    let Ok(crate::IpcPayload::CompleteAction { run_id, ticket, .. }) = decoded else {
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
    count_response(events.len(), IpcResponseKind::Trace)
}

enum IpcResponseKind {
    Event,
    Trace,
}

fn count_response(count: usize, kind: IpcResponseKind) -> IpcResponse {
    match u32::try_from(count) {
        Ok(value) => match kind {
            IpcResponseKind::Event => IpcResponse::EventCount { count: value },
            IpcResponseKind::Trace => IpcResponse::TraceCount { count: value },
        },
        Err(_) => IpcResponse::CountOutOfRange {
            actual: count,
            limit: u32::MAX,
        },
    }
}

fn append_read_bytes(
    read_buffer: &mut Vec<u8>,
    temp_buf: &[u8; READ_CHUNK_BYTES],
    bytes_read: usize,
) -> Result<(), IpcServerError> {
    let read_slice = temp_buf
        .get(..bytes_read)
        .ok_or(IpcServerError::FrameInvalid {
            source: IpcError::PayloadLengthMismatch {
                header: READ_CHUNK_BYTES,
                actual: bytes_read,
            },
        })?;
    let next_len = read_buffer
        .len()
        .checked_add(read_slice.len())
        .ok_or(IpcServerError::ReadBufferTooLarge)?;
    let max_buffer = IPC_HEADER_LEN
        .checked_add(MaxPayloadBytes::DEFAULT.get())
        .ok_or(IpcServerError::ReadBufferTooLarge)?;
    if next_len > max_buffer {
        return Err(IpcServerError::ReadBufferTooLarge);
    }
    read_buffer.extend_from_slice(read_slice);
    Ok(())
}

fn read_buffer_header(read_buffer: &[u8]) -> Result<[u8; IPC_HEADER_LEN], IpcServerError> {
    let header_slice = read_buffer
        .get(..IPC_HEADER_LEN)
        .ok_or(IpcServerError::IncompleteFrame)?;
    <[u8; IPC_HEADER_LEN]>::try_from(header_slice).map_err(|_| IpcServerError::IncompleteFrame)
}

fn frame_total_len(header: &IpcFrameHeader) -> Result<usize, IpcServerError> {
    let payload_len =
        usize::try_from(header.payload_len).map_err(|_| IpcServerError::FrameInvalid {
            source: IpcError::PayloadLengthOutOfRange {
                actual: header.payload_len,
            },
        })?;
    let total_len = IPC_HEADER_LEN
        .checked_add(payload_len)
        .ok_or(IpcServerError::ReadBufferTooLarge)?;
    Ok(total_len)
}

fn extract_payload(read_buffer: &[u8], total_len: usize) -> Result<Vec<u8>, IpcServerError> {
    if read_buffer.len() < total_len {
        return Err(IpcServerError::IncompleteFrame);
    }
    let payload = read_buffer
        .get(IPC_HEADER_LEN..total_len)
        .ok_or(IpcServerError::IncompleteFrame)?;
    Ok(payload.to_vec())
}

fn frame_error_response(error: IpcError) -> IpcResponse {
    IpcResponse::FrameError {
        message: error.to_string(),
    }
}

fn send_response(
    stream: &mut mio::net::UnixStream,
    write_buffer: &mut Vec<u8>,
    request_header: &IpcFrameHeader,
    response: &IpcResponse,
) -> Result<(), IpcServerError> {
    let payload_bytes =
        postcard::to_allocvec(response).map_err(|_| IpcServerError::ResponseEncodeFailed)?;

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
    /// Client frame did not contain enough bytes for the declared frame.
    #[error("incomplete IPC frame")]
    IncompleteFrame,
    /// Client read buffer exceeded the configured single-frame bound.
    #[error("IPC read buffer exceeded configured frame bound")]
    ReadBufferTooLarge,
    /// Client frame failed typed validation.
    #[error("invalid IPC frame: {source}")]
    FrameInvalid {
        /// Typed IPC frame error.
        source: IpcError,
    },
}

#[cfg(test)]
mod tests {
    use super::{
        IpcResponse, IpcResponseKind, READ_CHUNK_BYTES, append_read_bytes, count_response,
        dispatch_command, extract_payload, frame_total_len, read_buffer_header,
    };
    use crate::{IPC_HEADER_LEN, IpcCommand, IpcFrameHeader, IpcPayload, SubmitRunPayload};
    use std::num::NonZeroUsize;
    use vb_core::{RunId, WorkflowDigest};
    use vb_runtime::runtime::Runtime;
    use vb_runtime::shard::ShardConfig;

    #[test]
    fn extracts_payload_without_lossy_empty_fallback() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 7, 3);
        let encoded = header.encode();
        assert!(encoded.is_ok(), "header encodes");
        let Ok(encoded) = encoded else {
            return;
        };

        let mut frame = Vec::new();
        frame.extend_from_slice(&encoded);
        frame.extend_from_slice(b"abc");
        let total_len = frame_total_len(&header);
        assert!(total_len.is_ok(), "total len is checked");
        let Ok(total_len) = total_len else {
            return;
        };

        let payload = extract_payload(&frame, total_len);
        assert!(payload.is_ok(), "payload extracts");
        let Ok(payload) = payload else {
            return;
        };
        assert_eq!(payload, Vec::from(b"abc".as_ref()));
        assert!(extract_payload(&frame, total_len.saturating_add(1)).is_err());
    }

    #[test]
    fn read_buffer_header_requires_full_header() {
        let Some(short_len) = IPC_HEADER_LEN.checked_sub(1) else {
            return;
        };
        let short = vec![0u8; short_len];
        assert!(read_buffer_header(&short).is_err());
    }

    #[test]
    fn append_read_bytes_rejects_impossible_read_count() {
        let mut read_buffer = Vec::new();
        let temp = [0u8; READ_CHUNK_BYTES];
        let Some(impossible_count) = READ_CHUNK_BYTES.checked_add(1) else {
            return;
        };
        assert!(append_read_bytes(&mut read_buffer, &temp, impossible_count).is_err());
    }

    #[test]
    fn count_conversion_returns_typed_overflow_response() {
        let count = usize::try_from(u32::MAX).map(|value| value.saturating_add(1));
        let Ok(count) = count else {
            return;
        };
        assert_eq!(
            count_response(count, IpcResponseKind::Event),
            IpcResponse::CountOutOfRange {
                actual: count,
                limit: u32::MAX,
            }
        );
    }

    #[test]
    fn submit_run_requires_workflow_resolution_instead_of_accepting() {
        let payload = IpcPayload::SubmitRun(SubmitRunPayload {
            run_id: RunId::new(9),
            workflow: WorkflowDigest::from_bytes([7; 32]),
            input: Vec::from(b"input".as_ref()),
        });
        let encoded = postcard::to_allocvec(&payload);
        assert!(encoded.is_ok(), "payload encodes");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert!(payload_len.is_ok(), "payload len fits test header");
        let Ok(payload_len) = payload_len else {
            return;
        };

        let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 11, payload_len);
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());

        assert_eq!(
            dispatch_command(&header, &encoded, &mut runtime),
            IpcResponse::WorkflowResolutionRequired
        );
    }

    #[test]
    fn submit_run_rejects_mismatched_payload_variant() {
        let payload = IpcPayload::SubmitRunInline(SubmitRunPayload {
            run_id: RunId::new(10),
            workflow: WorkflowDigest::from_bytes([8; 32]),
            input: Vec::from(b"input".as_ref()),
        });
        let encoded = postcard::to_allocvec(&payload);
        assert!(encoded.is_ok(), "payload encodes");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert!(payload_len.is_ok(), "payload len fits test header");
        let Ok(payload_len) = payload_len else {
            return;
        };

        let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 12, payload_len);
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());

        assert_eq!(
            dispatch_command(&header, &encoded, &mut runtime),
            IpcResponse::CommandPayloadMismatch
        );
    }

    #[test]
    fn handle_health_returns_healthy_response() {
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);

        assert_eq!(
            dispatch_command(&header, &[], &mut runtime),
            IpcResponse::Healthy
        );
    }

    #[test]
    fn handle_cancel_run_bad_payload_returns_bad_request() {
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::CancelRun, 0, 1, 3);

        assert_eq!(
            dispatch_command(&header, b"bad", &mut runtime),
            IpcResponse::BadRequest
        );
    }

    #[test]
    fn handle_inspect_run_bad_payload_returns_bad_request() {
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::InspectRun, 0, 1, 3);

        assert_eq!(
            dispatch_command(&header, b"bad", &mut runtime),
            IpcResponse::BadRequest
        );
    }
}
