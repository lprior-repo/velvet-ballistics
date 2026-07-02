#![forbid(unsafe_code)]
//! Storage error types with diagnostic codes.

use crate::types::EventSeq;
use std::path::Path;
use vb_core::{RunId, WorkflowDigest, WorkflowError};

mod artifact;
pub(crate) mod codes;
pub mod key_decode;
pub mod warnings;

pub use self::artifact::{ArtifactEnvelopeError, ArtifactInvalidSource};
pub use self::key_decode::KeyDecodeError;
pub use self::warnings::{
    AdmissionWarnings, VERIFICATION_WARNING_SCHEMA_MISMATCH_CODE, VerificationWarning,
};

/// Storage and journal operation errors.
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("fjall journal operation failed: {0}")]
    Fjall(#[from] fjall::Error),
    #[error("journal event encoding failed: {0}")]
    Encode(#[from] postcard::Error),
    #[error("artifact postcard encode failed: {0}")]
    PostcardEncodeFailed(#[source] postcard::Error),
    #[error("journal key capacity exceeded")]
    KeyCapacity,
    #[error("duplicate journal event for run {run:?} seq {seq:?}")]
    DuplicateEvent { run: RunId, seq: EventSeq },
    #[error("duplicate journal event staged in the same batch for run {run:?} seq {seq:?}")]
    DuplicateStagedKey { run: RunId, seq: EventSeq },
    #[error("journal write lock is poisoned")]
    WriteLockPoisoned,
    #[error("journal writer queue capacity must be non-zero")]
    QueueCapacity,
    #[error("journal writer queue is full")]
    QueueFull,
    #[error("journal batch byte budget exceeded: attempted {attempted} > limit {limit}")]
    JournalBatchBytesExceeded { attempted: u64, limit: u64 },
    #[error("journal write batch was aborted; commit is a no-op")]
    BatchAborted,
    #[error("journal writer queue is shut down")]
    QueueShutdown,
    #[error("journal replay returned run {actual:?}, expected {expected:?}")]
    WrongRun { expected: RunId, actual: RunId },
    #[error("journal replay sequence gap: expected {expected:?}, actual {actual:?}")]
    SequenceGap {
        expected: EventSeq,
        actual: EventSeq,
    },
    #[error(
        "journal replay key/payload mismatch for run {run:?}: key_seq={key_seq} payload_seq={payload_seq}"
    )]
    ReplayKeyMismatch {
        run: RunId,
        key_seq: u64,
        payload_seq: u64,
    },
    #[error(
        "journal replay envelope/payload sequence mismatch for run {run:?}: envelope_seq={envelope_seq} payload_seq={payload_seq}"
    )]
    ReplayEnvelopeSequenceMismatch {
        run: RunId,
        envelope_seq: u64,
        payload_seq: u64,
    },
    #[error("journal event sequence overflow")]
    SequenceOverflow,
    #[error("bad record magic: {found:#010x}")]
    BadMagic { found: u32 },
    #[error("unsupported record schema version: {version}")]
    UnsupportedSchemaVersion { version: u16 },
    #[error("record schema migration required from {from} to {to}")]
    MigrationRequired { from: u16, to: u16 },
    #[error("unknown record kind: {kind}")]
    UnknownRecordKind { kind: u16 },
    #[error("record kind {kind} does not belong to magic {magic:#010x}")]
    RecordKindFamilyMismatch { magic: u32, kind: u16 },
    #[error(
        "journal event payload kind {payload_kind} does not match envelope kind {envelope_kind}"
    )]
    RecordKindPayloadMismatch {
        envelope_kind: u16,
        payload_kind: u16,
    },
    #[error("record header length mismatch: {found}")]
    HeaderLengthMismatch { found: u32 },
    #[error("record payload too large: {len} > {max}")]
    PayloadTooLarge { len: u32, max: u32 },
    #[error("record header checksum mismatch")]
    HeaderChecksumMismatch,
    #[error("record payload digest mismatch")]
    PayloadDigestMismatch,
    #[error("unexpected end of record")]
    UnexpectedEof,
    #[error(
        "malformed keyspace row under prefix {prefix:#04x}: actual_len={actual_len} expected_len={expected_len}"
    )]
    MalformedKeyspaceRow {
        prefix: u8,
        expected_len: usize,
        actual_len: usize,
    },
    #[error("postcard payload decode failed: {0}")]
    PostcardDecodeFailed(#[source] postcard::Error),
    #[error("journal event is structurally encoded but semantically invalid")]
    InvalidEvent,
    #[error("artifact structure validation failed")]
    ArtifactMalformed,
    #[error("compiled workflow reconstruction failed: {0}")]
    WorkflowReconstruction(#[source] WorkflowError),
    #[error("compiled_ir readback failed: {0}")]
    CompiledIrReadback(#[source] Box<JournalError>),
    #[error("admission pre-allocation failed: {0}")]
    AdmissionAllocationFailed(#[source] std::collections::TryReserveError),
    #[error("artifact checksum mismatch")]
    ArtifactChecksumMismatch,
    #[error("invalid gate count: {found}")]
    InvalidGateCount { found: u8 },
    #[error("missing required proof flag: {flag}")]
    MissingRequiredProofFlag { flag: &'static str },
    #[error("artifact not found: {digest:?}")]
    ArtifactNotFound { digest: WorkflowDigest },
    #[error("accepted artifact admission is required")]
    AdmissionRequired,
    #[error("artifact invalid: {source:?}")]
    ArtifactInvalid { source: ArtifactInvalidSource },
    #[error("runtime input too large: {len} > {max}")]
    InputTooLarge { len: u32, max: u32 },
    #[error("runtime input schema mismatch")]
    InputSchemaMismatch,
    #[error("runtime capability denied")]
    CapabilityDenied,
    #[error("runtime secret unavailable")]
    SecretUnavailable,
    #[error("run already exists")]
    RunAlreadyExists,
    #[error("invalid run identifier: {run:?}")]
    InvalidRunId { run: RunId },
    #[error("active run capacity exceeded")]
    ActiveRunCapacityExceeded,
    #[error("frame allocation failed")]
    FrameAllocationFailed,
    #[error("admission journal failed")]
    AdmissionJournalFailed,
    #[error(
        "IndexStatusState byte collision: 0x{byte:02x} below minimum 0x{min:02x} for named status range"
    )]
    IndexStatusStateCollision { byte: u8, min: u8 },
    #[error("strict durability failed")]
    StrictDurabilityFailed,
    #[error(
        "journal replay for run {run:?} exceeded event limit: observed {observed} > limit {limit}"
    )]
    TooManyEvents {
        run: RunId,
        limit: usize,
        observed: usize,
    },
    #[error("journal replay allocation failed for run {run:?}: requested {requested} events")]
    ReplayAllocationFailed { run: RunId, requested: usize },
    #[error("admission clock unavailable")]
    ClockUnavailable,
    #[error("invalid configuration for field {field}: {reason}")]
    InvalidConfig {
        field: &'static str,
        reason: &'static str,
    },
    #[error(
        "read-only journal open is not supported; Fjall does not expose a true read-only database open"
    )]
    UnsupportedReadOnly,
    #[error("process lock held by another process (pid: {holder_pid:?}) at {path}")]
    ProcessLockHeld {
        path: Box<Path>,
        source: rustix::io::Errno,
        holder_pid: Option<u32>,
    },
    #[error("process lock I/O error at {path}: {source}")]
    ProcessLockIo {
        path: Box<Path>,
        source: std::io::Error,
    },
    #[error("trim operation failed: {0}")]
    Trim(Box<crate::TrimError>),
}

#[path = "conversions.rs"]
mod conversions;
