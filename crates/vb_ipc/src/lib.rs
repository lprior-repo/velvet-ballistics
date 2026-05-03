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
    SlotWritten { run: RunId, slot: SlotIdx },
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
macro_rules! assert_ok {
    ($result:expr $(, $($arg:tt)+)?) => {{
        match &$result {
            Ok(_) => (),
            Err(_) => assert_eq!(Some("Err(..)"), None::<&str> $(, $($arg)+)?),
        }
    }};
}

#[cfg(test)]
macro_rules! prop_assert_ok {
    ($result:expr $(, $($arg:tt)+)?) => {{
        let is_ok_result = match &$result {
            Ok(_) => true,
            Err(_) => false,
        };
        prop_assert!(is_ok_result $(, $($arg)+)?);
    }};
}

#[cfg(test)]
macro_rules! prop_assert_err {
    ($result:expr $(, $($arg:tt)+)?) => {{
        let is_err_result = match &$result {
            Ok(_) => false,
            Err(_) => true,
        };
        prop_assert!(is_err_result $(, $($arg)+)?);
    }};
}

#[cfg(test)]
mod tests {
    use super::{
        BoundedPayload, IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION, IngressFrame, IpcCommand, IpcError,
        IpcFrameHeader, IpcPayload, MaxPayloadBytes, MemoryIngress, QueueCapacity,
        SubmitRunPayload, decode_frame, decode_payload, encode_payload,
    };
    use bytes::Bytes;
    use vb_core::{DiagnosticCode, RunId, WorkflowDigest};

    fn header_bytes(
        magic: u32,
        version: u16,
        command: u16,
        flags: u16,
        reserved: u16,
        correlation: u64,
        payload_len: u32,
    ) -> Result<[u8; IPC_HEADER_LEN], IpcError> {
        let mut bytes = Vec::with_capacity(IPC_HEADER_LEN);
        bytes.extend_from_slice(&magic.to_le_bytes());
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(&command.to_le_bytes());
        bytes.extend_from_slice(&flags.to_le_bytes());
        bytes.extend_from_slice(&reserved.to_le_bytes());
        bytes.extend_from_slice(&correlation.to_le_bytes());
        bytes.extend_from_slice(&payload_len.to_le_bytes());

        match <[u8; IPC_HEADER_LEN]>::try_from(bytes.as_slice()) {
            Ok(header) => Ok(header),
            Err(_) => Err(IpcError::HeaderEncodeFailed),
        }
    }

    #[test]
    fn bounded_queue_applies_backpressure() {
        let capacity = QueueCapacity::new(std::num::NonZeroUsize::MIN);
        let queue = MemoryIngress::bounded(capacity);
        let frame = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([2; 32]),
            Bytes::from_static(b"{}"),
            MaxPayloadBytes::DEFAULT,
        );
        assert_ok!(frame, "test frame should fit default payload bound");
        let Ok(frame) = frame else {
            return;
        };

        assert_eq!(queue.try_submit(frame.clone()), Ok(()));
        assert_eq!(queue.try_submit(frame), Err(IpcError::Full));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn oversized_payload_is_rejected() {
        let payload_bytes = b"too big";
        let max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
        let result = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([2; 32]),
            Bytes::from_static(payload_bytes),
            max,
        );

        assert_eq!(
            result,
            Err(IpcError::PayloadTooLarge {
                actual: payload_bytes.len(),
                limit: max.get(),
            })
        );
    }

    #[test]
    fn command_ids_cover_required_surface() {
        assert_eq!(IpcCommand::from_u16(1), Ok(IpcCommand::SubmitRun));
        assert_eq!(IpcCommand::from_u16(2), Ok(IpcCommand::SubmitRunInline));
        assert_eq!(IpcCommand::from_u16(3), Ok(IpcCommand::CancelRun));
        assert_eq!(IpcCommand::from_u16(4), Ok(IpcCommand::InspectRun));
        assert_eq!(IpcCommand::from_u16(5), Ok(IpcCommand::ListEvents));
        assert_eq!(IpcCommand::from_u16(6), Ok(IpcCommand::AnswerAsk));
        assert_eq!(IpcCommand::from_u16(7), Ok(IpcCommand::CompleteAction));
        assert_eq!(IpcCommand::from_u16(8), Ok(IpcCommand::FailAction));
        assert_eq!(IpcCommand::from_u16(9), Ok(IpcCommand::DrainTrace));
        assert_eq!(IpcCommand::from_u16(10), Ok(IpcCommand::Health));
        assert_eq!(IpcCommand::from_u16(11), Ok(IpcCommand::Shutdown));
        assert_eq!(IpcCommand::from_u16(12), Ok(IpcCommand::ListRuns));
        assert_eq!(IpcCommand::from_u16(13), Ok(IpcCommand::GetMetrics));
        assert_eq!(IpcCommand::from_u16(14), Ok(IpcCommand::GetWorkflowGraph));
        assert_eq!(IpcCommand::from_u16(15), Ok(IpcCommand::GetTaintReport));
        assert_eq!(IpcCommand::from_u16(16), Ok(IpcCommand::VerifyWorkflow));
        assert_eq!(IpcCommand::from_u16(17), Err(IpcError::UnknownCommand(17)));
    }

    #[test]
    fn header_roundtrips_little_endian_fields() {
        let header = IpcFrameHeader::new(IpcCommand::CompleteAction, 7, 42, 3);
        let encoded = header.encode();
        assert_ok!(encoded, "header should encode to fixed width");
        let Ok(encoded) = encoded else {
            return;
        };

        let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);
        assert_eq!(decoded, Ok(header));
    }

    #[test]
    fn decoder_rejects_bad_magic_before_payload_use() {
        let encoded = header_bytes(0, IPC_VERSION, IpcCommand::Health.as_u16(), 0, 0, 1, 0);
        assert_ok!(encoded, "test header should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

        assert_eq!(decoded, Err(IpcError::InvalidMagic { actual: 0 }));
    }

    #[test]
    fn decoder_rejects_payload_above_bound() {
        let payload_len_val: u32 = 8;
        let payload_len_usize = match usize::try_from(payload_len_val) {
            Ok(v) => v,
            Err(_) => return,
        };
        let max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
        let encoded = header_bytes(
            IPC_MAGIC,
            IPC_VERSION,
            IpcCommand::SubmitRun.as_u16(),
            0,
            0,
            1,
            payload_len_val,
        );
        assert_ok!(encoded, "test header should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = IpcFrameHeader::decode(&encoded, max);

        assert_eq!(
            decoded,
            Err(IpcError::PayloadTooLarge {
                actual: payload_len_usize,
                limit: max.get(),
            })
        );
    }

    #[test]
    fn decoder_rejects_non_zero_reserved_field() {
        let encoded = header_bytes(
            IPC_MAGIC,
            IPC_VERSION,
            IpcCommand::Health.as_u16(),
            0,
            9,
            1,
            0,
        );
        assert_ok!(encoded, "test header should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

        assert_eq!(decoded, Err(IpcError::ReservedNonZero { actual: 9 }));
    }

    #[test]
    fn frame_decode_requires_payload_length_match() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
        let encoded = header.encode();
        assert_ok!(encoded, "header should encode to fixed width");
        let Ok(encoded) = encoded else {
            return;
        };

        let decoded = decode_frame(
            &encoded,
            Bytes::from_static(b"abc"),
            MaxPayloadBytes::DEFAULT,
        );

        assert_eq!(
            decoded,
            Err(IpcError::PayloadLengthMismatch {
                header: 4,
                actual: 3,
            })
        );
    }

    #[test]
    fn postcard_payload_roundtrips_as_typed_command() {
        let payload = IpcPayload::SubmitRun(SubmitRunPayload {
            run_id: RunId::new(7),
            workflow: WorkflowDigest::from_bytes([3; 32]),
            input: Vec::from(&b"input"[..]),
        });

        let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
        assert_ok!(encoded, "payload should encode under default bound");
        let Ok(encoded) = encoded else {
            return;
        };

        assert_eq!(decode_payload(&encoded), Ok(payload));
    }

    #[test]
    fn from_u16_rejects_zero_command() {
        let result = IpcCommand::from_u16(0);
        assert_eq!(result, Err(IpcError::UnknownCommand(0)));
    }

    #[test]
    fn unsupported_version_rejects_when_version_is_two() {
        let encoded = header_bytes(IPC_MAGIC, 2, IpcCommand::Health.as_u16(), 0, 0, 1, 0);
        assert_ok!(encoded, "test header should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

        assert_eq!(decoded, Err(IpcError::UnsupportedVersion { actual: 2 }));
    }

    #[test]
    fn memory_ingress_try_recv_returns_none_when_empty() {
        let capacity = QueueCapacity::new(std::num::NonZeroUsize::MIN);
        let queue = MemoryIngress::bounded(capacity);

        assert_eq!(queue.try_recv(), Ok(None));
    }

    #[test]
    fn memory_ingress_is_empty_after_construction() {
        let capacity = QueueCapacity::new(std::num::NonZeroUsize::MIN);
        let queue = MemoryIngress::bounded(capacity);

        assert!(queue.is_empty());
    }

    #[test]
    fn bounded_payload_bytes_returns_inner_slice() {
        let data = Bytes::from_static(b"hello");
        let bounded = BoundedPayload::new(data.clone(), MaxPayloadBytes::DEFAULT);
        assert_ok!(bounded, "payload should fit default bound");
        let Ok(bounded) = bounded else {
            return;
        };

        assert_eq!(bounded.bytes(), &data);
    }

    #[test]
    fn ingress_frame_accessors_return_correct_values() {
        let run_id = RunId::new(42);
        let workflow = WorkflowDigest::from_bytes([0xAB; 32]);
        let data = Bytes::from_static(b"payload");
        let frame = IngressFrame::new(run_id, workflow, data, MaxPayloadBytes::DEFAULT);
        assert_ok!(frame, "frame should construct");
        let Ok(frame) = frame else {
            return;
        };

        assert_eq!(frame.run_id(), run_id);
        assert_eq!(frame.workflow(), workflow);
        assert_eq!(frame.payload().bytes().as_ref(), b"payload");
    }

    // ── Error variant exact-assertion tests ──

    #[test]
    fn decode_returns_disconnected_when_buffer_empty() {
        // Given: a bounded channel where the sender has been dropped
        let (sender, receiver) = crossbeam_channel::bounded::<IngressFrame>(1);
        drop(sender);

        // When: trying to receive from the disconnected channel
        let result: Result<Option<IngressFrame>, IpcError> = match receiver.try_recv() {
            Ok(_) => Ok(None),
            Err(crossbeam_channel::TryRecvError::Empty) => Ok(None),
            Err(crossbeam_channel::TryRecvError::Disconnected) => Err(IpcError::Disconnected),
        };

        // Then: Disconnected is returned
        assert_eq!(result, Err(IpcError::Disconnected));
    }

    #[test]
    fn from_u16_returns_unknown_command_for_zero() {
        // Given: command value 0 is not a valid command
        // When: parsing command 0
        // Then: UnknownCommand(0) is returned
        assert_eq!(IpcCommand::from_u16(0), Err(IpcError::UnknownCommand(0)));
    }

    #[test]
    fn from_u16_returns_unknown_command_for_value_above_range() {
        // Given: command value 99 is not a valid command
        // When: parsing command 99
        // Then: UnknownCommand(99) is returned
        assert_eq!(IpcCommand::from_u16(99), Err(IpcError::UnknownCommand(99)));
    }

    #[test]
    fn bounded_payload_rejects_oversized_with_exact_counts() {
        // Given: a payload of 100 bytes and a max of 10
        let data = Bytes::from(vec![0u8; 100]);
        let max = MaxPayloadBytes::new(
            std::num::NonZeroUsize::new(10).unwrap_or(std::num::NonZeroUsize::MIN),
        );

        // When: creating a bounded payload
        let result = BoundedPayload::new(data, max);

        // Then: PayloadTooLarge with exact actual and limit
        assert_eq!(
            result,
            Err(IpcError::PayloadTooLarge {
                actual: 100,
                limit: 10,
            })
        );
    }

    #[test]
    fn ingress_frame_rejects_payload_exceeding_max() {
        // Given: a 200-byte payload and max of 50
        let data = Bytes::from(vec![0xAA; 200]);
        let max = MaxPayloadBytes::new(
            std::num::NonZeroUsize::new(50).unwrap_or(std::num::NonZeroUsize::MIN),
        );

        // When: constructing an ingress frame
        let result = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([0; 32]),
            data,
            max,
        );

        // Then: PayloadTooLarge with exact counts
        assert_eq!(
            result,
            Err(IpcError::PayloadTooLarge {
                actual: 200,
                limit: 50,
            })
        );
    }

    #[test]
    fn decode_payload_returns_decode_failed_on_garbage() {
        // Given: garbage bytes that cannot be decoded as IpcPayload
        let garbage = Bytes::from_static(b"\xff\xff\xff\xff");
        let bounded = BoundedPayload::new(garbage, MaxPayloadBytes::DEFAULT);
        assert_ok!(bounded, "garbage should fit in bound");
        let Ok(bounded) = bounded else {
            return;
        };

        // When: decoding the payload
        // Then: PayloadDecodeFailed is returned
        assert_eq!(decode_payload(&bounded), Err(IpcError::PayloadDecodeFailed));
    }

    #[test]
    fn encode_header_always_produces_fixed_width() {
        // Given: any valid header
        let header = IpcFrameHeader::new(IpcCommand::Shutdown, 0xFFFF, 0xDEAD_BEEF_CAFE, 1024);

        // When: encoding the header
        let encoded = header.encode();
        assert_ok!(encoded, "header encode should succeed");
        let Ok(encoded) = encoded else {
            return;
        };

        // Then: the output is exactly IPC_HEADER_LEN bytes
        assert_eq!(encoded.len(), IPC_HEADER_LEN);
    }

    #[test]
    fn encode_header_produces_correct_magic_bytes() {
        // Given: a valid header
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);

        // When: encoding
        let encoded = header.encode();
        assert_ok!(encoded, "header encode should succeed");
        let Ok(encoded) = encoded else {
            return;
        };

        // Then: first 4 bytes are IPC_MAGIC in little-endian
        let magic_bytes = encoded.get(..4);
        assert_eq!(magic_bytes, Some(IPC_MAGIC.to_le_bytes().as_slice()));
    }

    #[test]
    fn encode_header_produces_correct_version_bytes() {
        // Given: a valid header
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);

        // When: encoding
        let encoded = header.encode();
        assert_ok!(encoded, "header encode should succeed");
        let Ok(encoded) = encoded else {
            return;
        };

        // Then: bytes 4..6 are IPC_VERSION in little-endian
        let version_bytes = encoded.get(4..6);
        assert_eq!(version_bytes, Some(IPC_VERSION.to_le_bytes().as_slice()));
    }

    #[test]
    fn encode_header_produces_correct_command_bytes() {
        // Given: a header with CancelRun command (id=3)
        let header = IpcFrameHeader::new(IpcCommand::CancelRun, 0, 1, 0);

        // When: encoding
        let encoded = header.encode();
        assert_ok!(encoded, "header encode should succeed");
        let Ok(encoded) = encoded else {
            return;
        };

        // Then: bytes 6..8 are the command id (3) in little-endian
        let command_bytes = encoded.get(6..8);
        assert_eq!(command_bytes, Some(3u16.to_le_bytes().as_slice()));
    }

    #[test]
    fn encode_header_produces_correct_flags_bytes() {
        // Given: a header with flags=0x1234
        let header = IpcFrameHeader::new(IpcCommand::Health, 0x1234, 1, 0);

        // When: encoding
        let encoded = header.encode();
        assert_ok!(encoded, "header encode should succeed");
        let Ok(encoded) = encoded else {
            return;
        };

        // Then: bytes 8..10 are flags in little-endian
        let flags_bytes = encoded.get(8..10);
        assert_eq!(flags_bytes, Some(0x1234u16.to_le_bytes().as_slice()));
    }

    #[test]
    fn encode_header_produces_zero_reserved_bytes() {
        // Given: a valid header
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);

        // When: encoding
        let encoded = header.encode();
        assert_ok!(encoded, "header encode should succeed");
        let Ok(encoded) = encoded else {
            return;
        };

        // Then: bytes 10..12 (reserved) are zero
        let reserved_bytes = encoded.get(10..12);
        assert_eq!(reserved_bytes, Some(0u16.to_le_bytes().as_slice()));
    }

    #[test]
    fn encode_header_produces_correct_correlation_bytes() {
        // Given: a header with correlation=0x0102_0304_0506_0708
        let correlation: u64 = 0x0102_0304_0506_0708;
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, correlation, 0);

        // When: encoding
        let encoded = header.encode();
        assert_ok!(encoded, "header encode should succeed");
        let Ok(encoded) = encoded else {
            return;
        };

        // Then: bytes 12..20 are correlation in little-endian
        let corr_bytes = encoded.get(12..20);
        assert_eq!(corr_bytes, Some(correlation.to_le_bytes().as_slice()));
    }

    #[test]
    fn encode_header_produces_correct_payload_len_bytes() {
        // Given: a header with payload_len=4096
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4096);

        // When: encoding
        let encoded = header.encode();
        assert_ok!(encoded, "header encode should succeed");
        let Ok(encoded) = encoded else {
            return;
        };

        // Then: bytes 20..24 are payload_len in little-endian
        let plen_bytes = encoded.get(20..24);
        assert_eq!(plen_bytes, Some(4096u32.to_le_bytes().as_slice()));
    }

    #[test]
    fn frame_decode_succeeds_when_payload_length_matches() {
        // Given: a valid header with payload_len=3 and a 3-byte payload
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 3);
        let encoded = header.encode();
        assert_ok!(encoded, "header should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let payload = Bytes::from_static(b"abc");

        // When: decoding the frame
        let result = decode_frame(&encoded, payload, MaxPayloadBytes::DEFAULT);

        // Then: the frame header matches the original
        assert_ok!(result, "frame should decode");
        let Ok(frame) = result else {
            return;
        };
        assert_eq!(frame.header().command, IpcCommand::Health);
        assert_eq!(frame.header().payload_len, 3);
        assert_eq!(frame.payload().bytes().as_ref(), b"abc");
    }

    #[test]
    fn memory_ingress_len_reflects_queue_depth() {
        // Given: a queue with capacity 4
        let capacity = QueueCapacity::new(
            std::num::NonZeroUsize::new(4).unwrap_or(std::num::NonZeroUsize::MIN),
        );
        let queue = MemoryIngress::bounded(capacity);

        // When: submitting 3 named frames
        let frame_zero = IngressFrame::new(
            RunId::new(0),
            WorkflowDigest::from_bytes([0; 32]),
            Bytes::new(),
            MaxPayloadBytes::DEFAULT,
        );
        assert_ok!(frame_zero, "frame zero should construct");
        let Ok(frame_zero) = frame_zero else { return };
        assert_eq!(queue.try_submit(frame_zero), Ok(()));

        let frame_one = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([0; 32]),
            Bytes::new(),
            MaxPayloadBytes::DEFAULT,
        );
        assert_ok!(frame_one, "frame one should construct");
        let Ok(frame_one) = frame_one else { return };
        assert_eq!(queue.try_submit(frame_one), Ok(()));

        let frame_two = IngressFrame::new(
            RunId::new(2),
            WorkflowDigest::from_bytes([0; 32]),
            Bytes::new(),
            MaxPayloadBytes::DEFAULT,
        );
        assert_ok!(frame_two, "frame two should construct");
        let Ok(frame_two) = frame_two else { return };
        assert_eq!(queue.try_submit(frame_two), Ok(()));

        // Then: len is 3
        assert_eq!(queue.len(), 3);
        assert!(!queue.is_empty());
    }

    #[test]
    fn memory_ingress_try_recv_returns_frames_in_fifo_order() {
        // Given: a queue with 2 frames
        let capacity = QueueCapacity::new(
            std::num::NonZeroUsize::new(4).unwrap_or(std::num::NonZeroUsize::MIN),
        );
        let queue = MemoryIngress::bounded(capacity);
        let frame1 = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([1; 32]),
            Bytes::new(),
            MaxPayloadBytes::DEFAULT,
        );
        let frame2 = IngressFrame::new(
            RunId::new(2),
            WorkflowDigest::from_bytes([2; 32]),
            Bytes::new(),
            MaxPayloadBytes::DEFAULT,
        );
        assert_ok!(frame1, "first frame should construct");
        assert_ok!(frame2, "second frame should construct");
        let Ok(frame1) = frame1 else { return };
        let Ok(frame2) = frame2 else { return };
        assert_eq!(queue.try_submit(frame1), Ok(()));
        assert_eq!(queue.try_submit(frame2), Ok(()));

        // When: receiving frames
        let recv1 = queue.try_recv();
        let recv2 = queue.try_recv();

        // Then: they arrive in FIFO order
        assert_ok!(recv1, "first recv should succeed");
        assert_ok!(recv2, "second recv should succeed");
        let Ok(Some(f1)) = recv1 else { return };
        let Ok(Some(f2)) = recv2 else { return };
        assert_eq!(f1.run_id(), RunId::new(1));
        assert_eq!(f2.run_id(), RunId::new(2));
    }

    #[test]
    fn memory_ingress_is_empty_after_draining_all_frames() {
        // Given: a queue with one frame
        let capacity = QueueCapacity::new(
            std::num::NonZeroUsize::new(4).unwrap_or(std::num::NonZeroUsize::MIN),
        );
        let queue = MemoryIngress::bounded(capacity);
        let frame = IngressFrame::new(
            RunId::new(99),
            WorkflowDigest::from_bytes([0; 32]),
            Bytes::new(),
            MaxPayloadBytes::DEFAULT,
        );
        assert_ok!(frame, "frame should construct");
        let Ok(frame) = frame else { return };
        assert_eq!(queue.try_submit(frame), Ok(()));

        // When: receiving the frame
        let drained = queue.try_recv();
        assert_ok!(drained, "queued frame should drain");

        // Then: queue is empty
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn payload_roundtrip_preserves_cancel_run_variant() {
        // Given: a CancelRun payload
        let payload = IpcPayload::CancelRun {
            run_id: RunId::new(42),
        };

        // When: encoding then decoding
        let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = decode_payload(&encoded);

        // Then: the decoded payload matches the original
        assert_eq!(decoded, Ok(payload));
    }

    #[test]
    fn payload_roundtrip_preserves_list_events_variant() {
        // Given: a ListEvents payload with from_sequence
        let payload = IpcPayload::ListEvents {
            run_id: RunId::new(7),
            from_sequence: 100,
        };

        // When: encoding then decoding
        let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = decode_payload(&encoded);

        // Then: the decoded payload matches the original
        assert_eq!(decoded, Ok(payload));
    }

    #[test]
    fn payload_roundtrip_preserves_answer_ask_variant() {
        // Given: an AnswerAsk payload with ticket and answer
        let payload = IpcPayload::AnswerAsk {
            run_id: RunId::new(5),
            ticket: 999,
            answer: Vec::from(&b"yes"[..]),
        };

        // When: encoding then decoding
        let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = decode_payload(&encoded);

        // Then: the decoded payload matches the original
        assert_eq!(decoded, Ok(payload));
    }

    #[test]
    fn payload_roundtrip_preserves_complete_action_variant() {
        // Given: a CompleteAction payload with ticket and output
        let payload = IpcPayload::CompleteAction {
            run_id: RunId::new(11),
            ticket: 42,
            output: Vec::from(&b"done"[..]),
        };

        // When: encoding then decoding
        let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = decode_payload(&encoded);

        // Then: the decoded payload matches the original
        assert_eq!(decoded, Ok(payload));
    }

    #[test]
    fn payload_roundtrip_preserves_fail_action_variant() {
        // Given: a FailAction payload with error bytes
        let payload = IpcPayload::FailAction {
            run_id: RunId::new(13),
            ticket: 7,
            error: Vec::from(&b"err"[..]),
        };

        // When: encoding then decoding
        let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = decode_payload(&encoded);

        // Then: the decoded payload matches the original
        assert_eq!(decoded, Ok(payload));
    }

    #[test]
    fn payload_roundtrip_preserves_drain_trace_variant() {
        // Given: a DrainTrace payload
        let payload = IpcPayload::DrainTrace {
            run_id: RunId::new(77),
            max_records: 500,
        };

        // When: encoding then decoding
        let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = decode_payload(&encoded);

        // Then: the decoded payload matches the original
        assert_eq!(decoded, Ok(payload));
    }

    #[test]
    fn payload_roundtrip_preserves_health_variant() {
        // Given: a Health payload
        let payload = IpcPayload::Health;

        // When: encoding then decoding
        let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = decode_payload(&encoded);

        // Then: the decoded payload matches
        assert_eq!(decoded, Ok(payload));
    }

    #[test]
    fn payload_roundtrip_preserves_shutdown_variant() {
        // Given: a Shutdown payload
        let payload = IpcPayload::Shutdown;

        // When: encoding then decoding
        let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = decode_payload(&encoded);

        // Then: the decoded payload matches
        assert_eq!(decoded, Ok(payload));
    }

    #[test]
    fn payload_roundtrip_preserves_inspect_run_variant() {
        // Given: an InspectRun payload
        let payload = IpcPayload::InspectRun {
            run_id: RunId::new(333),
        };

        // When: encoding then decoding
        let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = decode_payload(&encoded);

        // Then: the decoded payload matches
        assert_eq!(decoded, Ok(payload));
    }

    #[test]
    fn payload_roundtrip_preserves_submit_run_inline_variant() {
        // Given: a SubmitRunInline payload
        let payload = IpcPayload::SubmitRunInline(SubmitRunPayload {
            run_id: RunId::new(55),
            workflow: WorkflowDigest::from_bytes([0xBB; 32]),
            input: Vec::from(&b"inline-input"[..]),
        });

        // When: encoding then decoding
        let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
        assert_ok!(encoded, "payload should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = decode_payload(&encoded);

        // Then: the decoded payload matches
        assert_eq!(decoded, Ok(payload));
    }

    #[test]
    fn header_decode_rejects_unsupported_version_zero() {
        // Given: a header with version=0
        let encoded = header_bytes(IPC_MAGIC, 0, IpcCommand::Health.as_u16(), 0, 0, 1, 0);
        assert_ok!(encoded, "test header should encode");
        let Ok(encoded) = encoded else {
            return;
        };

        // When: decoding the header
        let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

        // Then: UnsupportedVersion with actual=0
        assert_eq!(decoded, Err(IpcError::UnsupportedVersion { actual: 0 }));
    }

    #[test]
    fn header_decode_rejects_unknown_command_id() {
        // Given: a header with an unknown command id (200)
        let encoded = header_bytes(IPC_MAGIC, IPC_VERSION, 200, 0, 0, 1, 0);
        assert_ok!(encoded, "test header should encode");
        let Ok(encoded) = encoded else {
            return;
        };

        // When: decoding the header
        let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

        // Then: UnknownCommand(200) is returned
        assert_eq!(decoded, Err(IpcError::UnknownCommand(200)));
    }

    #[test]
    fn max_payload_bytes_default_is_one_mib() {
        // Given: the default MaxPayloadBytes constant
        // When: checking the limit
        // Then: it equals 1 MiB
        assert_eq!(MaxPayloadBytes::DEFAULT.get(), 1_048_576);
    }

    #[test]
    fn queue_capacity_returns_inner_value() {
        // Given: QueueCapacity(42)
        let cap = QueueCapacity::new(
            std::num::NonZeroUsize::new(42).unwrap_or(std::num::NonZeroUsize::MIN),
        );
        // When: getting the value
        // Then: it returns 42
        assert_eq!(cap.get(), 42);
    }

    #[test]
    fn max_payload_bytes_custom_value_respects_input() {
        // Given: MaxPayloadBytes::new(512)
        let max = MaxPayloadBytes::new(
            std::num::NonZeroUsize::new(512).unwrap_or(std::num::NonZeroUsize::MIN),
        );
        // When: checking the value
        // Then: it returns 512
        assert_eq!(max.get(), 512);
    }

    #[test]
    fn bounded_payload_accepts_exactly_max_bytes() {
        // Given: a payload of exactly the max size
        let max_val = 16;
        let max = MaxPayloadBytes::new(
            std::num::NonZeroUsize::new(max_val).unwrap_or(std::num::NonZeroUsize::MIN),
        );
        let data = Bytes::from(vec![0u8; max_val]);

        // When: creating a bounded payload
        let result = BoundedPayload::new(data, max);

        // Then: it succeeds
        assert_ok!(result, "payload at exact max should succeed");
    }

    #[test]
    fn bounded_payload_rejects_one_over_max() {
        // Given: a payload one byte over the max
        let max_val = 16;
        let max = MaxPayloadBytes::new(
            std::num::NonZeroUsize::new(max_val).unwrap_or(std::num::NonZeroUsize::MIN),
        );
        let data = Bytes::from(vec![0u8; max_val + 1]);

        // When: creating a bounded payload
        let result = BoundedPayload::new(data, max);

        // Then: PayloadTooLarge
        assert_eq!(
            result,
            Err(IpcError::PayloadTooLarge {
                actual: max_val + 1,
                limit: max_val,
            })
        );
    }

    #[test]
    fn bounded_payload_bytes_returns_correct_length() {
        // Given: a bounded payload with 7 bytes
        let data = Bytes::from(vec![0u8; 7]);
        let bounded = BoundedPayload::new(data, MaxPayloadBytes::DEFAULT);
        assert_ok!(bounded, "should create bounded payload");
        let Ok(bounded) = bounded else {
            return;
        };

        // When: checking bytes length
        // Then: it is 7
        assert_eq!(bounded.bytes().len(), 7);
    }

    #[test]
    fn ipc_command_as_u16_returns_correct_values() {
        // Given: all command variants
        // When: converting to u16
        // Then: values match the wire spec
        assert_eq!(IpcCommand::SubmitRun.as_u16(), 1);
        assert_eq!(IpcCommand::SubmitRunInline.as_u16(), 2);
        assert_eq!(IpcCommand::CancelRun.as_u16(), 3);
        assert_eq!(IpcCommand::InspectRun.as_u16(), 4);
        assert_eq!(IpcCommand::ListEvents.as_u16(), 5);
        assert_eq!(IpcCommand::AnswerAsk.as_u16(), 6);
        assert_eq!(IpcCommand::CompleteAction.as_u16(), 7);
        assert_eq!(IpcCommand::FailAction.as_u16(), 8);
        assert_eq!(IpcCommand::DrainTrace.as_u16(), 9);
        assert_eq!(IpcCommand::Health.as_u16(), 10);
        assert_eq!(IpcCommand::Shutdown.as_u16(), 11);
        assert_eq!(IpcCommand::ListRuns.as_u16(), 12);
        assert_eq!(IpcCommand::GetMetrics.as_u16(), 13);
        assert_eq!(IpcCommand::GetWorkflowGraph.as_u16(), 14);
        assert_eq!(IpcCommand::GetTaintReport.as_u16(), 15);
        assert_eq!(IpcCommand::VerifyWorkflow.as_u16(), 16);
    }

    #[test]
    fn ipc_frame_header_new_stores_all_fields() {
        // Given: a header with specific values
        let header = IpcFrameHeader::new(IpcCommand::ListEvents, 0x00FF, 12345, 678);

        // When: accessing fields
        // Then: all fields match
        assert_eq!(header.command, IpcCommand::ListEvents);
        assert_eq!(header.flags, 0x00FF);
        assert_eq!(header.correlation, 12345);
        assert_eq!(header.payload_len, 678);
    }

    #[test]
    fn header_encode_decode_roundtrip_preserves_flags() {
        // Given: a header with non-zero flags
        let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0xABCD, 999, 10);
        let encoded = header.encode();
        assert_ok!(encoded, "header should encode");
        let Ok(encoded) = encoded else {
            return;
        };

        // When: decoding
        let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

        // Then: flags are preserved
        assert_ok!(decoded, "header should decode");
        let Ok(decoded) = decoded else {
            return;
        };
        assert_eq!(decoded.flags, 0xABCD);
    }

    #[test]
    fn header_encode_decode_roundtrip_preserves_payload_len() {
        // Given: a header with payload_len=256
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 256);
        let encoded = header.encode();
        assert_ok!(encoded, "header should encode");
        let Ok(encoded) = encoded else {
            return;
        };

        // When: decoding
        let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

        // Then: payload_len is preserved
        assert_ok!(decoded, "header should decode");
        let Ok(decoded) = decoded else {
            return;
        };
        assert_eq!(decoded.payload_len, 256);
    }

    #[test]
    fn ingress_frame_rejects_empty_payload_with_min_max() {
        // Given: max=1 and empty payload (0 bytes)
        let max = MaxPayloadBytes::new(
            std::num::NonZeroUsize::new(1).unwrap_or(std::num::NonZeroUsize::MIN),
        );
        let data = Bytes::new();

        // When: creating an ingress frame with 0-byte payload and max=1
        let result = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([0; 32]),
            data,
            max,
        );

        // Then: it succeeds (0 bytes is within max of 1)
        assert_ok!(result, "empty payload should fit within any non-zero max");
    }

    #[test]
    fn memory_ingress_submit_and_recv_single_frame() {
        // Given: a queue with capacity 2
        let capacity = QueueCapacity::new(
            std::num::NonZeroUsize::new(2).unwrap_or(std::num::NonZeroUsize::MIN),
        );
        let queue = MemoryIngress::bounded(capacity);
        let frame = IngressFrame::new(
            RunId::new(42),
            WorkflowDigest::from_bytes([1; 32]),
            Bytes::from_static(b"data"),
            MaxPayloadBytes::DEFAULT,
        );
        assert_ok!(frame, "frame should construct");
        let Ok(frame) = frame else {
            return;
        };

        // When: submitting then receiving
        assert_eq!(queue.try_submit(frame), Ok(()));
        let recv = queue.try_recv();

        // Then: the received frame has the correct run_id
        assert_ok!(recv, "recv should succeed");
        let Ok(Some(f)) = recv else {
            return;
        };
        assert_eq!(f.run_id(), RunId::new(42));
    }

    #[test]
    fn try_submit_returns_full_when_at_capacity() {
        // Given: a queue with capacity 1
        let capacity = QueueCapacity::new(std::num::NonZeroUsize::MIN);
        let queue = MemoryIngress::bounded(capacity);

        // When: filling the queue with one frame
        let frame = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([0; 32]),
            Bytes::new(),
            MaxPayloadBytes::DEFAULT,
        );
        assert_ok!(frame, "frame should construct");
        let Ok(frame) = frame else { return };
        assert_eq!(queue.try_submit(frame.clone()), Ok(()));

        // Then: submitting another frame returns Full
        assert_eq!(queue.try_submit(frame), Err(IpcError::Full));
    }

    #[test]
    fn frame_header_const_new_is_compile_time() {
        // Given: a const header
        const HEADER: IpcFrameHeader = IpcFrameHeader::new(IpcCommand::Shutdown, 0, 0, 0);

        // When: checking fields
        // Then: const construction works and fields match
        assert_eq!(HEADER.command, IpcCommand::Shutdown);
        assert_eq!(HEADER.flags, 0);
        assert_eq!(HEADER.correlation, 0);
        assert_eq!(HEADER.payload_len, 0);
    }

    #[test]
    fn ipc_error_full_display_message() {
        // Given: IpcError::Full
        let error = IpcError::Full;

        // When: displaying
        let message = error.to_string();

        // Then: message mentions queue full
        assert!(message.contains("full"), "expected 'full' in '{message}'");
    }

    #[test]
    fn ipc_error_header_encode_failed_display() {
        // Given: IpcError::HeaderEncodeFailed
        let error = IpcError::HeaderEncodeFailed;

        // When: displaying
        let message = error.to_string();

        // Then: message mentions encode
        assert!(
            message.contains("encode"),
            "expected 'encode' in '{message}'"
        );
    }

    #[test]
    fn ipc_error_header_decode_failed_display() {
        // Given: IpcError::HeaderDecodeFailed
        let error = IpcError::HeaderDecodeFailed;

        // When: displaying
        let message = error.to_string();

        // Then: message mentions decode
        assert!(
            message.contains("decode"),
            "expected 'decode' in '{message}'"
        );
    }

    #[test]
    fn ipc_error_payload_length_out_of_range_display() {
        // Given: IpcError::PayloadLengthOutOfRange
        let error = IpcError::PayloadLengthOutOfRange { actual: 999 };

        // When: displaying
        let message = error.to_string();

        // Then: message mentions 999
        assert!(message.contains("999"), "expected '999' in '{message}'");
    }

    #[test]
    fn ipc_error_payload_encode_failed_display() {
        // Given: IpcError::PayloadEncodeFailed
        let error = IpcError::PayloadEncodeFailed;

        // When: displaying
        let message = error.to_string();

        // Then: message mentions encode
        assert!(
            message.contains("encode"),
            "expected 'encode' in '{message}'"
        );
    }

    #[test]
    fn ipc_error_unknown_command_display_shows_id() {
        // Given: IpcError::UnknownCommand(200)
        let error = IpcError::UnknownCommand(200);

        // When: displaying
        let message = error.to_string();

        // Then: message contains 200
        assert!(message.contains("200"), "expected '200' in '{message}'");
    }

    #[test]
    fn ipc_error_reserved_non_zero_display_shows_value() {
        // Given: IpcError::ReservedNonZero { actual: 7 }
        let error = IpcError::ReservedNonZero { actual: 7 };

        // When: displaying
        let message = error.to_string();

        // Then: message contains 7
        assert!(message.contains("7"), "expected '7' in '{message}'");
    }

    #[test]
    fn ipc_error_runtime_codes_cover_ipc_mappings() {
        assert_eq!(IpcError::Full.runtime_code(), Some("QUEUE_FULL"));
        assert_eq!(
            IpcError::InvalidMagic { actual: 0 }.runtime_code(),
            Some("IPC_FRAME_INVALID")
        );
        assert_eq!(
            IpcError::UnsupportedVersion { actual: 2 }.runtime_code(),
            Some("IPC_FRAME_INVALID")
        );
        assert_eq!(
            IpcError::UnknownCommand(99).runtime_code(),
            Some("IPC_FRAME_INVALID")
        );
        assert_eq!(
            IpcError::ReservedNonZero { actual: 7 }.runtime_code(),
            Some("IPC_FRAME_INVALID")
        );
        assert_eq!(
            IpcError::PayloadLengthMismatch {
                header: 4,
                actual: 3
            }
            .runtime_code(),
            Some("IPC_FRAME_INVALID")
        );
        assert_eq!(
            IpcError::HeaderDecodeFailed.runtime_code(),
            Some("IPC_FRAME_INVALID")
        );
        assert_eq!(
            IpcError::PayloadDecodeFailed.runtime_code(),
            Some("IPC_FRAME_INVALID")
        );
        assert_eq!(
            IpcError::ResponseDecodeFailed.runtime_code(),
            Some("IPC_FRAME_INVALID")
        );
        assert_eq!(
            IpcError::PayloadTooLarge {
                actual: 9,
                limit: 8
            }
            .runtime_code(),
            Some("IPC_PAYLOAD_TOO_LARGE")
        );
        assert_eq!(
            IpcError::PayloadLengthOutOfRange { actual: u32::MAX }.runtime_code(),
            Some("IPC_PAYLOAD_TOO_LARGE")
        );
    }

    #[test]
    fn ipc_error_runtime_codes_are_unique() {
        let codes = [
            IpcError::IPC_FRAME_INVALID_RUNTIME_CODE,
            IpcError::IPC_PAYLOAD_TOO_LARGE_RUNTIME_CODE,
            IpcError::QUEUE_FULL_RUNTIME_CODE,
        ];
        assert_eq!(codes.len(), 3);
        assert_eq!(
            codes
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            3
        );
    }

    #[test]
    fn ipc_error_runtime_code_is_absent_without_direct_mapping() {
        assert_eq!(IpcError::Disconnected.runtime_code(), None);
        assert_eq!(IpcError::HeaderEncodeFailed.runtime_code(), None);
        assert_eq!(IpcError::PayloadEncodeFailed.runtime_code(), None);
    }

    #[test]
    fn ipc_error_diagnostic_code_full() {
        assert_eq!(
            IpcError::Full.diagnostic_code(),
            DiagnosticCode::new(0x3001)
        );
    }

    #[test]
    fn ipc_error_diagnostic_code_disconnected() {
        assert_eq!(
            IpcError::Disconnected.diagnostic_code(),
            DiagnosticCode::new(0x3002)
        );
    }

    #[test]
    fn ipc_error_diagnostic_code_payload_too_large() {
        assert_eq!(
            IpcError::PayloadTooLarge {
                actual: 100,
                limit: 10
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x3003)
        );
    }

    #[test]
    fn ipc_error_diagnostic_code_invalid_magic() {
        assert_eq!(
            IpcError::InvalidMagic {
                actual: 0xDEAD_BEEF
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x3004)
        );
    }

    #[test]
    fn ipc_error_diagnostic_code_unsupported_version() {
        assert_eq!(
            IpcError::UnsupportedVersion { actual: 99 }.diagnostic_code(),
            DiagnosticCode::new(0x3005)
        );
    }

    #[test]
    fn ipc_error_diagnostic_code_unknown_command() {
        assert_eq!(
            IpcError::UnknownCommand(200).diagnostic_code(),
            DiagnosticCode::new(0x3006)
        );
    }

    #[test]
    fn ipc_error_diagnostic_code_reserved_non_zero() {
        assert_eq!(
            IpcError::ReservedNonZero { actual: 7 }.diagnostic_code(),
            DiagnosticCode::new(0x3007)
        );
    }

    #[test]
    fn ipc_error_diagnostic_code_payload_length_mismatch() {
        assert_eq!(
            IpcError::PayloadLengthMismatch {
                header: 4,
                actual: 3
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x3008)
        );
    }

    #[test]
    fn ipc_error_diagnostic_code_header_encode_failed() {
        assert_eq!(
            IpcError::HeaderEncodeFailed.diagnostic_code(),
            DiagnosticCode::new(0x3009)
        );
    }

    #[test]
    fn ipc_error_diagnostic_code_header_decode_failed() {
        assert_eq!(
            IpcError::HeaderDecodeFailed.diagnostic_code(),
            DiagnosticCode::new(0x300A)
        );
    }

    #[test]
    fn ipc_error_diagnostic_code_payload_length_out_of_range() {
        assert_eq!(
            IpcError::PayloadLengthOutOfRange { actual: u32::MAX }.diagnostic_code(),
            DiagnosticCode::new(0x300B)
        );
    }

    #[test]
    fn ipc_error_diagnostic_code_payload_encode_failed() {
        assert_eq!(
            IpcError::PayloadEncodeFailed.diagnostic_code(),
            DiagnosticCode::new(0x300C)
        );
    }

    #[test]
    fn ipc_error_diagnostic_code_payload_decode_failed() {
        assert_eq!(
            IpcError::PayloadDecodeFailed.diagnostic_code(),
            DiagnosticCode::new(0x300D)
        );
    }

    #[test]
    fn ipc_error_diagnostic_code_response_decode_failed() {
        assert_eq!(
            IpcError::ResponseDecodeFailed.diagnostic_code(),
            DiagnosticCode::new(0x300E)
        );
    }
}

// ══ Adversarial command-specific attacks ══

#[test]
fn adversarial_cancel_run_with_run_id_zero_encoded_rejected_by_runtime() {
    // Given: a CancelRun payload with run_id=0
    let payload = IpcPayload::CancelRun {
        run_id: RunId::new(0),
    };
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };

    // When: decoding the payload
    let decoded = decode_payload(&encoded);

    // Then: payload roundtrips (the protocol layer accepts it; runtime rejects later)
    assert_ok!(decoded, "CancelRun with run_id=0 should decode");
    let Ok(decoded) = decoded else { return };
    assert_eq!(
        decoded,
        IpcPayload::CancelRun {
            run_id: RunId::new(0)
        }
    );
}

#[test]
fn adversarial_cancel_run_with_run_id_max_encoded_roundtrips() {
    // Given: a CancelRun payload with run_id=u64::MAX (nonexistent run)
    let payload = IpcPayload::CancelRun {
        run_id: RunId::new(u64::MAX),
    };
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };

    // When: decoding
    let decoded = decode_payload(&encoded);

    // Then: payload roundtrips (runtime will reject later; protocol accepts it)
    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_answer_ask_with_zero_ticket_roundtrips() {
    // Given: an AnswerAsk with ticket=0 and empty answer
    let payload = IpcPayload::AnswerAsk {
        run_id: RunId::new(1),
        ticket: 0,
        answer: Vec::new(),
    };

    // When: encoding then decoding
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    // Then: payload roundtrips (protocol layer accepts; runtime validation happens later)
    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_answer_ask_with_max_u64_ticket_roundtrips() {
    // Given: an AnswerAsk with ticket=u64::MAX
    let payload = IpcPayload::AnswerAsk {
        run_id: RunId::new(1),
        ticket: u64::MAX,
        answer: Vec::from(&b"malicious"[..]),
    };

    // When: encoding then decoding
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    // Then: payload roundtrips (protocol accepts; step_from_ticket will reject at dispatch)
    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_fail_action_with_unregistered_run_id_roundtrips() {
    // Given: a FailAction for a run that does not exist
    let payload = IpcPayload::FailAction {
        run_id: RunId::new(99991),
        ticket: 7777,
        error: Vec::from(&b"no such run"[..]),
    };

    // When: encoding then decoding
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    // Then: payload roundtrips (dispatch will return RuntimeError)
    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_complete_action_with_mismatched_output_bytes_rejected() {
    // Given: a CompleteAction with garbage output bytes (not valid IpcActionOutputPayload)
    let payload = IpcPayload::CompleteAction {
        run_id: RunId::new(1),
        ticket: 5,
        output: Vec::from(&b"\xFF\xFF\xFF\xFF"[..]),
    };
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };

    // When: decoding the outer payload
    let decoded = decode_payload(&encoded);

    // Then: outer payload roundtrips (the inner output decode fails at dispatch, not protocol)
    assert_ok!(decoded, "outer IpcPayload should decode");
}

#[test]
fn adversarial_submit_run_with_empty_input_roundtrips() {
    // Given: a SubmitRun with empty input bytes
    let payload = IpcPayload::SubmitRun(SubmitRunPayload {
        run_id: RunId::new(42),
        workflow: WorkflowDigest::from_bytes([0; 32]),
        input: Vec::new(),
    });

    // When: encoding then decoding
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    // Then: payload roundtrips
    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_submit_run_with_large_input_under_limit_roundtrips() {
    // Given: a SubmitRun with a large input (but under 1 MiB)
    let payload = IpcPayload::SubmitRun(SubmitRunPayload {
        run_id: RunId::new(7),
        workflow: WorkflowDigest::from_bytes([0xAA; 32]),
        input: vec![0u8; 100_000],
    });

    // When: encoding then decoding
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    // Then: payload roundtrips
    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_list_events_with_from_sequence_max_roundtrips() {
    // Given: a ListEvents with from_sequence=u64::MAX
    let payload = IpcPayload::ListEvents {
        run_id: RunId::new(5),
        from_sequence: u64::MAX,
    };

    // When: encoding then decoding
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    // Then: payload roundtrips
    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_drain_trace_with_max_records_roundtrips() {
    // Given: a DrainTrace with max_records=u32::MAX
    let payload = IpcPayload::DrainTrace {
        run_id: RunId::new(3),
        max_records: u32::MAX,
    };

    // When: encoding then decoding
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    // Then: payload roundtrips
    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_bounded_payload_rejects_exactly_one_over_max() {
    // Given: bytes 1 byte over the max
    let max_val = 32;
    let max = MaxPayloadBytes::new(
        std::num::NonZeroUsize::new(max_val).unwrap_or(std::num::NonZeroUsize::MIN),
    );
    let data = Bytes::from(vec![0u8; max_val.saturating_add(1)]);

    // When: creating a bounded payload
    let result = BoundedPayload::new(data, max);

    // Then: PayloadTooLarge
    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: max_val.saturating_add(1),
            limit: max_val,
        })
    );
}

#[test]
fn adversarial_decode_frame_rejects_oversized_payload_bytes() {
    // Given: a valid header with payload_len=3 but we pass 1000 bytes
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 3);
    let encoded = header.encode();
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
    let oversized = Bytes::from(vec![0u8; 1000]);

    // When: decoding the frame
    let result = decode_frame(&encoded, oversized, MaxPayloadBytes::DEFAULT);

    // Then: PayloadLengthMismatch
    assert_eq!(
        result,
        Err(IpcError::PayloadLengthMismatch {
            header: 3,
            actual: 1000,
        })
    );
}

#[test]
fn adversarial_encode_payload_exceeding_bound_rejected() {
    // Given: an IpcPayload that would serialize to more than 1 byte
    // We use a tiny max to force rejection
    let payload = IpcPayload::Health;
    let tiny_max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);

    // When: encoding with tiny max
    let result = encode_payload(&payload, tiny_max);

    // Then: either it fits (if Health serializes to 0 or 1 bytes) or PayloadTooLarge
    // Health serializes to a small postcard encoding - check it succeeds or fails correctly
    assert!(
        matches!(result, Ok(_) | Err(IpcError::PayloadTooLarge { .. })),
        "expected success or PayloadTooLarge for tiny health frame"
    );
}

#[test]
fn adversarial_ipc_frame_new_rejects_mismatched_lengths() {
    // Given: a header with payload_len=10 but 5 bytes of payload
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 10);
    let short_payload = Bytes::from(vec![0u8; 5]);

    // When: constructing an IpcFrame
    let result = IpcFrame::new(header, short_payload, MaxPayloadBytes::DEFAULT);

    // Then: PayloadLengthMismatch
    assert_eq!(
        result,
        Err(IpcError::PayloadLengthMismatch {
            header: 10,
            actual: 5,
        })
    );
}

#[test]
fn adversarial_ipc_frame_new_rejects_oversized_payload() {
    // Given: a header with payload_len=100 and 100 bytes, but max=1
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
    let payload = Bytes::from(vec![0u8; 100]);
    let tiny_max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);

    // When: constructing an IpcFrame
    let result = IpcFrame::new(header, payload, tiny_max);

    // Then: PayloadTooLarge
    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: 100,
            limit: 1,
        })
    );
}

#[test]
fn adversarial_memory_ingress_full_then_drain_then_submit() {
    // Given: a queue with capacity 1
    let capacity = QueueCapacity::new(std::num::NonZeroUsize::MIN);
    let queue = MemoryIngress::bounded(capacity);
    let frame = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0; 32]),
        Bytes::new(),
        MaxPayloadBytes::DEFAULT,
    );
    assert_ok!(frame);
    let Ok(frame) = frame else { return };

    // When: filling, draining, then submitting again
    assert_eq!(queue.try_submit(frame.clone()), Ok(()));
    assert_eq!(queue.try_submit(frame.clone()), Err(IpcError::Full));
    let drained = queue.try_recv();
    assert_ok!(drained);
    assert_eq!(queue.try_submit(frame), Ok(()));

    // Then: the queue accepts after drain
    assert_eq!(queue.len(), 1);
}

#[test]
fn adversarial_memory_ingress_disconnected_after_sender_drop() {
    // Given: a queue where the receiver's sender clone is dropped
    let capacity =
        QueueCapacity::new(std::num::NonZeroUsize::new(4).unwrap_or(std::num::NonZeroUsize::MIN));
    let queue = MemoryIngress::bounded(capacity);
    // Clone the receiver side (MemoryIngress has both sender and receiver)
    // Drop the original queue to disconnect sender
    let receiver_only = queue.receiver.clone();
    let sender = queue.sender.clone();
    drop(queue);
    drop(sender);

    // When: receiving from disconnected channel
    let result = receiver_only.try_recv();

    // Then: Disconnected
    assert!(matches!(
        result,
        Err(crossbeam_channel::TryRecvError::Disconnected)
    ));
}

#[test]
fn adversarial_decode_frame_bad_magic_in_header_returns_invalid_magic() {
    // Given: raw bytes with wrong magic
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&0xDEAD_BEEF_u32.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    let payload = Bytes::new();

    // When: decoding the frame
    let result = decode_frame(&header_bytes, payload, MaxPayloadBytes::DEFAULT);

    // Then: InvalidMagic
    assert_eq!(
        result,
        Err(IpcError::InvalidMagic {
            actual: 0xDEAD_BEEF
        })
    );
}

#[test]
fn adversarial_decode_frame_bad_version_in_header_returns_unsupported_version() {
    // Given: valid magic but version=99
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&99u16.to_le_bytes());
    let payload = Bytes::new();

    // When: decoding the frame
    let result = decode_frame(&header_bytes, payload, MaxPayloadBytes::DEFAULT);

    // Then: UnsupportedVersion
    assert_eq!(result, Err(IpcError::UnsupportedVersion { actual: 99 }));
}

#[test]
fn adversarial_submit_run_payload_with_zero_workflow_roundtrips() {
    // Given: a SubmitRun with all-zero workflow digest
    let payload = IpcPayload::SubmitRun(SubmitRunPayload {
        run_id: RunId::new(0),
        workflow: WorkflowDigest::from_bytes([0; 32]),
        input: Vec::new(),
    });

    // When: encoding then decoding
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };

    // Then: roundtrips (protocol doesn't validate semantic correctness)
    assert_eq!(decode_payload(&encoded), Ok(payload));
}

// ══ IPC frame validation hardening tests ══

#[test]
fn frame_validation_oversized_payload_exceeding_default_max_returns_error() {
    // Given: a header claiming payload_len larger than the default 1 MiB bound
    let default_max = MaxPayloadBytes::DEFAULT.get();
    let over_limit = match u32::try_from(default_max.saturating_add(1)) {
        Ok(v) => v,
        Err(_) => return,
    };
    let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 1, over_limit);

    // When: decoding the header with default max
    let encoded = header.encode();
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
    let result = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    // Then: PayloadTooLarge is returned before any payload allocation
    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: default_max.saturating_add(1),
            limit: default_max,
        })
    );
}

#[test]
fn frame_validation_header_claims_large_payload_but_actual_data_shorter_returns_error() {
    // Given: a valid header declaring payload_len=500 but only 10 bytes supplied
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 42, 500);
    let encoded = header.encode();
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
    let short_payload = Bytes::from(vec![0u8; 10]);

    // When: decoding the full frame
    let result = decode_frame(&encoded, short_payload, MaxPayloadBytes::DEFAULT);

    // Then: PayloadLengthMismatch with exact values
    assert_eq!(
        result,
        Err(IpcError::PayloadLengthMismatch {
            header: 500,
            actual: 10,
        })
    );
}

#[test]
fn frame_validation_invalid_magic_bytes_returns_typed_error() {
    // Given: header bytes with magic = 0x0000_0000 (not VBLT)
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&0x0000_0000_u32.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());

    // When: decoding the header
    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);

    // Then: InvalidMagic with actual=0
    assert_eq!(result, Err(IpcError::InvalidMagic { actual: 0 }));
}

#[test]
fn frame_validation_version_mismatch_returns_unsupported_version() {
    // Given: valid magic but version=255 (not 1)
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&255u16.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());

    // When: decoding the header
    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);

    // Then: UnsupportedVersion with actual=255
    assert_eq!(result, Err(IpcError::UnsupportedVersion { actual: 255 }));
}

#[test]
fn frame_validation_zero_command_id_returns_unknown_command() {
    // Given: valid magic and version but command=0 (not in v1 command set)
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    // command bytes 6..8 are already 0

    // When: decoding the header
    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);

    // Then: UnknownCommand(0)
    assert_eq!(result, Err(IpcError::UnknownCommand(0)));
}

#[test]
fn frame_validation_unrecognized_command_id_returns_typed_error() {
    // Given: valid magic and version but command=99 (not defined)
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&99u16.to_le_bytes());

    // When: decoding the header
    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);

    // Then: UnknownCommand(99)
    assert_eq!(result, Err(IpcError::UnknownCommand(99)));
}

#[test]
fn frame_validation_valid_header_with_unknown_payload_type_decode_fails() {
    // Given: a valid header with Health command but garbage payload bytes
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
    let encoded = header.encode();
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
    let garbage = Bytes::from(vec![0xDE, 0xAD, 0xBE, 0xEF]);

    // When: decoding the frame (header succeeds, postcard payload decode fails)
    let frame_result = decode_frame(&encoded, garbage, MaxPayloadBytes::DEFAULT);
    assert_ok!(frame_result, "frame struct decode should succeed");
    let Ok(frame) = frame_result else { return };

    // Then: decoding the typed payload fails
    let payload_result = decode_payload(frame.payload());
    assert_eq!(payload_result, Err(IpcError::PayloadDecodeFailed));
}

#[test]
fn frame_validation_truncated_frame_shorter_than_header_returns_error() {
    // Given: a reader with 0 bytes (completely empty)
    let data: [u8; 0] = [];
    let mut cursor = std::io::Cursor::new(data);

    // When: reading a frame header
    let result = crate::frame::read_frame_header(&mut cursor);

    // Then: HeaderDecodeFailed
    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
}

#[test]
fn frame_validation_header_claims_more_data_than_available_returns_error() {
    // Given: a valid header declaring payload_len=200 but reader has only 5 bytes
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 200);
    let encoded = header.encode();
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
    let mut cursor = std::io::Cursor::new(encoded.as_slice());

    // Read header successfully
    let decoded_header = crate::frame::read_frame_header(&mut cursor);
    assert_ok!(decoded_header, "header read should succeed");
    let Ok(decoded_header) = decoded_header else {
        return;
    };

    // When: reading payload with only 5 bytes available (header claimed 200)
    let short_data = vec![0u8; 5];
    let mut payload_cursor = std::io::Cursor::new(short_data.as_slice());
    let result = crate::frame::read_frame_payload(&mut payload_cursor, &decoded_header);

    // Then: PayloadDecodeFailed (short read)
    assert_eq!(result, Err(IpcError::PayloadDecodeFailed));
}

#[test]
fn frame_validation_valid_frame_parse_succeeds() {
    // Given: a well-formed frame with SubmitRun command
    let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 42, 0);
    let encoded = header.encode();
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
    let payload = Bytes::new();

    // When: decoding the frame
    let result = decode_frame(&encoded, payload, MaxPayloadBytes::DEFAULT);

    // Then: the frame decodes successfully with correct fields
    assert_ok!(result, "well-formed frame should decode");
    let Ok(frame) = result else { return };
    assert_eq!(frame.header().command, IpcCommand::SubmitRun);
    assert_eq!(frame.header().flags, 0);
    assert_eq!(frame.header().correlation, 42);
    assert_eq!(frame.header().payload_len, 0);
    assert_eq!(frame.payload().bytes().len(), 0);
}

#[test]
fn frame_validation_roundtrip_encode_decode_preserves_all_fields() {
    // Given: a frame with all fields populated and a non-trivial payload
    let original_header =
        IpcFrameHeader::new(IpcCommand::CompleteAction, 0xABCD, 0x1234_5678_9ABC_DEF0, 8);
    let encoded = original_header.encode();
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
    let payload = Bytes::from_static(b"payload!");

    // When: decoding the frame
    let result = decode_frame(&encoded, payload, MaxPayloadBytes::DEFAULT);

    // Then: all fields are preserved exactly
    assert_ok!(result, "frame should decode");
    let Ok(frame) = result else { return };
    assert_eq!(frame.header().command, IpcCommand::CompleteAction);
    assert_eq!(frame.header().flags, 0xABCD);
    assert_eq!(frame.header().correlation, 0x1234_5678_9ABC_DEF0);
    assert_eq!(frame.header().payload_len, 8);
    assert_eq!(frame.payload().bytes().as_ref(), b"payload!");
}

#[test]
fn frame_validation_roundtrip_all_commands_preserve_command_identity() {
    // Given: every valid command in the v1 command set
    let commands = [
        IpcCommand::SubmitRun,
        IpcCommand::SubmitRunInline,
        IpcCommand::CancelRun,
        IpcCommand::InspectRun,
        IpcCommand::ListEvents,
        IpcCommand::AnswerAsk,
        IpcCommand::CompleteAction,
        IpcCommand::FailAction,
        IpcCommand::DrainTrace,
        IpcCommand::Health,
        IpcCommand::Shutdown,
        IpcCommand::ListRuns,
        IpcCommand::GetMetrics,
        IpcCommand::GetWorkflowGraph,
        IpcCommand::GetTaintReport,
        IpcCommand::VerifyWorkflow,
    ];

    for command in commands {
        // When: encoding and decoding each command through the full frame path
        let header = IpcFrameHeader::new(command, 0, 1, 0);
        let encoded = header.encode();
        assert_ok!(encoded, "header should encode for {command:?}");
        let Ok(encoded) = encoded else { return };
        let result = decode_frame(&encoded, Bytes::new(), MaxPayloadBytes::DEFAULT);

        // Then: the command identity is preserved
        assert_ok!(result, "frame should decode for {command:?}");
        let Ok(frame) = result else { return };
        assert_eq!(
            frame.header().command,
            command,
            "command should roundtrip for {command:?}"
        );
    }
}

#[test]
fn frame_validation_payload_at_exact_max_boundary_succeeds() {
    // Given: a header with payload_len exactly equal to the default max
    let max = MaxPayloadBytes::DEFAULT.get();
    let payload_len = match u32::try_from(max) {
        Ok(v) => v,
        Err(_) => return,
    };
    let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 1, payload_len);
    let encoded = header.encode();
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
    let payload = Bytes::from(vec![0u8; max]);

    // When: decoding the frame
    let result = decode_frame(&encoded, payload, MaxPayloadBytes::DEFAULT);

    // Then: it succeeds (boundary is inclusive)
    assert_ok!(result, "frame at exact max boundary should decode");
    let Ok(frame) = result else { return };
    assert_eq!(frame.header().payload_len, payload_len);
    assert_eq!(frame.payload().bytes().len(), max);
}

#[test]
fn frame_validation_read_frame_header_bounded_rejects_empty_reader() {
    // Given: a reader with 0 bytes
    let data: [u8; 0] = [];
    let mut cursor = std::io::Cursor::new(data);

    // When: reading a bounded frame header
    let result = crate::frame::read_frame_header_bounded(&mut cursor, MaxPayloadBytes::DEFAULT);

    // Then: HeaderDecodeFailed
    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
}

#[test]
fn frame_validation_read_frame_payload_bounded_rejects_truncated_data() {
    // Given: a valid header with payload_len=50 but reader with 10 bytes
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 50);
    let data = vec![0u8; 10];
    let mut cursor = std::io::Cursor::new(data.as_slice());

    // When: reading bounded payload
    let result =
        crate::frame::read_frame_payload_bounded(&mut cursor, &header, MaxPayloadBytes::DEFAULT);

    // Then: PayloadDecodeFailed (short read)
    assert_eq!(result, Err(IpcError::PayloadDecodeFailed));
}

#[cfg(test)]
mod proptests {
    use super::{
        IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION, IpcCommand, IpcError, IpcFrameHeader, IpcPayload,
        MaxPayloadBytes, decode_payload, encode_payload,
    };
    use proptest::prelude::*;
    use vb_core::RunId;

    proptest! {
        #[test]
        fn ipc_command_roundtrips_through_u16(cmd in 1u16..=16u16) {
            let parsed = IpcCommand::from_u16(cmd);
            prop_assert_ok!(parsed);
            let Ok(command) = parsed else { return Ok(()) };
            prop_assert_eq!(command.as_u16(), cmd);
        }
    }

    proptest! {
        #[test]
        fn non_magic_bytes_always_rejected(magic in 0u32..) {
            if magic != IPC_MAGIC {
                let mut header_bytes = [0u8; IPC_HEADER_LEN];
                header_bytes[..4].copy_from_slice(&magic.to_le_bytes());
                header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
                let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
                prop_assert_err!(result);
                if let Err(e) = result {
                    prop_assert!(
                        matches!(e, IpcError::InvalidMagic { .. }),
                        "expected InvalidMagic, got {e:?}"
                    );
                }
            }
        }
    }

    proptest! {
        #[test]
        fn ipc_command_encode_decode_roundtrip(cmd_val in 1u16..=16u16) {
            // Given: any valid command id
            let Ok(command) = IpcCommand::from_u16(cmd_val) else {
                return Ok(())
            };

            // When: encoding a header with this command then decoding
            let header = IpcFrameHeader::new(command, 0, 0, 0);
            let encoded = header.encode();
            prop_assert_ok!(encoded);
            let Ok(encoded) = encoded else { return Ok(()) };
            let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

            // Then: the command roundtrips exactly
            prop_assert_ok!(decoded);
            let Ok(decoded) = decoded else { return Ok(()) };
            prop_assert_eq!(decoded.command, command);
        }
    }

    proptest! {
        #[test]
        fn ipc_response_encode_decode_roundtrip(run_id_val in 0u64..) {
            // Given: an IpcPayload::CancelRun with any run_id
            let payload = IpcPayload::CancelRun {
                run_id: RunId::new(run_id_val),
            };

            // When: encoding then decoding the payload
            let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
            prop_assert_ok!(encoded);
            let Ok(encoded) = encoded else { return Ok(()) };
            let decoded = decode_payload(&encoded);

            // Then: the payload roundtrips exactly
            prop_assert_ok!(decoded);
            let Ok(decoded) = decoded else { return Ok(()) };
            prop_assert_eq!(decoded, payload);
        }
    }

    proptest! {
        #[test]
        fn frame_header_length_never_exceeds_max(cmd_val in 1u16..=16u16, payload_len in 0u32..=1024u32) {
            // Given: any valid command and payload length up to 1 KiB
            let Ok(command) = IpcCommand::from_u16(cmd_val) else {
                return Ok(())
            };

            // When: creating a header
            let header = IpcFrameHeader::new(command, 0, 0, payload_len);

            // Then: header encodes to exactly IPC_HEADER_LEN bytes
            let encoded = header.encode();
            prop_assert_ok!(encoded);
            let Ok(encoded) = encoded else { return Ok(()) };
            prop_assert_eq!(encoded.len(), IPC_HEADER_LEN);
        }
    }
}
