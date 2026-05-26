use crate::error::JournalError;
use vb_core::diagnostic::{HasSymbolicCode, SymbolicCode};
use vb_core::DiagnosticCode;

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
    /// Diagnostic code for semantically invalid journal event (run_id=0, seq overflow, or attempt=0).
    pub const INVALID_EVENT_CODE: DiagnosticCode = DiagnosticCode::new(0x4020);
    /// Diagnostic code for artifact malformed.
    pub const ARTIFACT_MALFORMED_CODE: DiagnosticCode = DiagnosticCode::new(0x4017);
    /// Diagnostic code for artifact checksum mismatch.
    pub const ARTIFACT_CHECKSUM_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x4018);
    /// Diagnostic code for invalid gate count.
    pub const INVALID_GATE_COUNT_CODE: DiagnosticCode = DiagnosticCode::new(0x401C);
    /// Diagnostic code for missing required proof flag.
    pub const MISSING_REQUIRED_PROOF_FLAG_CODE: DiagnosticCode = DiagnosticCode::new(0x401D);
    /// Diagnostic code for artifact not found.
    pub const ARTIFACT_NOT_FOUND_CODE: DiagnosticCode = DiagnosticCode::new(0x4019);
    /// Diagnostic code for process lock held by another process.
    pub const PROCESS_LOCK_HELD_CODE: DiagnosticCode = DiagnosticCode::new(0x401A);
    /// Diagnostic code for process lock I/O error.
    pub const PROCESS_LOCK_IO_CODE: DiagnosticCode = DiagnosticCode::new(0x401B);
    /// Diagnostic code for replay event limit exceeded.
    pub const TOO_MANY_EVENTS_CODE: DiagnosticCode = DiagnosticCode::new(0x401E);
    /// Diagnostic code for replay allocation failure.
    pub const REPLAY_ALLOCATION_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x401F);

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
            Self::InvalidEvent => Self::INVALID_EVENT_CODE,
            Self::ArtifactMalformed => Self::ARTIFACT_MALFORMED_CODE,
            Self::ArtifactChecksumMismatch => Self::ARTIFACT_CHECKSUM_MISMATCH_CODE,
            Self::InvalidGateCount { .. } => Self::INVALID_GATE_COUNT_CODE,
            Self::MissingRequiredProofFlag { .. } => Self::MISSING_REQUIRED_PROOF_FLAG_CODE,
            Self::ArtifactNotFound { .. } => Self::ARTIFACT_NOT_FOUND_CODE,
            Self::AdmissionRequired
            | Self::ArtifactInvalid { .. }
            | Self::InputTooLarge { .. }
            | Self::InputSchemaMismatch
            | Self::CapabilityDenied
            | Self::SecretUnavailable
            | Self::RunAlreadyExists
            | Self::ActiveRunCapacityExceeded
            | Self::FrameAllocationFailed
            | Self::AdmissionJournalFailed
            | Self::StrictDurabilityFailed
            | Self::ClockUnavailable => Self::ARTIFACT_MALFORMED_CODE,
            Self::TooManyEvents { .. } => Self::TOO_MANY_EVENTS_CODE,
            Self::ReplayAllocationFailed { .. } => Self::REPLAY_ALLOCATION_FAILED_CODE,
            Self::ProcessLockHeld { .. } => Self::PROCESS_LOCK_HELD_CODE,
            Self::ProcessLockIo { .. } => Self::PROCESS_LOCK_IO_CODE,
            Self::Trim(_) => Self::FJALL_CODE, // Map trim errors to a generic code
        }
    }
}

impl HasSymbolicCode for JournalError {
    /// Returns the [`SymbolicCode`] for this journal error.
    ///
    /// Delegates to [`JournalError::diagnostic_code`] and converts the
    /// numeric code to its registered symbolic name via
    /// [`DiagnosticCode::symbolic_code`]. Falls back to
    /// [`SymbolicCode::INTERNAL_INVARIANT`] when the numeric code is
    /// not yet registered in [`CODE_REGISTRY`].
    fn symbolic_code(&self) -> SymbolicCode {
        self.diagnostic_code()
            .symbolic_code()
            .unwrap_or(SymbolicCode::INTERNAL_INVARIANT)
    }
}
