#![forbid(unsafe_code)]
//! Inspect-view journal facade.
//!
//! `InspectView` wraps `FjallJournal` and exposes only its read methods.
//! The inner journal is private and cannot be accessed mutably through the
//! wrapper. Rust's type system enforces this at compile time;
//! the crate also uses `#![forbid(unsafe_code)]` to prevent circumvention.
//!
//! **Not a true read-only open.** Fjall does not expose a read-only
//! `Database::open` mode. Every open performs LSM-tree recovery and
//! acquires the process-level write lock (see `ProcessLock::acquire`).
//! When another writer holds that lock, [`InspectView::open_inspect_view`]
//! returns [`JournalError::ProcessLockHeld`].

use crate::error::JournalError;
use crate::events::JournalEvent;
use crate::journal::core::FjallJournal;
use vb_core::RunId;

/// An inspect-view wrapper that exposes only read methods of the underlying journal.
///
/// The inner `FjallJournal` is private. External modules cannot access it
/// directly. All public methods take `&self` (shared reference), preventing
/// mutation through this wrapper.
///
/// Write methods (`append_journaled`, `persist_strict`, `put_workflow_source`,
/// `put_compiled_ir`, `put_run_header`, `put_snapshot`, `put_blob`) are NOT
/// exposed through this wrapper.
///
/// # Open semantics
///
/// [`InspectView::open_inspect_view`] opens the Fjall database at `path` and
/// acquires the process-level write lock, just like a normal writer open.
/// The read-only guarantee is purely at the type level: callers cannot call
/// any write method through this facade.
pub struct InspectView(pub(crate) FjallJournal);

impl core::fmt::Debug for InspectView {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InspectView").finish_non_exhaustive()
    }
}

impl InspectView {
    /// Wraps an existing `FjallJournal`.
    #[must_use]
    #[expect(dead_code)]
    pub(crate) fn new(inner: FjallJournal) -> Self {
        Self(inner)
    }

    /// Opens an inspect-view at `path`.
    ///
    /// **Not a true read-only open.** Fjall does not expose a read-only
    /// `Database::open` mode; every open performs LSM-tree recovery and
    /// acquires the process-level write lock (see `ProcessLock::acquire`).
    /// When another writer currently holds that lock, this call returns
    /// [`JournalError::ProcessLockHeld`].
    ///
    /// The wrapper type enforces a read-only surface: callers cannot
    /// invoke any write method through `InspectView`. If you need to open
    /// this without contention, close the writer first; after drop the
    /// lock is released and a follow-up call succeeds with the writer's
    /// eventual consistency visible.
    pub fn open_inspect_view(path: impl AsRef<std::path::Path>) -> Result<Self, JournalError> {
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
    pub fn has_action_index_entry(&self, key: impl AsRef<[u8]>) -> Result<bool, JournalError> {
        self.0.has_action_index_entry(key)
    }

    /// Returns whether the status index contains an entry for the given key.
    pub fn has_status_index_entry(&self, key: impl AsRef<[u8]>) -> Result<bool, JournalError> {
        self.0.has_status_index_entry(key)
    }

    /// Returns whether the workflow index contains an entry for the given key.
    pub fn has_workflow_index_entry(&self, key: impl AsRef<[u8]>) -> Result<bool, JournalError> {
        self.0.has_workflow_index_entry(key)
    }
}
