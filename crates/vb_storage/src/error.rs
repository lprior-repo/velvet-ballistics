//! Storage error types with diagnostic codes.

use std::path::Path;

use crate::types::EventSeq;
use vb_core::{DiagnosticCode, RunId, WorkflowDigest};

/// Non-critical issues detected during storage admission.
///
/// These do not prevent admission but should be reported to the caller
/// for logging, monitoring, or informational purposes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum VerificationWarning {
    /// Record schema version is older than current but still compatible.
    #[error(
        "record schema version {found} is older than current {current} — migration may be required"
    )]
    SchemaVersionMismatch {
        /// Found schema version.
        found: u16,
        /// Current schema version.
        current: u16,
    },
}

/// Diagnostic code for schema version mismatch warning.
pub const VERIFICATION_WARNING_SCHEMA_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x5001);

impl VerificationWarning {
    /// Returns the stable diagnostic code for this warning.
    #[must_use]
    pub const fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            Self::SchemaVersionMismatch { .. } => VERIFICATION_WARNING_SCHEMA_MISMATCH_CODE,
        }
    }
}

/// Container for multiple verification warnings.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct AdmissionWarnings {
    /// List of warnings collected during admission.
    warnings: Vec<VerificationWarning>,
}

impl AdmissionWarnings {
    /// Creates a new empty warnings container.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if there are no warnings.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.warnings.is_empty()
    }

    /// Returns the number of warnings.
    #[must_use]
    pub fn len(&self) -> usize {
        self.warnings.len()
    }

    /// Adds a warning to the container.
    pub fn push(&mut self, warning: VerificationWarning) {
        self.warnings.push(warning);
    }

    /// Returns an iterator over the warnings.
    pub fn iter(&self) -> std::slice::Iter<'_, VerificationWarning> {
        self.warnings.iter()
    }
}

impl IntoIterator for AdmissionWarnings {
    type Item = VerificationWarning;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.warnings.into_iter()
    }
}

impl<'a> IntoIterator for &'a AdmissionWarnings {
    type Item = &'a VerificationWarning;
    type IntoIter = std::slice::Iter<'a, VerificationWarning>;

    fn into_iter(self) -> Self::IntoIter {
        self.warnings.iter()
    }
}

/// Storage and journal operation errors.
#[derive(Debug, thiserror::Error)]
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
    /// Requested artifact digest was not found in storage.
    #[error("artifact not found: {digest:?}")]
    ArtifactNotFound {
        /// Digest of the missing artifact.
        digest: WorkflowDigest,
    },
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
}

impl JournalError {
    /// Diagnostic code for fjall operation failure.
    pub const FJALL_CODE: DiagnosticCode = DiagnosticCode::new(0x4001);
    /// Diagnostic code for binary encoding failure.
    pub const ENCODE_CODE: DiagnosticCode = DiagnosticCode::new(0x4002);
    /// Diagnostic code for key capacity exceeded.
    pub const KEY_CAPACITY_CODE: DiagnosticCode = DiagnosticCode::new(0x4003);
    /// Diagnostic code for duplicate event.
    pub const DUPLICATE_EVENT_CODE: DiagnosticCode = DiagnosticCode::new(0x4004);
    /// Diagnostic code for write lock poisoned.
    pub const WRITE_LOCK_POISONED_CODE: DiagnosticCode = DiagnosticCode::new(0x4005);
    /// Diagnostic code for queue capacity zero.
    pub const QUEUE_CAPACITY_CODE: DiagnosticCode = DiagnosticCode::new(0x4006);
    /// Diagnostic code for queue full.
    pub const QUEUE_FULL_CODE: DiagnosticCode = DiagnosticCode::new(0x4007);
    /// Diagnostic code for queue shutdown.
    pub const QUEUE_SHUTDOWN_CODE: DiagnosticCode = DiagnosticCode::new(0x4016);
    /// Diagnostic code for wrong run.
    pub const WRONG_RUN_CODE: DiagnosticCode = DiagnosticCode::new(0x4008);
    /// Diagnostic code for sequence gap.
    pub const SEQUENCE_GAP_CODE: DiagnosticCode = DiagnosticCode::new(0x4009);
    /// Diagnostic code for sequence overflow.
    pub const SEQUENCE_OVERFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x400A);
    /// Diagnostic code for bad magic.
    pub const BAD_MAGIC_CODE: DiagnosticCode = DiagnosticCode::new(0x400B);
    /// Diagnostic code for unsupported schema version.
    pub const UNSUPPORTED_SCHEMA_VERSION_CODE: DiagnosticCode = DiagnosticCode::new(0x400C);
    /// Diagnostic code for migration required.
    pub const MIGRATION_REQUIRED_CODE: DiagnosticCode = DiagnosticCode::new(0x400D);
    /// Diagnostic code for unknown record kind.
    pub const UNKNOWN_RECORD_KIND_CODE: DiagnosticCode = DiagnosticCode::new(0x400E);
    /// Diagnostic code for record kind family mismatch.
    pub const RECORD_KIND_FAMILY_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x400F);
    /// Diagnostic code for header length mismatch.
    pub const HEADER_LENGTH_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x4010);
    /// Diagnostic code for payload too large.
    pub const PAYLOAD_TOO_LARGE_CODE: DiagnosticCode = DiagnosticCode::new(0x4011);
    /// Diagnostic code for header checksum mismatch.
    pub const HEADER_CHECKSUM_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x4012);
    /// Diagnostic code for payload digest mismatch.
    pub const PAYLOAD_DIGEST_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x4013);
    /// Diagnostic code for unexpected eof.
    pub const UNEXPECTED_EOF_CODE: DiagnosticCode = DiagnosticCode::new(0x4014);
    /// Diagnostic code for postcard decode failed.
    pub const POSTCARD_DECODE_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x4015);
    /// Diagnostic code for artifact malformed.
    pub const ARTIFACT_MALFORMED_CODE: DiagnosticCode = DiagnosticCode::new(0x4017);
    /// Diagnostic code for artifact checksum mismatch.
    pub const ARTIFACT_CHECKSUM_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x4018);
    /// Diagnostic code for artifact not found.
    pub const ARTIFACT_NOT_FOUND_CODE: DiagnosticCode = DiagnosticCode::new(0x4019);
    /// Diagnostic code for process lock held by another process.
    pub const PROCESS_LOCK_HELD_CODE: DiagnosticCode = DiagnosticCode::new(0x401A);
    /// Diagnostic code for process lock I/O error.
    pub const PROCESS_LOCK_IO_CODE: DiagnosticCode = DiagnosticCode::new(0x401B);

    /// Returns the stable diagnostic code for this error.
    #[must_use]
    pub const fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            Self::Fjall(_) => Self::FJALL_CODE,
            Self::Encode(_) => Self::ENCODE_CODE,
            Self::KeyCapacity => Self::KEY_CAPACITY_CODE,
            Self::DuplicateEvent { .. } => Self::DUPLICATE_EVENT_CODE,
            Self::WriteLockPoisoned => Self::WRITE_LOCK_POISONED_CODE,
            Self::QueueCapacity => Self::QUEUE_CAPACITY_CODE,
            Self::QueueFull => Self::QUEUE_FULL_CODE,
            Self::QueueShutdown => Self::QUEUE_SHUTDOWN_CODE,
            Self::WrongRun { .. } => Self::WRONG_RUN_CODE,
            Self::SequenceGap { .. } => Self::SEQUENCE_GAP_CODE,
            Self::SequenceOverflow => Self::SEQUENCE_OVERFLOW_CODE,
            Self::BadMagic { .. } => Self::BAD_MAGIC_CODE,
            Self::UnsupportedSchemaVersion { .. } => Self::UNSUPPORTED_SCHEMA_VERSION_CODE,
            Self::MigrationRequired { .. } => Self::MIGRATION_REQUIRED_CODE,
            Self::UnknownRecordKind { .. } => Self::UNKNOWN_RECORD_KIND_CODE,
            Self::RecordKindFamilyMismatch { .. } => Self::RECORD_KIND_FAMILY_MISMATCH_CODE,
            Self::HeaderLengthMismatch { .. } => Self::HEADER_LENGTH_MISMATCH_CODE,
            Self::PayloadTooLarge { .. } => Self::PAYLOAD_TOO_LARGE_CODE,
            Self::HeaderChecksumMismatch => Self::HEADER_CHECKSUM_MISMATCH_CODE,
            Self::PayloadDigestMismatch => Self::PAYLOAD_DIGEST_MISMATCH_CODE,
            Self::UnexpectedEof => Self::UNEXPECTED_EOF_CODE,
            Self::PostcardDecodeFailed => Self::POSTCARD_DECODE_FAILED_CODE,
            Self::ArtifactMalformed => Self::ARTIFACT_MALFORMED_CODE,
            Self::ArtifactChecksumMismatch => Self::ARTIFACT_CHECKSUM_MISMATCH_CODE,
            Self::ArtifactNotFound { .. } => Self::ARTIFACT_NOT_FOUND_CODE,
            Self::ProcessLockHeld { .. } => Self::PROCESS_LOCK_HELD_CODE,
            Self::ProcessLockIo { .. } => Self::PROCESS_LOCK_IO_CODE,
        }
    }
}
