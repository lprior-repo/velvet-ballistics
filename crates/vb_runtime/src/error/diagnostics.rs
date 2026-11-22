use super::RuntimeError;
use vb_core::{DiagnosticCode, HasSymbolicCode, SymbolicCode};

#[path = "diagnostics/runtime_code.rs"]
mod runtime_code;

impl RuntimeError {
    pub const SECRET_UNAVAILABLE_CODE: DiagnosticCode = DiagnosticCode::new(0x2026);
    pub const QUEUE_FULL_RUNTIME_CODE: &str = "QUEUE_FULL";
    pub const STORAGE_ERROR_RUNTIME_CODE: &str = "STORAGE_ERROR";
    pub const ADMISSION_DURABILITY_ERROR_RUNTIME_CODE: &str = "ADMISSION_DURABILITY_ERROR";
    pub const ACTION_FAILED_RUNTIME_CODE: &str = "ACTION_FAILED";

    pub const SHARD_ALLOCATION_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x202A);
    pub const TRACE_CAPACITY_OVERFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x202B);
    pub const QUEUE_FULL_CODE: DiagnosticCode = DiagnosticCode::new(0x2001);
    pub const RUN_NOT_FOUND_CODE: DiagnosticCode = DiagnosticCode::new(0x2002);
    pub const ACTIVE_RUN_CAPACITY_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x2003);
    pub const RUN_ALREADY_EXISTS_CODE: DiagnosticCode = DiagnosticCode::new(0x2004);
    pub const UNSUPPORTED_OPERATION_CODE: DiagnosticCode = DiagnosticCode::new(0x2005);
    pub const SHUTDOWN_IN_PROGRESS_CODE: DiagnosticCode = DiagnosticCode::new(0x2006);
    pub const JOURNAL_POISONED_CODE: DiagnosticCode = DiagnosticCode::new(0x2007);
    pub const STORAGE_JOURNAL_APPEND_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x2008);
    pub const ROLLBACK_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x2020);
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
    pub const ADMISSION_ACTION_REGISTRY_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x2021);
    pub const ADMISSION_ACTION_ABI_DIGEST_MISMATCH_CODE: DiagnosticCode =
        DiagnosticCode::new(0x2022);
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
    pub const INTROSPECTION_EPOCH_EXHAUSTED_CODE: DiagnosticCode = DiagnosticCode::new(0x2029);
    pub const RETRY_BLOCKED_ON_UNDRAINED_JOURNAL_CODE: DiagnosticCode = DiagnosticCode::new(0x2023);
    pub const PER_RUN_QUEUE_DEPTH_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x2024);
    pub const COMPAT_GATE_BLOCKED_CODE: DiagnosticCode = DiagnosticCode::new(0x2025);
    pub const OPEN_GATE_CAPACITY_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x2027);
    pub const OPEN_GATE_CAPACITY_ZERO_CODE: DiagnosticCode = DiagnosticCode::new(0x2028);

    #[must_use]
    pub fn diagnostic_code(&self) -> DiagnosticCode {
        self.diagnostic_code_capacity()
            .or_else(|| self.diagnostic_code_action_completion())
            .or_else(|| self.diagnostic_code_admission())
            .or_else(|| self.diagnostic_code_persistence())
            .or_else(|| self.diagnostic_code_recovery())
            .unwrap_or_else(|| self.diagnostic_code_other())
    }

    fn diagnostic_code_persistence(&self) -> Option<DiagnosticCode> {
        match self {
            Self::JournalPoisoned => Some(Self::JOURNAL_POISONED_CODE),
            Self::StorageJournalAppend { .. } => Some(Self::STORAGE_JOURNAL_APPEND_FAILED_CODE),
            Self::RollbackFailed { .. } => Some(Self::ROLLBACK_FAILED_CODE),
            Self::UnsupportedAsyncStrictAck => Some(Self::UNSUPPORTED_ASYNC_STRICT_ACK_CODE),
            Self::Core { source } => Some(match source.as_ref() {
                vb_core::errors::CoreError::QueueFull => Self::QUEUE_FULL_CODE,
                _ => Self::STORAGE_JOURNAL_APPEND_FAILED_CODE,
            }),
            Self::Recovery { .. } => Some(Self::STORAGE_JOURNAL_APPEND_FAILED_CODE),
            _ => None,
        }
    }

    fn diagnostic_code_recovery(&self) -> Option<DiagnosticCode> {
        match self {
            Self::UnsupportedFullRecoveryHydration => {
                Some(Self::UNSUPPORTED_FULL_RECOVERY_HYDRATION_CODE)
            }
            Self::InvalidRecoveryHydration
            | Self::RecoveryNotAvailable
            | Self::RecoveryCannotResume { .. } => Some(Self::INVALID_RECOVERY_HYDRATION_CODE),
            _ => None,
        }
    }

    fn diagnostic_code_capacity(&self) -> Option<DiagnosticCode> {
        match self {
            Self::ShardAllocationFailed { .. } => Some(Self::SHARD_ALLOCATION_FAILED_CODE),
            Self::TraceCapacityOverflow { .. } => Some(Self::TRACE_CAPACITY_OVERFLOW_CODE),
            Self::QueueFull => Some(Self::QUEUE_FULL_CODE),
            Self::PerRunQueueDepthExceeded { .. } => Some(Self::PER_RUN_QUEUE_DEPTH_EXCEEDED_CODE),
            Self::ActiveRunCapacityExceeded { .. } => Some(Self::ACTIVE_RUN_CAPACITY_EXCEEDED_CODE),
            Self::OpenGateCapacityExceeded { .. } => Some(Self::OPEN_GATE_CAPACITY_EXCEEDED_CODE),
            Self::OpenGateCapacityZero => Some(Self::OPEN_GATE_CAPACITY_ZERO_CODE),
            Self::JournalFull { .. } => Some(Self::JOURNAL_FULL_CODE),
            Self::CommandQueueCapacityExceeded { .. } => {
                Some(Self::COMMAND_QUEUE_CAPACITY_EXCEEDED_CODE)
            }
            Self::ActiveRunCapacityZero => Some(Self::ACTIVE_RUN_CAPACITY_ZERO_CODE),
            Self::FramePoolUnavailable => Some(Self::FRAME_POOL_UNAVAILABLE_CODE),
            _ => None,
        }
    }

    fn diagnostic_code_action_completion(&self) -> Option<DiagnosticCode> {
        match self {
            Self::InvalidActionCompletion
            | Self::InvalidGateDecisionRecord
            | Self::InvalidGateOpenedRecord
            | Self::GateAlreadyOpen { .. }
            | Self::GateNotOpen { .. }
            | Self::GateEvaluationFailed { .. }
            | Self::GateRunNotSuspendable { .. }
            | Self::GateJournalBudgetExceeded { .. }
            | Self::GateWrongDelegate { .. }
            | Self::GateRunMismatch { .. }
            | Self::GateOpenBlocksResume { .. }
            | Self::StaleAttempt { .. }
            | Self::StaleFencingToken { .. }
            | Self::AttemptBeyondMax { .. }
            | Self::ActionOutputLengthMismatch { .. }
            | Self::ActionOutputTooLarge { .. }
            | Self::ActionOutputBlobTooLarge { .. }
            | Self::ActionTaintDowngrade { .. } => Some(Self::INVALID_ACTION_COMPLETION_CODE),
            Self::InvalidTimerFire => Some(Self::INVALID_TIMER_FIRE_CODE),
            Self::EngineDriveFailed { .. } => Some(Self::ENGINE_DRIVE_FAILED_CODE),
            _ => None,
        }
    }

    fn diagnostic_code_admission(&self) -> Option<DiagnosticCode> {
        match self {
            Self::AdmissionArtifactNotFound { .. } => Some(Self::ADMISSION_ARTIFACT_NOT_FOUND_CODE),
            Self::AdmissionArtifactInvalid { .. } => Some(Self::ADMISSION_ARTIFACT_INVALID_CODE),
            Self::ResourceQuotaExceeded { .. } | Self::ResourceAccountingUnderflow { .. } => {
                Some(vb_core::errors::CoreError::INTERNAL_INVARIANT_CODE)
            }
            Self::AdmissionArtifactDigestMismatch { .. } => {
                Some(Self::ADMISSION_ARTIFACT_DIGEST_MISMATCH_CODE)
            }
            Self::AdmissionCapabilityDenied { .. } => Some(Self::ADMISSION_CAPABILITY_DENIED_CODE),
            Self::AdmissionCapabilityCountMismatch { .. } => {
                Some(Self::ADMISSION_CAPABILITY_COUNT_MISMATCH_CODE)
            }
            Self::AdmissionActionRegistryMismatch { .. } => {
                Some(Self::ADMISSION_ACTION_REGISTRY_MISMATCH_CODE)
            }
            Self::AdmissionActionAbiDigestMismatch { .. } => {
                Some(Self::ADMISSION_ACTION_ABI_DIGEST_MISMATCH_CODE)
            }
            Self::AdmissionArtifactStale { .. } => Some(Self::ADMISSION_ARTIFACT_STALE_CODE),
            Self::AdmissionDigestMismatch { .. } => Some(Self::ADMISSION_DIGEST_MISMATCH_CODE),
            Self::AdmissionHeaderPersistenceFailed { .. } => {
                Some(Self::ADMISSION_HEADER_PERSISTENCE_FAILED_CODE)
            }
            Self::ContractValidationError { .. } => Some(Self::ADMISSION_CAPABILITY_DENIED_CODE),
            Self::EncodeFailed => Some(Self::ENCODE_FAILED_CODE),
            _ => None,
        }
    }

    fn diagnostic_code_other(&self) -> DiagnosticCode {
        match self {
            Self::SecretUnavailable => Self::SECRET_UNAVAILABLE_CODE,
            Self::RunNotFound => Self::RUN_NOT_FOUND_CODE,
            Self::RunAlreadyExists => Self::RUN_ALREADY_EXISTS_CODE,
            Self::UnsupportedOperation { .. } => Self::UNSUPPORTED_OPERATION_CODE,
            Self::ShutdownInProgress => Self::SHUTDOWN_IN_PROGRESS_CODE,
            Self::SecretResultNotAllowed => Self::SECRET_RESULT_NOT_ALLOWED_CODE,
            Self::IpcPayloadSizeExceeded { .. } => Self::IPC_PAYLOAD_SIZE_EXCEEDED_CODE,
            Self::UnsupportedRuntimeJournalEventMapping { .. }
            | Self::RuntimeJournalTimestampOutOfRange { .. }
            | Self::UnsupportedDurabilityProfile { .. } => {
                vb_core::errors::CoreError::INTERNAL_INVARIANT_CODE
            }
            Self::RetryBlockedOnUndrainedJournal { .. } => {
                Self::RETRY_BLOCKED_ON_UNDRAINED_JOURNAL_CODE
            }
            Self::ShardNotFound { .. } => Self::SHARD_NOT_FOUND_CODE,
            Self::MigrateSelf => Self::MIGRATE_SELF_CODE,
            Self::IntrospectionEpochExhausted => Self::INTROSPECTION_EPOCH_EXHAUSTED_CODE,
            Self::RuntimeInputSlotKindMismatch { .. } => Self::UNSUPPORTED_OPERATION_CODE,
            Self::CompatGateBlocked { .. } => Self::COMPAT_GATE_BLOCKED_CODE,
            _ => vb_core::errors::CoreError::INTERNAL_INVARIANT_CODE,
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
            Self::StorageJournalAppend { .. } | Self::Core { .. } => {
                Some(Self::storage_append_symbolic_code())
            }
            Self::RollbackFailed { primary, .. } => Some(primary.symbolic_code()),
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
