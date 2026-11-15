use super::RuntimeError;

impl RuntimeError {
    #[must_use]
    pub fn runtime_code(&self) -> Option<&'static str> {
        self.runtime_code_input()
            .or_else(|| self.runtime_code_secret())
            .or_else(|| self.runtime_code_capacity())
            .or_else(|| self.runtime_code_storage())
            .or_else(|| self.runtime_code_action())
    }

    fn runtime_code_input(&self) -> Option<&'static str> {
        match self {
            Self::RuntimeInputSlotKindMismatch { .. } => {
                Some(vb_core::errors::CoreError::INPUT_TYPE_MISMATCH_RUNTIME_CODE)
            }
            _ => None,
        }
    }

    fn runtime_code_secret(&self) -> Option<&'static str> {
        match self {
            Self::SecretUnavailable => Some("SECRET_UNAVAILABLE"),
            _ => None,
        }
    }

    fn runtime_code_capacity(&self) -> Option<&'static str> {
        match self {
            Self::QueueFull
            | Self::PerRunQueueDepthExceeded { .. }
            | Self::ActiveRunCapacityExceeded { .. }
            | Self::OpenGateCapacityExceeded { .. }
            | Self::JournalFull { .. } => Some(Self::QUEUE_FULL_RUNTIME_CODE),
            _ => None,
        }
    }

    fn runtime_code_storage(&self) -> Option<&'static str> {
        match self {
            Self::JournalPoisoned | Self::UnsupportedAsyncStrictAck => {
                Some(Self::STORAGE_ERROR_RUNTIME_CODE)
            }
            Self::StorageJournalAppend { .. } => Some(Self::STORAGE_ERROR_RUNTIME_CODE),
            Self::RollbackFailed { primary, .. } => primary.runtime_code(),
            Self::AdmissionHeaderPersistenceFailed { .. }
            | Self::AdmissionArtifactDigestMismatch { .. }
            | Self::AdmissionArtifactStale { .. }
            | Self::AdmissionDigestMismatch { .. } => {
                Some(Self::ADMISSION_DURABILITY_ERROR_RUNTIME_CODE)
            }
            Self::Core { source } => source
                .runtime_code()
                .or(Some(Self::STORAGE_ERROR_RUNTIME_CODE)),
            _ => None,
        }
    }

    fn runtime_code_action(&self) -> Option<&'static str> {
        match self {
            Self::InvalidActionCompletion
            | Self::InvalidGateDecisionRecord
            | Self::InvalidGateOpenedRecord
            | Self::GateAlreadyOpen { .. }
            | Self::GateNotOpen { .. }
            | Self::GateEvaluationFailed { .. }
            | Self::GateRunNotSuspendable { .. }
            | Self::GateWrongDelegate { .. }
            | Self::GateRunMismatch { .. }
            | Self::GateOpenBlocksResume { .. }
            | Self::StaleAttempt { .. }
            | Self::StaleFencingToken { .. }
            | Self::AttemptBeyondMax { .. }
            | Self::ActionOutputLengthMismatch { .. }
            | Self::ActionOutputTooLarge { .. }
            | Self::ActionOutputBlobTooLarge { .. }
            | Self::ActionTaintDowngrade { .. }
            | Self::EngineDriveFailed { .. }
            | Self::RetryBlockedOnUndrainedJournal { .. } => Some(Self::ACTION_FAILED_RUNTIME_CODE),
            _ => None,
        }
    }
}
