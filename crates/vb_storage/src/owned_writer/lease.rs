use super::types::Key;
use super::{OwnedWriterError, OwnedWriterQuarantine};
use crate::FjallJournal;
use std::sync::Arc;

/// Non-Clone capability retaining its actual journal until release or Drop.
/// No public field constructor or deserializer can fabricate an owner.
pub struct OwnedWriterLease {
    journal: Arc<FjallJournal>,
    pub(super) key: Key,
    drop_checked: bool,
}

impl OwnedWriterLease {
    pub(super) fn new(journal: Arc<FjallJournal>, key: Key) -> Self {
        Self {
            journal,
            key,
            drop_checked: false,
        }
    }

    pub(super) fn journal(&self) -> &Arc<FjallJournal> {
        &self.journal
    }

    /// Actual owning run identifier.
    pub fn run(&self) -> vb_core::RunId {
        self.key.run
    }

    /// Checked store-lifetime generation; never reset by healthy run retirement.
    pub fn generation(&self) -> u64 {
        self.key.generation.get()
    }

    /// Publishes sticky uncertainty directly in the actual store-owned entry.
    /// Repeated calls preserve and return the first uncertainty reason.
    pub fn quarantine(
        &self,
        reason: OwnedWriterQuarantine,
    ) -> Result<OwnedWriterQuarantine, OwnedWriterError> {
        self.journal.owned_writers.quarantine(self.key, reason)
    }

    /// Makes exactly one nonblocking healthy release attempt. A refusal retains
    /// the entry; Drop does not silently retry or clear quarantined ownership.
    pub fn try_release(mut self) -> Result<(), OwnedWriterError> {
        self.drop_checked = true;
        self.journal.owned_writers.release(self.key)
    }
}

impl core::fmt::Debug for OwnedWriterLease {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("OwnedWriterLease")
            .field("run", &self.key.run)
            .field("generation", &self.key.generation)
            .finish_non_exhaustive()
    }
}

impl Drop for OwnedWriterLease {
    fn drop(&mut self) {
        if !self.drop_checked {
            self.drop_checked = true;
            self.journal.owned_writers.release_on_drop(self.key);
        }
    }
}
