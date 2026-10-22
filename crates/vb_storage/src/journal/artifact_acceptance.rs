use crate::constants::{ARTIFACT_ACCEPTANCE_LAST_ISSUED, PREFIX_ARTIFACT_ACCEPTANCE};
use crate::error::JournalError;
use crate::journal::FjallJournal;
use crate::types::EventSeq;

/// Private wrapper for issued artifact sequence numbers.
///
/// Wraps `NonZeroU64` because issued sequences always start at 1.
/// This type is not exported; only `EventSeq` appears in the public API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IssuedArtifactSequence(u64);

impl IssuedArtifactSequence {
    /// Creates a new issued sequence from a validated non-zero value.
    /// The caller must have already incremented the counter.
    #[inline]
    fn new(value: u64) -> Self {
        // value is always >= 1 when called from increment_last_issued
        Self(value)
    }

    /// Converts to the public EventSeq type for embedding in the artifact.
    #[inline]
    pub(crate) fn to_event_seq(self) -> EventSeq {
        EventSeq::new(self.0)
    }
}

/// Reads the last issued artifact sequence from the metadata keyspace.
///
/// Returns 0 if the counter key is absent (no artifacts issued yet).
fn read_last_issued(journal: &FjallJournal) -> Result<u64, JournalError> {
    let key = artifact_acceptance_key(ARTIFACT_ACCEPTANCE_LAST_ISSUED)?;
    match journal.artifact_acceptance.get(key.as_slice())? {
        Some(value) => {
            let value_len = u32::try_from(value.len()).map_err(|_| {
                JournalError::MalformedKeyspaceRow {
                    prefix: PREFIX_ARTIFACT_ACCEPTANCE,
                    expected_len: 8,
                    actual_len: value.len(),
                }
            })?;
            if value_len != 8 {
                return Err(JournalError::MalformedKeyspaceRow {
                    prefix: PREFIX_ARTIFACT_ACCEPTANCE,
                    expected_len: 8,
                    actual_len: value.len(),
                });
            }
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&value[..8]);
            Ok(u64::from_be_bytes(bytes))
        }
        None => Ok(0),
    }
}

/// Encodes a u64 as big-endian 8 bytes.
fn encode_u64_be(value: u64) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    bytes.to_vec()
}

/// Builds the storage key for an artifact acceptance metadata entry.
fn artifact_acceptance_key(name: &str) -> Result<Vec<u8>, JournalError> {
    let name_bytes = name.as_bytes();
    let name_len = u32::try_from(name_bytes.len()).map_err(|_| JournalError::KeyCapacity)?;
    let mut key = Vec::with_capacity(1 + name_len as usize);
    key.push(PREFIX_ARTIFACT_ACCEPTANCE);
    key.extend_from_slice(name_bytes);
    Ok(key)
}

/// Atomically increments the artifact acceptance counter within a batch.
///
/// This is the core counter helper that performs the RMW sequence inside a
/// single OwnedWriteBatch transaction. The caller must already hold the
/// write_lock and must commit the batch before releasing the lock.
///
/// Returns the new issued sequence number (always >= 1).
pub(crate) fn issue_next_artifact_sequence(
    journal: &FjallJournal,
    batch: &mut crate::batch::JournalWriteBatch<'_>,
) -> Result<IssuedArtifactSequence, JournalError> {
    // Read the last issued value (0 means none issued yet)
    let last = read_last_issued(journal)?;

    // Increment with overflow check
    let next = last
        .checked_add(1)
        .ok_or(JournalError::SequenceOverflow)?;

    // Stage the updated counter in the batch
    let counter_key = artifact_acceptance_key(ARTIFACT_ACCEPTANCE_LAST_ISSUED)?;
    let counter_value = encode_u64_be(next);
    batch
        .inner
        .insert(&journal.artifact_acceptance, counter_key, counter_value);

    // Return the issued sequence
    Ok(IssuedArtifactSequence::new(next))
}

#[cfg(test)]
pub(crate) fn read_last_issued_for_test(journal: &FjallJournal) -> Result<u64, JournalError> {
    read_last_issued(journal)
}
