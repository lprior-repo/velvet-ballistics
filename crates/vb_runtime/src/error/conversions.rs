use super::RuntimeError;
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
