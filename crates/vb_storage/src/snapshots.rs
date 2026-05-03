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
    /// Stores a compact run snapshot.
    pub fn put_snapshot(&self, snapshot: &RunSnapshot) -> Result<(), JournalError> {
        let key = run_snapshot_key(snapshot.run, snapshot.seq)?;
        let value = encode_record(
            MAGIC_SNAPSHOT,
            crate::records::RecordKind::Snapshot,
            snapshot.seq.get(),
            snapshot,
            MAX_SNAPSHOT_BYTES,
        )?;
        self.run_snapshot.insert(key.to_vec(), value)?;
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
