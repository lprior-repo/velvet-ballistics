#![forbid(unsafe_code)]
//! Recovery-stamp storage operations.
//!
//! Provides storage and retrieval of `RecoveryStampRecord` values
//! (master contract §18, wire ID 7, magic `MAGIC_RECOVERY_STAMP`).
//!
//! A recovery stamp is a small fixed-shape record written by the recovery
//! path to mark how far replay has progressed for a given run. The fields
//! are intentionally compact and bounded so a recovery stamp fits
//! comfortably in `MAX_RECOVERY_STAMP_BYTES` and decodes without allocation
//! beyond the postcard payload buffer.

use crate::{
    codec::encode_record,
    constants::{MAGIC_RECOVERY_STAMP, MAX_RECOVERY_STAMP_BYTES},
    error::JournalError,
    keys::recovery_stamp_key,
    records::RecoveryStampRecord,
    types::EventSeq,
};

use crate::journal::FjallJournal;

impl FjallJournal {
    /// Persists a recovery-stamp record for the given `(run, seq)` pair.
    ///
    /// The encoded envelope uses `MAGIC_RECOVERY_STAMP` and the dedicated
    /// `recovery_stamp` keyspace; the key format is `[0x40][run_id][seq]`.
    pub fn put_recovery_stamp(
        &self,
        run: vb_core::RunId,
        seq: EventSeq,
        stamp: RecoveryStampRecord,
    ) -> Result<(), JournalError> {
        let key = recovery_stamp_key(run, seq)?;
        let value = encode_record(
            MAGIC_RECOVERY_STAMP,
            crate::records::RecordKind::RecoveryStamp,
            seq.get(),
            &stamp,
            MAX_RECOVERY_STAMP_BYTES,
        )?;
        self.recovery_stamp.insert(key.to_vec(), value)?;
        Ok(())
    }

    /// Loads a previously persisted recovery-stamp record for `(run, seq)`.
    ///
    /// Returns `Ok(None)` if no record has been written for the given key.
    pub fn get_recovery_stamp(
        &self,
        run: vb_core::RunId,
        seq: EventSeq,
    ) -> Result<Option<RecoveryStampRecord>, JournalError> {
        let key = recovery_stamp_key(run, seq)?;
        self.decode_optional(
            &self.recovery_stamp,
            key.as_slice(),
            MAGIC_RECOVERY_STAMP,
            MAX_RECOVERY_STAMP_BYTES,
        )
    }
}
