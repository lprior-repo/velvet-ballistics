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

/// Failure of a complete bounded run-header scan or its visitor.
#[derive(Debug, thiserror::Error)]
pub enum RunHeaderScanError<E: std::error::Error + 'static> {
    /// The limit cannot admit a bounded overflow probe.
    #[error("invalid bounded run-header scan limit")]
    InvalidLimit,
    /// More headers exist than the scan permits.
    #[error("run-header scan exceeds limit {limit}")]
    LimitExceeded {
        /// Configured maximum number of headers.
        limit: usize,
    },
    /// A key, envelope or storage operation failed.
    #[error("run-header scan failed: {0}")]
    Storage(#[source] JournalError),
    /// The visitor refused a validated header.
    #[error("run-header visitor failed: {0}")]
    Visitor(#[source] E),
}

impl FjallJournal {
    /// Visits a complete bounded snapshot in key order after validating every row.
    ///
    /// Overflow or malformed rows fail before any visitor runs. Each pass decodes
    /// one record at a time from the same pinned snapshot.
    ///
    /// # Examples
    /// ```
    /// fn scan(journal: &vb_storage::FjallJournal) -> Result<(), vb_storage::headers::RunHeaderScanError<vb_storage::JournalError>> {
    ///     journal.try_for_each_run_header_bounded(std::num::NonZeroUsize::MIN, |_header| Ok(()))
    /// }
    /// ```
    pub fn try_for_each_run_header_bounded<E, F>(
        &self,
        limit: std::num::NonZeroUsize,
        mut visit: F,
    ) -> Result<(), RunHeaderScanError<E>>
    where
        E: std::error::Error + 'static,
        F: FnMut(RunHeaderRecord) -> Result<(), E>,
    {
        use fjall::Readable;
        let probe = limit
            .get()
            .checked_add(1)
            .ok_or(RunHeaderScanError::InvalidLimit)?;
        let snapshot = self.database.snapshot();
        let mut count = 0usize;
        for item in snapshot
            .prefix(&self.run_header, [PREFIX_RUN_HEADER])
            .take(probe)
        {
            count = count
                .checked_add(1)
                .ok_or(RunHeaderScanError::InvalidLimit)?;
            if count > limit.get() {
                return Err(RunHeaderScanError::LimitExceeded { limit: limit.get() });
            }
            let (key, value) = item
                .into_inner()
                .map_err(JournalError::from)
                .map_err(RunHeaderScanError::Storage)?;
            decode_scanned_header(key.as_ref(), value.as_ref())
                .map_err(RunHeaderScanError::Storage)?;
        }
        for item in snapshot
            .prefix(&self.run_header, [PREFIX_RUN_HEADER])
            .take(limit.get())
        {
            let (key, value) = item
                .into_inner()
                .map_err(JournalError::from)
                .map_err(RunHeaderScanError::Storage)?;
            let header = decode_scanned_header(key.as_ref(), value.as_ref())
                .map_err(RunHeaderScanError::Storage)?;
            visit(header).map_err(RunHeaderScanError::Visitor)?;
        }
        Ok(())
    }
}

fn decode_scanned_header(key: &[u8], value: &[u8]) -> Result<RunHeaderRecord, JournalError> {
    let run = run_from_header_key(key)?;
    let (envelope, header) = decode_record(
        value,
        MAGIC_INDEX_RECORD,
        crate::constants::MAX_RUN_HEADER_BYTES,
    )?;
    if envelope.record_kind != crate::records::RecordKind::RunHeader.id() {
        return Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind: envelope.record_kind,
            payload_kind: crate::records::RecordKind::RunHeader.id(),
        });
    }
    validate_run_header_read(&envelope, run, &header)?;
    Ok(header)
}

#[cfg(test)]
#[path = "startup_header_scan_tests.rs"]
mod startup_header_scan_tests;
