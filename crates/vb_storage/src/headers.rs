#![forbid(unsafe_code)]
//! Run header storage operations.
//!
//! Provides storage and retrieval of run metadata records.

use crate::{
    codec::decode_record,
    constants::{MAGIC_INDEX_RECORD, PREFIX_RUN_HEADER, RUN_ONLY_KEY_BYTES},
    error::JournalError,
    keys::{decode_storage_key, run_header_key},
    records::RunHeaderRecord,
    types::{RecordEnvelope, StorageKey},
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
        self.decode_optional_with(
            &self.run_header,
            key.as_slice(),
            MAGIC_INDEX_RECORD,
            crate::constants::MAX_RUN_HEADER_BYTES,
            |envelope, record| validate_run_header_read(envelope, run, record),
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
        // CC-003 capacity hint: 16 is a Holzmann-Rust bounded knowledge
        // estimate for the typical case (per-workflow running set of
        // active runs). The Vec grows via standard doubling if more
        // headers exist, but this avoids the initial 0->4 doubling.
        let mut headers = Vec::with_capacity(16);
        let prefix = [PREFIX_RUN_HEADER];
        for item in self.run_header.prefix(prefix) {
            let (raw_key, value) = item.into_inner()?;
            let run = run_from_header_key(raw_key.as_ref())?;
            let (envelope, header) = decode_record(
                value.as_ref(),
                MAGIC_INDEX_RECORD,
                crate::constants::MAX_RUN_HEADER_BYTES,
            )?;
            validate_run_header_read(&envelope, run, &header)?;
            headers.push(header);
        }
        Ok(headers)
    }
}

fn validate_run_header_read(
    envelope: &RecordEnvelope,
    run: vb_core::RunId,
    record: &RunHeaderRecord,
) -> Result<(), JournalError> {
    if record.run != run {
        return Err(JournalError::WrongRun {
            expected: run,
            actual: record.run,
        });
    }
    if envelope.sequence != run.get() {
        return Err(JournalError::ReplayEnvelopeSequenceMismatch {
            run,
            envelope_seq: envelope.sequence,
            payload_seq: run.get(),
        });
    }
    Ok(())
}

fn run_from_header_key(key: &[u8]) -> Result<vb_core::RunId, JournalError> {
    match decode_storage_key(key) {
        Ok(StorageKey::RunHeader { run }) => Ok(run),
        Ok(_) | Err(_) => Err(JournalError::MalformedKeyspaceRow {
            prefix: PREFIX_RUN_HEADER,
            expected_len: RUN_ONLY_KEY_BYTES,
            actual_len: key.len(),
        }),
    }
}
