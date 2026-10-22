//! Private artifact acceptance counter and issuance helpers.
//!
//! This module owns the artifact sequence counter stored in the
//! `artifact_acceptance` metadata keyspace. It provides a single private
//! helper, `issue_artifact`, that performs the entire RMW sequence:
//! read counter, increment, stamp, rebind digest, encode record, stage
//! both writes in one `fjall::OwnedWriteBatch`, and commit with policy-
//! appropriate durability.
//!
//! All helpers here are private (`fn` not `pub fn`). No new public API is
//! exposed through this module.

use std::num::NonZeroU64;

use crate::admission::AcceptedArtifact;
use crate::admission::bind_artifact_digest;
use crate::constants::MAGIC_COMPILED_ARTIFACT;
use crate::constants::MAX_COMPILED_IR_BYTES;
use crate::error::JournalError;
use crate::journal::FjallJournal;
use crate::journal::admission::verify_compiled_ir_record_digest;
use crate::keys::compiled_ir_key;
use crate::records::CompiledIrRecord;
use crate::records::RecordKind;
use crate::types::EventSeq;
use vb_core::RuntimePolicy;

/// Private wrapper for issued artifact sequence numbers.
///
/// Wraps `NonZeroU64` because issued sequences always start at 1.
/// This type is not exported; only `EventSeq` appears in the public API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IssuedArtifactSequence(std::num::NonZeroU64);

impl IssuedArtifactSequence {
    /// Creates a new issued sequence from a validated non-zero value.
    /// The caller must have already incremented the counter and ensured
    /// the value is strictly greater than zero.
    #[inline]
    fn new(value: u64) -> Result<Self, JournalError> {
        let nz = NonZeroU64::new(value).ok_or(JournalError::SequenceOverflow)?;
        Ok(Self(nz))
    }

    /// Converts to the public EventSeq type for embedding in the artifact.
    #[inline]
    pub(crate) fn to_event_seq(self) -> EventSeq {
        EventSeq::new(self.0.get())
    }
}

/// The literal key for the last-issued counter in the artifact_acceptance
/// keyspace. The dedicated keyspace already isolates this key; no prefix
/// is needed.
const LAST_ISSUED_KEY: &[u8] = b"last_issued";

/// Reads the last issued artifact sequence from the metadata keyspace.
///
/// Returns 0 if the counter key is absent (no artifacts issued yet).
fn read_counter(journal: &FjallJournal) -> Result<u64, JournalError> {
    match journal.artifact_acceptance.get(LAST_ISSUED_KEY)? {
        Some(value) => decode_counter_value(&value),
        None => Ok(0),
    }
}

/// Decodes a counter value from bytes.
///
/// Uses safe fixed array TryFrom; no unchecked slicing or indexing.
/// Value must be exactly 8 bytes (big-endian u64).
fn decode_counter_value(value: &[u8]) -> Result<u64, JournalError> {
    if value.len() != 8 {
        return Err(JournalError::MalformedKeyspaceRow {
            prefix: 0,
            expected_len: 8,
            actual_len: value.len(),
        });
    }
    // Safe fixed array TryFrom (no unchecked indexing or copy_from_slice)
    let bytes: [u8; 8] = value
        .try_into()
        .map_err(|_| JournalError::MalformedKeyspaceRow {
            prefix: 0,
            expected_len: 8,
            actual_len: value.len(),
        })?;
    Ok(u64::from_be_bytes(bytes))
}

/// Encodes a counter value to big-endian bytes.
fn encode_counter_value(value: u64) -> [u8; 8] {
    value.to_be_bytes()
}

/// Increment the counter and prepare the stamped artifact.
fn prepare_artifact(
    last: u64,
    artifact: AcceptedArtifact,
) -> Result<(u64, AcceptedArtifact), JournalError> {
    let next = last.checked_add(1).ok_or(JournalError::SequenceOverflow)?;
    let seq = IssuedArtifactSequence::new(next)?;
    let mut stamped = artifact;
    stamped.accepted_at_seq = seq.to_event_seq();
    bind_artifact_digest(&mut stamped)?;
    Ok((next, stamped))
}

/// Encode the stamped artifact into a compiled IR record and envelope.
fn encode_record(stamped: &AcceptedArtifact) -> Result<(CompiledIrRecord, Vec<u8>), JournalError> {
    let ir = postcard::to_allocvec(stamped).map_err(JournalError::PostcardEncodeFailed)?;
    let record = CompiledIrRecord {
        digest: stamped.digest,
        ir,
    };
    verify_compiled_ir_record_digest(&record)?;
    let encoded = crate::codec::encode_record(
        MAGIC_COMPILED_ARTIFACT,
        RecordKind::CompiledIr,
        0,
        &record,
        MAX_COMPILED_IR_BYTES,
    )?;
    Ok((record, encoded))
}

/// Build the raw OwnedWriteBatch with counter and artifact writes.
fn build_batch(
    journal: &FjallJournal,
    next: u64,
    stamped: &AcceptedArtifact,
    encoded: &[u8],
) -> Result<fjall::OwnedWriteBatch, JournalError> {
    let mut batch = journal.database.batch();
    let counter_value = encode_counter_value(next);
    batch.insert(
        &journal.artifact_acceptance,
        LAST_ISSUED_KEY,
        counter_value.as_ref(),
    );
    let artifact_key = compiled_ir_key(stamped.digest.as_bytes())?;
    batch.insert(&journal.compiled_ir, &artifact_key, encoded);
    Ok(batch)
}

/// Apply policy durability and commit the batch.
fn commit_batch(
    _journal: &FjallJournal,
    mut batch: fjall::OwnedWriteBatch,
    policy: RuntimePolicy,
) -> Result<(), JournalError> {
    if policy == RuntimePolicy::Strict {
        #[cfg(test)]
        if _journal.consume_persist_failure_for_test() {
            return Err(JournalError::StrictDurabilityFailed);
        }
        batch = batch.durability(Some(fjall::PersistMode::SyncAll));
    }
    #[cfg(test)]
    if _journal.consume_batch_commit_failure_for_test() {
        return Err(JournalError::Fjall(fjall::Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "injected batch commit failure (test hook)",
        ))));
    }
    batch.commit()?;
    Ok(())
}

/// Atomically issues the next artifact sequence and persists the
/// stamped artifact in a single `fjall::OwnedWriteBatch`.
fn issue_artifact(
    journal: &FjallJournal,
    artifact: AcceptedArtifact,
    policy: RuntimePolicy,
) -> Result<AcceptedArtifact, JournalError> {
    let _write_guard = journal
        .write_lock
        .lock()
        .map_err(|_| JournalError::WriteLockPoisoned)?;

    let last = read_counter(journal)?;
    let (next, stamped) = prepare_artifact(last, artifact)?;
    let (_record, encoded) = encode_record(&stamped)?;
    let batch = build_batch(journal, next, &stamped, &encoded)?;
    commit_batch(journal, batch, policy)?;

    Ok(stamped)
}

/// Exposes the private issuance helper to admission.rs via pub(crate).
pub(crate) fn issue_next_artifact_sequence(
    journal: &FjallJournal,
    artifact: AcceptedArtifact,
    policy: RuntimePolicy,
) -> Result<AcceptedArtifact, JournalError> {
    issue_artifact(journal, artifact, policy)
}
