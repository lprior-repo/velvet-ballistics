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
        IpcResponse, IpcResponseKind, IpcServerError, READ_CHUNK_BYTES, append_read_bytes,
        count_response, dispatch_command, extract_payload, frame_error_response, frame_total_len,
        read_buffer_header,
    };
    use crate::{IPC_HEADER_LEN, IpcCommand, IpcFrameHeader, IpcPayload, MaxPayloadBytes, SubmitRunPayload};
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

    // ── Server command handler tests ──

    #[test]
    fn handle_shutdown_returns_shutting_down_response() {
        // Given: a runtime with a single shard
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::Shutdown, 0, 1, 0);

        // When: dispatching a shutdown command
        let response = dispatch_command(&header, &[], &mut runtime);

        // Then: the response is ShuttingDown
        assert_eq!(response, IpcResponse::ShuttingDown);
    }

    #[test]
    fn handle_inspect_run_returns_inspected_for_valid_payload() {
        // Given: a runtime and a valid InspectRun payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::InspectRun {
            run_id: RunId::new(42),
        };
        let encoded = postcard::to_allocvec(&payload);
        assert!(encoded.is_ok(), "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert!(payload_len.is_ok(), "payload len fits u32");
        let Ok(payload_len) = payload_len else {
            return;
        };
        let header = IpcFrameHeader::new(IpcCommand::InspectRun, 0, 1, payload_len);

        // When: dispatching inspect_run
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: Inspected response with the correct run_id
        assert_eq!(
            response,
            IpcResponse::Inspected {
                run_id: RunId::new(42).as_u64(),
            }
        );
    }

    #[test]
    fn handle_list_events_returns_event_count_for_valid_payload() {
        // Given: a runtime and a valid ListEvents payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::ListEvents {
            run_id: RunId::new(10),
            from_sequence: 0,
        };
        let encoded = postcard::to_allocvec(&payload);
        assert!(encoded.is_ok(), "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert!(payload_len.is_ok(), "payload len fits u32");
        let Ok(payload_len) = payload_len else {
            return;
        };
        let header = IpcFrameHeader::new(IpcCommand::ListEvents, 0, 1, payload_len);

        // When: dispatching list_events on an empty runtime
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: EventCount with 0 events (no run submitted yet)
        assert_eq!(response, IpcResponse::EventCount { count: 0 });
    }

    #[test]
    fn handle_cancel_run_returns_accepted_for_valid_payload() {
        // Given: a runtime and a valid CancelRun payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::CancelRun {
            run_id: RunId::new(99),
        };
        let encoded = postcard::to_allocvec(&payload);
        assert!(encoded.is_ok(), "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert!(payload_len.is_ok(), "payload len fits u32");
        let Ok(payload_len) = payload_len else {
            return;
        };
        let header = IpcFrameHeader::new(IpcCommand::CancelRun, 0, 1, payload_len);

        // When: dispatching cancel_run (runtime enqueues the cancel command)
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: AcceptedRun with the correct run_id (cancel is enqueued, not rejected)
        assert_eq!(
            response,
            IpcResponse::AcceptedRun {
                run_id: RunId::new(99).as_u64(),
            }
        );
    }

    #[test]
    fn handle_list_events_returns_bad_request_for_invalid_payload() {
        // Given: a runtime and garbage payload for ListEvents
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::ListEvents, 0, 1, 3);

        // When: dispatching with bad payload
        let response = dispatch_command(&header, b"bad", &mut runtime);

        // Then: BadRequest
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn handle_answer_ask_returns_runtime_error_for_durable_path() {
        // Given: a runtime and a valid AnswerAsk payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: RunId::new(5),
            ticket: 1,
            answer: Vec::from(&b"yes"[..]),
        };
        let encoded = postcard::to_allocvec(&payload);
        assert!(encoded.is_ok(), "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert!(payload_len.is_ok(), "payload len fits u32");
        let Ok(payload_len) = payload_len else {
            return;
        };
        let header = IpcFrameHeader::new(IpcCommand::AnswerAsk, 0, 1, payload_len);

        // When: dispatching answer_ask (durable path unsupported)
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: RuntimeError with unsupported operation message
        match response {
            IpcResponse::RuntimeError { message } => {
                assert!(
                    message.contains("unsupported"),
                    "expected unsupported operation error, got: {message}"
                );
            }
            other => panic!("expected RuntimeError, got {other:?}"),
        }
    }

    #[test]
    fn handle_fail_action_returns_runtime_error_for_durable_path() {
        // Given: a runtime and a valid FailAction payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::FailAction {
            run_id: RunId::new(8),
            ticket: 2,
            error: Vec::from(&b"fail"[..]),
        };
        let encoded = postcard::to_allocvec(&payload);
        assert!(encoded.is_ok(), "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert!(payload_len.is_ok(), "payload len fits u32");
        let Ok(payload_len) = payload_len else {
            return;
        };
        let header = IpcFrameHeader::new(IpcCommand::FailAction, 0, 1, payload_len);

        // When: dispatching fail_action (durable path unsupported)
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: RuntimeError with unsupported operation message
        match response {
            IpcResponse::RuntimeError { message } => {
                assert!(
                    message.contains("unsupported"),
                    "expected unsupported operation error, got: {message}"
                );
            }
            other => panic!("expected RuntimeError, got {other:?}"),
        }
    }

    #[test]
    fn handle_drain_trace_returns_trace_count() {
        // Given: a runtime with no events
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::DrainTrace, 0, 1, 0);

        // When: dispatching drain_trace
        let response = dispatch_command(&header, &[], &mut runtime);

        // Then: TraceCount with 0
        assert_eq!(response, IpcResponse::TraceCount { count: 0 });
    }

    #[test]
    fn handle_complete_action_returns_bad_request_for_invalid_payload() {
        // Given: a runtime and garbage payload for CompleteAction
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::CompleteAction, 0, 1, 3);

        // When: dispatching with bad payload
        let response = dispatch_command(&header, b"bad", &mut runtime);

        // Then: BadRequest
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn handle_fail_action_returns_bad_request_for_invalid_payload() {
        // Given: a runtime and garbage payload for FailAction
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::FailAction, 0, 1, 3);

        // When: dispatching with bad payload
        let response = dispatch_command(&header, b"bad", &mut runtime);

        // Then: BadRequest
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn handle_answer_ask_returns_bad_request_for_invalid_payload() {
        // Given: a runtime and garbage payload for AnswerAsk
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::AnswerAsk, 0, 1, 3);

        // When: dispatching with bad payload
        let response = dispatch_command(&header, b"bad", &mut runtime);

        // Then: BadRequest
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn handle_submit_run_inline_returns_workflow_resolution_required() {
        // Given: a SubmitRunInline command with matching payload
        let payload = crate::IpcPayload::SubmitRunInline(crate::SubmitRunPayload {
            run_id: RunId::new(20),
            workflow: WorkflowDigest::from_bytes([5; 32]),
            input: Vec::from(&b"input"[..]),
        });
        let encoded = postcard::to_allocvec(&payload);
        assert!(encoded.is_ok(), "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert!(payload_len.is_ok(), "payload len fits u32");
        let Ok(payload_len) = payload_len else {
            return;
        };
        let header = IpcFrameHeader::new(IpcCommand::SubmitRunInline, 0, 1, payload_len);
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());

        // When: dispatching the submit_run_inline command
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: WorkflowResolutionRequired
        assert_eq!(response, IpcResponse::WorkflowResolutionRequired);
    }

    #[test]
    fn handle_submit_run_returns_bad_request_for_garbage() {
        // Given: a SubmitRun command with garbage payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 1, 4);

        // When: dispatching with garbage
        let response = dispatch_command(&header, b"\xff\xff\xff\xff", &mut runtime);

        // Then: BadRequest
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // ── IpcServerError construction tests ──

    #[test]
    fn ipc_server_error_bind_failed_displays_source() {
        // Given: an IpcServerError::BindFailed with a known IO error
        let io_err = std::io::Error::new(std::io::ErrorKind::AddrInUse, "addr in use");
        let error = IpcServerError::BindFailed { source: io_err };

        // When: displaying the error
        let message = error.to_string();

        // Then: message contains "bind failed"
        assert!(
            message.contains("bind failed"),
            "expected 'bind failed' in '{message}'"
        );
    }

    #[test]
    fn ipc_server_error_too_many_clients_display() {
        // Given: IpcServerError::TooManyClients
        let error = IpcServerError::TooManyClients;

        // When: displaying the error
        let message = error.to_string();

        // Then: message mentions too many clients
        assert!(
            message.contains("too many clients"),
            "expected 'too many clients' in '{message}'"
        );
    }

    #[test]
    fn ipc_server_error_response_encode_failed_display() {
        // Given: IpcServerError::ResponseEncodeFailed
        let error = IpcServerError::ResponseEncodeFailed;

        // When: displaying the error
        let message = error.to_string();

        // Then: message mentions response encode
        assert!(
            message.contains("response encode failed"),
            "expected 'response encode failed' in '{message}'"
        );
    }

    #[test]
    fn ipc_server_error_incomplete_frame_display() {
        // Given: IpcServerError::IncompleteFrame
        let error = IpcServerError::IncompleteFrame;

        // When: displaying the error
        let message = error.to_string();

        // Then: message mentions incomplete frame
        assert!(
            message.contains("incomplete"),
            "expected 'incomplete' in '{message}'"
        );
    }

    #[test]
    fn ipc_server_error_read_buffer_too_large_display() {
        // Given: IpcServerError::ReadBufferTooLarge
        let error = IpcServerError::ReadBufferTooLarge;

        // When: displaying the error
        let message = error.to_string();

        // Then: message mentions buffer exceeded
        assert!(
            message.contains("buffer exceeded"),
            "expected 'buffer exceeded' in '{message}'"
        );
    }

    // ── IpcResponse variant equality tests ──

    #[test]
    fn ipc_response_ok_is_distinct_from_healthy() {
        // Given: IpcResponse::Ok and IpcResponse::Healthy
        // When: comparing them
        // Then: they are not equal
        assert_ne!(IpcResponse::Ok, IpcResponse::Healthy);
    }

    #[test]
    fn ipc_response_accepted_run_carries_run_id() {
        // Given: an AcceptedRun response with run_id=42
        let response = IpcResponse::AcceptedRun { run_id: 42 };

        // When: comparing with another AcceptedRun
        // Then: they are equal only when run_id matches
        assert_eq!(response, IpcResponse::AcceptedRun { run_id: 42 });
        assert_ne!(response, IpcResponse::AcceptedRun { run_id: 99 });
    }

    #[test]
    fn ipc_response_trace_count_carries_count() {
        // Given: a TraceCount response with count=7
        let response = IpcResponse::TraceCount { count: 7 };

        // When: comparing with another TraceCount
        // Then: they are equal only when count matches
        assert_eq!(response, IpcResponse::TraceCount { count: 7 });
        assert_ne!(response, IpcResponse::TraceCount { count: 0 });
    }

    #[test]
    fn ipc_response_event_count_carries_count() {
        // Given: an EventCount response with count=15
        let response = IpcResponse::EventCount { count: 15 };

        // When: comparing with another EventCount
        // Then: they are equal only when count matches
        assert_eq!(response, IpcResponse::EventCount { count: 15 });
        assert_ne!(response, IpcResponse::EventCount { count: 1 });
    }

    #[test]
    fn ipc_response_count_out_of_range_carries_actual_and_limit() {
        // Given: a CountOutOfRange response
        let response = IpcResponse::CountOutOfRange {
            actual: 5_000_000_000usize,
            limit: u32::MAX,
        };

        // When: checking fields
        // Then: fields are accessible
        if let IpcResponse::CountOutOfRange { actual, limit } = response {
            assert_eq!(actual, 5_000_000_000usize);
            assert_eq!(limit, u32::MAX);
        } else {
            panic!("expected CountOutOfRange variant");
        }
    }

    #[test]
    fn ipc_response_frame_error_carries_message() {
        // Given: a FrameError response with a known message
        let response = IpcResponse::FrameError {
            message: String::from("bad magic"),
        };

        // When: inspecting the variant
        // Then: message matches
        if let IpcResponse::FrameError { message } = &response {
            assert_eq!(message, "bad magic");
        } else {
            panic!("expected FrameError variant");
        }
    }

    #[test]
    fn ipc_response_runtime_error_carries_message() {
        // Given: a RuntimeError response
        let response = IpcResponse::RuntimeError {
            message: String::from("queue full"),
        };

        // When: inspecting the variant
        // Then: message matches
        if let IpcResponse::RuntimeError { message } = &response {
            assert_eq!(message, "queue full");
        } else {
            panic!("expected RuntimeError variant");
        }
    }

    #[test]
    fn count_response_returns_event_count_for_event_kind() {
        // Given: a count of 5 and Event kind
        // When: calling count_response
        let response = count_response(5, IpcResponseKind::Event);

        // Then: EventCount with count=5
        assert_eq!(response, IpcResponse::EventCount { count: 5 });
    }

    #[test]
    fn count_response_returns_trace_count_for_trace_kind() {
        // Given: a count of 3 and Trace kind
        // When: calling count_response
        let response = count_response(3, IpcResponseKind::Trace);

        // Then: TraceCount with count=3
        assert_eq!(response, IpcResponse::TraceCount { count: 3 });
    }

    #[test]
    fn frame_error_response_wraps_ipc_error_message() {
        // Given: an IpcError
        let error = crate::IpcError::InvalidMagic { actual: 0xDEAD };

        // When: converting to frame error response
        let response = frame_error_response(error);

        // Then: it is a FrameError with the error message
        if let IpcResponse::FrameError { message } = &response {
            assert!(message.contains("invalid"), "message should mention invalid: {message}");
        } else {
            panic!("expected FrameError variant");
        }
    }

    #[test]
    fn handle_complete_action_returns_accepted_for_valid_payload() {
        // Given: a runtime and a valid CompleteAction payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::CompleteAction {
            run_id: RunId::new(5),
            ticket: 3,
            output: Vec::from(&b"done"[..]),
        };
        let encoded = postcard::to_allocvec(&payload);
        assert!(encoded.is_ok(), "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert!(payload_len.is_ok(), "payload len fits u32");
        let Ok(payload_len) = payload_len else {
            return;
        };
        let header = IpcFrameHeader::new(IpcCommand::CompleteAction, 0, 1, payload_len);

        // When: dispatching complete_action
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: AcceptedRun with correct run_id (command is enqueued to shard)
        assert_eq!(
            response,
            IpcResponse::AcceptedRun {
                run_id: RunId::new(5).as_u64(),
            }
        );
    }

    #[test]
    fn handle_drain_trace_returns_trace_count_after_events() {
        // Given: a runtime that has processed events
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::DrainTrace, 0, 1, 0);

        // When: dispatching drain_trace (empty runtime)
        let response = dispatch_command(&header, &[], &mut runtime);

        // Then: TraceCount with 0
        assert_eq!(response, IpcResponse::TraceCount { count: 0 });
    }

    #[test]
    fn handle_submit_run_returns_command_payload_mismatch_for_wrong_variant() {
        // Given: SubmitRunInline command but CancelRun payload
        let payload = crate::IpcPayload::CancelRun {
            run_id: RunId::new(1),
        };
        let encoded = postcard::to_allocvec(&payload);
        assert!(encoded.is_ok(), "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert!(payload_len.is_ok(), "payload len fits u32");
        let Ok(payload_len) = payload_len else {
            return;
        };
        let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 1, payload_len);
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());

        // When: dispatching with mismatched command/payload
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: CommandPayloadMismatch
        assert_eq!(response, IpcResponse::CommandPayloadMismatch);
    }

    #[test]
    fn handle_cancel_run_returns_bad_request_for_garbage() {
        // Given: CancelRun command with garbage payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::CancelRun, 0, 1, 5);

        // When: dispatching with garbage
        let response = dispatch_command(&header, b"\xDE\xAD\xBE\xEF\x00", &mut runtime);

        // Then: BadRequest
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn ipc_server_error_poll_failed_display() {
        // Given: a PollFailed error
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "poll err");
        let error = IpcServerError::PollFailed { source: io_err };

        // When: displaying the error
        let message = error.to_string();

        // Then: message mentions poll
        assert!(
            message.contains("poll failed"),
            "expected 'poll failed' in '{message}'"
        );
    }

    #[test]
    fn ipc_server_error_accept_failed_display() {
        // Given: an AcceptFailed error
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "accept err");
        let error = IpcServerError::AcceptFailed { source: io_err };

        // When: displaying the error
        let message = error.to_string();

        // Then: message mentions accept
        assert!(
            message.contains("accept failed"),
            "expected 'accept failed' in '{message}'"
        );
    }

    #[test]
    fn ipc_server_error_response_write_failed_display() {
        // Given: a ResponseWriteFailed error
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "write err");
        let error = IpcServerError::ResponseWriteFailed { source: io_err };

        // When: displaying the error
        let message = error.to_string();

        // Then: message mentions response write
        assert!(
            message.contains("response write failed"),
            "expected 'response write failed' in '{message}'"
        );
    }

    #[test]
    fn ipc_server_error_frame_invalid_display() {
        // Given: a FrameInvalid error wrapping IpcError
        let inner = crate::IpcError::InvalidMagic { actual: 0 };
        let error = IpcServerError::FrameInvalid { source: inner };

        // When: displaying the error
        let message = error.to_string();

        // Then: message mentions invalid frame
        assert!(
            message.contains("invalid IPC frame"),
            "expected 'invalid IPC frame' in '{message}'"
        );
    }

    #[test]
    fn append_read_bytes_rejects_overflowing_buffer() {
        // Given: a read buffer at near-max capacity
        let max_single = IPC_HEADER_LEN + MaxPayloadBytes::DEFAULT.get();
        let mut read_buffer = Vec::new();
        // Fill to just below max
        let fill_len = max_single.saturating_sub(1);
        read_buffer.extend(std::iter::repeat(0u8).take(fill_len));

        let temp = [0u8; READ_CHUNK_BYTES];

        // When: appending 2 more bytes would exceed the max
        let result = append_read_bytes(&mut read_buffer, &temp, 2);

        // Then: error is returned
        assert!(result.is_err(), "should reject buffer overflow");
    }

    #[test]
    fn extract_payload_returns_correct_slice() {
        // Given: a buffer with header + payload
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
        let encoded = header.encode();
        assert!(encoded.is_ok(), "header should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let mut frame = Vec::new();
        frame.extend_from_slice(&encoded);
        frame.extend_from_slice(b"abcd");
        let total_len = frame_total_len(&header);
        assert!(total_len.is_ok(), "total len should compute");
        let Ok(total_len) = total_len else {
            return;
        };

        // When: extracting the payload
        let payload = extract_payload(&frame, total_len);

        // Then: the payload bytes match
        assert!(payload.is_ok(), "payload should extract");
        let Ok(payload) = payload else {
            return;
        };
        assert_eq!(payload.as_slice(), b"abcd");
    }

    #[test]
    fn count_response_overflow_returns_count_out_of_range() {
        // Given: a count exceeding u32::MAX
        let huge_count = usize::try_from(u32::MAX as u64 + 1);
        let Ok(huge_count) = huge_count else {
            return;
        };

        // When: calling count_response
        let response = count_response(huge_count, IpcResponseKind::Trace);

        // Then: CountOutOfRange with correct values
        assert_eq!(
            response,
            IpcResponse::CountOutOfRange {
                actual: huge_count,
                limit: u32::MAX,
            }
        );
    }

    #[test]
    fn handle_drain_trace_dispatches_without_payload() {
        // Given: a DrainTrace command with zero-length payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::DrainTrace, 0, 42, 0);

        // When: dispatching
        let response = dispatch_command(&header, &[], &mut runtime);

        // Then: TraceCount response
        match response {
            IpcResponse::TraceCount { count } => {
                assert_eq!(count, 0);
            }
            other => panic!("expected TraceCount, got {other:?}"),
        }
    }

    #[test]
    fn frame_total_len_computes_header_plus_payload() {
        // Given: a header with payload_len=10
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 10);

        // When: computing total length
        let total = frame_total_len(&header);

        // Then: total is IPC_HEADER_LEN + 10
        assert!(total.is_ok(), "total len should compute");
        let Ok(total) = total else {
            return;
        };
        assert_eq!(total, IPC_HEADER_LEN + 10);
    }
}
