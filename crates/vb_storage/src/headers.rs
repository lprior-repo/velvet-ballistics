#![forbid(unsafe_code)]
//! Run header storage operations.
//!
//! Provides storage and retrieval of run metadata records.

use crate::{
    codec::decode_record,
    constants::{MAGIC_INDEX_RECORD, PREFIX_RUN_HEADER},
    error::JournalError,
    keys::run_header_key,
    records::RunHeaderRecord,
};

use crate::journal::FjallJournal;

impl FjallJournal {
    /// Stores run metadata by run id.
    pub fn put_run_header(&self, record: &RunHeaderRecord) -> Result<(), JournalError> {
        let key = run_header_key(record.run)?;
        let value = crate::codec::encode_record(
            MAGIC_INDEX_RECORD,
            crate::records::RecordKind::RunHeader,
            record.run.get(),
            record,
            crate::constants::MAX_RUN_HEADER_BYTES,
        )?;
        self.run_header.insert(key.to_vec(), value)?;
        self.persist_strict()?;
        Ok(())
    }

    /// Loads run metadata by run id.
    ///
    /// Returns `Err(InvalidRunId)` if `run` is zero, as zero is not a valid run identifier
    /// per the storage contract.
    pub fn run_header(&self, run: vb_core::RunId) -> Result<Option<RunHeaderRecord>, JournalError> {
        if run.get() == 0 {
            return Err(JournalError::InvalidRunId { run });
        }
        let key = run_header_key(run)?;
        self.decode_optional(
            &self.run_header,
            key.as_slice(),
            MAGIC_INDEX_RECORD,
            crate::constants::MAX_RUN_HEADER_BYTES,
        )
    }

    /// Loads all run metadata records in key order.
    ///
    /// Scans the run-header keyspace with
    /// [`KeyspaceScanPolicy::FailClosed`](crate::keys::KeyspaceScanPolicy::FailClosed)
    /// (the production default): if a row's key is not exactly
    /// `RUN_ONLY_KEY_BYTES` long, the scan aborts and surfaces
    /// [`JournalError::MalformedKeyspaceRow`] instead of silently
    /// dropping the row or producing an inconsistent `headers` vector.
    pub fn run_headers(&self) -> Result<Vec<RunHeaderRecord>, JournalError> {
        let mut headers = Vec::new();
        let prefix = [PREFIX_RUN_HEADER];
        for item in self.run_header.prefix(prefix) {
            let (raw_key, value) = item.into_inner()?;
            let key_len = raw_key.len();
            if key_len != crate::constants::RUN_ONLY_KEY_BYTES {
                return Err(JournalError::MalformedKeyspaceRow {
                    prefix: PREFIX_RUN_HEADER,
                    expected_len: crate::constants::RUN_ONLY_KEY_BYTES,
                    actual_len: key_len,
                });
            }
            let (_, header) = decode_record(
                value.as_ref(),
                MAGIC_INDEX_RECORD,
                crate::constants::MAX_RUN_HEADER_BYTES,
            )?;
            headers.push(header);
        }
        Ok(headers)
    }
}
