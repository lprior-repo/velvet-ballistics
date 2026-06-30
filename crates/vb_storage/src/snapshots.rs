#![forbid(unsafe_code)]
//! Snapshot storage operations.
//!
//! Provides storage and retrieval of compact run snapshots.

use crate::{
    codec::encode_record,
    constants::{MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES},
    error::JournalError,
    keys::run_snapshot_key,
    recovery::RunSnapshot,
    types::EventSeq,
};

use crate::journal::FjallJournal;

impl FjallJournal {
    /// Stores a compact run snapshot with a strict durability barrier.
    ///
    /// The snapshot key becomes visible to readers only after the WAL fsync
    /// has returned, so `latest_durable_snapshot_seq` cannot name a snapshot
    /// that would not survive a crash immediately after this call returns.
    ///
    /// Implementation: routes through a `fjall::OwnedWriteBatch` so the
    /// insert and the `PersistMode::SyncAll` barrier commit atomically.
    /// Without this barrier, `latest_durable_snapshot_seq` would observe a
    /// snapshot that the Fjall memtable has accepted but the WAL has not
    /// yet flushed, and a subsequent trim would delete pre-snapshot events
    /// that the snapshot was supposed to cover — violating crash-consistency
    /// (master §49 Crash-Consistency Rule).
    pub fn put_snapshot(&self, snapshot: &RunSnapshot) -> Result<(), JournalError> {
        let key = run_snapshot_key(snapshot.run, snapshot.seq)?;
        let value = encode_record(
            MAGIC_SNAPSHOT,
            crate::records::RecordKind::Snapshot,
            snapshot.seq.get(),
            snapshot,
            MAX_SNAPSHOT_BYTES,
        )?;
        let mut batch = self.database.batch();
        batch.insert(&self.run_snapshot, key.to_vec(), value);
        let batch = batch.durability(Some(fjall::PersistMode::SyncAll));
        batch.commit()?;
        Ok(())
    }

    /// Loads a compact run snapshot.
    pub fn snapshot(
        &self,
        run: vb_core::RunId,
        seq: EventSeq,
    ) -> Result<Option<RunSnapshot>, JournalError> {
        let key = run_snapshot_key(run, seq)?;
        self.decode_optional(
            &self.run_snapshot,
            key.as_slice(),
            MAGIC_SNAPSHOT,
            MAX_SNAPSHOT_BYTES,
        )
    }
}
