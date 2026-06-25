use crate::{error::JournalError, events::JournalEvent, journal::FjallJournal};

impl FjallJournal {
    /// Appends one event without forcing a durability barrier.
    pub fn append_journaled(&self, event: &JournalEvent) -> Result<(), JournalError> {
        self.append_unfsynced(event)
    }

    /// Appends one event and forces a strict durability barrier before returning.
    pub fn append_strict(&self, event: &JournalEvent) -> Result<(), JournalError> {
        self.append_unfsynced(event)?;
        self.persist_strict()
    }

    /// Appends multiple events with a single strict durability barrier.
    ///
    /// Atomicity: every event in `events` either becomes durable together or
    /// no event is made durable. Implemented via a `fjall::OwnedWriteBatch`
    /// wrapped through [`crate::batch::JournalWriteBatch`]: events are
    /// staged into a single cross-event batch and committed with
    /// `PersistMode::SyncAll`, so a process crash mid-batch leaves no
    /// partial, durable-visible record set. Master §49 Crash-Consistency
    /// Rule requires this single-barrier semantic; the previous
    /// per-event `append_unfsynced` loop violated it.
    pub fn append_strict_batch(&self, events: &[JournalEvent]) -> Result<(), JournalError> {
        if events.is_empty() {
            return Ok(());
        }
        let mut batch = self.batch();
        for event in events {
            batch.append_event(event)?;
        }
        batch.strict().commit()
    }

    /// Forces a strict durability barrier.
    pub fn persist_strict(&self) -> Result<(), JournalError> {
        #[cfg(test)]
        if self.consume_persist_failure_for_test() {
            return Err(JournalError::StrictDurabilityFailed);
        }
        self.database.persist(fjall::PersistMode::SyncAll)?;
        Ok(())
    }
}
