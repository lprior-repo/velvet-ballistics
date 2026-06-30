#![forbid(unsafe_code)]
//! Fjall-backed journal implementation.
//!
//! Provides the main storage interface for workflow artifacts,
//! run metadata, journal events, snapshots, and blobs.

use std::path::Path;
use std::sync::Mutex;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    constants::{
        KEYSPACE_BLOB, KEYSPACE_COMPILED_IR, KEYSPACE_INDEX_ACTION, KEYSPACE_INDEX_STATUS,
        KEYSPACE_INDEX_WORKFLOW, KEYSPACE_RUN_EVENT, KEYSPACE_RUN_HEADER, KEYSPACE_RUN_SNAPSHOT,
        KEYSPACE_WORKFLOW_SOURCE,
    },
    error::JournalError,
    process_lock::ProcessLock,
    types::{FjallConfig, KeyspaceProfile},
};

/// Bounded replay limit for journal event collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventReplayLimit {
    max_events: usize,
}

impl EventReplayLimit {
    /// Conservative default for operator-facing replay collection.
    pub const DEFAULT: Self = Self { max_events: 65_536 };

    /// Creates a replay limit; returns `None` when the limit is zero.
    #[must_use]
    pub const fn new(max_events: usize) -> Option<Self> {
        if max_events == 0 {
            None
        } else {
            Some(Self { max_events })
        }
    }

    /// Returns the maximum number of events that may be collected.
    #[must_use]
    pub const fn max_events(self) -> usize {
        self.max_events
    }
}

/// Fjall-backed append journal.
pub struct FjallJournal {
    pub(crate) database: fjall::Database,
    pub(crate) workflow_source: fjall::Keyspace,
    pub(crate) compiled_ir: fjall::Keyspace,
    pub(crate) run_header: fjall::Keyspace,
    pub(crate) events: fjall::Keyspace,
    pub(crate) run_snapshot: fjall::Keyspace,
    pub(crate) blob: fjall::Keyspace,
    pub(crate) index_status: fjall::Keyspace,
    pub(crate) index_workflow: fjall::Keyspace,
    pub(crate) index_action: fjall::Keyspace,
    #[cfg(test)]
    pub(crate) fail_next_persist: AtomicBool,
    #[cfg(test)]
    pub(crate) fail_next_compiled_ir_readback: AtomicBool,
    // SAFETY: write_lock is used in append_unfsynced() for poison detection.
    // The lock guard is acquired and dropped, never read directly.
    pub(crate) write_lock: Mutex<()>,
    pub(crate) _process_lock: ProcessLock,
}

impl FjallJournal {
    /// Opens or creates the journal at `path`.
    pub fn open(path: impl AsRef<Path>, config: Option<FjallConfig>) -> Result<Self, JournalError> {
        let config = config.unwrap_or_default();
        let path_ref = path.as_ref();
        let database = fjall::Database::builder(path_ref)
            .cache_size(config.cache_size_bytes)
            .open()?;

        let workflow_source = database.keyspace(KEYSPACE_WORKFLOW_SOURCE, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Cold)
        })?;
        let compiled_ir = database.keyspace(KEYSPACE_COMPILED_IR, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Cold)
        })?;
        let run_header = database.keyspace(KEYSPACE_RUN_HEADER, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Hot)
        })?;
        let events = database.keyspace(KEYSPACE_RUN_EVENT, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Hot)
        })?;
        let run_snapshot = database.keyspace(KEYSPACE_RUN_SNAPSHOT, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Cold)
        })?;
        let blob = database.keyspace(KEYSPACE_BLOB, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Blob)
        })?;
        let index_status = database.keyspace(KEYSPACE_INDEX_STATUS, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Hot)
        })?;
        let index_workflow = database.keyspace(KEYSPACE_INDEX_WORKFLOW, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Hot)
        })?;
        let index_action = database.keyspace(KEYSPACE_INDEX_ACTION, || {
            crate::types::keyspace_options_for(KeyspaceProfile::Hot)
        })?;
        let _process_lock = ProcessLock::acquire(path_ref)?;
        Ok(Self {
            database,
            workflow_source,
            compiled_ir,
            run_header,
            events,
            run_snapshot,
            blob,
            index_status,
            index_workflow,
            index_action,
            #[cfg(test)]
            fail_next_persist: AtomicBool::new(false),
            #[cfg(test)]
            fail_next_compiled_ir_readback: AtomicBool::new(false),
            write_lock: Mutex::new(()),
            _process_lock,
        })
    }

    /// Returns all declared keyspace names after a successful open.
    #[must_use]
    pub const fn declared_keyspaces() -> [&'static str; 9] {
        [
            KEYSPACE_WORKFLOW_SOURCE,
            KEYSPACE_COMPILED_IR,
            KEYSPACE_RUN_HEADER,
            KEYSPACE_RUN_EVENT,
            KEYSPACE_RUN_SNAPSHOT,
            KEYSPACE_BLOB,
            KEYSPACE_INDEX_STATUS,
            KEYSPACE_INDEX_WORKFLOW,
            KEYSPACE_INDEX_ACTION,
        ]
    }

    /// Returns whether the action index contains an entry for the given key.
    ///
    /// This is a public query API to support external verification of index writes
    /// from outside the `vb_storage` crate (e.g., integration tests).
    ///
    /// # Errors
    ///
    /// Returns `JournalError` if the underlying keyspace query fails.
    pub fn has_action_index_entry(&self, key: impl AsRef<[u8]>) -> Result<bool, JournalError> {
        Ok(self.index_action.contains_key(key.as_ref())?)
    }

    /// Returns whether the status index contains an entry for the given key.
    ///
    /// This is a public query API to support external verification of index writes
    /// from outside the `vb_storage` crate (e.g., integration tests).
    ///
    /// # Errors
    ///
    /// Returns `JournalError` if the underlying keyspace query fails.
    pub fn has_status_index_entry(&self, key: impl AsRef<[u8]>) -> Result<bool, JournalError> {
        Ok(self.index_status.contains_key(key.as_ref())?)
    }

    /// Returns whether the workflow index contains an entry for the given key.
    ///
    /// This is a public query API to support external verification of index writes
    /// from outside the `vb_storage` crate (e.g., integration tests).
    ///
    /// # Errors
    ///
    /// Returns `JournalError` if the underlying keyspace query fails.
    pub fn has_workflow_index_entry(&self, key: impl AsRef<[u8]>) -> Result<bool, JournalError> {
        Ok(self.index_workflow.contains_key(key.as_ref())?)
    }

    /// Performs a read-only health probe across the opened storage keyspaces.
    ///
    /// # Errors
    ///
    /// Returns `JournalError` if any underlying keyspace cannot be read.
    pub fn probe_health(&self) -> Result<(), JournalError> {
        let empty_key: &[u8] = &[];
        let _ = self.workflow_source.contains_key(empty_key)?;
        let _ = self.compiled_ir.contains_key(empty_key)?;
        let _ = self.run_header.contains_key(empty_key)?;
        let _ = self.events.contains_key(empty_key)?;
        let _ = self.run_snapshot.contains_key(empty_key)?;
        let _ = self.blob.contains_key(empty_key)?;
        let _ = self.index_status.contains_key(empty_key)?;
        let _ = self.index_workflow.contains_key(empty_key)?;
        let _ = self.index_action.contains_key(empty_key)?;
        Ok(())
    }

    /// Closes the journal, forcing a strict durability barrier before ownership is released.
    ///
    /// This method **must** be called explicitly by callers who require durable persistence.
    /// Drop does NOT call `close()` — it only releases the process lock.
    /// Errors from `close()` cannot be propagated through Drop, so callers who need
    /// fail-closed behavior must invoke this method and handle the result.
    ///
    /// # Errors
    ///
    /// Returns `JournalError` if the underlying storage fails to persist.
    pub fn close(&mut self) -> Result<(), JournalError> {
        self.persist_strict()
    }

    #[cfg(test)]
    pub(crate) fn fail_next_persist_for_test(&self) {
        self.fail_next_persist.store(true, Ordering::SeqCst);
    }

    #[cfg(test)]
    pub(crate) fn consume_persist_failure_for_test(&self) -> bool {
        self.fail_next_persist.swap(false, Ordering::SeqCst)
    }

    /// Test-only hook: forces the next `compiled_ir` readback to return
    /// `Ok(None)`, simulating a silent persistence failure in which the
    /// `put_compiled_ir` insert succeeded but the value vanished from the
    /// LSM by the time the readback runs.
    #[cfg(test)]
    pub(crate) fn fail_next_compiled_ir_readback_for_test(&self) {
        self.fail_next_compiled_ir_readback
            .store(true, Ordering::SeqCst);
    }

    #[cfg(test)]
    pub(crate) fn consume_compiled_ir_readback_failure_for_test(&self) -> bool {
        self.fail_next_compiled_ir_readback
            .swap(false, Ordering::SeqCst)
    }
}

impl Drop for FjallJournal {
    fn drop(&mut self) {
        // Drop cannot propagate errors from close(), so callers who need
        // guaranteed durability must call close() explicitly.
        // Drop only releases the process lock here.
    }
}
