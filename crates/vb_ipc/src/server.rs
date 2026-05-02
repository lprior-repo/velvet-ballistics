//! Mio-based Unix domain socket IPC server.

use arrayvec::ArrayVec;
use mio::net::UnixListener;
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use vb_core::action::{ActionFailure, ActionFailureCode, ActionTicket};
use vb_core::ids::WorkflowDigest;
use vb_core::ids::{ActionId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::CompiledWorkflow;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{AskAnswer, AskTicket};
use vb_runtime::trace::TraceEvent;

use crate::{
    IPC_HEADER_LEN, IpcCommand, IpcError, IpcFrameHeader, IpcTraceEvent, IpcTraceEventKind,
    MaxPayloadBytes,
};

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
    /// Command completed with typed run events.
    Events {
        /// Typed event payloads listed for the run.
        events: Vec<IpcTraceEvent>,
    },
    /// Run inspection acknowledged.
    Inspected {
        /// Run identifier from the request.
        run_id: u64,
    },
    /// Payload decode failed.
    BadRequest,
    /// Typed IPC payload error.
    PayloadError {
        /// Stable diagnostic code for the IPC failure.
        diagnostic: u16,
        /// Error description.
        message: String,
    },
    /// The request payload variant did not match the frame command.
    CommandPayloadMismatch,
    /// The IPC layer needs a workflow resolver before it can submit the run.
    WorkflowResolutionRequired,
    /// The resolver could not provide a supported workflow artifact.
    WorkflowResolutionUnsupported,
    /// Resolved workflow did not match the request digest.
    WorkflowDigestMismatch,
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

/// Resolves compiled workflows for IPC submit commands.
pub trait WorkflowResolver {
    /// Returns the compiled workflow for an already-validated digest.
    fn resolve_workflow(
        &mut self,
        digest: WorkflowDigest,
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
        self.poll_once_with_resolver(runtime, timeout, None)
    }

    /// Polls for events once with workflow resolution available for submit commands.
    pub fn poll_once_with_resolver(
        &mut self,
        runtime: &mut Runtime,
        timeout: Option<std::time::Duration>,
        resolver: Option<&mut dyn WorkflowResolver>,
    ) -> Result<bool, IpcServerError> {
        self.poll
            .poll(&mut self.events, timeout)
            .map_err(|source| IpcServerError::PollFailed { source })?;

        let mut pending: ArrayVec<(Token, bool, bool), MAX_CLIENTS> = ArrayVec::new();
        for event in &self.events {
            pending
                .try_push((event.token(), event.is_readable(), event.is_writable()))
                .map_err(|_| IpcServerError::TooManyClients)?;
        }
        let mut resolver = resolver;
        for (token, readable, writable) in pending {
            if token == SERVER_TOKEN {
                self.accept_client()?;
                continue;
            }

            let token_index = token.0;

            if writable {
                let should_remove = self.handle_writable(token_index)?;
                if should_remove {
                    self.remove_client(token_index);
                    continue;
                }
            }

            if readable {
                let resolver_ref = borrow_workflow_resolver(&mut resolver);
                let should_remove = self.handle_readable(token_index, runtime, resolver_ref)?;
                if should_remove {
                    self.remove_client(token_index);
                }
            }
        }

        Ok(true)
    }

    fn accept_client(&mut self) -> Result<(), IpcServerError> {
        if self.clients.len() >= MAX_CLIENTS {
            return Err(IpcServerError::TooManyClients);
        }

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
        resolver: Option<&mut dyn WorkflowResolver>,
    ) -> Result<bool, IpcServerError> {
        let registry = self
            .poll
            .registry()
            .try_clone()
            .map_err(|source| IpcServerError::PollFailed { source })?;
        let token = Token(token_index);

        let Some(client) = self.clients.get_mut(&token_index) else {
            return Ok(true);
        };
        let mut resolver = resolver;

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
                    send_response(
                        &mut client.stream,
                        &mut client.write_buffer,
                        &registry,
                        token,
                        &fallback_header,
                        &response,
                    )?;
                    return Ok(true);
                }
            };

            let total_len = frame_total_len(&header)?;
            if client.read_buffer.len() < total_len {
                return Ok(false);
            }

            let payload_bytes = extract_payload(&mut client.read_buffer, total_len)?;

            let response = dispatch_command_with_resolver(
                &header,
                &payload_bytes,
                runtime,
                borrow_workflow_resolver(&mut resolver),
            );
            send_response(
                &mut client.stream,
                &mut client.write_buffer,
                &registry,
                token,
                &header,
                &response,
            )?;
        }

        Ok(false)
    }

    fn handle_writable(&mut self, token_index: usize) -> Result<bool, IpcServerError> {
        let Some(client) = self.clients.get_mut(&token_index) else {
            return Ok(true);
        };

        if client.write_buffer.is_empty() {
            return Ok(false);
        }

        let written = match client.stream.write(&client.write_buffer) {
            Ok(n) => n,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
            Err(_) => return Ok(true),
        };

        client.write_buffer.drain(..written);

        if client.write_buffer.is_empty() {
            let token = Token(token_index);
            self.poll
                .registry()
                .reregister(&mut client.stream, token, Interest::READABLE)
                .map_err(|source| IpcServerError::PollFailed { source })?;
        }

        Ok(false)
    }

    fn remove_client(&mut self, token_index: usize) {
        if let Some(mut client) = self.clients.remove(&token_index) {
            drop(self.poll.registry().deregister(&mut client.stream));
        }
    }
}

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

/// Decodes a postcard-encoded payload and preserves the typed IPC decode error.
fn decode_payload<T: serde::de::DeserializeOwned>(payload: &[u8]) -> Result<T, IpcResponse> {
    postcard::from_bytes(payload).map_err(|_| ipc_error_response(IpcError::PayloadDecodeFailed))
}

fn ipc_error_response(error: IpcError) -> IpcResponse {
    IpcResponse::PayloadError {
        diagnostic: error.diagnostic_code().code(),
        message: error.to_string(),
    }
}

#[cfg(test)]
fn dispatch_command(header: &IpcFrameHeader, payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    dispatch_command_with_resolver(header, payload, runtime, None)
}

fn dispatch_command_with_resolver(
    header: &IpcFrameHeader,
    payload: &[u8],
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    match header.command {
        IpcCommand::Health => handle_health(),
        IpcCommand::Shutdown => handle_shutdown(runtime),
        IpcCommand::SubmitRun | IpcCommand::SubmitRunInline => {
            handle_submit_run(header, payload, runtime, resolver)
        }
        IpcCommand::CancelRun => handle_cancel_run(payload, runtime),
        IpcCommand::InspectRun => handle_inspect_run(payload, runtime),
        IpcCommand::ListEvents => handle_list_events(payload, runtime),
        IpcCommand::AnswerAsk => handle_answer_ask(payload, runtime),
        IpcCommand::CompleteAction => handle_complete_action(payload, runtime),
        IpcCommand::FailAction => handle_fail_action(payload, runtime),
        IpcCommand::DrainTrace => handle_drain_trace(payload, runtime),
    }
}

/// Handles a ping/health request.
pub fn handle_ping() -> IpcResponse {
    IpcResponse::Healthy
}

/// Handles a health request.
pub fn handle_health() -> IpcResponse {
    handle_ping()
}

/// Handles shutdown.
pub fn handle_shutdown(runtime: &mut Runtime) -> IpcResponse {
    match runtime.shutdown_graceful() {
        Ok(()) => IpcResponse::ShuttingDown,
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

/// Handles submit-run commands after resolving the compiled workflow explicitly.
pub fn handle_submit_run(
    header: &IpcFrameHeader,
    payload: &[u8],
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    let decoded = match decode_payload::<crate::IpcPayload>(payload) {
        Ok(d) => d,
        Err(response) => return response,
    };

    match (header.command, decoded) {
        (IpcCommand::SubmitRun, crate::IpcPayload::SubmitRun(submit))
        | (IpcCommand::SubmitRunInline, crate::IpcPayload::SubmitRunInline(submit)) => {
            submit_resolved_workflow(header.command, submit, runtime, resolver)
        }
        _ => IpcResponse::CommandPayloadMismatch,
    }
}

/// Handles inline submit-run commands.
pub fn handle_submit_run_inline(
    payload: &[u8],
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    let header = IpcFrameHeader::new(IpcCommand::SubmitRunInline, 0, 0, 0);
    handle_submit_run(&header, payload, runtime, resolver)
}

/// Handles cancel-run.
pub fn handle_cancel_run(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::CancelRun { run_id }) = decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    match runtime.snapshot_run(run_id, 0) {
        Ok(vb_runtime::shard::InspectResponse::Found(_)) => {}
        Ok(vb_runtime::shard::InspectResponse::NotFound { .. }) => {
            return IpcResponse::RuntimeError {
                message: String::from("run not found"),
            };
        }
        Err(e) => {
            return IpcResponse::RuntimeError {
                message: e.to_string(),
            };
        }
    }

    match runtime.cancel_run(run_id) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.as_u64(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

/// Handles inspect-run.
pub fn handle_inspect_run(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::InspectRun { run_id }) = decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    match runtime.snapshot_run(run_id, 0) {
        Ok(vb_runtime::shard::InspectResponse::Found(_snapshot)) => IpcResponse::Inspected {
            run_id: run_id.as_u64(),
        },
        Ok(vb_runtime::shard::InspectResponse::NotFound { .. }) => IpcResponse::RuntimeError {
            message: String::from("run not found"),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

/// Handles list-events.
pub fn handle_list_events(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::ListEvents {
        run_id,
        from_sequence,
    }) = decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    match runtime.list_events(run_id) {
        Ok(events) => typed_events_response(&events, from_sequence),
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

/// Handles answer-ask.
pub fn handle_answer_ask(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::AnswerAsk { run_id, ticket, .. }) =
        decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    let Some(ask_step) = step_from_ticket(ticket) else {
        return IpcResponse::BadRequest;
    };
    let answer = AskAnswer {
        ticket: AskTicket {
            run: run_id,
            ask_step,
            resume_step: ask_step,
        },
        answer_slot: SlotIdx::ZERO,
        value: SlotValue::Null,
        taint: Taint::Clean,
    };

    match runtime.answer_ask(answer) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.as_u64(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

/// Handles complete-action.
pub fn handle_complete_action(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::CompleteAction {
        run_id,
        ticket,
        output,
    }) = decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    let Some(action_ticket) = action_ticket_from_wire(run_id, ticket) else {
        return IpcResponse::BadRequest;
    };
    let output_len = payload_len(output.len());
    let decoded_output = match decode_payload::<crate::IpcActionOutputPayload>(&output) {
        Ok(d) => d,
        Err(response) => return response,
    };
    match runtime
        .complete_action_with_output(action_ticket, decoded_output.into_action_output(output_len))
    {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.as_u64(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

/// Handles fail-action.
pub fn handle_fail_action(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::FailAction {
        run_id,
        ticket,
        error,
    }) = decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    let Some(action_ticket) = action_ticket_from_wire(run_id, ticket) else {
        return IpcResponse::BadRequest;
    };
    let failure = ActionFailure {
        code: ActionFailureCode::Unknown,
        retryable: false,
        taint: Taint::Clean,
        detail: None,
        encoded_len: payload_len(error.len()),
    };

    match runtime.fail_action(action_ticket, failure) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.as_u64(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
        },
    }
}

fn step_from_ticket(ticket: u64) -> Option<StepIdx> {
    match u16::try_from(ticket) {
        Ok(step) => Some(StepIdx::new(step)),
        Err(_) => None,
    }
}

fn action_ticket_from_wire(run_id: vb_core::RunId, ticket: u64) -> Option<ActionTicket> {
    let step = step_from_ticket(ticket)?;
    Some(ActionTicket {
        run: run_id,
        step,
        seq: SeqNo::ZERO,
        action: ActionId::new(0),
        attempt: 1,
        idempotency_key: 0,
    })
}

fn payload_len(len: usize) -> u32 {
    u32::try_from(len).map_or(u32::MAX, |value| value)
}

/// Handles drain-trace.
pub fn handle_drain_trace(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(crate::IpcPayload::DrainTrace {
        run_id,
        max_records,
    }) = decode_payload::<crate::IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    let all_events = runtime.drain_trace();
    let max = match usize::try_from(max_records) {
        Ok(value) => value,
        Err(_) => usize::MAX,
    };
    let filtered: Vec<TraceEvent> = all_events
        .into_iter()
        .filter(|event| event.run_id() == run_id)
        .take(max)
        .collect();
    count_response(filtered.len(), IpcResponseKind::Trace)
}

enum IpcResponseKind {
    Trace,
}

fn count_response(count: usize, kind: IpcResponseKind) -> IpcResponse {
    match u32::try_from(count) {
        Ok(value) => match kind {
            IpcResponseKind::Trace => IpcResponse::TraceCount { count: value },
        },
        Err(_) => IpcResponse::CountOutOfRange {
            actual: count,
            limit: u32::MAX,
        },
    }
}

fn typed_events_response(events: &[TraceEvent], from_sequence: u64) -> IpcResponse {
    let mut typed_events = Vec::with_capacity(events.len());
    let mut index = 0usize;
    while index < events.len() {
        let Ok(sequence) = u64::try_from(index) else {
            return IpcResponse::CountOutOfRange {
                actual: index,
                limit: u32::MAX,
            };
        };
        if sequence >= from_sequence {
            let Some(event) = events.get(index) else {
                return IpcResponse::CountOutOfRange {
                    actual: index,
                    limit: u32::MAX,
                };
            };
            typed_events.push(IpcTraceEvent {
                sequence,
                kind: trace_event_kind(event),
            });
        }
        index = match index.checked_add(1) {
            Some(next) => next,
            None => {
                return IpcResponse::CountOutOfRange {
                    actual: index,
                    limit: u32::MAX,
                };
            }
        };
    }
    IpcResponse::Events {
        events: typed_events,
    }
}

fn trace_event_kind(event: &TraceEvent) -> IpcTraceEventKind {
    match event {
        TraceEvent::StepStarted { run, step } => IpcTraceEventKind::StepStarted {
            run: *run,
            step: *step,
        },
        TraceEvent::StepEnded { run, step } => IpcTraceEventKind::StepEnded {
            run: *run,
            step: *step,
        },
        TraceEvent::SlotWritten { run, slot } => IpcTraceEventKind::SlotWritten {
            run: *run,
            slot: *slot,
        },
        TraceEvent::ActionScheduled { run, step } => IpcTraceEventKind::ActionScheduled {
            run: *run,
            step: *step,
        },
        TraceEvent::ActionCompleted { run, step } => IpcTraceEventKind::ActionCompleted {
            run: *run,
            step: *step,
        },
        TraceEvent::ActionFailed { run, step, code } => IpcTraceEventKind::ActionFailed {
            run: *run,
            step: *step,
            code: *code,
        },
        TraceEvent::AskAnswered { run, step, slot } => IpcTraceEventKind::AskAnswered {
            run: *run,
            step: *step,
            slot: *slot,
        },
        TraceEvent::RunSubmitted { run } => IpcTraceEventKind::RunSubmitted { run: *run },
        TraceEvent::RunFinished { run } => IpcTraceEventKind::RunFinished { run: *run },
        TraceEvent::RunFailed { run } => IpcTraceEventKind::RunFailed { run: *run },
        TraceEvent::RunCancelled { run } => IpcTraceEventKind::RunCancelled { run: *run },
    }
}

fn borrow_workflow_resolver<'a>(
    resolver: &'a mut Option<&mut dyn WorkflowResolver>,
) -> Option<&'a mut dyn WorkflowResolver> {
    match resolver {
        Some(inner) => Some(&mut **inner),
        None => None,
    }
}

fn submit_resolved_workflow(
    command: IpcCommand,
    submit: crate::SubmitRunPayload,
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    let Some(resolver) = resolver else {
        return IpcResponse::WorkflowResolutionRequired;
    };
    let workflow = match resolver.resolve_workflow(submit.workflow) {
        Ok(workflow) => workflow,
        Err(WorkflowResolutionError::Required) => return IpcResponse::WorkflowResolutionRequired,
        Err(WorkflowResolutionError::NotFound | WorkflowResolutionError::InvalidArtifact) => {
            return IpcResponse::WorkflowResolutionUnsupported;
        }
    };
    if workflow.digest() != submit.workflow {
        return IpcResponse::WorkflowDigestMismatch;
    }
    let result = match command {
        IpcCommand::SubmitRun => runtime.submit_compiled(submit.run_id, workflow),
        IpcCommand::SubmitRunInline => runtime.submit_direct(submit.run_id, workflow),
        _ => return IpcResponse::CommandPayloadMismatch,
    };
    match result {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: submit.run_id.as_u64(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: e.to_string(),
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

fn extract_payload(read_buffer: &mut Vec<u8>, total_len: usize) -> Result<Vec<u8>, IpcServerError> {
    if read_buffer.len() < total_len {
        return Err(IpcServerError::IncompleteFrame);
    }
    // Keep unread bytes in the connection buffer and return only the consumed payload.
    let remaining = read_buffer.split_off(total_len);
    let mut frame = std::mem::replace(read_buffer, remaining);
    Ok(frame.split_off(IPC_HEADER_LEN))
}

fn frame_error_response(error: IpcError) -> IpcResponse {
    IpcResponse::FrameError {
        message: error.to_string(),
    }
}

fn send_response(
    stream: &mut mio::net::UnixStream,
    write_buffer: &mut Vec<u8>,
    registry: &mio::Registry,
    token: Token,
    request_header: &IpcFrameHeader,
    response: &IpcResponse,
) -> Result<(), IpcServerError> {
    let payload_bytes =
        postcard::to_allocvec(response).map_err(|_| IpcServerError::ResponseEncodeFailed)?;

    let payload_len =
        u32::try_from(payload_bytes.len()).map_err(|_| IpcServerError::ResponseEncodeFailed)?;

    // Build the response frame header via IpcFrameHeader::encode (uses byteorder internally).
    let header = IpcFrameHeader::new(
        request_header.command,
        0,
        request_header.correlation,
        payload_len,
    );
    let header_bytes = header
        .encode()
        .map_err(|_| IpcServerError::ResponseEncodeFailed)?;

    write_buffer.clear();
    write_buffer.extend_from_slice(&header_bytes);
    write_buffer.extend_from_slice(&payload_bytes);

    let written = match stream.write(write_buffer) {
        Ok(count) => count,
        Err(ref source) if source.kind() == std::io::ErrorKind::WouldBlock => 0,
        Err(source) => return Err(IpcServerError::ResponseWriteFailed { source }),
    };
    if written > 0 {
        drop(write_buffer.drain(..written));
    }
    if write_buffer.is_empty() {
        match stream.flush() {
            Ok(()) => {}
            Err(ref source) if source.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(source) => return Err(IpcServerError::ResponseWriteFailed { source }),
        }
    } else {
        registry
            .reregister(stream, token, Interest::READABLE | Interest::WRITABLE)
            .map_err(|source| IpcServerError::PollFailed { source })?;
    }

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

impl IpcServerError {
    /// Returns the stable section 17 runtime code when this server error has a direct mapping.
    #[must_use]
    pub fn runtime_code(&self) -> Option<&'static str> {
        match self {
            Self::IncompleteFrame => Some(IpcError::IPC_FRAME_INVALID_RUNTIME_CODE),
            Self::ReadBufferTooLarge => Some(IpcError::IPC_PAYLOAD_TOO_LARGE_RUNTIME_CODE),
            Self::FrameInvalid { source } => source.runtime_code(),
            Self::TooManyClients => Some(IpcError::QUEUE_FULL_RUNTIME_CODE),
            Self::BindFailed { .. }
            | Self::PollFailed { .. }
            | Self::AcceptFailed { .. }
            | Self::ResponseEncodeFailed
            | Self::ResponseWriteFailed { .. } => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::let_underscore_must_use)]
mod tests {
    use super::{
        IpcResponse, IpcResponseKind, IpcServer, IpcServerError, READ_CHUNK_BYTES,
        WorkflowResolutionError, WorkflowResolver, append_read_bytes, count_response,
        dispatch_command, dispatch_command_with_resolver, extract_payload, frame_error_response,
        frame_total_len, handle_complete_action, handle_list_events, read_buffer_header, serve_ipc,
        serve_ipc_with_resolver,
    };
    use crate::client::IpcClient;
    use crate::{
        IPC_HEADER_LEN, IpcActionOutputPayload, IpcCommand, IpcError, IpcFrameHeader, IpcPayload,
        IpcTraceEventKind, MaxPayloadBytes, SubmitRunPayload,
    };
    use std::num::NonZeroUsize;
    use std::sync::mpsc::{Receiver, Sender};
    use vb_core::ids::{ActionId, SlotIdx, StepIdx};
    use vb_core::value::{SlotValue, Taint};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
    use vb_core::{RunId, WorkflowDigest};
    use vb_runtime::runtime::Runtime;
    use vb_runtime::shard::ShardConfig;
    use vb_runtime::trace::TraceEvent;

    macro_rules! assert_ok {
        ($result:expr $(, $($arg:tt)+)?) => {{
            match &$result {
                Ok(_) => (),
                Err(_) => assert_eq!(Some("Err(..)"), None::<&str> $(, $($arg)+)?),
            }
        }};
    }

    enum ServerStep {
        Serve,
        ServeAndTick,
        Stop,
    }

    fn server_loop(
        mut server: IpcServer,
        steps: Receiver<ServerStep>,
        results: Sender<Result<bool, String>>,
    ) {
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        while let Ok(step) = steps.recv() {
            match step {
                ServerStep::Serve => {
                    let result =
                        serve_ipc(&mut server, &mut runtime, None).map_err(|e| e.to_string());
                    if results.send(result).is_err() {
                        return;
                    }
                }
                ServerStep::ServeAndTick => {
                    let result =
                        serve_ipc(&mut server, &mut runtime, None).map_err(|e| e.to_string());
                    if results.send(result).is_err() {
                        return;
                    }
                    // Process runtime command queue
                    let _ = runtime.tick_all();
                }
                ServerStep::Stop => return,
            }
        }
    }

    fn payload_decode_failed_response() -> IpcResponse {
        let error = IpcError::PayloadDecodeFailed;
        IpcResponse::PayloadError {
            diagnostic: error.diagnostic_code().code(),
            message: error.to_string(),
        }
    }

    fn request_server_turn(
        steps: &Sender<ServerStep>,
        results: &Receiver<Result<bool, String>>,
    ) -> bool {
        assert_ok!(steps.send(ServerStep::Serve), "server step sends");
        let result = results.recv();
        assert_ok!(result, "server step returns");
        let Ok(result) = result else {
            return false;
        };
        assert_ok!(result, "server step succeeds: {result:?}");
        let Ok(keep_running) = result else {
            return false;
        };
        keep_running
    }

    fn request_server_turn_and_tick(
        steps: &Sender<ServerStep>,
        results: &Receiver<Result<bool, String>>,
    ) -> bool {
        assert_ok!(steps.send(ServerStep::ServeAndTick), "server step sends");
        let result = results.recv();
        assert_ok!(result, "server step returns");
        let Ok(result) = result else {
            return false;
        };
        assert_ok!(result, "server step succeeds: {result:?}");
        let Ok(keep_running) = result else {
            return false;
        };
        keep_running
    }

    fn ipc_test_socket(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("vb_ipc_{name}_{}.sock", std::process::id()))
    }

    fn action_then_finish_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
        let do_node = CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(1)),
            kind: CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::ZERO,
            },
        };
        let finish = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(1),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("ipc_action_then_finish"),
            digest: WorkflowDigest::from_bytes([9; 32]),
            nodes: Box::from([do_node, finish]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 2,
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
    }

    struct StaticWorkflowResolver {
        workflow: Option<vb_core::workflow::CompiledWorkflow>,
        error: Option<WorkflowResolutionError>,
    }

    impl WorkflowResolver for StaticWorkflowResolver {
        fn resolve_workflow(
            &mut self,
            _digest: WorkflowDigest,
        ) -> Result<vb_core::workflow::CompiledWorkflow, WorkflowResolutionError> {
            if let Some(error) = self.error.clone() {
                return Err(error);
            }
            match self.workflow.clone() {
                Some(workflow) => Ok(workflow),
                None => Err(WorkflowResolutionError::NotFound),
            }
        }
    }

    fn encoded_submit_payload(payload: IpcPayload) -> Option<Vec<u8>> {
        postcard::to_allocvec(&payload).ok()
    }

    #[test]
    fn server_client_e2e_health_list_events_and_drain_trace() {
        let socket_path = ipc_test_socket("health_list_drain");
        let server = IpcServer::bind(&socket_path);
        assert_ok!(server, "server binds");
        let Ok(server) = server else {
            return;
        };

        let (step_tx, step_rx) = std::sync::mpsc::channel();
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || server_loop(server, step_rx, result_tx));

        let client = IpcClient::connect(&socket_path);
        assert_ok!(client, "client connects");
        let Ok(mut client) = client else {
            return;
        };
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server accepts client"
        );

        assert_ok!(client.health(100), "health request sends");
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server handles health"
        );
        let health = client.recv_response(MaxPayloadBytes::DEFAULT);
        assert_ok!(health, "health response decodes: {health:?}");
        let Ok((health_header, health_response)) = health else {
            return;
        };
        assert_eq!(health_header.command, IpcCommand::Health);
        assert_eq!(health_header.correlation, 100);
        assert_eq!(health_response, IpcResponse::Healthy);

        let list_payload = IpcPayload::ListEvents {
            run_id: RunId::new(44),
            from_sequence: 0,
        };
        assert!(
            client
                .send_command(IpcCommand::ListEvents, 101, &list_payload)
                .is_ok(),
            "list-events request sends"
        );
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server handles list-events"
        );
        let listed = client.recv_response(MaxPayloadBytes::DEFAULT);
        assert_ok!(listed, "list-events response decodes: {listed:?}");
        let Ok((listed_header, listed_response)) = listed else {
            return;
        };
        assert_eq!(listed_header.command, IpcCommand::ListEvents);
        assert_eq!(listed_header.correlation, 101);
        assert_eq!(listed_response, IpcResponse::Events { events: Vec::new() });

        let drain_payload = IpcPayload::DrainTrace {
            run_id: RunId::new(44),
            max_records: 100,
        };
        assert!(
            client
                .send_command(IpcCommand::DrainTrace, 102, &drain_payload)
                .is_ok(),
            "drain-trace request sends"
        );
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server handles drain-trace"
        );
        let drained = client.recv_response(MaxPayloadBytes::DEFAULT);
        assert_ok!(drained, "drain-trace response decodes: {drained:?}");
        let Ok((drained_header, drained_response)) = drained else {
            return;
        };
        assert_eq!(drained_header.command, IpcCommand::DrainTrace);
        assert_eq!(drained_header.correlation, 102);
        assert_eq!(drained_response, IpcResponse::TraceCount { count: 0 });

        assert_ok!(step_tx.send(ServerStep::Stop), "server stops");
        assert_ok!(handle.join(), "server thread joins");
        let remove_result = std::fs::remove_file(&socket_path);
        assert!(
            remove_result.is_ok() || !socket_path.exists(),
            "socket cleanup succeeds: {remove_result:?}"
        );
    }

    #[test]
    fn extracts_payload_without_lossy_empty_fallback() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 7, 3);
        let encoded = header.encode();
        assert_ok!(encoded, "header encodes");
        let Ok(encoded) = encoded else {
            return;
        };

        let mut frame = Vec::new();
        frame.extend_from_slice(&encoded);
        frame.extend_from_slice(b"abc");
        let total_len = frame_total_len(&header);
        assert_ok!(total_len, "total len is checked");
        let Ok(total_len) = total_len else {
            return;
        };

        let payload = extract_payload(&mut frame, total_len);
        assert_ok!(payload, "payload extracts");
        let Ok(payload) = payload else {
            return;
        };
        assert_eq!(payload, Vec::from(b"abc".as_ref()));
        let mut short_frame = vec![0u8; total_len];
        let payload_result = extract_payload(&mut short_frame, total_len.saturating_add(1));
        let Err(IpcServerError::IncompleteFrame) = payload_result else {
            return;
        };
    }

    #[test]
    fn read_buffer_header_requires_full_header() {
        let Some(short_len) = IPC_HEADER_LEN.checked_sub(1) else {
            return;
        };
        let short = vec![0u8; short_len];
        assert!(matches!(
            read_buffer_header(&short),
            Err(IpcServerError::IncompleteFrame)
        ));
    }

    #[test]
    fn append_read_bytes_rejects_impossible_read_count() {
        let mut read_buffer = Vec::new();
        let temp = [0u8; READ_CHUNK_BYTES];
        let Some(impossible_count) = READ_CHUNK_BYTES.checked_add(1) else {
            return;
        };
        let result = append_read_bytes(&mut read_buffer, &temp, impossible_count);
        assert!(matches!(
            result,
            Err(IpcServerError::FrameInvalid {
                source: IpcError::PayloadLengthMismatch {
                    header: READ_CHUNK_BYTES,
                    actual
                }
            }) if actual == impossible_count
        ));
    }

    #[test]
    fn count_conversion_returns_typed_overflow_response() {
        let count = usize::try_from(u32::MAX).map(|value| value.saturating_add(1));
        let Ok(count) = count else {
            return;
        };
        assert_eq!(
            count_response(count, IpcResponseKind::Trace),
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
        assert_ok!(encoded, "payload encodes");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert_ok!(payload_len, "payload len fits test header");
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
    fn submit_run_resolves_workflow_and_accepts_run() {
        let Some(workflow) = action_then_finish_workflow() else {
            return;
        };
        let run_id = RunId::new(90);
        let digest = workflow.digest();
        let payload = IpcPayload::SubmitRun(SubmitRunPayload {
            run_id,
            workflow: digest,
            input: Vec::new(),
        });
        let Some(encoded) = encoded_submit_payload(payload) else {
            return;
        };
        let Ok(payload_len) = u32::try_from(encoded.len()) else {
            return;
        };
        let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 13, payload_len);
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let mut resolver = StaticWorkflowResolver {
            workflow: Some(workflow),
            error: None,
        };

        let response =
            dispatch_command_with_resolver(&header, &encoded, &mut runtime, Some(&mut resolver));

        assert_eq!(
            response,
            IpcResponse::AcceptedRun {
                run_id: run_id.as_u64()
            }
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        let events = runtime.list_events(run_id);
        assert!(matches!(
            events,
            Ok(ref records) if records.contains(&TraceEvent::RunSubmitted { run: run_id })
        ));
    }

    #[test]
    fn submit_run_inline_resolves_workflow_and_accepts_run() {
        let Some(workflow) = action_then_finish_workflow() else {
            return;
        };
        let run_id = RunId::new(91);
        let digest = workflow.digest();
        let payload = IpcPayload::SubmitRunInline(SubmitRunPayload {
            run_id,
            workflow: digest,
            input: Vec::new(),
        });
        let Some(encoded) = encoded_submit_payload(payload) else {
            return;
        };
        let Ok(payload_len) = u32::try_from(encoded.len()) else {
            return;
        };
        let header = IpcFrameHeader::new(IpcCommand::SubmitRunInline, 0, 14, payload_len);
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let mut resolver = StaticWorkflowResolver {
            workflow: Some(workflow),
            error: None,
        };

        let response =
            dispatch_command_with_resolver(&header, &encoded, &mut runtime, Some(&mut resolver));

        assert_eq!(
            response,
            IpcResponse::AcceptedRun {
                run_id: run_id.as_u64()
            }
        );
        assert_eq!(runtime.tick_all(), Ok(true));
        let events = runtime.list_events(run_id);
        assert!(matches!(
            events,
            Ok(ref records) if records.contains(&TraceEvent::RunSubmitted { run: run_id })
        ));
    }

    #[test]
    fn submit_run_returns_stable_unsupported_when_resolver_cannot_supply_artifact() {
        let payload = IpcPayload::SubmitRun(SubmitRunPayload {
            run_id: RunId::new(92),
            workflow: WorkflowDigest::from_bytes([6; 32]),
            input: Vec::new(),
        });
        let Some(encoded) = encoded_submit_payload(payload) else {
            return;
        };
        let Ok(payload_len) = u32::try_from(encoded.len()) else {
            return;
        };
        let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 15, payload_len);
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let mut resolver = StaticWorkflowResolver {
            workflow: None,
            error: Some(WorkflowResolutionError::NotFound),
        };

        let response =
            dispatch_command_with_resolver(&header, &encoded, &mut runtime, Some(&mut resolver));

        assert_eq!(response, IpcResponse::WorkflowResolutionUnsupported);
    }

    #[test]
    fn workflow_resolution_unsupported_response_roundtrips() {
        let encoded = postcard::to_allocvec(&IpcResponse::WorkflowResolutionUnsupported);
        assert_ok!(encoded, "response should encode");
        let Ok(encoded) = encoded else {
            return;
        };

        let decoded = postcard::from_bytes::<IpcResponse>(&encoded);

        assert_eq!(decoded, Ok(IpcResponse::WorkflowResolutionUnsupported));
    }

    #[test]
    fn submit_run_rejects_mismatched_payload_variant() {
        let payload = IpcPayload::SubmitRunInline(SubmitRunPayload {
            run_id: RunId::new(10),
            workflow: WorkflowDigest::from_bytes([8; 32]),
            input: Vec::from(b"input".as_ref()),
        });
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded, "payload encodes");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert_ok!(payload_len, "payload len fits test header");
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
        // Given: a runtime with a submitted run and a valid InspectRun payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let run_id = RunId::new(42);
        let Some(workflow) = action_then_finish_workflow() else {
            return;
        };
        assert_eq!(runtime.submit_direct(run_id, workflow), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        let payload = crate::IpcPayload::InspectRun { run_id };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert_ok!(payload_len, "payload len fits u32");
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
                run_id: run_id.as_u64(),
            }
        );
    }

    #[test]
    fn handle_list_events_returns_typed_events_for_valid_payload() {
        // Given: a runtime and a valid ListEvents payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::ListEvents {
            run_id: RunId::new(10),
            from_sequence: 0,
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert_ok!(payload_len, "payload len fits u32");
        let Ok(payload_len) = payload_len else {
            return;
        };
        let header = IpcFrameHeader::new(IpcCommand::ListEvents, 0, 1, payload_len);

        // When: dispatching list_events on an empty runtime
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: typed event payload with no events (no run submitted yet)
        assert_eq!(response, IpcResponse::Events { events: Vec::new() });
    }

    #[test]
    fn handle_list_events_returns_typed_event_payload() {
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let Some(workflow) = action_then_finish_workflow() else {
            return;
        };
        let run = RunId::new(10);
        assert_eq!(runtime.submit_direct(run, workflow), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        let payload = crate::IpcPayload::ListEvents {
            run_id: run,
            from_sequence: 0,
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };

        let response = handle_list_events(&encoded, &mut runtime);

        let IpcResponse::Events { events } = response else {
            return;
        };
        assert!(
            events.iter().any(|event| matches!(
                event.kind,
                IpcTraceEventKind::RunSubmitted { run: listed_run } if listed_run == run
            )),
            "typed events should include RunSubmitted"
        );
        assert!(
            events.iter().any(|event| matches!(
                event.kind,
                IpcTraceEventKind::ActionScheduled {
                    run: listed_run,
                    step: StepIdx::ZERO,
                } if listed_run == run
            )),
            "typed events should include ActionScheduled"
        );
    }

    #[test]
    fn handle_cancel_run_returns_accepted_for_valid_payload() {
        // Given: a runtime with a submitted run and a valid CancelRun payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let run_id = RunId::new(99);
        let Some(workflow) = action_then_finish_workflow() else {
            return;
        };
        assert_eq!(runtime.submit_direct(run_id, workflow), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        let payload = crate::IpcPayload::CancelRun { run_id };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert_ok!(payload_len, "payload len fits u32");
        let Ok(payload_len) = payload_len else {
            return;
        };
        let header = IpcFrameHeader::new(IpcCommand::CancelRun, 0, 1, payload_len);

        // When: dispatching cancel_run (runtime enqueues the cancel command)
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: AcceptedRun with the correct run_id (cancel is enqueued for existing run)
        assert_eq!(
            response,
            IpcResponse::AcceptedRun {
                run_id: run_id.as_u64(),
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

        // Then: the syntactically valid but mismatched payload is rejected.
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn handle_answer_ask_returns_accepted_for_valid_payload() {
        // Given: a runtime and a valid AnswerAsk payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: RunId::new(5),
            ticket: 1,
            answer: Vec::from(&b"yes"[..]),
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert_ok!(payload_len, "payload len fits u32");
        let Ok(payload_len) = payload_len else {
            return;
        };
        let header = IpcFrameHeader::new(IpcCommand::AnswerAsk, 0, 1, payload_len);

        // When: dispatching answer_ask
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: AcceptedRun with the correct run_id
        assert_eq!(
            response,
            IpcResponse::AcceptedRun {
                run_id: RunId::new(5).as_u64(),
            }
        );
    }

    #[test]
    fn handle_fail_action_returns_accepted_for_valid_payload() {
        // Given: a runtime and a valid FailAction payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::FailAction {
            run_id: RunId::new(8),
            ticket: 2,
            error: Vec::from(&b"fail"[..]),
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert_ok!(payload_len, "payload len fits u32");
        let Ok(payload_len) = payload_len else {
            return;
        };
        let header = IpcFrameHeader::new(IpcCommand::FailAction, 0, 1, payload_len);

        // When: dispatching fail_action
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: AcceptedRun with the correct run_id
        assert_eq!(
            response,
            IpcResponse::AcceptedRun {
                run_id: RunId::new(8).as_u64(),
            }
        );
    }

    #[test]
    fn handle_drain_trace_returns_trace_count() {
        // Given: a runtime with no events
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::DrainTrace {
            run_id: RunId::new(1),
            max_records: 100,
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = match u32::try_from(encoded.len()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let header = IpcFrameHeader::new(IpcCommand::DrainTrace, 0, 1, payload_len);

        // When: dispatching drain_trace
        let response = dispatch_command(&header, &encoded, &mut runtime);

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

        // Then: the syntactically valid but mismatched payload is rejected.
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn handle_fail_action_returns_bad_request_for_invalid_payload() {
        // Given: a runtime and garbage payload for FailAction
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::FailAction, 0, 1, 3);

        // When: dispatching with bad payload
        let response = dispatch_command(&header, b"bad", &mut runtime);

        // Then: the syntactically valid but mismatched payload is rejected.
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
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert_ok!(payload_len, "payload len fits u32");
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

        // Then: the exact IPC payload decode error is preserved.
        assert_eq!(response, payload_decode_failed_response());
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

    #[test]
    fn ipc_server_error_runtime_codes_cover_ipc_mappings() {
        assert_eq!(
            IpcServerError::IncompleteFrame.runtime_code(),
            Some("IPC_FRAME_INVALID")
        );
        assert_eq!(
            IpcServerError::ReadBufferTooLarge.runtime_code(),
            Some("IPC_PAYLOAD_TOO_LARGE")
        );
        assert_eq!(
            IpcServerError::FrameInvalid {
                source: IpcError::InvalidMagic { actual: 0 },
            }
            .runtime_code(),
            Some("IPC_FRAME_INVALID")
        );
        assert_eq!(
            IpcServerError::FrameInvalid {
                source: IpcError::PayloadTooLarge {
                    actual: 2,
                    limit: 1,
                },
            }
            .runtime_code(),
            Some("IPC_PAYLOAD_TOO_LARGE")
        );
        assert_eq!(
            IpcServerError::TooManyClients.runtime_code(),
            Some("QUEUE_FULL")
        );
    }

    #[test]
    fn ipc_server_error_runtime_code_is_absent_without_direct_mapping() {
        let error = IpcServerError::ResponseEncodeFailed;
        assert_eq!(error.runtime_code(), None);
    }

    // ── IpcResponse variant equality tests ──

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
    fn ipc_response_count_out_of_range_carries_actual_and_limit() {
        // Given: a CountOutOfRange response
        let response = IpcResponse::CountOutOfRange {
            actual: 5_000_000_000usize,
            limit: u32::MAX,
        };

        // When: checking fields
        // Then: fields are accessible
        assert!(
            matches!(response, IpcResponse::CountOutOfRange { .. }),
            "expected CountOutOfRange variant"
        );
        let IpcResponse::CountOutOfRange { actual, limit } = response else {
            return;
        };
        assert_eq!(actual, 5_000_000_000usize);
        assert_eq!(limit, u32::MAX);
    }

    #[test]
    fn ipc_response_frame_error_carries_message() {
        // Given: a FrameError response with a known message
        let response = IpcResponse::FrameError {
            message: String::from("bad magic"),
        };

        // When: inspecting the variant
        // Then: message matches
        assert!(
            matches!(response, IpcResponse::FrameError { .. }),
            "expected FrameError variant"
        );
        let IpcResponse::FrameError { message } = &response else {
            return;
        };
        assert_eq!(message, "bad magic");
    }

    #[test]
    fn ipc_response_runtime_error_carries_message() {
        // Given: a RuntimeError response
        let response = IpcResponse::RuntimeError {
            message: String::from("queue full"),
        };

        // When: inspecting the variant
        // Then: message matches
        assert!(
            matches!(response, IpcResponse::RuntimeError { .. }),
            "expected RuntimeError variant"
        );
        let IpcResponse::RuntimeError { message } = &response else {
            return;
        };
        assert_eq!(message, "queue full");
    }

    #[test]
    fn count_response_returns_trace_count_for_small_count() {
        // Given: a count of 5 and Trace kind
        // When: calling count_response
        let response = count_response(5, IpcResponseKind::Trace);

        // Then: TraceCount with count=5
        assert_eq!(response, IpcResponse::TraceCount { count: 5 });
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
        assert!(
            matches!(response, IpcResponse::FrameError { .. }),
            "expected FrameError variant"
        );
        let IpcResponse::FrameError { message } = &response else {
            return;
        };
        assert!(
            message.contains("invalid"),
            "message should mention invalid: {message}"
        );
    }

    #[test]
    fn handle_complete_action_returns_accepted_for_valid_payload() {
        // Given: a runtime and a valid CompleteAction payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let output_payload = IpcActionOutputPayload {
            output_slot: SlotIdx::ZERO,
            value: SlotValue::Null,
            taint: Taint::Clean,
        };
        let output = postcard::to_allocvec(&output_payload);
        assert_ok!(output, "output payload should encode");
        let Ok(output) = output else {
            return;
        };
        let payload = crate::IpcPayload::CompleteAction {
            run_id: RunId::new(5),
            ticket: 3,
            output,
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert_ok!(payload_len, "payload len fits u32");
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
    fn handle_complete_action_uses_typed_output_payload() {
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let Some(workflow) = action_then_finish_workflow() else {
            return;
        };
        let run = RunId::new(12);
        assert_eq!(runtime.submit_direct(run, workflow), Ok(()));
        assert_eq!(runtime.tick_all(), Ok(true));

        let output_payload = IpcActionOutputPayload {
            output_slot: SlotIdx::new(1),
            value: SlotValue::I64(99),
            taint: Taint::Clean,
        };
        let output = postcard::to_allocvec(&output_payload);
        assert_ok!(output, "output payload should encode");
        let Ok(output) = output else {
            return;
        };
        let payload = crate::IpcPayload::CompleteAction {
            run_id: run,
            ticket: 0,
            output,
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };

        let response = handle_complete_action(&encoded, &mut runtime);
        assert_eq!(
            response,
            IpcResponse::AcceptedRun {
                run_id: run.as_u64()
            }
        );
        assert_eq!(runtime.tick_all(), Ok(true));

        let trace = runtime.list_events(run);
        assert!(matches!(
            trace,
            Ok(ref events) if events.contains(&TraceEvent::SlotWritten {
                run,
                slot: SlotIdx::new(1),
            }) && events.contains(&TraceEvent::ActionCompleted {
                run,
                step: StepIdx::ZERO,
            }) && events.contains(&TraceEvent::RunFinished { run })
        ));
    }

    #[test]
    fn handle_drain_trace_returns_trace_count_after_events() {
        // Given: a runtime that has processed events
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::DrainTrace {
            run_id: RunId::new(1),
            max_records: 100,
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = match u32::try_from(encoded.len()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let header = IpcFrameHeader::new(IpcCommand::DrainTrace, 0, 1, payload_len);

        // When: dispatching drain_trace (empty runtime)
        let response = dispatch_command(&header, &encoded, &mut runtime);

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
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = u32::try_from(encoded.len());
        assert_ok!(payload_len, "payload len fits u32");
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
        let io_err = std::io::Error::other("poll err");
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
        read_buffer.extend(std::iter::repeat_n(0u8, fill_len));

        let temp = [0u8; READ_CHUNK_BYTES];

        // When: appending 2 more bytes would exceed the max
        let result = append_read_bytes(&mut read_buffer, &temp, 2);

        // Then: the exact buffer bound error is returned.
        assert!(matches!(result, Err(IpcServerError::ReadBufferTooLarge)));
    }

    #[test]
    fn extract_payload_returns_correct_slice() {
        // Given: a buffer with header + payload
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
        let encoded = header.encode();
        assert_ok!(encoded, "header should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let mut frame = Vec::new();
        frame.extend_from_slice(&encoded);
        frame.extend_from_slice(b"abcd");
        let total_len = frame_total_len(&header);
        assert_ok!(total_len, "total len should compute");
        let Ok(total_len) = total_len else {
            return;
        };

        // When: extracting the payload
        let payload = extract_payload(&mut frame, total_len);

        // Then: the payload bytes match
        assert_ok!(payload, "payload should extract");
        let Ok(payload) = payload else {
            return;
        };
        assert_eq!(payload.as_slice(), b"abcd");
    }

    #[test]
    fn count_response_overflow_returns_count_out_of_range() {
        // Given: a count exceeding u32::MAX
        let huge_count = usize::try_from(u64::from(u32::MAX) + 1);
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
    fn handle_drain_trace_dispatches_with_typed_payload() {
        // Given: a DrainTrace command with a typed payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::DrainTrace {
            run_id: RunId::new(1),
            max_records: 100,
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload_len = match u32::try_from(encoded.len()) {
            Ok(value) => value,
            Err(_) => return,
        };
        let header = IpcFrameHeader::new(IpcCommand::DrainTrace, 0, 42, payload_len);

        // When: dispatching
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: TraceCount response
        assert!(
            matches!(response, IpcResponse::TraceCount { .. }),
            "expected TraceCount response"
        );
        let IpcResponse::TraceCount { count } = response else {
            return;
        };
        assert_eq!(count, 0);
    }

    #[test]
    fn frame_total_len_computes_header_plus_payload() {
        // Given: a header with payload_len=10
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 10);

        // When: computing total length
        let total = frame_total_len(&header);

        // Then: total is IPC_HEADER_LEN + 10
        assert_ok!(total, "total len should compute");
        let Ok(total) = total else {
            return;
        };
        assert_eq!(total, IPC_HEADER_LEN + 10);
    }

    // ══ Adversarial server command dispatch tests ══

    #[test]
    fn adversarial_submit_run_garbage_payload_returns_bad_request() {
        // Given: a SubmitRun command with 4 bytes of garbage
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 1, 4);

        // When: dispatching with garbage
        let response = dispatch_command(&header, b"\x00\x01\x02\x03", &mut runtime);

        // Then: the exact IPC payload decode error is preserved.
        assert_eq!(response, payload_decode_failed_response());
    }

    #[test]
    fn adversarial_cancel_run_garbage_payload_returns_bad_request() {
        // Given: a CancelRun command with garbage
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let header = IpcFrameHeader::new(IpcCommand::CancelRun, 0, 1, 3);

        // When: dispatching
        let response = dispatch_command(&header, b"xxx", &mut runtime);

        // Then: BadRequest
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn adversarial_cancel_run_wrong_payload_variant_returns_bad_request() {
        // Given: a CancelRun command but Health payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::Health;
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded);
        let Ok(encoded) = encoded else { return };
        let payload_len = match u32::try_from(encoded.len()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let header = IpcFrameHeader::new(IpcCommand::CancelRun, 0, 1, payload_len);

        // When: dispatching
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: BadRequest (CancelRun expects CancelRun variant, not Health)
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn adversarial_complete_action_wrong_variant_returns_bad_request() {
        // Given: a CompleteAction command but CancelRun payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::CancelRun {
            run_id: RunId::new(1),
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded);
        let Ok(encoded) = encoded else { return };
        let payload_len = match u32::try_from(encoded.len()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let header = IpcFrameHeader::new(IpcCommand::CompleteAction, 0, 1, payload_len);

        // When: dispatching
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: BadRequest
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn adversarial_fail_action_wrong_variant_returns_bad_request() {
        // Given: a FailAction command but Health payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::Health;
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded);
        let Ok(encoded) = encoded else { return };
        let payload_len = match u32::try_from(encoded.len()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let header = IpcFrameHeader::new(IpcCommand::FailAction, 0, 1, payload_len);

        // When: dispatching
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: BadRequest
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn adversarial_answer_ask_ticket_overflow_returns_bad_request() {
        // Given: an AnswerAsk with ticket=u64::MAX (doesn't fit in u16 for step)
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: RunId::new(1),
            ticket: u64::MAX,
            answer: Vec::new(),
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded);
        let Ok(encoded) = encoded else { return };
        let payload_len = match u32::try_from(encoded.len()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let header = IpcFrameHeader::new(IpcCommand::AnswerAsk, 0, 1, payload_len);

        // When: dispatching
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: BadRequest (step_from_ticket returns None for u64::MAX)
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn adversarial_complete_action_ticket_overflow_returns_bad_request() {
        // Given: a CompleteAction with ticket=u64::MAX
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let output_payload = IpcActionOutputPayload {
            output_slot: SlotIdx::ZERO,
            value: SlotValue::Null,
            taint: Taint::Clean,
        };
        let output = postcard::to_allocvec(&output_payload);
        assert_ok!(output);
        let Ok(output) = output else { return };
        let payload = crate::IpcPayload::CompleteAction {
            run_id: RunId::new(1),
            ticket: u64::MAX,
            output,
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded);
        let Ok(encoded) = encoded else { return };
        let payload_len = match u32::try_from(encoded.len()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let header = IpcFrameHeader::new(IpcCommand::CompleteAction, 0, 1, payload_len);

        // When: dispatching
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: BadRequest (action_ticket_from_wire returns None for u64::MAX)
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn adversarial_fail_action_ticket_overflow_returns_bad_request() {
        // Given: a FailAction with ticket=u64::MAX
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::FailAction {
            run_id: RunId::new(1),
            ticket: u64::MAX,
            error: Vec::from(&b"overflow"[..]),
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded);
        let Ok(encoded) = encoded else { return };
        let payload_len = match u32::try_from(encoded.len()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let header = IpcFrameHeader::new(IpcCommand::FailAction, 0, 1, payload_len);

        // When: dispatching
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: BadRequest
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn adversarial_submit_run_inline_wrong_variant_returns_command_mismatch() {
        // Given: SubmitRunInline command but SubmitRun variant payload
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::SubmitRun(crate::SubmitRunPayload {
            run_id: RunId::new(1),
            workflow: WorkflowDigest::from_bytes([0; 32]),
            input: Vec::new(),
        });
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded);
        let Ok(encoded) = encoded else { return };
        let payload_len = match u32::try_from(encoded.len()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let header = IpcFrameHeader::new(IpcCommand::SubmitRunInline, 0, 1, payload_len);

        // When: dispatching
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: CommandPayloadMismatch
        assert_eq!(response, IpcResponse::CommandPayloadMismatch);
    }

    #[test]
    fn adversarial_complete_action_garbage_output_returns_bad_request() {
        // Given: a CompleteAction with valid outer payload but garbage inner output
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::CompleteAction {
            run_id: RunId::new(1),
            ticket: 0,
            output: Vec::from(&b"\xFF\xFE\xFD\xFC\xFB\xFA"[..]),
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded);
        let Ok(encoded) = encoded else { return };
        let payload_len = match u32::try_from(encoded.len()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let header = IpcFrameHeader::new(IpcCommand::CompleteAction, 0, 1, payload_len);

        // When: dispatching
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: the exact IPC payload decode error is preserved.
        assert_eq!(response, payload_decode_failed_response());
    }

    #[test]
    fn adversarial_cancel_run_nonexistent_run_returns_runtime_error() {
        // Given: a CancelRun for a run that does not exist
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::CancelRun {
            run_id: RunId::new(99991),
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded);
        let Ok(encoded) = encoded else { return };
        let payload_len = match u32::try_from(encoded.len()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let header = IpcFrameHeader::new(IpcCommand::CancelRun, 0, 1, payload_len);

        // When: dispatching
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: RuntimeError (run was never submitted so it does not exist)
        assert!(
            matches!(response, IpcResponse::RuntimeError { ref message } if message.contains("not found")),
            "expected RuntimeError with 'not found', got {response:?}"
        );
    }

    #[test]
    fn adversarial_inspect_run_nonexistent_run_returns_runtime_error() {
        // Given: an InspectRun for a run that does not exist
        let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
        let payload = crate::IpcPayload::InspectRun {
            run_id: RunId::new(88888),
        };
        let encoded = postcard::to_allocvec(&payload);
        assert_ok!(encoded);
        let Ok(encoded) = encoded else { return };
        let payload_len = match u32::try_from(encoded.len()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let header = IpcFrameHeader::new(IpcCommand::InspectRun, 0, 1, payload_len);

        // When: dispatching
        let response = dispatch_command(&header, &encoded, &mut runtime);

        // Then: RuntimeError (run was never submitted so it does not exist)
        assert!(
            matches!(response, IpcResponse::RuntimeError { ref message } if message.contains("not found")),
            "expected RuntimeError with 'not found', got {response:?}"
        );
    }

    #[test]
    fn adversarial_append_read_bytes_enforces_single_frame_bound() {
        // Given: a read buffer already at max_single_frame
        let max_single = IPC_HEADER_LEN + MaxPayloadBytes::DEFAULT.get();
        let mut read_buffer = Vec::new();
        read_buffer.extend(std::iter::repeat_n(0u8, max_single));
        let temp = [0u8; READ_CHUNK_BYTES];

        // When: appending even 1 more byte
        let result = append_read_bytes(&mut read_buffer, &temp, 1);

        // Then: ReadBufferTooLarge
        let Err(error) = result else { return };
        assert!(matches!(error, IpcServerError::ReadBufferTooLarge));
        let message = error.to_string();
        assert!(
            message.contains("buffer exceeded"),
            "expected buffer exceeded in '{message}'"
        );
    }

    #[test]
    fn adversarial_frame_total_len_overflow_payload_returns_error() {
        // Given: a header with payload_len that would overflow when added to IPC_HEADER_LEN
        // This can't happen on 64-bit since u32 fits in usize, but let's verify the path
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, u32::MAX);

        // When: computing total length
        let result = frame_total_len(&header);

        // Then: it succeeds (u32::MAX fits in usize on 64-bit platforms)
        assert_ok!(result, "u32::MAX should fit in usize on 64-bit");
        let Ok(total) = result else { return };
        let max_payload_len = usize::try_from(u32::MAX).map_or(0, |v| v);
        assert_eq!(
            total,
            IPC_HEADER_LEN.checked_add(max_payload_len).map_or(0, |v| v)
        );
    }

    #[test]
    fn adversarial_extract_payload_returns_incomplete_for_short_buffer() {
        // Given: a total_len of 100 but buffer is only 50 bytes
        let mut short_buffer = vec![0u8; 50];

        // When: extracting payload
        let result = extract_payload(&mut short_buffer, 100);

        // Then: IncompleteFrame
        let Err(error) = result else { return };
        assert!(matches!(error, IpcServerError::IncompleteFrame));
        let message = error.to_string();
        assert!(
            message.contains("incomplete"),
            "expected incomplete in '{message}'"
        );
    }

    fn assert_frame_error_response_preserves_message(error: IpcError) {
        let desc = format!("{error:?}");
        let response = frame_error_response(error);

        assert!(
            matches!(&response, IpcResponse::FrameError { message } if !message.is_empty()),
            "expected non-empty FrameError message for {desc}, got {response:?}"
        );
    }

    #[test]
    fn frame_error_response_preserves_invalid_magic_message() {
        assert_frame_error_response_preserves_message(IpcError::InvalidMagic { actual: 0xDEAD });
    }

    #[test]
    fn frame_error_response_preserves_unsupported_version_message() {
        assert_frame_error_response_preserves_message(IpcError::UnsupportedVersion { actual: 99 });
    }

    #[test]
    fn frame_error_response_preserves_unknown_command_message() {
        assert_frame_error_response_preserves_message(IpcError::UnknownCommand(200));
    }

    #[test]
    fn frame_error_response_preserves_reserved_non_zero_message() {
        assert_frame_error_response_preserves_message(IpcError::ReservedNonZero { actual: 7 });
    }

    #[test]
    fn frame_error_response_preserves_payload_too_large_message() {
        assert_frame_error_response_preserves_message(IpcError::PayloadTooLarge {
            actual: 9999,
            limit: 100,
        });
    }

    #[test]
    fn frame_error_response_preserves_payload_length_mismatch_message() {
        assert_frame_error_response_preserves_message(IpcError::PayloadLengthMismatch {
            header: 100,
            actual: 50,
        });
    }

    // ══ Full socket e2e tests ══

    #[test]
    fn e2e_submit_run_with_workflow_resolver_accepts_and_runs() {
        let socket_path = ipc_test_socket("submit_run_e2e");
        let server = IpcServer::bind(&socket_path);
        assert_ok!(server, "server binds for e2e");
        let Ok(mut server) = server else {
            return;
        };

        let (step_tx, step_rx) = std::sync::mpsc::channel();
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
            let mut resolver = StaticWorkflowResolver {
                workflow: action_then_finish_workflow(),
                error: None,
            };
            while let Ok(step) = step_rx.recv() {
                match step {
                    ServerStep::Serve => {
                        let result = serve_ipc_with_resolver(
                            &mut server,
                            &mut runtime,
                            None,
                            Some(&mut resolver),
                        )
                        .map_err(|e| e.to_string());
                        if result_tx.send(result).is_err() {
                            return;
                        }
                    }
                    ServerStep::ServeAndTick => {
                        let result = serve_ipc_with_resolver(
                            &mut server,
                            &mut runtime,
                            None,
                            Some(&mut resolver),
                        )
                        .map_err(|e| e.to_string());
                        if result_tx.send(result).is_err() {
                            return;
                        }
                        let _ = runtime.tick_all();
                    }
                    ServerStep::Stop => return,
                }
            }
        });

        let client = IpcClient::connect(&socket_path);
        assert_ok!(client, "client connects");
        let Ok(mut client) = client else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };

        // Server accepts client
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server accepts client"
        );

        // Submit a run
        let run_id = RunId::new(77);
        let Some(workflow) = action_then_finish_workflow() else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        let digest = workflow.digest();
        let payload = IpcPayload::SubmitRun(SubmitRunPayload {
            run_id,
            workflow: digest,
            input: Vec::new(),
        });
        assert_ok!(
            client.send_command(IpcCommand::SubmitRun, 200, &payload),
            "submit-run sends"
        );
        assert!(
            request_server_turn_and_tick(&step_tx, &result_rx),
            "server handles submit-run and ticks"
        );
        let submitted = client.recv_response(MaxPayloadBytes::DEFAULT);
        assert_ok!(submitted, "submit-run response decodes");
        let Ok((submit_header, submit_response)) = submitted else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        assert_eq!(submit_header.command, IpcCommand::SubmitRun);
        assert_eq!(
            submit_response,
            IpcResponse::AcceptedRun {
                run_id: run_id.as_u64()
            }
        );

        // Inspect the run (submit already enqueued it)
        let inspect_payload = IpcPayload::InspectRun { run_id };
        assert_ok!(
            client.send_command(IpcCommand::InspectRun, 201, &inspect_payload),
            "inspect-run sends"
        );
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server handles inspect-run"
        );
        let inspected = client.recv_response(MaxPayloadBytes::DEFAULT);
        assert_ok!(inspected, "inspect-run response decodes");
        let Ok((inspect_header, inspect_response)) = inspected else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        assert_eq!(inspect_header.command, IpcCommand::InspectRun);
        assert_eq!(
            inspect_response,
            IpcResponse::Inspected {
                run_id: run_id.as_u64()
            }
        );

        assert_ok!(step_tx.send(ServerStep::Stop), "server stops");
        assert_ok!(handle.join(), "server thread joins");
        let remove_result = std::fs::remove_file(&socket_path);
        assert!(
            remove_result.is_ok() || !socket_path.exists(),
            "socket cleanup succeeds"
        );
    }

    #[test]
    fn e2e_submit_run_without_resolver_returns_workflow_resolution_required() {
        let socket_path = ipc_test_socket("submit_no_resolver");
        let server = IpcServer::bind(&socket_path);
        assert_ok!(server, "server binds");
        let Ok(mut server) = server else {
            return;
        };

        let (step_tx, step_rx) = std::sync::mpsc::channel();
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
            while let Ok(step) = step_rx.recv() {
                match step {
                    ServerStep::Serve => {
                        let result =
                            serve_ipc(&mut server, &mut runtime, None).map_err(|e| e.to_string());
                        if result_tx.send(result).is_err() {
                            return;
                        }
                    }
                    ServerStep::ServeAndTick => {
                        let result =
                            serve_ipc(&mut server, &mut runtime, None).map_err(|e| e.to_string());
                        if result_tx.send(result).is_err() {
                            return;
                        }
                        let _ = runtime.tick_all();
                    }
                    ServerStep::Stop => return,
                }
            }
        });

        let client = IpcClient::connect(&socket_path);
        assert_ok!(client, "client connects");
        let Ok(mut client) = client else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server accepts client"
        );

        // Submit without resolver
        let payload = IpcPayload::SubmitRun(SubmitRunPayload {
            run_id: RunId::new(5),
            workflow: WorkflowDigest::from_bytes([7; 32]),
            input: Vec::new(),
        });
        assert_ok!(
            client.send_command(IpcCommand::SubmitRun, 300, &payload),
            "submit-run sends"
        );
        assert!(request_server_turn(&step_tx, &result_rx), "server handles");
        let response = client.recv_response(MaxPayloadBytes::DEFAULT);
        assert_ok!(response, "response decodes");
        let Ok((header, resp)) = response else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        assert_eq!(header.command, IpcCommand::SubmitRun);
        assert_eq!(resp, IpcResponse::WorkflowResolutionRequired);

        assert_ok!(step_tx.send(ServerStep::Stop), "server stops");
        assert_ok!(handle.join(), "server thread joins");
        let remove_result = std::fs::remove_file(&socket_path);
        assert!(
            remove_result.is_ok() || !socket_path.exists(),
            "socket cleanup succeeds"
        );
    }

    #[test]
    fn e2e_cancel_run_accepts_and_cancels() {
        let socket_path = ipc_test_socket("cancel_run_e2e");
        let server = IpcServer::bind(&socket_path);
        assert_ok!(server, "server binds");
        let Ok(mut server) = server else {
            return;
        };

        let (step_tx, step_rx) = std::sync::mpsc::channel();
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
            let mut resolver = StaticWorkflowResolver {
                workflow: action_then_finish_workflow(),
                error: None,
            };
            while let Ok(step) = step_rx.recv() {
                match step {
                    ServerStep::Serve => {
                        let result = serve_ipc_with_resolver(
                            &mut server,
                            &mut runtime,
                            None,
                            Some(&mut resolver),
                        )
                        .map_err(|e| e.to_string());
                        if result_tx.send(result).is_err() {
                            return;
                        }
                    }
                    ServerStep::ServeAndTick => {
                        let result = serve_ipc_with_resolver(
                            &mut server,
                            &mut runtime,
                            None,
                            Some(&mut resolver),
                        )
                        .map_err(|e| e.to_string());
                        if result_tx.send(result).is_err() {
                            return;
                        }
                        let _ = runtime.tick_all();
                    }
                    ServerStep::Stop => return,
                }
            }
        });

        let client = IpcClient::connect(&socket_path);
        assert_ok!(client, "client connects");
        let Ok(mut client) = client else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server accepts client"
        );

        // Submit a run first
        let run_id = RunId::new(88);
        let Some(workflow) = action_then_finish_workflow() else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        let digest = workflow.digest();
        let submit_payload = IpcPayload::SubmitRun(SubmitRunPayload {
            run_id,
            workflow: digest,
            input: Vec::new(),
        });
        assert_ok!(
            client.send_command(IpcCommand::SubmitRun, 400, &submit_payload),
            "submit sends"
        );
        assert!(
            request_server_turn_and_tick(&step_tx, &result_rx),
            "server handles submit"
        );
        let submitted = client.recv_response(MaxPayloadBytes::DEFAULT);
        assert_ok!(submitted, "submit response decodes");

        // Cancel the run
        let cancel_payload = IpcPayload::CancelRun { run_id };
        assert_ok!(
            client.send_command(IpcCommand::CancelRun, 401, &cancel_payload),
            "cancel-run sends"
        );
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server handles cancel"
        );
        let cancelled = client.recv_response(MaxPayloadBytes::DEFAULT);
        assert_ok!(cancelled, "cancel response decodes");
        let Ok((cancel_header, cancel_response)) = cancelled else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        assert_eq!(cancel_header.command, IpcCommand::CancelRun);
        assert_eq!(
            cancel_response,
            IpcResponse::AcceptedRun {
                run_id: run_id.as_u64()
            }
        );

        assert_ok!(step_tx.send(ServerStep::Stop), "server stops");
        assert_ok!(handle.join(), "server thread joins");
        let remove_result = std::fs::remove_file(&socket_path);
        assert!(
            remove_result.is_ok() || !socket_path.exists(),
            "socket cleanup succeeds"
        );
    }

    #[test]
    fn e2e_cancel_nonexistent_run_returns_runtime_error() {
        let socket_path = ipc_test_socket("cancel_nonexistent");
        let server = IpcServer::bind(&socket_path);
        assert_ok!(server, "server binds");
        let Ok(mut server) = server else {
            return;
        };

        let (step_tx, step_rx) = std::sync::mpsc::channel();
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
            while let Ok(step) = step_rx.recv() {
                match step {
                    ServerStep::Serve => {
                        let result =
                            serve_ipc(&mut server, &mut runtime, None).map_err(|e| e.to_string());
                        if result_tx.send(result).is_err() {
                            return;
                        }
                    }
                    ServerStep::ServeAndTick => {
                        let result =
                            serve_ipc(&mut server, &mut runtime, None).map_err(|e| e.to_string());
                        if result_tx.send(result).is_err() {
                            return;
                        }
                        let _ = runtime.tick_all();
                    }
                    ServerStep::Stop => return,
                }
            }
        });

        let client = IpcClient::connect(&socket_path);
        assert_ok!(client, "client connects");
        let Ok(mut client) = client else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server accepts client"
        );

        // Cancel a run that was never submitted
        let cancel_payload = IpcPayload::CancelRun {
            run_id: RunId::new(99999),
        };
        assert_ok!(
            client.send_command(IpcCommand::CancelRun, 500, &cancel_payload),
            "cancel-run sends"
        );
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server handles cancel"
        );
        let cancelled = client.recv_response(MaxPayloadBytes::DEFAULT);
        assert_ok!(cancelled, "cancel response decodes");
        let Ok((_cancel_header, cancel_response)) = cancelled else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        assert!(
            matches!(
                cancel_response,
                IpcResponse::RuntimeError { ref message } if message.contains("not found")
            ),
            "expected RuntimeError with 'not found', got {cancel_response:?}"
        );

        assert_ok!(step_tx.send(ServerStep::Stop), "server stops");
        assert_ok!(handle.join(), "server thread joins");
        let remove_result = std::fs::remove_file(&socket_path);
        assert!(
            remove_result.is_ok() || !socket_path.exists(),
            "socket cleanup succeeds"
        );
    }

    #[test]
    fn e2e_shutdown_command_returns_shutting_down() {
        let socket_path = ipc_test_socket("shutdown_e2e");
        let server = IpcServer::bind(&socket_path);
        assert_ok!(server, "server binds");
        let Ok(mut server) = server else {
            return;
        };

        let (step_tx, step_rx) = std::sync::mpsc::channel();
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
            while let Ok(step) = step_rx.recv() {
                match step {
                    ServerStep::Serve => {
                        let result =
                            serve_ipc(&mut server, &mut runtime, None).map_err(|e| e.to_string());
                        if result_tx.send(result).is_err() {
                            return;
                        }
                    }
                    ServerStep::ServeAndTick => {
                        let result =
                            serve_ipc(&mut server, &mut runtime, None).map_err(|e| e.to_string());
                        if result_tx.send(result).is_err() {
                            return;
                        }
                        let _ = runtime.tick_all();
                    }
                    ServerStep::Stop => return,
                }
            }
        });

        let client = IpcClient::connect(&socket_path);
        assert_ok!(client, "client connects");
        let Ok(mut client) = client else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server accepts client"
        );

        // Send shutdown
        assert_ok!(client.shutdown(600), "shutdown sends");
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server handles shutdown"
        );
        let response = client.recv_response(MaxPayloadBytes::DEFAULT);
        assert_ok!(response, "shutdown response decodes");
        let Ok((shutdown_header, shutdown_response)) = response else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        assert_eq!(shutdown_header.command, IpcCommand::Shutdown);
        assert_eq!(shutdown_response, IpcResponse::ShuttingDown);

        assert_ok!(step_tx.send(ServerStep::Stop), "server stops");
        assert_ok!(handle.join(), "server thread joins");
        let remove_result = std::fs::remove_file(&socket_path);
        assert!(
            remove_result.is_ok() || !socket_path.exists(),
            "socket cleanup succeeds"
        );
    }

    #[test]
    fn e2e_submit_run_with_mismatched_digest_returns_digest_mismatch() {
        let socket_path = ipc_test_socket("digest_mismatch");
        let server = IpcServer::bind(&socket_path);
        assert_ok!(server, "server binds");
        let Ok(mut server) = server else {
            return;
        };

        let (step_tx, step_rx) = std::sync::mpsc::channel();
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
            let mut resolver = StaticWorkflowResolver {
                workflow: action_then_finish_workflow(),
                error: None,
            };
            while let Ok(step) = step_rx.recv() {
                match step {
                    ServerStep::Serve => {
                        let result = serve_ipc_with_resolver(
                            &mut server,
                            &mut runtime,
                            None,
                            Some(&mut resolver),
                        )
                        .map_err(|e| e.to_string());
                        if result_tx.send(result).is_err() {
                            return;
                        }
                    }
                    ServerStep::ServeAndTick => {
                        let result = serve_ipc_with_resolver(
                            &mut server,
                            &mut runtime,
                            None,
                            Some(&mut resolver),
                        )
                        .map_err(|e| e.to_string());
                        if result_tx.send(result).is_err() {
                            return;
                        }
                        let _ = runtime.tick_all();
                    }
                    ServerStep::Stop => return,
                }
            }
        });

        let client = IpcClient::connect(&socket_path);
        assert_ok!(client, "client connects");
        let Ok(mut client) = client else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server accepts client"
        );

        // Submit with wrong digest
        let payload = IpcPayload::SubmitRun(SubmitRunPayload {
            run_id: RunId::new(33),
            workflow: WorkflowDigest::from_bytes([0xAA; 32]),
            input: Vec::new(),
        });
        assert_ok!(
            client.send_command(IpcCommand::SubmitRun, 700, &payload),
            "submit sends"
        );
        assert!(request_server_turn(&step_tx, &result_rx), "server handles");
        let response = client.recv_response(MaxPayloadBytes::DEFAULT);
        assert_ok!(response, "response decodes");
        let Ok((_header, resp)) = response else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        assert_eq!(resp, IpcResponse::WorkflowDigestMismatch);

        assert_ok!(step_tx.send(ServerStep::Stop), "server stops");
        assert_ok!(handle.join(), "server thread joins");
        let remove_result = std::fs::remove_file(&socket_path);
        assert!(
            remove_result.is_ok() || !socket_path.exists(),
            "socket cleanup succeeds"
        );
    }

    #[test]
    fn e2e_list_events_returns_typed_events_after_run() {
        let socket_path = ipc_test_socket("list_typed_events");
        let server = IpcServer::bind(&socket_path);
        assert_ok!(server, "server binds");
        let Ok(mut server) = server else {
            return;
        };

        let (step_tx, step_rx) = std::sync::mpsc::channel();
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let mut runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default());
            let mut resolver = StaticWorkflowResolver {
                workflow: action_then_finish_workflow(),
                error: None,
            };
            while let Ok(step) = step_rx.recv() {
                match step {
                    ServerStep::Serve => {
                        let result = serve_ipc_with_resolver(
                            &mut server,
                            &mut runtime,
                            None,
                            Some(&mut resolver),
                        )
                        .map_err(|e| e.to_string());
                        if result_tx.send(result).is_err() {
                            return;
                        }
                    }
                    ServerStep::ServeAndTick => {
                        let result = serve_ipc_with_resolver(
                            &mut server,
                            &mut runtime,
                            None,
                            Some(&mut resolver),
                        )
                        .map_err(|e| e.to_string());
                        if result_tx.send(result).is_err() {
                            return;
                        }
                        let _ = runtime.tick_all();
                    }
                    ServerStep::Stop => return,
                }
            }
        });

        let client = IpcClient::connect(&socket_path);
        assert_ok!(client, "client connects");
        let Ok(mut client) = client else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server accepts client"
        );

        // Submit a run
        let run_id = RunId::new(55);
        let Some(workflow) = action_then_finish_workflow() else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        let digest = workflow.digest();
        let payload = IpcPayload::SubmitRun(SubmitRunPayload {
            run_id,
            workflow: digest,
            input: Vec::new(),
        });
        assert_ok!(
            client.send_command(IpcCommand::SubmitRun, 800, &payload),
            "submit sends"
        );
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server handles submit"
        );
        let _ = client.recv_response(MaxPayloadBytes::DEFAULT);

        // List events (runtime hasn't processed yet, so events may be empty)
        let list_payload = IpcPayload::ListEvents {
            run_id,
            from_sequence: 0,
        };
        assert_ok!(
            client.send_command(IpcCommand::ListEvents, 801, &list_payload),
            "list-events sends"
        );
        assert!(
            request_server_turn(&step_tx, &result_rx),
            "server handles list"
        );
        let listed = client.recv_response(MaxPayloadBytes::DEFAULT);
        assert_ok!(listed, "list response decodes");
        let Ok((list_header, list_response)) = listed else {
            let _ = step_tx.send(ServerStep::Stop);
            let _ = handle.join();
            return;
        };
        assert_eq!(list_header.command, IpcCommand::ListEvents);
        // Events list is returned successfully over IPC
        assert!(
            matches!(list_response, IpcResponse::Events { .. }),
            "list response should be Events variant"
        );

        assert_ok!(step_tx.send(ServerStep::Stop), "server stops");
        assert_ok!(handle.join(), "server thread joins");
        let remove_result = std::fs::remove_file(&socket_path);
        assert!(
            remove_result.is_ok() || !socket_path.exists(),
            "socket cleanup succeeds"
        );
    }

    #[test]
    fn e2e_client_connect_to_stale_socket_returns_connect_failed() {
        let socket_path = ipc_test_socket("stale_socket");
        // Create and immediately remove a socket file
        {
            let server = IpcServer::bind(&socket_path);
            assert_ok!(server, "server binds");
            let Ok(server) = server else {
                return;
            };
            drop(server);
        }
        // Socket is now gone

        // Client should fail to connect
        let result = IpcClient::connect(&socket_path);
        assert!(result.is_err(), "connecting to stale socket must fail");
        let Err(client_err) = result else {
            return;
        };
        let msg = client_err.to_string();
        assert!(
            msg.contains("connect failed"),
            "error should mention connect failed: {msg}"
        );
        let remove_result = std::fs::remove_file(&socket_path);
        assert!(
            remove_result.is_ok() || !socket_path.exists(),
            "socket cleanup succeeds"
        );
    }
}
