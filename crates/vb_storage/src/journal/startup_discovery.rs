//! Bounded cold-start discovery from retained headers and durable journal rows.
use crate::{
    JournalError,
    codec::{decode_journal_event, validate_replayed_event},
    constants::{
        JOURNAL_KEY_BYTES, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, PREFIX_RUN_EVENT,
    },
    headers::decode_scanned_header,
    journal::FjallJournal,
    keys::decode_storage_key,
    types::StorageKey,
};
use fjall::Readable;
use std::num::NonZeroUsize;
use vb_core::RunId;

/// Failure before discovery completes, or refusal by the recovery visitor.
#[derive(Debug, thiserror::Error)]
pub enum StartupDiscoveryError<E: std::error::Error + 'static> {
    /// The row limit cannot admit an overflow probe.
    #[error("invalid startup discovery limit")]
    InvalidLimit,
    /// The combined keyspaces exceed the row-work limit.
    #[error("startup discovery exceeds {limit} rows")]
    RowLimitExceeded {
        /// Configured maximum number of rows.
        limit: usize,
    },
    /// The combined encoded values exceed the byte-work limit.
    #[error("startup discovery exceeds {limit} encoded bytes")]
    ByteLimitExceeded {
        /// Configured maximum number of encoded bytes.
        limit: usize,
    },
    /// Bounded run identifier storage could not be reserved.
    #[error("startup discovery allocation failed")]
    AllocationFailed,
    /// A retained key, record or storage operation failed validation.
    #[error("startup discovery storage failure: {0}")]
    Storage(#[source] JournalError),
    /// Recovery refused a discovered run.
    #[error("startup discovery visitor failure: {0}")]
    Visitor(#[source] E),
}

impl FjallJournal {
    /// Visits each run once after validating both complete keyspaces from one snapshot.
    ///
    /// Row, byte, allocation and record failures invoke no visitors. The byte limit
    /// bounds encoded values processed, but Fjall materializes each row before this
    /// application check. Visitors retain responsibility for recovery authority.
    pub fn try_for_each_startup_run_bounded<E, F>(
        &self,
        max_rows: NonZeroUsize,
        max_encoded_bytes: NonZeroUsize,
        visit: F,
    ) -> Result<(), StartupDiscoveryError<E>>
    where
        E: std::error::Error + 'static,
        F: FnMut(RunId) -> Result<(), E>,
    {
        self.try_for_each_startup_run_with_snapshot(
            max_rows,
            max_encoded_bytes,
            visit,
            || self.database.snapshot(),
            || Ok(()),
        )
    }

    fn try_for_each_startup_run_with_snapshot<E, F, S, H>(
        &self,
        max_rows: NonZeroUsize,
        max_encoded_bytes: NonZeroUsize,
        mut visit: F,
        mut make_snapshot: S,
        mut after_headers: H,
    ) -> Result<(), StartupDiscoveryError<E>>
    where
        E: std::error::Error + 'static,
        F: FnMut(RunId) -> Result<(), E>,
        S: FnMut() -> fjall::Snapshot,
        H: FnMut() -> Result<(), JournalError>,
    {
        let probe = max_rows
            .get()
            .checked_add(1)
            .ok_or(StartupDiscoveryError::InvalidLimit)?;
        let mut runs = Vec::new();
        runs.try_reserve_exact(max_rows.get())
            .map_err(|_| StartupDiscoveryError::AllocationFailed)?;
        let snapshot = make_snapshot();
        let mut rows = 0usize;
        let mut bytes = 0usize;
        for (keyspace, is_header) in [(&self.run_header, true), (&self.events, false)] {
            for item in snapshot.iter(keyspace).take(probe) {
                rows = rows
                    .checked_add(1)
                    .ok_or(StartupDiscoveryError::InvalidLimit)?;
                if rows > max_rows.get() {
                    return Err(StartupDiscoveryError::RowLimitExceeded {
                        limit: max_rows.get(),
                    });
                }
                let (key, value) = item
                    .into_inner()
                    .map_err(JournalError::from)
                    .map_err(StartupDiscoveryError::Storage)?;
                bytes = bytes
                    .checked_add(value.len())
                    .filter(|total| *total <= max_encoded_bytes.get())
                    .ok_or(StartupDiscoveryError::ByteLimitExceeded {
                        limit: max_encoded_bytes.get(),
                    })?;
                let run = if is_header {
                    decode_scanned_header(key.as_ref(), value.as_ref()).map(|header| header.run)
                } else {
                    decode_discovery_event(key.as_ref(), value.as_ref())
                }
                .map_err(StartupDiscoveryError::Storage)?;
                runs.push(run);
            }
            if is_header {
                after_headers().map_err(StartupDiscoveryError::Storage)?;
            }
        }
        runs.sort_unstable();
        runs.dedup();
        for run in runs {
            visit(run).map_err(StartupDiscoveryError::Visitor)?;
        }
        Ok(())
    }
}

fn decode_discovery_event(key: &[u8], value: &[u8]) -> Result<RunId, JournalError> {
    let (run, seq) = match decode_storage_key(key) {
        Ok(StorageKey::RunEvent { run, seq }) => (run, seq),
        Ok(_) | Err(_) => {
            return Err(JournalError::MalformedKeyspaceRow {
                prefix: PREFIX_RUN_EVENT,
                expected_len: JOURNAL_KEY_BYTES,
                actual_len: key.len(),
            });
        }
    };
    let (envelope, event) =
        decode_journal_event(value, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    if envelope.sequence != seq.get() || event.seq() != seq {
        return Err(JournalError::ReplayKeyMismatch {
            run,
            key_seq: seq.get(),
            payload_seq: event.seq().get(),
        });
    }
    validate_replayed_event(run, seq, &event)?;
    Ok(run)
}

#[cfg(test)]
#[path = "startup_discovery_tests.rs"]
mod tests;
