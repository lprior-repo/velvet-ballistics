use super::RuntimeError;
use crate::shard::ResumeError;
use std::sync::Arc;

impl From<vb_core::errors::CoreError> for RuntimeError {
    fn from(error: vb_core::errors::CoreError) -> Self {
        Self::Core {
            source: Box::new(error),
        }
    }
}

impl From<vb_storage::JournalError> for RuntimeError {
    fn from(error: vb_storage::JournalError) -> Self {
        Self::StorageJournalAppend {
            source: Arc::new(error),
        }
    }
}

impl RuntimeError {
    pub(crate) fn rollback_failed(operation: &'static str, primary: Self, rollback: Self) -> Self {
        Self::RollbackFailed {
            operation,
            primary: Box::new(primary),
            rollback: Box::new(rollback),
        }
    }

    pub(crate) fn admission_header_persistence_failed(error: Self) -> Self {
        match error {
            Self::StorageJournalAppend { source }
            | Self::AdmissionHeaderPersistenceFailed { source } => {
                Self::AdmissionHeaderPersistenceFailed { source }
            }
            other => other,
        }
    }
}

impl From<ResumeError> for RuntimeError {
    fn from(error: ResumeError) -> Self {
        match error {
            ResumeError::RunIdNotFound { run_id: _ } => Self::RunNotFound,
            ResumeError::NotResumable {
                run_id: _,
                current_state: _,
            } => Self::InvalidActionCompletion,
            ResumeError::IncompleteHydration { run_id: _ } => {
                Self::UnsupportedFullRecoveryHydration
            }
            ResumeError::JournalAppendFailed => {
                error
                    .source_runtime_error()
                    .unwrap_or(Self::StorageJournalAppend {
                        source: Arc::new(vb_storage::JournalError::WriteLockPoisoned),
                    })
            }
            ResumeError::JournalAppendFailedWithSource { source } => *source,
            ResumeError::StructuredOutputFailed => Self::EncodeFailed,
        }
    }
}
