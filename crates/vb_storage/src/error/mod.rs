#![forbid(unsafe_code)]
//! Storage error types with diagnostic codes.

use crate::types::EventSeq;
use std::path::Path;
use vb_core::{RunId, WorkflowDigest};

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
#[non_exhaustive]
pub enum JournalError {
    /// Fjall operation failed.
    #[error("fjall journal operation failed: {0}")]
    Fjall(#[from] fjall::Error),
    /// Binary encoding failed.
    #[error("journal event encoding failed: {0}")]
    Encode(#[from] postcard::Error),
    /// Fixed-size key construction failed.
    #[error("journal key capacity exceeded")]
    KeyCapacity,
    /// Append attempted to overwrite an immutable event.
    #[error("duplicate journal event for run {run:?} seq {seq:?}")]
    DuplicateEvent {
        /// Run identifier.
        run: RunId,
        /// Existing sequence.
        seq: EventSeq,
    },
    /// Serialized append lock was poisoned by a panicking holder.
    #[error("journal write lock is poisoned")]
    WriteLockPoisoned,
    /// Queue capacity or batch size was zero.
    #[error("journal writer queue capacity must be non-zero")]
    QueueCapacity,
    /// Queue has no room for another event.
    #[error("journal writer queue is full")]
    QueueFull,
    /// Journal batch accumulated encoded-byte budget exceeded.
    #[error("journal batch byte budget exceeded: attempted {attempted} > limit {limit}")]
    JournalBatchBytesExceeded {
        /// Computed total staged bytes that would have exceeded the budget.
        attempted: u64,
        /// Configured byte budget for this batch.
        limit: u64,
    },
    /// Queue has started deterministic shutdown and rejects new writes.
    #[error("journal writer queue is shut down")]
    QueueShutdown,
    /// Replay returned an event for a different run than requested.
    #[error("journal replay returned run {actual:?}, expected {expected:?}")]
    WrongRun {
        /// Expected run id.
        expected: RunId,
        /// Actual run id.
        actual: RunId,
    },
    /// Replay found a non-contiguous event sequence.
    #[error("journal replay sequence gap: expected {expected:?}, actual {actual:?}")]
    SequenceGap {
        /// Expected sequence.
        expected: EventSeq,
        /// Actual sequence.
        actual: EventSeq,
    },
    /// Sequence number overflowed.
    #[error("journal event sequence overflow")]
    SequenceOverflow,
    /// Record magic did not match the expected family.
    #[error("bad record magic: {found:#010x}")]
    BadMagic {
        /// Found magic value.
        found: u32,
    },
    /// Record schema version is not supported.
    #[error("unsupported record schema version: {version}")]
    UnsupportedSchemaVersion {
        /// Found schema version.
        version: u16,
    },
    /// Record schema requires explicit migration.
    #[error("record schema migration required from {from} to {to}")]
    MigrationRequired {
        /// Found schema version.
        from: u16,
        /// Current schema version.
        to: u16,
    },
    /// Record kind is not known.
    #[error("unknown record kind: {kind}")]
    UnknownRecordKind {
        /// Found kind.
        kind: u16,
    },
    /// Record kind is not valid for this magic family.
    #[error("record kind {kind} does not belong to magic {magic:#010x}")]
    RecordKindFamilyMismatch {
        /// Magic value.
        magic: u32,
        /// Record kind.
        kind: u16,
    },
    /// Header length was not the contract value.
    #[error("record header length mismatch: {found}")]
    HeaderLengthMismatch {
        /// Found header length.
        found: u32,
    },
    /// Payload length exceeded the configured maximum.
    #[error("record payload too large: {len} > {max}")]
    PayloadTooLarge {
        /// Payload length.
        len: u32,
        /// Maximum allowed length.
        max: u32,
    },
    /// Payload or entry-count length could not be represented as `u32`.
    ///
    /// Distinct from [`PayloadTooLarge`](Self::PayloadTooLarge): that variant
    /// reports a payload that exceeded a *configured* `u32` maximum, while
    /// this variant reports an observed length that cannot even fit in `u32`.
    /// Carries the real `u64` observation rather than fabricating a `u32`.
    #[error("length {len} exceeds u32::MAX and cannot be represented")]
    PayloadLenOverflow {
        /// Observed length that did not fit in `u32`.
        len: u64,
    },
    /// Header CRC32C did not match.
    #[error("record header checksum mismatch")]
    HeaderChecksumMismatch,
    /// Payload BLAKE3 digest did not match.
    #[error("record payload digest mismatch")]
    PayloadDigestMismatch,
    /// Record ended before the declared header or payload length.
    #[error("unexpected end of record")]
    UnexpectedEof,
    /// Record contained bytes after the declared payload ended.
    #[error("record has trailing bytes: declared end {declared_end}, actual length {actual_len}")]
    UnexpectedTrailingBytes {
        /// Exclusive byte offset where the declared payload ended.
        declared_end: usize,
        /// Actual input length including trailing bytes.
        actual_len: usize,
    },
    /// Postcard payload decode failed.
    #[error("postcard payload decode failed")]
    PostcardDecodeFailed,
    /// JournalEvent decoded from bytes is semantically invalid (run_id=0, seq overflow, or attempt=0).
    #[error("journal event is structurally encoded but semantically invalid")]
    InvalidEvent,
    /// Artifact structure validation failed.
    #[error("artifact structure validation failed")]
    ArtifactMalformed,
    /// Artifact digest checksum mismatch.
    #[error("artifact checksum mismatch")]
    ArtifactChecksumMismatch,
    /// Attempted to write a compiled IR record with the same digest but
    /// different metadata than what was previously stored. This indicates a
    /// potential metadata mutation attack.
    #[error("artifact metadata mutation detected for digest {digest:?}")]
    MetadataMutation {
        /// The digest that was targeted.
        digest: WorkflowDigest,
    },
    /// Invalid verification gate count.
    #[error("invalid gate count: {found}")]
    InvalidGateCount {
        /// Found gate count.
        found: u8,
    },
    /// A required proof flag is false.
    #[error("missing required proof flag: {flag}")]
    MissingRequiredProofFlag {
        /// The flag that is missing.
        flag: &'static str,
    },
    /// Requested artifact digest was not found in storage.
    #[error("artifact not found: {digest:?}")]
    ArtifactNotFound {
        /// Digest of the missing artifact.
        digest: WorkflowDigest,
    },
    /// Raw admission was attempted while accepted artifacts are required.
    #[error("accepted artifact admission is required")]
    AdmissionRequired,
    /// Stored artifact failed accepted-artifact validation.
    #[error("artifact invalid: {source:?}")]
    ArtifactInvalid {
        /// Validation failure source.
        source: ArtifactInvalidSource,
    },
    /// Runtime input exceeded the bounded admission payload limit.
    #[error("runtime input too large: {len} > {max}")]
    InputTooLarge {
        /// Observed input length.
        len: u32,
        /// Maximum accepted input length.
        max: u32,
    },
    /// Runtime input does not match the accepted artifact schema.
    #[error("runtime input schema mismatch")]
    InputSchemaMismatch,
    /// Runtime capability grant does not cover artifact requirements.
    #[error("runtime capability denied")]
    CapabilityDenied,
    /// Required secret identifier is unavailable.
    #[error("runtime secret unavailable")]
    SecretUnavailable,
    /// Run identifier is already active or durably accepted.
    #[error("run already exists")]
    RunAlreadyExists,
    /// Run identifier is invalid (must be non-zero).
    #[error("invalid run identifier: {run:?}")]
    InvalidRunId {
        /// The invalid run identifier.
        run: RunId,
    },
    /// `IndexStatusState::Other(v)` byte collides with a named variant
    /// discriminant in the storage key.
    #[error("index status state byte {byte} collides with named variant")]
    IndexStatusStateCollision {
        /// Byte value that would have collided with a named variant.
        byte: u8,
    },
    /// `EventSeq` value `u64::MAX` is reserved by the key decoder and
    /// therefore must not be encoded into a storage key.
    #[error("event sequence u64::MAX is reserved and cannot be encoded")]
    ReservedSeqSentinel,
    /// Runtime active run capacity is exhausted.
    #[error("active run capacity exceeded")]
    ActiveRunCapacityExceeded,
    /// Runtime frame allocation failed.
    #[error("frame allocation failed")]
    FrameAllocationFailed,
    /// Runtime admission journal append failed.
    #[error("admission journal failed")]
    AdmissionJournalFailed,
    /// Strict durability failed.
    #[error("strict durability failed")]
    StrictDurabilityFailed,
    /// Replay exceeded the caller-provided event limit.
    #[error(
        "journal replay for run {run:?} exceeded event limit: observed {observed} > limit {limit}"
    )]
    TooManyEvents {
        /// Run being replayed.
        run: RunId,
        /// Maximum event count allowed.
        limit: usize,
        /// Observed event count that crossed the limit.
        observed: usize,
    },
    /// Replay collection could not reserve bounded memory.
    #[error("journal replay allocation failed for run {run:?}: requested {requested} events")]
    ReplayAllocationFailed {
        /// Run being replayed.
        run: RunId,
        /// Event capacity requested.
        requested: usize,
    },
    /// Admission clock could not provide a timestamp.
    #[error("admission clock unavailable")]
    ClockUnavailable,
    /// Another process holds the exclusive storage lock.
    #[error("process lock held by another process (pid: {holder_pid:?}) at {path}")]
    ProcessLockHeld {
        /// Lock file path.
        path: Box<Path>,
        /// Underlying flock error.
        source: rustix::io::Errno,
        /// PID of the holding process, if discoverable.
        holder_pid: Option<u32>,
    },
    /// I/O error while creating or opening the process lock file.
    #[error("process lock I/O error at {path}: {source}")]
    ProcessLockIo {
        /// Lock file path.
        path: Box<Path>,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// Journal trim operation error.
    #[error("trim operation failed: {0}")]
    Trim(Box<crate::TrimError>),
}

impl From<crate::TrimError> for JournalError {
    fn from(err: crate::TrimError) -> Self {
        match err {
            crate::TrimError::Fjall(e) => Self::Fjall(e),
            crate::TrimError::Journal(e) => e,
            _ => Self::Trim(Box::new(err)),
        }
    }
}

impl From<std::io::Error> for JournalError {
    fn from(_: std::io::Error) -> Self {
        JournalError::UnexpectedEof
    }
}

/// Manual `PartialEq` that compares only the variant discriminant and
/// structurally-comparable fields. Variants whose inner payload is a foreign
/// error type that does not implement `PartialEq` (`fjall::Error`,
/// `std::io::Error`, `TrimError`, `ArtifactInvalidSource`) are treated as
/// equal when their discriminant matches.
impl PartialEq for JournalError {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
            && match (self, other) {
                (Self::DuplicateEvent { run: a, seq: b }, Self::DuplicateEvent { run: c, seq: d }) => {
                    a == c && b == d
                }
                (
                    Self::WrongRun {
                        expected: a,
                        actual: b,
                    },
                    Self::WrongRun {
                        expected: c,
                        actual: d,
                    },
                ) => a == c && b == d,
                (
                    Self::SequenceGap {
                        expected: a,
                        actual: b,
                    },
                    Self::SequenceGap {
                        expected: c,
                        actual: d,
                    },
                ) => a == c && b == d,
                (Self::BadMagic { found: a }, Self::BadMagic { found: b }) => a == b,
                (
                    Self::UnsupportedSchemaVersion { version: a },
                    Self::UnsupportedSchemaVersion { version: b },
                ) => a == b,
                (
                    Self::MigrationRequired { from: a, to: b },
                    Self::MigrationRequired { from: c, to: d },
                ) => a == c && b == d,
                (Self::UnknownRecordKind { kind: a }, Self::UnknownRecordKind { kind: b }) => {
                    a == b
                }
                (
                    Self::RecordKindFamilyMismatch { magic: a, kind: b },
                    Self::RecordKindFamilyMismatch { magic: c, kind: d },
                ) => a == c && b == d,
                (Self::HeaderLengthMismatch { found: a }, Self::HeaderLengthMismatch { found: b }) => {
                    a == b
                }
                (
                    Self::PayloadTooLarge { len: a, max: b },
                    Self::PayloadTooLarge { len: c, max: d },
                ) => a == c && b == d,
                (Self::PayloadLenOverflow { len: a }, Self::PayloadLenOverflow { len: b }) => {
                    a == b
                }
                (
                    Self::UnexpectedTrailingBytes {
                        declared_end: a,
                        actual_len: b,
                    },
                    Self::UnexpectedTrailingBytes {
                        declared_end: c,
                        actual_len: d,
                    },
                ) => a == c && b == d,
                (Self::InvalidGateCount { found: a }, Self::InvalidGateCount { found: b }) => {
                    a == b
                }
                (
                    Self::MissingRequiredProofFlag { flag: a },
                    Self::MissingRequiredProofFlag { flag: b },
                ) => a == b,
                (Self::ArtifactNotFound { digest: a }, Self::ArtifactNotFound { digest: b }) => {
                    a == b
                }
                (Self::MetadataMutation { digest: a }, Self::MetadataMutation { digest: b }) => {
                    a == b
                }
                (Self::InvalidRunId { run: a }, Self::InvalidRunId { run: b }) => a == b,
                (
                    Self::IndexStatusStateCollision { byte: a },
                    Self::IndexStatusStateCollision { byte: b },
                ) => a == b,
                (
                    Self::TooManyEvents {
                        run: a,
                        limit: b,
                        observed: c,
                    },
                    Self::TooManyEvents {
                        run: d,
                        limit: e,
                        observed: f,
                    },
                ) => a == d && b == e && c == f,
                (
                    Self::ReplayAllocationFailed {
                        run: a,
                        requested: b,
                    },
                    Self::ReplayAllocationFailed {
                        run: c,
                        requested: d,
                    },
                ) => a == c && b == d,
                (
                    Self::JournalBatchBytesExceeded {
                        attempted: a,
                        limit: b,
                    },
                    Self::JournalBatchBytesExceeded {
                        attempted: c,
                        limit: d,
                    },
                ) => a == c && b == d,
                (Self::InputTooLarge { len: a, max: b }, Self::InputTooLarge { len: c, max: d }) => {
                    a == c && b == d
                }
                _ => true,
            }
    }
}

impl Eq for JournalError {}
