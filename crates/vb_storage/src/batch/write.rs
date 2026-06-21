//! Atomic cross-keyspace batch state and lifecycle methods.

use super::types::{BatchByteLimit, BatchState, DEFAULT_JOURNAL_BATCH_BYTE_LIMIT};
use crate::{error::JournalError, journal::FjallJournal};

/// Atomic cross-keyspace write batch backed by Fjall.
///
/// Accumulates writes across multiple keyspaces and commits them atomically with
/// a single WAL fsync.
///
/// # Invariant I1
/// `JournalWriteBatch` is `!Send + !Sync` because it contains
/// `PhantomData<*mut FjallJournal>` which is `!Send + !Sync`, preventing any
/// batch handle from crossing thread boundaries.
pub struct JournalWriteBatch<'j> {
    pub(super) inner: fjall::OwnedWriteBatch,
    pub(super) journal: &'j FjallJournal,
    pub(super) staged_event_keys: std::collections::BTreeSet<[u8; crate::constants::JOURNAL_KEY_BYTES]>,
    #[cfg(test)]
    pub(super) staged_ir_hashes: std::collections::HashMap<vb_core::WorkflowDigest, [u8; 32]>,
    pub(super) state: BatchState,
    pub(super) staged_bytes: u64,
    pub(super) byte_limit: BatchByteLimit,
    _not_send_or_sync: core::marker::PhantomData<*mut FjallJournal>,
}

impl<'j> JournalWriteBatch<'j> {
    /// Creates a new batch for the given journal.
    pub fn new(journal: &'j FjallJournal) -> Self {
        Self {
            inner: journal.database.batch(),
            journal,
            staged_event_keys: std::collections::BTreeSet::new(),
            #[cfg(test)]
            staged_ir_hashes: std::collections::HashMap::new(),
            state: BatchState::default(),
            staged_bytes: 0,
            byte_limit: BatchByteLimit::bounded(DEFAULT_JOURNAL_BATCH_BYTE_LIMIT),
            _not_send_or_sync: core::marker::PhantomData,
        }
    }

    /// Returns the number of operations in the batch.
    #[must_use]
    pub fn len(&self) -> usize {
        if self.state.is_aborted() {
            0
        } else {
            self.inner.len()
        }
    }

    /// Returns true if the batch contains no operations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the accumulated encoded-byte total for journal events.
    #[must_use]
    pub fn staged_event_bytes(&self) -> u64 {
        self.staged_bytes
    }

    /// Returns the byte limit for this batch, if one is set.
    #[must_use]
    pub fn byte_limit(&self) -> Option<u64> {
        let limit = self.byte_limit.as_u64();
        if limit > 0 { Some(limit) } else { None }
    }

    /// Sets strict durability for the commit.
    pub fn strict(mut self) -> Self {
        self.inner = self.inner.durability(Some(fjall::PersistMode::SyncAll));
        self
    }

    /// Commits the batch atomically.
    pub fn commit(self) -> Result<(), JournalError> {
        if self.state.is_aborted() {
            return Ok(());
        }
        self.inner.commit()?;
        Ok(())
    }
}
