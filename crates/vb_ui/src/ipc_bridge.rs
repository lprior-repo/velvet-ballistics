#![forbid(unsafe_code)]
//! IPC bridge connecting the Makepad UI to a running Velvet Ballistics server.
//!
//! Runs the IPC client on a background thread and communicates with the UI
//! thread through `std::sync::mpsc` channels. The UI sends `IpcRequest`s and
//! polls for `IpcReply`s without blocking the render loop.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use vb_core::WorkflowDigest;
use vb_core::ids::RunId;
use vb_ipc::client::IpcClient;
use vb_ipc::server::IpcResponse;
use vb_ipc::{IpcCommand, IpcPayload, MaxPayloadBytes, SubmitRunPayload};

/// Request from the UI thread to the background IPC thread.
#[derive(Debug)]
pub enum IpcRequest {
    /// Connect to a Unix domain socket endpoint.
    Connect {
        /// Path to the VBLT socket.
        socket_path: PathBuf,
    },
    /// Disconnect from the current server (drops the client).
    Disconnect,
    /// Submit a compiled workflow run.
    SubmitRun {
        /// Caller-selected run identifier.
        run_id: RunId,
        /// Compiled workflow digest.
        workflow: WorkflowDigest,
        /// Postcard-encoded runtime input.
        input: Vec<u8>,
    },
    /// Cancel an active or queued run.
    CancelRun {
        /// Target run identifier.
        run_id: RunId,
    },
    /// Inspect the state of a run.
    InspectRun {
        /// Target run identifier.
        run_id: RunId,
    },
    /// List persisted events for a run.
    ListEvents {
        /// Target run identifier.
        run_id: RunId,
        /// First event sequence to return.
        from_sequence: u64,
    },
    /// Answer a suspended ask ticket.
    AnswerAsk {
        /// Target run identifier.
        run_id: RunId,
        /// Ask ticket identifier.
        ticket: u64,
        /// Postcard-compatible answer bytes.
        answer: Vec<u8>,
    },
    /// Drain bounded trace records.
    DrainTrace {
        /// Target run identifier.
        run_id: RunId,
        /// Maximum records to return.
        max_records: u32,
    },
    /// Verify a compiled workflow and retrieve certificates.
    VerifyWorkflow {
        /// Compiled workflow digest to verify.
        digest: WorkflowDigest,
    },
    /// Request taint analysis for a run's workflow.
    RequestTaintReport {
        /// Run whose workflow should be analyzed.
        run_id: RunId,
        /// Compiled workflow digest to analyze.
        digest: WorkflowDigest,
    },
    /// Request graph data for a compiled workflow.
    RequestWorkflowGraph {
        /// Compiled workflow digest to look up.
        digest: WorkflowDigest,
    },
    /// Probe runtime health.
    Health,
    /// Request graceful shutdown.
    Shutdown,
}

/// Response from the background IPC thread to the UI thread.
#[derive(Debug)]
pub enum IpcReply {
    /// Successfully connected to the server.
    Connected,
    /// Client disconnected.
    Disconnected,
    /// Connection attempt failed.
    ConnectionFailed(String),
    /// Run was accepted by the server.
    RunAccepted(RunId),
    /// Run was cancelled.
    RunCancelled(RunId),
    /// Inspection result from the server.
    Inspected(IpcResponse),
    /// Event list returned by the server.
    Events(IpcResponse),
    /// Trace drain count.
    TraceCount(u32),
    /// Health check succeeded.
    Healthy,
    /// Shutdown acknowledged.
    ShuttingDown,
    /// An error occurred.
    Error(String),
    /// Request type not yet implemented.
    NotImplemented(String),
    /// Verification workflow result received.
    VerifyWorkflowResult(IpcResponse),
    /// Taint report received.
    TaintReportReceived(IpcResponse),
    /// Workflow graph received.
    WorkflowGraphReceived(IpcResponse),
}

/// Thread-safe bridge that owns the communication channels.
///
/// The UI creates this once and calls `send` / `poll` from the render loop.
/// The background thread owns the `IpcClient` and serialises all socket I/O.
pub struct IpcBridge {
    tx: Sender<IpcRequest>,
    rx: Receiver<IpcReply>,
    connected: bool,
    _handle: Option<JoinHandle<()>>,
}

impl Default for IpcBridge {
    fn default() -> Self {
        let (req_tx, req_rx) = mpsc::channel::<IpcRequest>();
        let (rep_tx, rep_rx) = mpsc::channel::<IpcReply>();

        let handle = match thread::Builder::new()
            .name("vb-ipc".to_string())
            .spawn(move || {
                ipc_thread(req_rx, rep_tx);
            })
        {
            Ok(handle) => Some(handle),
            Err(error) => {
                eprintln!("ipc bridge thread spawn failed: {error}");
                None
            }
        };

        Self {
            tx: req_tx,
            rx: rep_rx,
            connected: false,
            _handle: handle,
        }
    }
}

impl IpcBridge {
    /// Creates a new bridge with a background IPC thread.
    ///
    /// If the thread cannot be spawned (resource exhaustion), the handle is
    /// `None` and subsequent `send` calls will return errors once the channel
    /// is disconnected.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sends a request to the background thread (non-blocking).
    ///
    /// Returns an error string if the channel is closed (thread died).
    pub fn send(&self, request: IpcRequest) -> Result<(), String> {
        self.tx
            .send(request)
            .map_err(|e| format!("IPC send failed: {e}"))
    }

    /// Polls for all pending replies without blocking the UI.
    pub fn poll(&mut self) -> Vec<IpcReply> {
        let mut replies = Vec::new();
        while let Ok(reply) = self.rx.try_recv() {
            if matches!(reply, IpcReply::Connected) {
                self.connected = true;
            } else if matches!(
                reply,
                IpcReply::Disconnected | IpcReply::ConnectionFailed(_)
            ) {
                self.connected = false;
            }
            replies.push(reply);
        }
        replies
    }

    /// Returns whether the bridge considers itself connected.
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

// ---------------------------------------------------------------------------
// Background thread
// ---------------------------------------------------------------------------

/// Default receive timeout in milliseconds. Keeps the thread responsive to
/// shutdown signals without busy-looping.
const RECV_TIMEOUT_MS: u64 = 100;

/// Default max payload for response reads.
const DEFAULT_MAX_PAYLOAD: MaxPayloadBytes = MaxPayloadBytes::DEFAULT;

/// Background thread entry point. Owns the `IpcClient` for the lifetime of
/// the connection.
fn ipc_thread(rx: Receiver<IpcRequest>, tx: Sender<IpcReply>) {
    let mut client: Option<IpcClient> = None;
    let mut correlation: u64 = 0;

    loop {
        let request = match rx.recv_timeout(Duration::from_millis(RECV_TIMEOUT_MS)) {
            Ok(req) => req,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };

        match request {
            IpcRequest::Connect { socket_path } => match IpcClient::connect(&socket_path) {
                Ok(c) => {
                    client = Some(c);
                    if let Err(_err) = tx.send(IpcReply::Connected) {
                        return;
                    }
                }
                Err(e) => {
                    if let Err(_err) = tx.send(IpcReply::ConnectionFailed(format!("{e}"))) {
                        return;
                    }
                }
            },

            IpcRequest::Disconnect => {
                client = None;
                if let Err(_err) = tx.send(IpcReply::Disconnected) {
                    return;
                }
            }

            IpcRequest::Health => {
                if let Some(ref mut c) = client {
                    let corr = next_correlation(&mut correlation);
                    match c.health(corr) {
                        Ok(()) => match c.recv_response(DEFAULT_MAX_PAYLOAD) {
                            Ok((_header, response)) => {
                                if let Err(_err) = tx.send(reply_from_response(response)) {
                                    return;
                                }
                            }
                            Err(e) => {
                                if let Err(_err) = tx.send(IpcReply::Error(format!("{e}"))) {
                                    return;
                                }
                            }
                        },
                        Err(e) => {
                            if let Err(_err) = tx.send(IpcReply::Error(format!("{e}"))) {
                                return;
                            }
                        }
                    }
                } else {
                    if let Err(_err) = tx.send(IpcReply::Error("Not connected".into())) {
                        return;
                    }
                }
            }

            IpcRequest::InspectRun { run_id } => {
                if let Some(ref mut c) = client {
                    let corr = next_correlation(&mut correlation);
                    let payload = IpcPayload::InspectRun { run_id };
                    match send_and_recv(c, IpcCommand::InspectRun, corr, &payload) {
                        Ok(response) => {
                            if let Err(_err) = tx.send(IpcReply::Inspected(response)) {
                                return;
                            }
                        }
                        Err(e) => {
                            if let Err(_err) = tx.send(IpcReply::Error(e)) {
                                return;
                            }
                        }
                    }
                } else {
                    if let Err(_err) = tx.send(IpcReply::Error("Not connected".into())) {
                        return;
                    }
                }
            }

            IpcRequest::CancelRun { run_id } => {
                if let Some(ref mut c) = client {
                    let corr = next_correlation(&mut correlation);
                    let payload = IpcPayload::CancelRun { run_id };
                    match send_and_recv(c, IpcCommand::CancelRun, corr, &payload) {
                        Ok(response) => {
                            if let Err(_err) = tx.send(IpcReply::RunCancelled(run_id)) {
                                return;
                            }
                            // Silently consume response; cancellation is
                            // fire-and-forget from the UI perspective.
                            let _ = response;
                        }
                        Err(e) => {
                            if let Err(_err) = tx.send(IpcReply::Error(e)) {
                                return;
                            }
                        }
                    }
                } else {
                    if let Err(_err) = tx.send(IpcReply::Error("Not connected".into())) {
                        return;
                    }
                }
            }

            IpcRequest::ListEvents {
                run_id,
                from_sequence,
            } => {
                if let Some(ref mut c) = client {
                    let corr = next_correlation(&mut correlation);
                    let payload = IpcPayload::ListEvents {
                        run_id,
                        from_sequence,
                    };
                    match send_and_recv(c, IpcCommand::ListEvents, corr, &payload) {
                        Ok(response) => {
                            if let Err(_err) = tx.send(IpcReply::Events(response)) {
                                return;
                            }
                        }
                        Err(e) => {
                            if let Err(_err) = tx.send(IpcReply::Error(e)) {
                                return;
                            }
                        }
                    }
                } else {
                    if let Err(_err) = tx.send(IpcReply::Error("Not connected".into())) {
                        return;
                    }
                }
            }

            IpcRequest::SubmitRun {
                run_id,
                workflow,
                input,
            } => {
                if let Some(ref mut c) = client {
                    let corr = next_correlation(&mut correlation);
                    let payload = IpcPayload::SubmitRun(SubmitRunPayload {
                        run_id,
                        workflow,
                        input,
                    });
                    match send_and_recv(c, IpcCommand::SubmitRun, corr, &payload) {
                        Ok(response) => {
                            if let Err(_err) = tx.send(reply_from_submit(response, run_id)) {
                                return;
                            }
                        }
                        Err(e) => {
                            if let Err(_err) = tx.send(IpcReply::Error(e)) {
                                return;
                            }
                        }
                    }
                } else {
                    if let Err(_err) = tx.send(IpcReply::Error("Not connected".into())) {
                        return;
                    }
                }
            }

            IpcRequest::AnswerAsk {
                run_id,
                ticket,
                answer,
            } => {
                if let Some(ref mut c) = client {
                    let corr = next_correlation(&mut correlation);
                    let payload = IpcPayload::AnswerAsk {
                        run_id,
                        ticket,
                        answer,
                        taint: None,
                    };
                    match send_and_recv(c, IpcCommand::AnswerAsk, corr, &payload) {
                        Ok(response) => {
                            if let Err(_err) = tx.send(reply_from_answer(response, run_id)) {
                                return;
                            }
                        }
                        Err(e) => {
                            if let Err(_err) = tx.send(IpcReply::Error(e)) {
                                return;
                            }
                        }
                    }
                } else {
                    if let Err(_err) = tx.send(IpcReply::Error("Not connected".into())) {
                        return;
                    }
                }
            }

            IpcRequest::DrainTrace {
                run_id,
                max_records,
            } => {
                if let Some(ref mut c) = client {
                    let corr = next_correlation(&mut correlation);
                    let payload = IpcPayload::DrainTrace {
                        run_id,
                        max_records,
                    };
                    match send_and_recv(c, IpcCommand::DrainTrace, corr, &payload) {
                        Ok(response) => {
                            if let Err(_err) = tx.send(reply_from_drain_trace(response)) {
                                return;
                            }
                        }
                        Err(e) => {
                            if let Err(_err) = tx.send(IpcReply::Error(e)) {
                                return;
                            }
                        }
                    }
                } else {
                    if let Err(_err) = tx.send(IpcReply::Error("Not connected".into())) {
                        return;
                    }
                }
            }

            IpcRequest::VerifyWorkflow { digest } => {
                if let Some(ref mut c) = client {
                    let corr = next_correlation(&mut correlation);
                    let payload = IpcPayload::VerifyWorkflow { digest };
                    match send_and_recv(c, IpcCommand::VerifyWorkflow, corr, &payload) {
                        Ok(response) => {
                            if let Err(_err) = tx.send(IpcReply::VerifyWorkflowResult(response)) {
                                return;
                            }
                        }
                        Err(e) => {
                            if let Err(_err) = tx.send(IpcReply::Error(e)) {
                                return;
                            }
                        }
                    }
                } else {
                    if let Err(_err) = tx.send(IpcReply::Error("Not connected".into())) {
                        return;
                    }
                }
            }

            IpcRequest::RequestTaintReport { run_id: _, digest } => {
                if let Some(ref mut c) = client {
                    let corr = next_correlation(&mut correlation);
                    let payload = IpcPayload::GetTaintReport { digest };
                    match send_and_recv(c, IpcCommand::GetTaintReport, corr, &payload) {
                        Ok(response) => {
                            if let Err(_err) = tx.send(IpcReply::TaintReportReceived(response)) {
                                return;
                            }
                        }
                        Err(e) => {
                            if let Err(_err) = tx.send(IpcReply::Error(e)) {
                                return;
                            }
                        }
                    }
                } else {
                    if let Err(_err) = tx.send(IpcReply::Error("Not connected".into())) {
                        return;
                    }
                }
            }

            IpcRequest::RequestWorkflowGraph { digest } => {
                if let Some(ref mut c) = client {
                    let corr = next_correlation(&mut correlation);
                    let payload = IpcPayload::GetWorkflowGraph { digest };
                    match send_and_recv(c, IpcCommand::GetWorkflowGraph, corr, &payload) {
                        Ok(response) => {
                            if let Err(_err) = tx.send(IpcReply::WorkflowGraphReceived(response)) {
                                return;
                            }
                        }
                        Err(e) => {
                            if let Err(_err) = tx.send(IpcReply::Error(e)) {
                                return;
                            }
                        }
                    }
                } else {
                    if let Err(_err) = tx.send(IpcReply::Error("Not connected".into())) {
                        return;
                    }
                }
            }

            IpcRequest::Shutdown => {
                if let Some(ref mut c) = client {
                    let corr = next_correlation(&mut correlation);
                    match c.shutdown(corr) {
                        Ok(()) => {
                            // Best-effort read of the shutdown ack.
                            let response = c.recv_response(DEFAULT_MAX_PAYLOAD).ok();
                            if let Err(_err) = tx.send(IpcReply::ShuttingDown) {
                                return;
                            }
                            let _ = response;
                        }
                        Err(e) => {
                            if let Err(_err) = tx.send(IpcReply::Error(format!("{e}"))) {
                                return;
                            }
                        }
                    }
                } else {
                    if let Err(_err) = tx.send(IpcReply::Error("Not connected".into())) {
                        return;
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Advances the correlation counter and returns the new value.
fn next_correlation(c: &mut u64) -> u64 {
    *c = c.wrapping_add(1);
    *c
}

/// Sends a typed command and reads one response frame.
fn send_and_recv(
    client: &mut IpcClient,
    command: IpcCommand,
    correlation: u64,
    payload: &IpcPayload,
) -> Result<IpcResponse, String> {
    client
        .send_command(command, correlation, payload)
        .map_err(|e| format!("{e}"))?;
    let (_header, response) = client
        .recv_response(DEFAULT_MAX_PAYLOAD)
        .map_err(|e| format!("{e}"))?;
    Ok(response)
}

/// Maps an `IpcResponse` from a health check into a UI-friendly reply.
fn reply_from_response(response: IpcResponse) -> IpcReply {
    match response {
        IpcResponse::Healthy => IpcReply::Healthy,
        IpcResponse::ShuttingDown => IpcReply::ShuttingDown,
        IpcResponse::RuntimeError { message } => IpcReply::Error(message),
        other => IpcReply::Error(format!("Unexpected health response: {other:?}")),
    }
}

/// Maps an `IpcResponse` from a submit-run command into a UI-friendly reply.
fn reply_from_submit(response: IpcResponse, run_id: RunId) -> IpcReply {
    match response {
        IpcResponse::AcceptedRun { .. } => IpcReply::RunAccepted(run_id),
        IpcResponse::RuntimeError { message } => IpcReply::Error(message),
        IpcResponse::WorkflowResolutionRequired => {
            IpcReply::Error("Workflow resolution required".into())
        }
        IpcResponse::WorkflowResolutionUnsupported => {
            IpcReply::Error("Workflow resolution unsupported".into())
        }
        IpcResponse::WorkflowDigestMismatch => IpcReply::Error("Workflow digest mismatch".into()),
        IpcResponse::PayloadError { message, .. } => IpcReply::Error(message),
        IpcResponse::CommandPayloadMismatch => IpcReply::Error("Command/payload mismatch".into()),
        other => IpcReply::Error(format!("Unexpected submit response: {other:?}")),
    }
}

/// Maps an `IpcResponse` from an answer-ask command into a UI-friendly reply.
fn reply_from_answer(response: IpcResponse, run_id: RunId) -> IpcReply {
    match response {
        IpcResponse::AcceptedRun { .. } => IpcReply::RunAccepted(run_id),
        IpcResponse::RuntimeError { message } => IpcReply::Error(message),
        IpcResponse::BadRequest => IpcReply::Error("Bad request".into()),
        IpcResponse::PayloadError { message, .. } => IpcReply::Error(message),
        other => IpcReply::Error(format!("Unexpected answer response: {other:?}")),
    }
}

/// Maps an `IpcResponse` from a drain-trace command into a UI-friendly reply.
fn reply_from_drain_trace(response: IpcResponse) -> IpcReply {
    match response {
        IpcResponse::TraceCount { count } => IpcReply::TraceCount(count),
        IpcResponse::RuntimeError { message } => IpcReply::Error(message),
        IpcResponse::BadRequest => IpcReply::Error("Bad request".into()),
        IpcResponse::PayloadError { message, .. } => IpcReply::Error(message),
        other => IpcReply::Error(format!("Unexpected drain-trace response: {other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_new_creates_channels_and_thread() {
        let mut bridge = IpcBridge::new();
        assert!(!bridge.is_connected());
        assert!(bridge.poll().is_empty());
    }

    #[test]
    fn bridge_connect_to_nonexistent_socket_fails() {
        let mut bridge = IpcBridge::new();
        let path = PathBuf::from("/tmp/vb_ipc_bridge_test_nonexistent_7f3a.socket");
        assert!(bridge.send(IpcRequest::Connect { socket_path: path }).is_ok());

        // Give the background thread time to process.
        let mut replies = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_millis(500);
        while std::time::Instant::now() < deadline {
            replies.extend(bridge.poll());
            if replies
                .iter()
                .any(|r| matches!(r, IpcReply::ConnectionFailed(_)))
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        let found = replies
            .iter()
            .any(|r| matches!(r, IpcReply::ConnectionFailed(_)));
        assert!(found, "expected ConnectionFailed reply");
        assert!(!bridge.is_connected());
    }

    #[test]
    fn bridge_send_without_connect_returns_not_connected_error() {
        let mut bridge = IpcBridge::new();
        assert!(bridge.send(IpcRequest::Health).is_ok());

        let mut replies = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_millis(500);
        while std::time::Instant::now() < deadline {
            replies.extend(bridge.poll());
            if replies.iter().any(|r| matches!(r, IpcReply::Error(_))) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        let found = replies
            .iter()
            .any(|r| matches!(r, IpcReply::Error(e) if e.contains("Not connected")));
        assert!(found, "expected 'Not connected' error reply");
    }

    #[test]
    fn bridge_submit_run_without_connect_returns_not_connected_error() {
        let mut bridge = IpcBridge::new();
        assert!(bridge
            .send(IpcRequest::SubmitRun {
                run_id: RunId::new(1),
                workflow: WorkflowDigest::from_bytes([0; 32]),
                input: Vec::new(),
            })
            .is_ok());

        let mut replies = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_millis(500);
        while std::time::Instant::now() < deadline {
            replies.extend(bridge.poll());
            if replies.iter().any(|r| matches!(r, IpcReply::Error(_))) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        let found = replies
            .iter()
            .any(|r| matches!(r, IpcReply::Error(e) if e.contains("Not connected")));
        assert!(found, "expected 'Not connected' error for SubmitRun");
    }

    #[test]
    fn bridge_answer_ask_without_connect_returns_not_connected_error() {
        let mut bridge = IpcBridge::new();
        assert!(bridge
            .send(IpcRequest::AnswerAsk {
                run_id: RunId::new(1),
                ticket: 0,
                answer: Vec::new(),
            }).is_ok());

        let mut replies = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_millis(500);
        while std::time::Instant::now() < deadline {
            replies.extend(bridge.poll());
            if replies.iter().any(|r| matches!(r, IpcReply::Error(_))) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        let found = replies
            .iter()
            .any(|r| matches!(r, IpcReply::Error(e) if e.contains("Not connected")));
        assert!(found, "expected 'Not connected' error for AnswerAsk");
    }

    #[test]
    fn bridge_drain_trace_without_connect_returns_not_connected_error() {
        let mut bridge = IpcBridge::new();
        assert!(bridge
            .send(IpcRequest::DrainTrace {
                run_id: RunId::new(1),
                max_records: 10,
            }).is_ok());

        let mut replies = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_millis(500);
        while std::time::Instant::now() < deadline {
            replies.extend(bridge.poll());
            if replies.iter().any(|r| matches!(r, IpcReply::Error(_))) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        let found = replies
            .iter()
            .any(|r| matches!(r, IpcReply::Error(e) if e.contains("Not connected")));
        assert!(found, "expected 'Not connected' error for DrainTrace");
    }

    #[test]
    fn next_correlation_advances() {
        let mut c: u64 = 0;
        assert_eq!(next_correlation(&mut c), 1);
        assert_eq!(next_correlation(&mut c), 2);
        assert_eq!(c, 2);
    }

    #[test]
    fn next_correlation_wraps_at_max() {
        let mut c: u64 = u64::MAX;
        let result = next_correlation(&mut c);
        assert_eq!(result, 0);
        assert_eq!(c, 0);
    }

    #[test]
    fn reply_from_response_healthy() {
        let reply = reply_from_response(IpcResponse::Healthy);
        assert!(matches!(reply, IpcReply::Healthy));
    }

    #[test]
    fn reply_from_response_shutting_down() {
        let reply = reply_from_response(IpcResponse::ShuttingDown);
        assert!(matches!(reply, IpcReply::ShuttingDown));
    }

    #[test]
    fn reply_from_response_runtime_error() {
        let reply = reply_from_response(IpcResponse::RuntimeError {
            message: "boom".into(),
        });
        assert!(matches!(reply, IpcReply::Error(ref e) if e == "boom"));
    }

    #[test]
    fn reply_from_response_unexpected_maps_to_error() {
        let reply = reply_from_response(IpcResponse::AcceptedRun { run_id: 42 });
        assert!(matches!(reply, IpcReply::Error(_)));
    }

    #[test]
    fn reply_from_submit_accepted_run() {
        let run_id = RunId::new(7);
        let reply = reply_from_submit(IpcResponse::AcceptedRun { run_id: 7 }, run_id);
        assert!(matches!(reply, IpcReply::RunAccepted(rid) if rid == run_id));
    }

    #[test]
    fn reply_from_submit_runtime_error() {
        let reply = reply_from_submit(
            IpcResponse::RuntimeError {
                message: "fail".into(),
            },
            RunId::new(1),
        );
        assert!(matches!(reply, IpcReply::Error(ref e) if e == "fail"));
    }

    #[test]
    fn reply_from_submit_workflow_resolution_required() {
        let reply = reply_from_submit(IpcResponse::WorkflowResolutionRequired, RunId::new(1));
        assert!(matches!(reply, IpcReply::Error(_)));
    }

    #[test]
    fn reply_from_submit_unexpected_maps_to_error() {
        let reply = reply_from_submit(IpcResponse::Healthy, RunId::new(1));
        assert!(matches!(reply, IpcReply::Error(_)));
    }

    #[test]
    fn reply_from_answer_accepted_run() {
        let run_id = RunId::new(3);
        let reply = reply_from_answer(IpcResponse::AcceptedRun { run_id: 3 }, run_id);
        assert!(matches!(reply, IpcReply::RunAccepted(rid) if rid == run_id));
    }

    #[test]
    fn reply_from_answer_runtime_error() {
        let reply = reply_from_answer(
            IpcResponse::RuntimeError {
                message: "err".into(),
            },
            RunId::new(1),
        );
        assert!(matches!(reply, IpcReply::Error(ref e) if e == "err"));
    }

    #[test]
    fn reply_from_answer_bad_request() {
        let reply = reply_from_answer(IpcResponse::BadRequest, RunId::new(1));
        assert!(matches!(reply, IpcReply::Error(_)));
    }

    #[test]
    fn reply_from_answer_unexpected_maps_to_error() {
        let reply = reply_from_answer(IpcResponse::Healthy, RunId::new(1));
        assert!(matches!(reply, IpcReply::Error(_)));
    }

    #[test]
    fn reply_from_drain_trace_count() {
        let reply = reply_from_drain_trace(IpcResponse::TraceCount { count: 42 });
        assert!(matches!(reply, IpcReply::TraceCount(42)));
    }

    #[test]
    fn reply_from_drain_trace_runtime_error() {
        let reply = reply_from_drain_trace(IpcResponse::RuntimeError {
            message: "boom".into(),
        });
        assert!(matches!(reply, IpcReply::Error(ref e) if e == "boom"));
    }

    #[test]
    fn reply_from_drain_trace_bad_request() {
        let reply = reply_from_drain_trace(IpcResponse::BadRequest);
        assert!(matches!(reply, IpcReply::Error(_)));
    }

    #[test]
    fn reply_from_drain_trace_unexpected_maps_to_error() {
        let reply = reply_from_drain_trace(IpcResponse::Healthy);
        assert!(matches!(reply, IpcReply::Error(_)));
    }
}
