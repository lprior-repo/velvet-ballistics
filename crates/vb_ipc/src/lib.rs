//! Bounded memory ingress and binary IPC for Velvet Ballastics.
//!
//! This crate deliberately exposes memory/IPC-shaped primitives only. HTTP is
//! not part of the hot control plane.

pub mod client;
pub mod frame;
pub mod server;

use bytes::Bytes;
use crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError, bounded};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read};
use std::num::NonZeroUsize;
use thiserror::Error;
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
        let mut bytes = Vec::with_capacity(IPC_HEADER_LEN);
        bytes.extend_from_slice(&IPC_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&IPC_VERSION.to_le_bytes());
        bytes.extend_from_slice(&self.command.as_u16().to_le_bytes());
        bytes.extend_from_slice(&self.flags.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&self.correlation.to_le_bytes());
        bytes.extend_from_slice(&self.payload_len.to_le_bytes());

        match <[u8; IPC_HEADER_LEN]>::try_from(bytes.as_slice()) {
            Ok(encoded) => Ok(encoded),
            Err(_) => Err(IpcError::HeaderEncodeFailed),
        }
    }

    /// Decodes and validates a fixed IPC header before payload allocation.
    pub fn decode(
        bytes: &[u8; IPC_HEADER_LEN],
        max_payload: MaxPayloadBytes,
    ) -> Result<Self, IpcError> {
        let mut cursor = Cursor::new(bytes.as_slice());
        let magic = read_u32_le(&mut cursor)?;
        if magic != IPC_MAGIC {
            return Err(IpcError::InvalidMagic { actual: magic });
        }

        let version = read_u16_le(&mut cursor)?;
        if version != IPC_VERSION {
            return Err(IpcError::UnsupportedVersion { actual: version });
        }

        let command = IpcCommand::from_u16(read_u16_le(&mut cursor)?)?;
        let flags = read_u16_le(&mut cursor)?;
        let reserved = read_u16_le(&mut cursor)?;
        if reserved != 0 {
            return Err(IpcError::ReservedNonZero { actual: reserved });
        }
        let correlation = read_u64_le(&mut cursor)?;
        let payload_len = read_u32_le(&mut cursor)?;
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
    /// Health probe.
    Health,
    /// Graceful shutdown request.
    Shutdown,
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
}

fn read_u16_le(cursor: &mut Cursor<&[u8]>) -> Result<u16, IpcError> {
    let mut bytes = [0_u8; 2];
    cursor
        .read_exact(&mut bytes)
        .map_err(|_| IpcError::HeaderDecodeFailed)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32_le(cursor: &mut Cursor<&[u8]>) -> Result<u32, IpcError> {
    let mut bytes = [0_u8; 4];
    cursor
        .read_exact(&mut bytes)
        .map_err(|_| IpcError::HeaderDecodeFailed)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64_le(cursor: &mut Cursor<&[u8]>) -> Result<u64, IpcError> {
    let mut bytes = [0_u8; 8];
    cursor
        .read_exact(&mut bytes)
        .map_err(|_| IpcError::HeaderDecodeFailed)?;
    Ok(u64::from_le_bytes(bytes))
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
mod tests {
    use super::{
        IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION, IngressFrame, IpcCommand, IpcError, IpcFrameHeader,
        IpcPayload, MaxPayloadBytes, MemoryIngress, QueueCapacity, SubmitRunPayload, decode_frame,
        decode_payload, encode_payload,
    };
    use bytes::Bytes;
    use vb_core::{RunId, WorkflowDigest};

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
        assert!(frame.is_ok(), "test frame should fit default payload bound");
        let Ok(frame) = frame else {
            return;
        };

        assert_eq!(queue.try_submit(frame.clone()), Ok(()));
        assert_eq!(queue.try_submit(frame), Err(IpcError::Full));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn oversized_payload_is_rejected() {
        let result = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([2; 32]),
            Bytes::from_static(b"too big"),
            MaxPayloadBytes::new(std::num::NonZeroUsize::MIN),
        );

        assert!(matches!(result, Err(IpcError::PayloadTooLarge { .. })));
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
        assert_eq!(IpcCommand::from_u16(12), Err(IpcError::UnknownCommand(12)));
    }

    #[test]
    fn header_roundtrips_little_endian_fields() {
        let header = IpcFrameHeader::new(IpcCommand::CompleteAction, 7, 42, 3);
        let encoded = header.encode();
        assert!(encoded.is_ok(), "header should encode to fixed width");
        let Ok(encoded) = encoded else {
            return;
        };

        let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);
        assert_eq!(decoded, Ok(header));
    }

    #[test]
    fn decoder_rejects_bad_magic_before_payload_use() {
        let encoded = header_bytes(0, IPC_VERSION, IpcCommand::Health.as_u16(), 0, 0, 1, 0);
        assert!(encoded.is_ok(), "test header should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

        assert_eq!(decoded, Err(IpcError::InvalidMagic { actual: 0 }));
    }

    #[test]
    fn decoder_rejects_payload_above_bound() {
        let encoded = header_bytes(
            IPC_MAGIC,
            IPC_VERSION,
            IpcCommand::SubmitRun.as_u16(),
            0,
            0,
            1,
            8,
        );
        assert!(encoded.is_ok(), "test header should encode");
        let Ok(encoded) = encoded else {
            return;
        };
        let decoded =
            IpcFrameHeader::decode(&encoded, MaxPayloadBytes::new(std::num::NonZeroUsize::MIN));

        assert!(matches!(decoded, Err(IpcError::PayloadTooLarge { .. })));
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
        assert!(encoded.is_ok(), "test header should encode");
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
        assert!(encoded.is_ok(), "header should encode to fixed width");
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
        assert!(encoded.is_ok(), "payload should encode under default bound");
        let Ok(encoded) = encoded else {
            return;
        };

        assert_eq!(decode_payload(&encoded), Ok(payload));
    }
}
