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
    pub const ADMISSION_CAPABILITY_COUNT_MISMATCH_CODE: DiagnosticCode =
        DiagnosticCode::new(0x201F);
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
    pub const INTROSPECTION_EPOCH_EXHAUSTED_CODE: DiagnosticCode = DiagnosticCode::new(0x201F);

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
            Self::RunStateRollbackFailed { .. } => Self::STORAGE_JOURNAL_APPEND_FAILED_CODE,
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
            Self::AdmissionCapabilityCountMismatch { .. } => {
                Self::ADMISSION_CAPABILITY_COUNT_MISMATCH_CODE
            }
            Self::AdmissionArtifactStale { .. } => Self::ADMISSION_ARTIFACT_STALE_CODE,
            Self::AdmissionDigestMismatch { .. } => Self::ADMISSION_DIGEST_MISMATCH_CODE,
            Self::EncodeFailed => Self::ENCODE_FAILED_CODE,
            Self::SecretResultNotAllowed => Self::SECRET_RESULT_NOT_ALLOWED_CODE,
            Self::IpcPayloadSizeExceeded { .. } => Self::IPC_PAYLOAD_SIZE_EXCEEDED_CODE,
            Self::EngineDriveFailed { .. } => Self::ENGINE_DRIVE_FAILED_CODE,
            Self::ShardNotFound { .. } => Self::SHARD_NOT_FOUND_CODE,
            Self::MigrateSelf => Self::MIGRATE_SELF_CODE,
            Self::IntrospectionEpochExhausted => Self::INTROSPECTION_EPOCH_EXHAUSTED_CODE,
            // VB-NOORE: typed profile-mismatch error. No dedicated
            // diagnostic code; routed to INTERNAL_INVARIANT.
            Self::UnsupportedDurabilityProfile { .. } => {
                vb_core::errors::CoreError::INTERNAL_INVARIANT_CODE
            }
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
            Self::RunStateRollbackFailed { .. } => Some(Self::STORAGE_ERROR_RUNTIME_CODE),
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
            | Self::AdmissionCapabilityCountMismatch { .. }
            | Self::EncodeFailed
            | Self::SecretResultNotAllowed
            | Self::IpcPayloadSizeExceeded { .. }
            | Self::ShardNotFound { .. }
            | Self::MigrateSelf
            | Self::IntrospectionEpochExhausted
            | Self::UnsupportedDurabilityProfile { .. } => None,
        }
    }

    /// Returns the stable symbolic diagnostic code for this error.
    #[must_use]
    pub fn symbolic_code(&self) -> SymbolicCode {
        match self.legacy_unregistered_symbolic_code() {
            Some(code) => code,
            None => self.registered_symbolic_code(),
        }
    }

    fn legacy_unregistered_symbolic_code(&self) -> Option<SymbolicCode> {
        match self {
            Self::StorageJournalAppend { .. }
            | Self::RunStateRollbackFailed { .. }
            | Self::Core { .. } => Some(Self::storage_append_symbolic_code()),
            // NOTE: #[non_exhaustive] - new RuntimeError variants return None for symbolic code.
            // Add explicit match arms for new variants.
            _ => None,
        }
    }

    fn storage_append_symbolic_code() -> SymbolicCode {
        match SymbolicCode::from_static("STORAGE_JOURNAL_APPEND") {
            Some(code) => code,
            None => SymbolicCode::INTERNAL_INVARIANT,
        }
    }

    fn registered_symbolic_code(&self) -> SymbolicCode {
        match self.diagnostic_code().symbolic_code() {
            Some(code) => code,
            None => SymbolicCode::INTERNAL_INVARIANT,
        }
    }
}

impl HasSymbolicCode for RuntimeError {
    /// Returns the [`SymbolicCode`] for this runtime error.
    ///
    /// Delegates to the inherent [`RuntimeError::symbolic_code`] method
    /// which uses a direct string-to-registry lookup with a fallback
    /// to [`SymbolicCode::INTERNAL_INVARIANT`].
    fn symbolic_code(&self) -> SymbolicCode {
        self.symbolic_code()
    }
}
