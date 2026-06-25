#![forbid(unsafe_code)]
use crate::error::JournalError;
use super::types::JournalWriteBatch;

impl<'j> JournalWriteBatch<'j> {
    /// Sets strict durability for the commit.
    pub fn strict(mut self) -> Self {
        self.inner = self.inner.durability(Some(fjall::PersistMode::SyncAll));
        self
    }

    /// Commits the batch atomically.
    ///
    /// Returns `Err(JournalError::BatchAborted)` when a prior `put_*` /
    /// `append_event` staging step set `self.aborted = true`. The batch
    /// is **never** committed in this case — neither the staged records
    /// nor any partial state are made durable. Master §49
    /// Crash-Consistency Rule forbids silent success on a partial
    /// barrier; the typed error is the only honest return value.
    pub fn commit(self) -> Result<(), JournalError> {
        if self.aborted {
            return Err(JournalError::BatchAborted);
        }
        self.inner.commit()?;
        Ok(())
    }
}