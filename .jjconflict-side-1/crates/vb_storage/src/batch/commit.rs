#![forbid(unsafe_code)]
use super::types::JournalWriteBatch;
use crate::error::JournalError;

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
        // Test-only fault injection: return a synthetic `JournalError::Fjall`
        // BEFORE the real `OwnedWriteBatch` reaches the storage engine. This
        // proves the `append_strict` commit-failure contract (vb-o6qcf.3): the
        // staged event is never made visible because the batch never commits,
        // and a retry re-stages and commits cleanly (idempotent Ok). The hook
        // is consumed exactly once, so a subsequent real commit succeeds.
        // cfg(test) guarantees this path is absent from non-test builds.
        #[cfg(test)]
        if self.journal.consume_batch_commit_failure_for_test() {
            return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "injected batch commit failure (test hook)",
            ))));
        }
        self.inner.commit()?;
        Ok(())
    }
}
