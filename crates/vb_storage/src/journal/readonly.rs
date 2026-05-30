#![forbid(unsafe_code)]
//! Read-only journal wrapper.
//!
//! `ReadOnlyJournal` wraps `FjallJournal` and exposes only its read methods.
//! The inner journal is private and cannot be accessed mutably through the
//! wrapper. Rust's type system enforces this at compile time;
//! the crate also uses `#![forbid(unsafe_code)]` to prevent circumvention.

use crate::journal::core::FjallJournal;
use crate::error::JournalError;
use crate::events::JournalEvent;
use vb_core::RunId;

/// A newtype wrapper that exposes only read methods of the underlying journal.
///
/// The inner `FjallJournal` is private. External modules cannot access it
/// directly. All public methods take `&self` (shared reference), preventing
/// mutation through this wrapper.
///
/// Write methods (`append_journaled`, `persist_strict`, `put_workflow_source`,
/// `put_compiled_ir`, `put_run_header`, `put_snapshot`, `put_blob`) are NOT
/// exposed through this wrapper.
pub struct ReadOnlyJournal(pub(crate) FjallJournal);

impl core::fmt::Debug for ReadOnlyJournal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ReadOnlyJournal").finish_non_exhaustive()
    }
}

impl ReadOnlyJournal {
    /// Wraps an existing `FjallJournal`.
    #[must_use]
    pub(crate) fn new(inner: FjallJournal) -> Self {
        Self(inner)
    }

    /// Opens a journal in read-only mode.
    ///
    /// This is just the normal open — the read-only guarantee comes from
    /// the wrapper type, not from the filesystem.
    pub fn open_read_only(
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, JournalError> {
        let journal = FjallJournal::open(path, None)?;
        Ok(Self(journal))
    }

    /// Returns all declared keyspace names.
    #[must_use]
    pub const fn declared_keyspaces() -> [&'static str; 9] {
        FjallJournal::declared_keyspaces()
    }

    /// Replays events for a single run, returning them in sequence order.
    pub fn events_for_run(&self, run: RunId) -> Result<Vec<JournalEvent>, JournalError> {
        self.0.events_for_run(run)
    }

    /// Reads a stored blob by digest.
    pub fn blob(
        &self,
        digest: [u8; crate::constants::DIGEST_BYTES],
    ) -> Result<Option<crate::records::BlobRecord>, JournalError> {
        self.0.blob(digest)
    }

    /// Returns whether the action index contains an entry for the given key.
    pub fn has_action_index_entry(
        &self,
        key: impl AsRef<[u8]>,
    ) -> Result<bool, JournalError> {
        self.0.has_action_index_entry(key)
    }

    /// Returns whether the status index contains an entry for the given key.
    pub fn has_status_index_entry(
        &self,
        key: impl AsRef<[u8]>,
    ) -> Result<bool, JournalError> {
        self.0.has_status_index_entry(key)
    }

    /// Returns whether the workflow index contains an entry for the given key.
    pub fn has_workflow_index_entry(
        &self,
        key: impl AsRef<[u8]>,
    ) -> Result<bool, JournalError> {
        self.0.has_workflow_index_entry(key)
    }
}
