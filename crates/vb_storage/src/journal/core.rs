#![forbid(unsafe_code)]
//! Fjall-backed journal implementation.
//!
//! Provides the main storage interface for workflow artifacts,
//! run metadata, journal events, snapshots, and blobs.

use std::path::Path;
use std::sync::Mutex;

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
    #[allow(dead_code)]
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
}

impl Drop for FjallJournal {
    fn drop(&mut self) {
        // Drop cannot propagate errors from close(), so callers who need
        // guaranteed durability must call close() explicitly.
        // Drop only releases the process lock here.
    }
}