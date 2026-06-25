#![forbid(unsafe_code)]
use crate::error::JournalError;
use super::types::JournalWriteBatch;
impl<'j> JournalWriteBatch<'j> {
    pub fn strict(mut self) -> Self {
        self.inner = self.inner.durability(Some(fjall::PersistMode::SyncAll));
        self
    }
    pub fn commit(self) -> Result<(), JournalError> {
        if self.aborted {
            return Err(JournalError::BatchAborted);
        }
        self.inner.commit()?;
        Ok(())
    }
}
