#![forbid(unsafe_code)]
//! Artifact acceptance sequence counter backed by the `artifact_acceptance` keyspace.
//!
//! Provides atomic read-modify-write of the last-issued artifact sequence number.
//! Counter value is stored as an 8-byte big-endian u64 at a fixed internal key.

use crate::constants::{ARTIFACT_LAST_ISSUED_KEY, KEYSPACE_ARTIFACT_ACCEPTANCE};
use crate::error::JournalError;
use crate::journal::FjallJournal;
use crate::types::EventSeq;
use std::sync::MutexGuard;

/// Returns the current last-issued artifact sequence number.
///
/// Returns 0 if no artifacts have been issued.
fn read_last_issued(journal: &FjallJournal) -> Result<u64, JournalError> {
    let raw = journal
        .artifact_acceptance
        .get(ARTIFACT_LAST_ISSUED_KEY.as_bytes())?;
    match raw {
        None => Ok(0),
        Some(bytes) => {
            if bytes.len() != 8 {
                return Err(JournalError::MalformedKeyspaceRow {
                    prefix: 0,
                    expected_len: 8,
                    actual_len: bytes.len(),
                });
            }
            Ok(u64::from_be_bytes(bytes.try_into().unwrap()))
        }
    }
}

/// Writes the last-issued artifact sequence number.
fn write_last_issued(journal: &FjallJournal, seq: u64) -> Result<(), JournalError> {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&seq.to_be_bytes());
    journal
        .artifact_acceptance
        .put(ARTIFACT_LAST_ISSUED_KEY.as_bytes(), bytes)?;
    Ok(())
}

/// Atomically increments and returns the next artifact acceptance sequence.
///
/// Must be called while holding the journal's `write_lock`.
/// Returns the newly issued sequence number (starts at 1).
pub fn next_artifact_sequence(
    journal: &FjallJournal,
    _lock: &MutexGuard<()>,
) -> Result<EventSeq, JournalError> {
    let current = read_last_issued(journal)?;
    let next = current
        .checked_add(1)
        .ok_or(JournalError::SequenceOverflow)?;
    write_last_issued(journal, next)?;
    Ok(EventSeq::new(next))
}
