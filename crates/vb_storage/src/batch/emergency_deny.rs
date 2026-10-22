#![forbid(unsafe_code)]
use super::types::JournalWriteBatch;
use crate::error::JournalError;

impl<'j> JournalWriteBatch<'j> {
    /// Inserts the emergency-deny constant active marker at the fixed key.
    /// Crate-private: no arbitrary key/value public API.
    pub(crate) fn put_emergency_deny(&mut self) -> Result<(), JournalError> {
        self.inner.insert(
            &self.journal.emergency_deny,
            crate::journal::EMERGENCY_DENY_KEY,
            crate::journal::EMERGENCY_DENY_ACTIVE_MARKER,
        );
        Ok(())
    }
}
