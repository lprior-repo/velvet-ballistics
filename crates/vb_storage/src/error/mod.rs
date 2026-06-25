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
    /// Fjall operation failed.
    #[error("fjall journal operation failed: {0}")]
    Fjall(#[from] fjall::Error),
    /// Binary encoding failed.
    #[error("journal event encoding failed: {0}")]
    Encode(#[source] postcard::Error),
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
    /// Two `append_event` calls in the same batch used the same
    /// `(run, seq)` key. The batch remains open so the caller can
    /// skip the duplicate and commit the prior staged events; the
    /// durable journal never sees the in-flight overwrite.
    #[error("duplicate staged journal event for run {run:?} seq {seq:?}")]
    DuplicateStagedKey {
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
    /// Cross-keyspace write batch was aborted by a fallible staging step.
    ///
    /// Returned by [`crate::batch::JournalWriteBatch::commit`] when a prior
    /// `put_*` / `append_event` operation set the batch's `aborted` flag.
    /// Per master §49 Crash-Consistency Rule, an aborted batch must NEVER
    /// silently return `Ok(())` — that path would let callers conclude a
    /// partial durability barrier succeeded. The commit always fails closed
    /// with this typed variant, so operators can detect the abort and retry
    /// with corrected inputs. No partial state was made durable.
    #[error("journal write batch was aborted; commit is a no-op")]
    BatchAborted,
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
    /// Journal event payload variant does not match the envelope record kind.
    #[error(
        "journal event payload kind {payload_kind} does not match envelope kind {envelope_kind}"
    )]
    RecordKindPayloadMismatch {
        /// Wire kind stored in the record envelope.
        envelope_kind: u16,
        /// Wire kind implied by the decoded payload variant.
        payload_kind: u16,
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
    /// Keyspace scan encountered a row whose key does not match the
    /// expected typed shape for its prefix. Returned only when the
    /// caller selected [`crate::keys::KeyspaceScanPolicy::FailClosed`];
    /// [`crate::keys::KeyspaceScanPolicy::SkipMalformed`] never
    /// surfaces this variant. The scan aborts on the first malformed
    /// row so the operator can act on the earliest evidence of
    /// corruption.
    #[error(
        "malformed keyspace row under prefix {prefix:#04x}: actual_len={actual_len} expected_len={expected_len}"
    )]
    MalformedKeyspaceRow {
        /// First byte of the offending key (the prefix).
        prefix: u8,
        /// Expected key length for this prefix in bytes.
        expected_len: usize,
        /// Actual key length observed in storage.
        actual_len: usize,
    },
    /// Postcard payload decode failed.
    #[error("postcard payload decode failed: {0}")]
    PostcardDecodeFailed(#[source] postcard::Error),
    /// JournalEvent decoded from bytes is semantically invalid (run_id=0, seq overflow, or attempt=0).
    #[error("journal event is structurally encoded but semantically invalid")]
    InvalidEvent,
    /// Artifact structure validation failed.
    #[error("artifact structure validation failed")]
    ArtifactMalformed,
    /// `CompiledWorkflow::try_from_parts` rejected an artifact whose parts
    /// were reconstructed from the caller-supplied workflow. The wrapped
    /// `WorkflowError` preserves the specific structural defect.
    /// (vb-l9jqs: replaces the prior `.map_err(|_| ArtifactMalformed)`
    /// swallowing pattern at L370 and L531 of `admission.rs`.)
    #[error("compiled workflow reconstruction failed: {0}")]
    WorkflowReconstruction(#[source] WorkflowError),
    /// `compiled_ir` readback returned a typed storage error rather than
    /// the expected artifact record. The inner `JournalError` carries the
    /// underlying failure rather than collapsing every readback error to
    /// `ArtifactMalformed`.
    /// (vb-l9jqs: replaces the `.map_err(|_| ArtifactMalformed)` at
    /// L435 of `admission.rs`.)
    #[error("compiled_ir readback failed: {0}")]
    CompiledIrReadback(#[source] Box<JournalError>),
    /// Vec::try_reserve failed during admission pre-allocation of the
    /// required-capability slice.
    /// (vb-l9jqs: replaces the `.map_err(|_| ArtifactMalformed)` at
    /// L454 of `admission.rs`.)
    #[error("admission pre-allocation failed: {0}")]
    AdmissionAllocationFailed(#[source] std::collections::TryReserveError),
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
    /// Run identifier is invalid (must be non-zero).
    #[error("invalid run identifier: {run:?}")]
    InvalidRunId {
        /// The invalid run identifier.
        run: RunId,
    },
    /// Runtime active run capacity is exhausted.
    #[error("active run capacity exceeded")]
    ActiveRunCapacityExceeded,
    /// Runtime frame allocation failed.
    #[error("frame allocation failed")]
    FrameAllocationFailed,
    /// Runtime admission journal append failed.
    #[error("admission journal failed")]
    AdmissionJournalFailed,
    /// `IndexStatusState::Other(v)` whose byte collides with a named
    /// status variant (must be `>= MIN_OTHER_STATUS_BYTE`).
    /// (SC-001 / vb-hexk6.)
    #[error("IndexStatusState byte collision: 0x{byte:02x} below minimum 0x{min:02x} for named status range")]
    IndexStatusStateCollision {
        /// The conflicting byte that was rejected.
        byte: u8,
        /// Minimum byte reserved for the `Other` range.
        min: u8,
    },
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

#[path = "conversions.rs"]
mod conversions;
