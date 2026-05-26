use super::RuntimeError;
use vb_core::{DiagnosticCode, HasSymbolicCode, SymbolicCode};

impl RuntimeError {
    pub const QUEUE_FULL_RUNTIME_CODE: &str = "QUEUE_FULL";
    pub const STORAGE_ERROR_RUNTIME_CODE: &str = "STORAGE_ERROR";
    pub const ADMISSION_DURABILITY_ERROR_RUNTIME_CODE: &str = "ADMISSION_DURABILITY_ERROR";
    pub const ACTION_FAILED_RUNTIME_CODE: &str = "ACTION_FAILED";

    pub const QUEUE_FULL_CODE: DiagnosticCode = DiagnosticCode::new(0x2001);
    pub const RUN_NOT_FOUND_CODE: DiagnosticCode = DiagnosticCode::new(0x2002);
    pub const ACTIVE_RUN_CAPACITY_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x2003);
    pub const RUN_ALREADY_EXISTS_CODE: DiagnosticCode = DiagnosticCode::new(0x2004);
    pub const UNSUPPORTED_OPERATION_CODE: DiagnosticCode = DiagnosticCode::new(0x2005);
    pub const SHUTDOWN_IN_PROGRESS_CODE: DiagnosticCode = DiagnosticCode::new(0x2006);
    pub const JOURNAL_POISONED_CODE: DiagnosticCode = DiagnosticCode::new(0x2007);
    pub const STORAGE_JOURNAL_APPEND_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x2008);
    pub const JOURNAL_FULL_CODE: DiagnosticCode = DiagnosticCode::new(0x201E);
    pub const ADMISSION_HEADER_PERSISTENCE_FAILED_CODE: DiagnosticCode =
        DiagnosticCode::new(0x2015);
    pub const UNSUPPORTED_ASYNC_STRICT_ACK_CODE: DiagnosticCode = DiagnosticCode::new(0x2009);
    pub const FRAME_POOL_UNAVAILABLE_CODE: DiagnosticCode = DiagnosticCode::new(0x200A);
    pub const INVALID_ACTION_COMPLETION_CODE: DiagnosticCode = DiagnosticCode::new(0x200B);
    pub const INVALID_TIMER_FIRE_CODE: DiagnosticCode = DiagnosticCode::new(0x200C);
    pub const UNSUPPORTED_FULL_RECOVERY_HYDRATION_CODE: DiagnosticCode =
        DiagnosticCode::new(0x200D);
    pub const INVALID_RECOVERY_HYDRATION_CODE: DiagnosticCode = DiagnosticCode::new(0x200E);
    pub const COMMAND_QUEUE_CAPACITY_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x200F);
    pub const ACTIVE_RUN_CAPACITY_ZERO_CODE: DiagnosticCode = DiagnosticCode::new(0x2010);
    pub const ADMISSION_ARTIFACT_NOT_FOUND_CODE: DiagnosticCode = DiagnosticCode::new(0x2011);
    pub const ADMISSION_CAPABILITY_DENIED_CODE: DiagnosticCode = DiagnosticCode::new(0x2012);
    pub const ADMISSION_ARTIFACT_INVALID_CODE: DiagnosticCode = DiagnosticCode::new(0x2014);
    pub const ENCODE_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x2013);
    pub const SECRET_RESULT_NOT_ALLOWED_CODE: DiagnosticCode = DiagnosticCode::new(0x2016);
    pub const IPC_PAYLOAD_SIZE_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x2017);
    pub const ADMISSION_ARTIFACT_DIGEST_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x2018);
    pub const ADMISSION_ARTIFACT_STALE_CODE: DiagnosticCode = DiagnosticCode::new(0x2019);
    pub const ADMISSION_DIGEST_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x201A);
    pub const ENGINE_DRIVE_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x201B);
    pub const SHARD_NOT_FOUND_CODE: DiagnosticCode = DiagnosticCode::new(0x201C);
    pub const MIGRATE_SELF_CODE: DiagnosticCode = DiagnosticCode::new(0x201D);

    #[must_use]
    pub fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            Self::QueueFull => Self::QUEUE_FULL_CODE,
            Self::RunNotFound => Self::RUN_NOT_FOUND_CODE,
            Self::ActiveRunCapacityExceeded { .. } => Self::ACTIVE_RUN_CAPACITY_EXCEEDED_CODE,
            Self::RunAlreadyExists => Self::RUN_ALREADY_EXISTS_CODE,
            Self::UnsupportedOperation { .. } => Self::UNSUPPORTED_OPERATION_CODE,
            Self::ShutdownInProgress => Self::SHUTDOWN_IN_PROGRESS_CODE,
            Self::JournalPoisoned => Self::JOURNAL_POISONED_CODE,
            Self::JournalFull { .. } => Self::JOURNAL_FULL_CODE,
            Self::StorageJournalAppend { .. } => Self::STORAGE_JOURNAL_APPEND_FAILED_CODE,
            Self::AdmissionHeaderPersistenceFailed { .. } => {
                Self::ADMISSION_HEADER_PERSISTENCE_FAILED_CODE
            }
            Self::Core { source } => match source.as_ref() {
                vb_core::errors::CoreError::QueueFull => Self::QUEUE_FULL_CODE,
                _ => Self::STORAGE_JOURNAL_APPEND_FAILED_CODE,
            },
            Self::UnsupportedAsyncStrictAck => Self::UNSUPPORTED_ASYNC_STRICT_ACK_CODE,
            Self::FramePoolUnavailable => Self::FRAME_POOL_UNAVAILABLE_CODE,
            Self::InvalidActionCompletion
            | Self::StaleAttempt { .. }
            | Self::AttemptBeyondMax { .. }
            | Self::ActionOutputLengthMismatch { .. }
            | Self::ActionOutputTooLarge { .. }
            | Self::ActionOutputBlobTooLarge { .. }
            | Self::ActionTaintDowngrade { .. } => Self::INVALID_ACTION_COMPLETION_CODE,
            Self::InvalidTimerFire => Self::INVALID_TIMER_FIRE_CODE,
            Self::UnsupportedFullRecoveryHydration => {
                Self::UNSUPPORTED_FULL_RECOVERY_HYDRATION_CODE
            }
            Self::InvalidRecoveryHydration => Self::INVALID_RECOVERY_HYDRATION_CODE,
            Self::CommandQueueCapacityExceeded { .. } => Self::COMMAND_QUEUE_CAPACITY_EXCEEDED_CODE,
            Self::ActiveRunCapacityZero => Self::ACTIVE_RUN_CAPACITY_ZERO_CODE,
            Self::AdmissionArtifactNotFound { .. } => Self::ADMISSION_ARTIFACT_NOT_FOUND_CODE,
            Self::AdmissionArtifactInvalid { .. } => Self::ADMISSION_ARTIFACT_INVALID_CODE,
            Self::AdmissionArtifactDigestMismatch { .. } => {
                Self::ADMISSION_ARTIFACT_DIGEST_MISMATCH_CODE
            }
            Self::AdmissionCapabilityDenied { .. } => Self::ADMISSION_CAPABILITY_DENIED_CODE,
            Self::AdmissionArtifactStale { .. } => Self::ADMISSION_ARTIFACT_STALE_CODE,
            Self::AdmissionDigestMismatch { .. } => Self::ADMISSION_DIGEST_MISMATCH_CODE,
            Self::EncodeFailed => Self::ENCODE_FAILED_CODE,
            Self::SecretResultNotAllowed => Self::SECRET_RESULT_NOT_ALLOWED_CODE,
            Self::IpcPayloadSizeExceeded { .. } => Self::IPC_PAYLOAD_SIZE_EXCEEDED_CODE,
            Self::EngineDriveFailed { .. } => Self::ENGINE_DRIVE_FAILED_CODE,
            Self::ShardNotFound { .. } => Self::SHARD_NOT_FOUND_CODE,
            Self::MigrateSelf => Self::MIGRATE_SELF_CODE,
        }
    }

    #[must_use]
    pub fn runtime_code(&self) -> Option<&'static str> {
        match self {
            Self::QueueFull | Self::ActiveRunCapacityExceeded { .. } => {
                Some(Self::QUEUE_FULL_RUNTIME_CODE)
            }
            Self::JournalFull { .. } => Some(Self::QUEUE_FULL_RUNTIME_CODE),
            Self::JournalPoisoned | Self::UnsupportedAsyncStrictAck => {
                Some(Self::STORAGE_ERROR_RUNTIME_CODE)
            }
            Self::StorageJournalAppend { .. } => Some(Self::STORAGE_ERROR_RUNTIME_CODE),
            Self::AdmissionHeaderPersistenceFailed { .. } => {
                Some(Self::ADMISSION_DURABILITY_ERROR_RUNTIME_CODE)
            }
            Self::AdmissionArtifactDigestMismatch { .. } => {
                Some(Self::ADMISSION_DURABILITY_ERROR_RUNTIME_CODE)
            }
            Self::AdmissionArtifactStale { .. } => {
                Some(Self::ADMISSION_DURABILITY_ERROR_RUNTIME_CODE)
            }
            Self::AdmissionDigestMismatch { .. } => {
                Some(Self::ADMISSION_DURABILITY_ERROR_RUNTIME_CODE)
            }
            Self::Core { source } => match source.as_ref() {
                vb_core::errors::CoreError::QueueFull => Some(Self::QUEUE_FULL_RUNTIME_CODE),
                _ => Some(Self::STORAGE_ERROR_RUNTIME_CODE),
            },
            Self::InvalidActionCompletion
            | Self::StaleAttempt { .. }
            | Self::AttemptBeyondMax { .. }
            | Self::ActionOutputLengthMismatch { .. }
            | Self::ActionOutputTooLarge { .. }
            | Self::ActionOutputBlobTooLarge { .. }
            | Self::ActionTaintDowngrade { .. } => Some(Self::ACTION_FAILED_RUNTIME_CODE),
            Self::EngineDriveFailed { .. } => Some(Self::ACTION_FAILED_RUNTIME_CODE),
            Self::RunNotFound
            | Self::RunAlreadyExists
            | Self::UnsupportedOperation { .. }
            | Self::ShutdownInProgress
            | Self::FramePoolUnavailable
            | Self::InvalidTimerFire
            | Self::UnsupportedFullRecoveryHydration
            | Self::InvalidRecoveryHydration
            | Self::CommandQueueCapacityExceeded { .. }
            | Self::ActiveRunCapacityZero
            | Self::AdmissionArtifactNotFound { .. }
            | Self::AdmissionArtifactInvalid { .. }
            | Self::AdmissionCapabilityDenied { .. }
            | Self::EncodeFailed
            | Self::SecretResultNotAllowed
            | Self::IpcPayloadSizeExceeded { .. }
            | Self::ShardNotFound { .. }
            | Self::MigrateSelf => None,
        }
    }

    /// Returns the stable symbolic diagnostic code for this error.
    #[must_use]
    pub fn symbolic_code(&self) -> SymbolicCode {
        let s: &'static str = match self {
            Self::QueueFull => "QUEUE_FULL",
            Self::RunNotFound => "RUN_NOT_FOUND",
            Self::ActiveRunCapacityExceeded { .. } => "ACTIVE_RUN_CAPACITY_EXCEEDED",
            Self::RunAlreadyExists => "RUN_ALREADY_EXISTS",
            Self::UnsupportedOperation { .. } => "UNSUPPORTED_OPERATION",
            Self::ShutdownInProgress => "SHUTDOWN_IN_PROGRESS",
            Self::JournalPoisoned => "JOURNAL_POISONED",
            Self::JournalFull { .. } => "JOURNAL_FULL",
            Self::StorageJournalAppend { .. } => "STORAGE_JOURNAL_APPEND",
            Self::AdmissionHeaderPersistenceFailed { .. } => "ADMISSION_HEADER_PERSISTENCE_FAILED",
            Self::Core { .. } => "STORAGE_JOURNAL_APPEND",
            Self::UnsupportedAsyncStrictAck => "UNSUPPORTED_ASYNC_STRICT_ACK",
            Self::FramePoolUnavailable => "FRAME_POOL_UNAVAILABLE",
            Self::InvalidActionCompletion
            | Self::StaleAttempt { .. }
            | Self::AttemptBeyondMax { .. }
            | Self::ActionOutputLengthMismatch { .. }
            | Self::ActionOutputTooLarge { .. }
            | Self::ActionOutputBlobTooLarge { .. }
            | Self::ActionTaintDowngrade { .. } => "INVALID_ACTION_COMPLETION",
            Self::InvalidTimerFire => "INVALID_TIMER_FIRE",
            Self::UnsupportedFullRecoveryHydration => "UNSUPPORTED_FULL_RECOVERY_HYDRATION",
            Self::InvalidRecoveryHydration => "INVALID_RECOVERY_HYDRATION",
            Self::CommandQueueCapacityExceeded { .. } => "COMMAND_QUEUE_CAPACITY_EXCEEDED",
            Self::ActiveRunCapacityZero => "ACTIVE_RUN_CAPACITY_ZERO",
            Self::AdmissionArtifactNotFound { .. } => "ADMISSION_ARTIFACT_NOT_FOUND",
            Self::AdmissionArtifactInvalid { .. } => "ADMISSION_ARTIFACT_INVALID",
            Self::AdmissionArtifactDigestMismatch { .. } => "ADMISSION_ARTIFACT_DIGEST_MISMATCH",
            Self::AdmissionCapabilityDenied { .. } => "ADMISSION_CAPABILITY_DENIED",
            Self::AdmissionArtifactStale { .. } => "ADMISSION_ARTIFACT_STALE",
            Self::AdmissionDigestMismatch { .. } => "ADMISSION_DIGEST_MISMATCH",
            Self::EncodeFailed => "ENCODE_FAILED",
            Self::SecretResultNotAllowed => "SECRET_RESULT_NOT_ALLOWED",
            Self::IpcPayloadSizeExceeded { .. } => "IPC_PAYLOAD_SIZE_EXCEEDED",
            Self::EngineDriveFailed { .. } => "ENGINE_DRIVE_FAILED",
            Self::ShardNotFound { .. } => "SHARD_NOT_FOUND",
            Self::MigrateSelf => "MIGRATE_SELF",
        };
        if let Some(code) = SymbolicCode::from_static(s) {
            return code;
        }
        // Unreachable: all match arms use registered symbolic names.
        SymbolicCode::from_parts("INTERNAL_INVARIANT_VIOLATION", 0x1309)
    }
}

impl HasSymbolicCode for RuntimeError {
    fn symbolic_code(&self) -> SymbolicCode {
        self.symbolic_code()
    }
}
