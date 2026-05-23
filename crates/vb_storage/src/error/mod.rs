#![forbid(unsafe_code)]
//! Storage error types with diagnostic codes.

use crate::types::EventSeq;
use std::path::Path;
use vb_core::{RunId, WorkflowDigest};

mod artifact;
pub(crate) mod codes;
pub mod warnings;

pub use self::artifact::{ArtifactEnvelopeError, ArtifactInvalidSource};
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
    /// Header CRC32C did not match.
    #[error("record header checksum mismatch")]
    HeaderChecksumMismatch,
    /// Payload BLAKE3 digest did not match.
    #[error("record payload digest mismatch")]
    PayloadDigestMismatch,
    /// Record ended before the declared header or payload length.
    #[error("unexpected end of record")]
    UnexpectedEof,
    /// Postcard payload decode failed.
    #[error("postcard payload decode failed")]
    PostcardDecodeFailed,
    /// Artifact structure validation failed.
    #[error("artifact structure validation failed")]
    ArtifactMalformed,
    /// Artifact digest checksum mismatch.
    #[error("artifact checksum mismatch")]
    ArtifactChecksumMismatch,
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
