#![forbid(unsafe_code)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::needless_pass_by_value)]
//! Bounded memory ingress and binary IPC for Velvet Ballastics.
//!
//! This crate deliberately exposes memory/IPC-shaped primitives only. HTTP is
//! not part of the hot control plane.

pub mod client;
pub mod frame;
pub mod server;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use bytes::Bytes;
use crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError, bounded};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::num::NonZeroUsize;
use thiserror::Error;
use vb_core::DiagnosticCode;
use vb_core::action::ActionOutputReady;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::{RunId, WorkflowDigest};

/// IPC frame magic: `VBLT` little-endian.
pub const IPC_MAGIC: u32 = 0x5642_4C54;
/// Supported IPC schema version.
pub const IPC_VERSION: u16 = 1;
/// Fixed IPC header length in bytes.
pub const IPC_HEADER_LEN: usize = 24;

/// Binary IPC command identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum IpcCommand {
    /// Submit a run using a previously compiled workflow artifact.
    SubmitRun = 1,
    /// Submit a run with inline validated runtime inputs.
    SubmitRunInline = 2,
    /// Cancel an active or queued run.
    CancelRun = 3,
    /// Inspect run state.
    InspectRun = 4,
    /// List persisted events for a run.
    ListEvents = 5,
    /// Answer a suspended ask.
    AnswerAsk = 6,
    /// Complete an external action ticket.
    CompleteAction = 7,
    /// Fail an external action ticket.
    FailAction = 8,
    /// Drain bounded trace records.
    DrainTrace = 9,
    /// Probe runtime health.
    Health = 10,
    /// Request graceful shutdown.
    Shutdown = 11,
    /// List active runs.
    ListRuns = 12,
    /// Query runtime metrics (queue depths, shard load, throughput).
    GetMetrics = 13,
    /// Retrieve the graph structure of a compiled workflow.
    GetWorkflowGraph = 14,
    /// Get taint report for a compiled workflow (secret-to-sink paths).
    GetTaintReport = 15,
    /// Verify a compiled workflow and return validation certificates.
    VerifyWorkflow = 16,
}

impl IpcCommand {
    /// Parses a wire command identifier.
    pub fn from_u16(value: u16) -> Result<Self, IpcError> {
        match value {
            1 => Ok(Self::SubmitRun),
            2 => Ok(Self::SubmitRunInline),
            3 => Ok(Self::CancelRun),
            4 => Ok(Self::InspectRun),
            5 => Ok(Self::ListEvents),
            6 => Ok(Self::AnswerAsk),
            7 => Ok(Self::CompleteAction),
            8 => Ok(Self::FailAction),
            9 => Ok(Self::DrainTrace),
            10 => Ok(Self::Health),
            11 => Ok(Self::Shutdown),
            12 => Ok(Self::ListRuns),
            13 => Ok(Self::GetMetrics),
            14 => Ok(Self::GetWorkflowGraph),
            15 => Ok(Self::GetTaintReport),
            16 => Ok(Self::VerifyWorkflow),
            other => Err(IpcError::UnknownCommand(other)),
        }
    }

    /// Returns the wire command identifier.
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        match self {
            Self::SubmitRun => 1,
            Self::SubmitRunInline => 2,
            Self::CancelRun => 3,
            Self::InspectRun => 4,
            Self::ListEvents => 5,
            Self::AnswerAsk => 6,
            Self::CompleteAction => 7,
            Self::FailAction => 8,
            Self::DrainTrace => 9,
            Self::Health => 10,
            Self::Shutdown => 11,
            Self::ListRuns => 12,
            Self::GetMetrics => 13,
            Self::GetWorkflowGraph => 14,
            Self::GetTaintReport => 15,
            Self::VerifyWorkflow => 16,
        }
    }
}

/// Fixed binary IPC frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcFrameHeader {
    /// IPC command kind.
    pub command: IpcCommand,
    /// Command-specific flags.
    pub flags: u16,
    /// Correlates requests and replies.
    pub correlation: u64,
    /// Postcard payload byte length.
    pub payload_len: u32,
}

impl IpcFrameHeader {
    /// Creates an IPC frame header.
    #[must_use]
    pub const fn new(command: IpcCommand, flags: u16, correlation: u64, payload_len: u32) -> Self {
        Self {
            command,
            flags,
            correlation,
            payload_len,
        }
    }

    /// Encodes the header using the §21 little-endian wire layout.
    ///
    /// Wire format (24 bytes total):
    /// - Bytes [0:4]:   magic u32 little-endian (0x5642_4C54 "VBLT")
    /// - Bytes [4:6]:   version u16 little-endian
    /// - Bytes [6:8]:   command u16 little-endian
    /// - Bytes [8:10]:  flags u16 little-endian
    /// - Bytes [10:12]: reserved u16 little-endian (always 0)
    /// - Bytes [12:20]: correlation u64 little-endian
    /// - Bytes [20:24]: payload_len u32 little-endian
    pub fn encode(self) -> Result<[u8; IPC_HEADER_LEN], IpcError> {
        let mut bytes = [0u8; IPC_HEADER_LEN];
        let mut cursor = std::io::Cursor::new(&mut bytes[..]);
        cursor
            .write_u32::<LittleEndian>(IPC_MAGIC)
            .map_err(|_| IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u16::<LittleEndian>(IPC_VERSION)
            .map_err(|_| IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u16::<LittleEndian>(self.command.as_u16())
            .map_err(|_| IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u16::<LittleEndian>(self.flags)
            .map_err(|_| IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u16::<LittleEndian>(0_u16)
            .map_err(|_| IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u64::<LittleEndian>(self.correlation)
            .map_err(|_| IpcError::HeaderEncodeFailed)?;
        cursor
            .write_u32::<LittleEndian>(self.payload_len)
            .map_err(|_| IpcError::HeaderEncodeFailed)?;
        Ok(bytes)
    }

    /// Decodes and validates a fixed IPC header before payload allocation.
    pub fn decode(
        bytes: &[u8; IPC_HEADER_LEN],
        max_payload: MaxPayloadBytes,
    ) -> Result<Self, IpcError> {
        let mut cursor = Cursor::new(bytes.as_slice());
        let magic = cursor
            .read_u32::<LittleEndian>()
            .map_err(|_| IpcError::HeaderDecodeFailed)?;
        if magic != IPC_MAGIC {
            return Err(IpcError::InvalidMagic { actual: magic });
        }

        let version = cursor
            .read_u16::<LittleEndian>()
            .map_err(|_| IpcError::HeaderDecodeFailed)?;
        if version != IPC_VERSION {
            return Err(IpcError::UnsupportedVersion { actual: version });
        }

        let command = IpcCommand::from_u16(
            cursor
                .read_u16::<LittleEndian>()
                .map_err(|_| IpcError::HeaderDecodeFailed)?,
        )?;
        let flags = cursor
            .read_u16::<LittleEndian>()
            .map_err(|_| IpcError::HeaderDecodeFailed)?;
        let reserved = cursor
            .read_u16::<LittleEndian>()
            .map_err(|_| IpcError::HeaderDecodeFailed)?;
        if reserved != 0 {
            return Err(IpcError::ReservedNonZero { actual: reserved });
        }
        let correlation = cursor
            .read_u64::<LittleEndian>()
            .map_err(|_| IpcError::HeaderDecodeFailed)?;
        let payload_len = cursor
            .read_u32::<LittleEndian>()
            .map_err(|_| IpcError::HeaderDecodeFailed)?;
        let payload_len_usize = u32_to_usize(payload_len)?;
        if payload_len_usize > max_payload.get() {
            return Err(IpcError::PayloadTooLarge {
                actual: payload_len_usize,
                limit: max_payload.get(),
            });
        }

        Ok(Self {
            command,
            flags,
            correlation,
            payload_len,
        })
    }
}

/// Decoded IPC frame with bounded postcard payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcFrame {
    header: IpcFrameHeader,
    payload: BoundedPayload,
}

impl IpcFrame {
    /// Builds a frame after enforcing header/payload length agreement.
    pub fn new(
        header: IpcFrameHeader,
        payload: Bytes,
        max_payload: MaxPayloadBytes,
    ) -> Result<Self, IpcError> {
        let actual_len = payload.len();
        let expected_len = u32_to_usize(header.payload_len)?;
        if actual_len != expected_len {
            return Err(IpcError::PayloadLengthMismatch {
                header: expected_len,
                actual: actual_len,
            });
        }

        Ok(Self {
            header,
            payload: BoundedPayload::new(payload, max_payload)?,
        })
    }

    /// Returns the decoded frame header.
    #[must_use]
    pub const fn header(&self) -> IpcFrameHeader {
        self.header
    }

    /// Returns bounded postcard payload bytes.
    #[must_use]
    pub const fn payload(&self) -> &BoundedPayload {
        &self.payload
    }
}

/// Decodes a fixed header and already-read payload bytes into a bounded frame.
pub fn decode_frame(
    header: &[u8; IPC_HEADER_LEN],
    payload: Bytes,
    max_payload: MaxPayloadBytes,
) -> Result<IpcFrame, IpcError> {
    IpcFrame::new(
        IpcFrameHeader::decode(header, max_payload)?,
        payload,
        max_payload,
    )
}

/// Submit a compiled workflow run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitRunPayload {
    /// Caller-selected run identifier.
    pub run_id: RunId,
    /// Compiled workflow digest.
    pub workflow: WorkflowDigest,
    /// Runtime input bytes owned by the IPC payload.
    pub input: Vec<u8>,
}

/// Payloads accepted by the binary IPC command surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IpcPayload {
    /// Submit a compiled workflow run.
    SubmitRun(SubmitRunPayload),
    /// Submit a compiled workflow run with inline inputs.
    SubmitRunInline(SubmitRunPayload),
    /// Cancel a run.
    CancelRun {
        /// Target run identifier.
        run_id: RunId,
    },
    /// Inspect a run.
    InspectRun {
        /// Target run identifier.
        run_id: RunId,
    },
    /// List run events from a sequence number.
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
    /// Complete an external action ticket.
    CompleteAction {
        /// Target run identifier.
        run_id: RunId,
        /// Action ticket identifier.
        ticket: u64,
        /// Action output bytes.
        output: Vec<u8>,
    },
    /// Fail an external action ticket.
    FailAction {
        /// Target run identifier.
        run_id: RunId,
        /// Action ticket identifier.
        ticket: u64,
        /// Encoded failure payload.
        error: Vec<u8>,
    },
    /// Drain trace records for a run.
    DrainTrace {
        /// Target run identifier.
        run_id: RunId,
        /// Maximum records to return.
        max_records: u32,
    },
    /// List active and recent runs.
    ListRuns {
        /// Maximum number of runs to return.
        limit: u32,
        /// Filter to runs matching this workflow digest (optional).
        workflow: Option<WorkflowDigest>,
    },
    /// Health probe.
    Health,
    /// Graceful shutdown request.
    Shutdown,
    /// Query runtime metrics.
    GetMetrics,
    /// Get taint report for a compiled workflow.
    GetTaintReport {
        /// Compiled workflow digest to analyze.
        digest: WorkflowDigest,
    },
    /// Retrieve the graph structure of a compiled workflow.
    GetWorkflowGraph {
        /// Compiled workflow digest to look up.
        digest: WorkflowDigest,
    },
    /// Verify a compiled workflow and return validation certificates.
    VerifyWorkflow {
        /// Compiled workflow digest to verify.
        digest: WorkflowDigest,
    },
}

/// Run state reported in list-run responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunListState {
    /// Run is actively executing or suspended on this shard.
    Active,
    /// Run finished successfully.
    Finished,
    /// Run failed.
    Failed,
    /// Run was cancelled.
    Cancelled,
}

/// Summary of a run for list-run responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSummary {
    /// Run identifier.
    pub run_id: RunId,
    /// Compiled workflow digest.
    pub workflow: WorkflowDigest,
    /// Current run state.
    pub state: RunListState,
    /// Submitted sequence number (0 for active in-memory runs).
    pub submitted_seq: u64,
    /// Finished sequence number, if the run reached a terminal state.
    pub finished_seq: Option<u64>,
    /// Number of steps in the workflow.
    pub step_count: u16,
    /// Steps that reached a terminal state (Succeeded, Failed, Skipped, or Cancelled).
    pub steps_completed: u16,
}

/// Runtime metrics response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeMetrics {
    /// Per-shard metrics.
    pub shards: Vec<ShardMetrics>,
    /// Journal metrics.
    pub journal: JournalMetrics,
    /// IPC connection metrics.
    pub ipc: IpcMetrics,
    /// Aggregate totals across all shards.
    pub totals: AggregateMetrics,
}

/// Per-shard metrics snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShardMetrics {
    /// Shard index.
    pub shard_id: u32,
    /// Number of active runs on this shard.
    pub active_runs: u32,
    /// Number of commands waiting in the ready queue.
    pub ready_queue_depth: u32,
    /// Remaining capacity in the command queue.
    pub action_queue_depth: u32,
    /// Number of pending timers.
    pub timer_count: u32,
    /// Free frames in the frame pool.
    pub frame_pool_free: u32,
    /// Total capacity of the frame pool.
    pub frame_pool_total: u32,
    /// Trace ring fill percentage (0.0 - 100.0).
    pub trace_ring_fill_pct: f32,
    /// Total steps executed on this shard.
    pub steps_total: u64,
    /// Total actions completed on this shard.
    pub actions_total: u64,
}

/// Journal metrics snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalMetrics {
    /// Journal writer queue depth.
    pub writer_queue_depth: u32,
    /// Total events written.
    pub total_events: u64,
    /// Total runs recorded.
    pub total_runs: u64,
}

/// IPC connection metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpcMetrics {
    /// Currently connected IPC clients.
    pub connected_clients: u32,
    /// Total IPC commands processed.
    pub commands_processed: u64,
}

/// Aggregate totals across all shards.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregateMetrics {
    /// Total runs currently active across all shards.
    pub runs_active: u32,
    /// Total runs waiting (suspended on actions or timers).
    pub runs_waiting: u32,
    /// Total runs failed since startup.
    pub runs_failed_total: u64,
    /// Total runs finished since startup.
    pub runs_finished_total: u64,
}

/// Verification outcome for a compiled workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Per-gate certificate details.
    pub certificates: Vec<CertificateWire>,
    /// Total number of gate checks performed.
    pub total_checks: u32,
    /// Number of gate checks that passed.
    pub pass_count: u32,
    /// Number of gate checks that failed.
    pub fail_count: u32,
}

/// One gate-check certificate in a verification result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificateWire {
    /// Gate identifier (e.g. `"gate_07_expression_stack_depth"`).
    pub kind: String,
    /// `"Pass"` or `"Fail"`.
    pub status: String,
    /// Human-readable details (empty on pass).
    pub details: String,
}

/// One edge in a taint propagation path, serialized for IPC transport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaintPathWire {
    /// Source step index.
    pub from: u16,
    /// Destination step index.
    pub to: u16,
    /// Whether this edge is dangerous (reaches Finish) or just a warning.
    pub status: String,
}

/// Lightweight descriptor for a single workflow node returned by GetWorkflowGraph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeDescriptor {
    /// Step index of this node.
    pub step_idx: u16,
    /// Kind of this node (e.g. "Nop", "Do", "Choose", "Finish").
    pub kind: String,
    /// Fallthrough target step index, if any.
    pub next: Option<u16>,
    /// Human-readable step name.
    pub title: String,
}

/// Lightweight descriptor for a workflow edge returned by GetWorkflowGraph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeDescriptor {
    /// Source step index.
    pub from: u16,
    /// Target step index.
    pub to: u16,
    /// Optional label for this edge (e.g. branch condition, "otherwise").
    pub label: Option<String>,
    /// Edge type (e.g. "fallthrough", "branch", "loop_body", "error_handler").
    pub edge_type: String,
}

/// Typed IPC action output payload carried by `CompleteAction`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcActionOutputPayload {
    /// Output slot receiving the action result.
    pub output_slot: SlotIdx,
    /// Runtime value produced by the action.
    pub value: SlotValue,
    /// Taint attached to the result.
    pub taint: Taint,
}

impl IpcActionOutputPayload {
    /// Converts the wire payload into the runtime completion shape.
    pub fn into_action_output(self, encoded_len: u32) -> ActionOutputReady {
        ActionOutputReady {
            output_slot: self.output_slot,
            value: self.value,
            taint: self.taint,
            encoded_len,
        }
    }
}

/// Typed trace event returned by `ListEvents`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcTraceEvent {
    /// Monotonic sequence assigned by the IPC snapshot response.
    pub sequence: u64,
    /// Event payload.
    pub kind: IpcTraceEventKind,
}

/// Stable IPC event payload independent of runtime internals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IpcTraceEventKind {
    /// A step began execution.
    StepStarted { run: RunId, step: StepIdx },
    /// A step completed execution.
    StepEnded { run: RunId, step: StepIdx },
    /// A slot was written.
    SlotWritten {
        run: RunId,
        slot: SlotIdx,
        /// Encoded slot value bytes (postcard-encoded SlotValue).
        value: Vec<u8>,
    },
    /// An action was scheduled.
    ActionScheduled { run: RunId, step: StepIdx },
    /// An action completed.
    ActionCompleted { run: RunId, step: StepIdx },
    /// An action failed.
    ActionFailed {
        run: RunId,
        step: StepIdx,
        code: vb_core::action::ActionFailureCode,
    },
    /// An ask was answered.
    AskAnswered {
        run: RunId,
        step: StepIdx,
        slot: SlotIdx,
    },
    /// A run was submitted.
    RunSubmitted { run: RunId },
    /// A run finished.
    RunFinished { run: RunId },
    /// A run failed.
    RunFailed { run: RunId },
    /// A run was cancelled.
    RunCancelled { run: RunId },
}

/// Encodes a typed IPC payload with Postcard.
pub fn encode_payload(
    payload: &IpcPayload,
    max_payload: MaxPayloadBytes,
) -> Result<BoundedPayload, IpcError> {
    let bytes = postcard::to_allocvec(payload).map_err(|_| IpcError::PayloadEncodeFailed)?;
    BoundedPayload::new(Bytes::from(bytes), max_payload)
}

/// Decodes a typed IPC payload with Postcard after frame-length validation.
pub fn decode_payload(payload: &BoundedPayload) -> Result<IpcPayload, IpcError> {
    postcard::from_bytes(payload.bytes()).map_err(|_| IpcError::PayloadDecodeFailed)
}

/// Queue capacity for memory ingress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct QueueCapacity(NonZeroUsize);

impl QueueCapacity {
    /// Creates a non-zero queue capacity.
    pub const fn new(value: NonZeroUsize) -> Self {
        Self(value)
    }

    pub(crate) fn get(self) -> usize {
        self.0.get()
    }
}

/// Maximum accepted payload bytes for an ingress frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct MaxPayloadBytes(NonZeroUsize);

impl MaxPayloadBytes {
    /// Default single-frame payload bound: 1 MiB.
    pub const DEFAULT: Self = Self(match NonZeroUsize::new(1_048_576) {
        Some(value) => value,
        None => NonZeroUsize::MIN,
    });

    /// Creates a non-zero payload limit.
    pub const fn new(value: NonZeroUsize) -> Self {
        Self(value)
    }

    pub(crate) fn get(self) -> usize {
        self.0.get()
    }
}

/// Payload accepted after a caller-visible size check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedPayload(Bytes);

impl BoundedPayload {
    /// Creates a checked bounded payload.
    pub fn new(payload: Bytes, max: MaxPayloadBytes) -> Result<Self, IpcError> {
        if payload.len() > max.get() {
            Err(IpcError::PayloadTooLarge {
                actual: payload.len(),
                limit: max.get(),
            })
        } else {
            Ok(Self(payload))
        }
    }

    /// Returns shared payload bytes.
    #[must_use]
    pub const fn bytes(&self) -> &Bytes {
        &self.0
    }
}

/// Binary frame submitted by an in-process or IPC producer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngressFrame {
    run_id: RunId,
    workflow: WorkflowDigest,
    payload: BoundedPayload,
}

impl IngressFrame {
    /// Creates a frame after applying the payload size contract.
    pub fn new(
        run_id: RunId,
        workflow: WorkflowDigest,
        payload: Bytes,
        max_payload: MaxPayloadBytes,
    ) -> Result<Self, IpcError> {
        Ok(Self {
            run_id,
            workflow,
            payload: BoundedPayload::new(payload, max_payload)?,
        })
    }

    /// Run identifier selected by the caller or allocator.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }

    /// Compiled workflow digest this frame targets.
    #[must_use]
    pub const fn workflow(&self) -> WorkflowDigest {
        self.workflow
    }

    /// Raw input bytes. Parsing/mapping is a cold boundary concern.
    #[must_use]
    pub const fn payload(&self) -> &BoundedPayload {
        &self.payload
    }
}

/// Bounded multi-producer, single-consumer memory ingress queue.
#[derive(Debug, Clone)]
pub struct MemoryIngress {
    sender: Sender<IngressFrame>,
    receiver: Receiver<IngressFrame>,
}

impl MemoryIngress {
    /// Creates a bounded memory ingress queue.
    #[must_use]
    pub fn bounded(capacity: QueueCapacity) -> Self {
        let (sender, receiver) = bounded(capacity.get());
        Self { sender, receiver }
    }

    /// Attempts to submit a frame without blocking.
    pub fn try_submit(&self, frame: IngressFrame) -> Result<(), IpcError> {
        self.sender.try_send(frame).map_err(map_try_send)
    }

    /// Attempts to receive one frame without blocking.
    pub fn try_recv(&self) -> Result<Option<IngressFrame>, IpcError> {
        match self.receiver.try_recv() {
            Ok(frame) => Ok(Some(frame)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(IpcError::Disconnected),
        }
    }

    /// Current approximate queue depth.
    #[must_use]
    pub fn len(&self) -> usize {
        self.receiver.len()
    }

    /// Returns true when no frames are queued.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.receiver.is_empty()
    }
}

/// IPC/memory ingress failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum IpcError {
    /// Queue is full and the producer must apply backpressure.
    #[error("memory ingress queue is full")]
    Full,
    /// All producers or consumers have disconnected.
    #[error("memory ingress queue is disconnected")]
    Disconnected,
    /// Payload exceeds the configured frame limit.
    #[error("ingress payload is too large: actual={actual}, limit={limit}")]
    PayloadTooLarge {
        /// Actual payload bytes.
        actual: usize,
        /// Configured limit.
        limit: usize,
    },
    /// Frame magic did not match `VBLT`.
    #[error("invalid IPC frame magic: actual={actual:#010x}")]
    InvalidMagic {
        /// Decoded magic value.
        actual: u32,
    },
    /// Frame version is not supported by this crate.
    #[error("unsupported IPC frame version: actual={actual}")]
    UnsupportedVersion {
        /// Decoded version value.
        actual: u16,
    },
    /// Command id is not part of the v1 command set.
    #[error("unknown IPC command: {0}")]
    UnknownCommand(u16),
    /// Reserved header field must remain zero.
    #[error("IPC reserved header field is non-zero: actual={actual}")]
    ReservedNonZero {
        /// Decoded reserved value.
        actual: u16,
    },
    /// Header payload length and supplied payload bytes disagree.
    #[error("IPC payload length mismatch: header={header}, actual={actual}")]
    PayloadLengthMismatch {
        /// Header-declared payload length.
        header: usize,
        /// Actual payload bytes supplied to the decoder.
        actual: usize,
    },
    /// Header could not be encoded to the fixed wire length.
    #[error("failed to encode IPC header")]
    HeaderEncodeFailed,
    /// Header bytes could not be read as fixed-width fields.
    #[error("failed to decode IPC header")]
    HeaderDecodeFailed,
    /// Payload length cannot fit this target architecture.
    #[error("IPC payload length cannot fit usize: actual={actual}")]
    PayloadLengthOutOfRange {
        /// Header-declared payload length.
        actual: u32,
    },
    /// Typed Postcard payload encoding failed.
    #[error("failed to encode IPC payload")]
    PayloadEncodeFailed,
    /// Typed Postcard payload decoding failed.
    #[error("failed to decode IPC payload")]
    PayloadDecodeFailed,
    /// Typed response payload decoding failed.
    #[error("failed to decode IPC response")]
    ResponseDecodeFailed,
}

impl IpcError {
    /// Runtime code for structurally invalid IPC frames.
    pub const IPC_FRAME_INVALID_RUNTIME_CODE: &str = "IPC_FRAME_INVALID";
    /// Runtime code for IPC payloads exceeding a configured bound.
    pub const IPC_PAYLOAD_TOO_LARGE_RUNTIME_CODE: &str = "IPC_PAYLOAD_TOO_LARGE";
    /// Runtime code for bounded IPC ingress queues at capacity.
    pub const QUEUE_FULL_RUNTIME_CODE: &str = "QUEUE_FULL";

    /// Diagnostic code for queue full.
    pub const FULL_CODE: DiagnosticCode = DiagnosticCode::new(0x3001);
    /// Diagnostic code for disconnected.
    pub const DISCONNECTED_CODE: DiagnosticCode = DiagnosticCode::new(0x3002);
    /// Diagnostic code for payload too large.
    pub const PAYLOAD_TOO_LARGE_CODE: DiagnosticCode = DiagnosticCode::new(0x3003);
    /// Diagnostic code for invalid magic.
    pub const INVALID_MAGIC_CODE: DiagnosticCode = DiagnosticCode::new(0x3004);
    /// Diagnostic code for unsupported version.
    pub const UNSUPPORTED_VERSION_CODE: DiagnosticCode = DiagnosticCode::new(0x3005);
    /// Diagnostic code for unknown command.
    pub const UNKNOWN_COMMAND_CODE: DiagnosticCode = DiagnosticCode::new(0x3006);
    /// Diagnostic code for reserved non-zero.
    pub const RESERVED_NON_ZERO_CODE: DiagnosticCode = DiagnosticCode::new(0x3007);
    /// Diagnostic code for payload length mismatch.
    pub const PAYLOAD_LENGTH_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x3008);
    /// Diagnostic code for header encode failed.
    pub const HEADER_ENCODE_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x3009);
    /// Diagnostic code for header decode failed.
    pub const HEADER_DECODE_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x300A);
    /// Diagnostic code for payload length out of range.
    pub const PAYLOAD_LENGTH_OUT_OF_RANGE_CODE: DiagnosticCode = DiagnosticCode::new(0x300B);
    /// Diagnostic code for payload encode failed.
    pub const PAYLOAD_ENCODE_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x300C);
    /// Diagnostic code for payload decode failed.
    pub const PAYLOAD_DECODE_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x300D);
    /// Diagnostic code for response decode failed.
    pub const RESPONSE_DECODE_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x300E);

    /// Returns the stable diagnostic code for this error.
    #[must_use]
    pub const fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            Self::Full => Self::FULL_CODE,
            Self::Disconnected => Self::DISCONNECTED_CODE,
            Self::PayloadTooLarge { .. } => Self::PAYLOAD_TOO_LARGE_CODE,
            Self::InvalidMagic { .. } => Self::INVALID_MAGIC_CODE,
            Self::UnsupportedVersion { .. } => Self::UNSUPPORTED_VERSION_CODE,
            Self::UnknownCommand(_) => Self::UNKNOWN_COMMAND_CODE,
            Self::ReservedNonZero { .. } => Self::RESERVED_NON_ZERO_CODE,
            Self::PayloadLengthMismatch { .. } => Self::PAYLOAD_LENGTH_MISMATCH_CODE,
            Self::HeaderEncodeFailed => Self::HEADER_ENCODE_FAILED_CODE,
            Self::HeaderDecodeFailed => Self::HEADER_DECODE_FAILED_CODE,
            Self::PayloadLengthOutOfRange { .. } => Self::PAYLOAD_LENGTH_OUT_OF_RANGE_CODE,
            Self::PayloadEncodeFailed => Self::PAYLOAD_ENCODE_FAILED_CODE,
            Self::PayloadDecodeFailed => Self::PAYLOAD_DECODE_FAILED_CODE,
            Self::ResponseDecodeFailed => Self::RESPONSE_DECODE_FAILED_CODE,
        }
    }

    /// Returns the stable section 17 runtime code when this IPC error has a direct mapping.
    #[must_use]
    pub const fn runtime_code(&self) -> Option<&'static str> {
        match self {
            Self::Full => Some(Self::QUEUE_FULL_RUNTIME_CODE),
            Self::PayloadTooLarge { .. } | Self::PayloadLengthOutOfRange { .. } => {
                Some(Self::IPC_PAYLOAD_TOO_LARGE_RUNTIME_CODE)
            }
            Self::InvalidMagic { .. }
            | Self::UnsupportedVersion { .. }
            | Self::UnknownCommand(_)
            | Self::ReservedNonZero { .. }
            | Self::PayloadLengthMismatch { .. }
            | Self::HeaderDecodeFailed
            | Self::PayloadDecodeFailed
            | Self::ResponseDecodeFailed => Some(Self::IPC_FRAME_INVALID_RUNTIME_CODE),
            Self::Disconnected | Self::HeaderEncodeFailed | Self::PayloadEncodeFailed => None,
        }
    }
}

fn u32_to_usize(value: u32) -> Result<usize, IpcError> {
    match usize::try_from(value) {
        Ok(converted) => Ok(converted),
        Err(_) => Err(IpcError::PayloadLengthOutOfRange { actual: value }),
    }
}

fn map_try_send(error: TrySendError<IngressFrame>) -> IpcError {
    match error {
        TrySendError::Full(_) => IpcError::Full,
        TrySendError::Disconnected(_) => IpcError::Disconnected,
    }
}

#[cfg(test)]
mod tests;
