use std::sync::Arc;
use vb_storage::{DurabilityProfile, FjallJournal, JournalWriterFlushReport, JournalWriterQueue};

use crate::{RuntimeError, RuntimeResult, journal::SharedRuntimeJournal};

/// Queued journal adapter that batches writes through `JournalWriterQueue`.
pub(crate) struct QueuedStorageRuntimeJournal {
    journal: Arc<FjallJournal>,
    queue: Arc<JournalWriterQueue>,
    profile: DurabilityProfile,
}

impl QueuedStorageRuntimeJournal {
    /// Creates a queued adapter that enqueues journaled requests.
    #[must_use]
    pub(crate) fn journaled(journal: Arc<FjallJournal>, queue: Arc<JournalWriterQueue>) -> Self {
        Self {
            journal,
            queue,
            profile: DurabilityProfile::Journaled,
        }
    }

    /// Creates a queued adapter that enqueues strict requests.
    #[must_use]
    pub(crate) fn strict(journal: Arc<FjallJournal>, queue: Arc<JournalWriterQueue>) -> Self {
        Self {
            journal,
            queue,
            profile: DurabilityProfile::Strict,
        }
    }

    /// Creates a shared queued journaled adapter for direct runtime construction.
    #[must_use]
    pub(crate) fn shared_journaled(
        journal: Arc<FjallJournal>,
        queue: Arc<JournalWriterQueue>,
    ) -> SharedRuntimeJournal {
        Arc::new(Self::journaled(journal, queue))
    }

    /// Creates a shared queued strict adapter for direct runtime construction.
    #[must_use]
    pub(crate) fn shared_strict(
        journal: Arc<FjallJournal>,
        queue: Arc<JournalWriterQueue>,
    ) -> SharedRuntimeJournal {
        Arc::new(Self::strict(journal, queue))
    }

    /// Flushes a bounded batch from the queue into Fjall.
    pub(crate) fn flush_batch(&self) -> RuntimeResult<JournalWriterFlushReport> {
        self.queue
            .flush_batch(&self.journal)
            .map_err(RuntimeError::from)
    }

    /// Drains all queued journal writes into Fjall.
    pub(crate) fn drain_all(&self) -> RuntimeResult<JournalWriterFlushReport> {
        self.queue
            .drain_all(&self.journal)
            .map_err(RuntimeError::from)
    }

    /// Returns the durability profile.
    #[must_use]
    pub(crate) fn profile(&self) -> DurabilityProfile {
        self.profile
    }

    /// Returns the journal writer queue.
    #[must_use]
    pub(crate) fn queue(&self) -> &Arc<JournalWriterQueue> {
        &self.queue
    }

    /// Returns the underlying Fjall journal.
    #[must_use]
    pub(crate) fn journal(&self) -> &Arc<FjallJournal> {
        &self.journal
    }
}
