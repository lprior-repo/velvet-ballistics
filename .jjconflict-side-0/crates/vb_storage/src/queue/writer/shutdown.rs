//! Durable shutdown owns failed batches until their barrier succeeds.
use super::{FjallJournal, JournalError, JournalWriterFlushReport, JournalWriterQueue};

impl JournalWriterQueue {
    /// Stops admission and synchronously commits each accepted batch before
    /// removing it. Failed batches remain queued and this operation is retryable.
    pub fn shutdown_durable(
        &self,
        journal: &FjallJournal,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        self.close_admission()?;
        self.drain_durable(journal)
    }

    fn close_admission(&self) -> Result<(), JournalError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        state.shutdown = true;
        Ok(())
    }

    // Admission is closed; every nonempty flush removes at least one item.
    // Capacity plus one includes the final empty observation.
    fn drain_durable(
        &self,
        journal: &FjallJournal,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        let mut total = JournalWriterFlushReport {
            drained: 0,
            written: 0,
        };
        for _ in 0..self.capacity.saturating_add(1) {
            let report = self.flush_batch_durable(journal)?;
            if report.drained == 0 {
                return Ok(total);
            }
            total.drained = total.drained.saturating_add(report.drained);
            total.written = total.written.saturating_add(report.written);
        }
        Err(JournalError::QueueFull)
    }
}
