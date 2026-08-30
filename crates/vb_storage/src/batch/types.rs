#![forbid(unsafe_code)]
use std::collections::HashSet;

use crate::constants::JOURNAL_KEY_BYTES;
use crate::journal::FjallJournal;

/// Default journal batch encoded-byte budget (1 MiB).
///
/// Matches the core `max_journal_batch_bytes` default of `1_048_576`.
pub const DEFAULT_JOURNAL_BATCH_BYTE_LIMIT: u64 = 1_048_576;

/// Atomic cross-keyspace write batch backed by Fjall.
///
/// Accumulates writes across multiple keyspaces and commits them
/// atomically with a single WAL fsync.
///
/// # Invariant I1
/// `JournalWriteBatch` is `!Send + !Sync` because it contains
/// `PhantomData<*mut FjallJournal>` which is `!Send + !Sync`,
/// preventing any batch handle from crossing thread boundaries.
pub struct JournalWriteBatch<'j> {
    pub(super) inner: fjall::OwnedWriteBatch,
    pub(super) journal: &'j FjallJournal,
    pub(super) staged_event_keys: HashSet<[u8; JOURNAL_KEY_BYTES]>,
    pub(super) aborted: bool,
    pub(super) staged_bytes: u64,
    pub(super) byte_limit: Option<u64>,
    pub(super) _not_send_or_sync: core::marker::PhantomData<*mut FjallJournal>,
}

impl<'j> JournalWriteBatch<'j> {
    /// Creates a new batch for the given journal.
    pub fn new(journal: &'j FjallJournal) -> Self {
        Self {
            inner: journal.database.batch(),
            journal,
            staged_event_keys: HashSet::new(),
            aborted: false,
            staged_bytes: 0,
            byte_limit: Some(DEFAULT_JOURNAL_BATCH_BYTE_LIMIT),
            _not_send_or_sync: core::marker::PhantomData,
        }
    }

    /// Returns the number of operations in the batch.
    #[must_use]
    pub fn len(&self) -> usize {
        if self.aborted { 0 } else { self.inner.len() }
    }

    /// Returns true if the batch contains no operations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if the batch is in the aborted state.
    ///
    /// A batch becomes aborted when a fallible step sets
    /// `self.aborted = true` before propagating a typed error
    /// (`DuplicateEvent`, `KeyCapacity`, `PayloadTooLarge`, etc.).
    /// The `commit()` short-circuit then refuses to persist the
    /// partial batch. This accessor is the canonical way to
    /// distinguish an aborted batch from a fresh empty batch (both
    /// have `len() == 0`).
    #[must_use]
    pub fn is_aborted(&self) -> bool {
        self.aborted
    }

    /// Returns the accumulated encoded-byte total for journal events
    /// accepted in this batch so far.
    #[must_use]
    pub fn staged_event_bytes(&self) -> u64 {
        self.staged_bytes
    }

    /// Returns the byte limit for this batch, if one is set.
    #[must_use]
    pub fn byte_limit(&self) -> Option<u64> {
        self.byte_limit
    }
}
