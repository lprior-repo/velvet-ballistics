#![forbid(unsafe_code)]

use crate::error::JournalError;
use crate::journal::artifact_acceptance::{IssuedArtifactSequence, LAST_ISSUED_KEY};

use super::types::JournalWriteBatch;

fn compute_put_artifact_sequence(
    current_metadata: u64,
    current_bytes: u64,
    byte_limit: Option<u64>,
) -> Result<u64, JournalError> {
    // 19 bytes total: 11-byte LAST_ISSUED_KEY + 8-byte counter value
    let metadata_bytes = 19u64;
    let new_metadata = current_metadata.checked_add(metadata_bytes).ok_or(
        JournalError::JournalBatchBytesExceeded {
            attempted: u64::MAX,
            limit: byte_limit.map_or(u64::MAX, |limit| limit),
        },
    )?;
    if let Some(limit) = byte_limit {
        let attempted = current_bytes.checked_add(new_metadata).ok_or(
            JournalError::JournalBatchBytesExceeded {
                attempted: u64::MAX,
                limit,
            },
        )?;
        if attempted > limit {
            return Err(JournalError::JournalBatchBytesExceeded { attempted, limit });
        }
    }
    Ok(new_metadata)
}

impl<'j> JournalWriteBatch<'j> {
    pub(crate) fn put_artifact_sequence(
        &mut self,
        seq: IssuedArtifactSequence,
    ) -> Result<(), JournalError> {
        if self.aborted {
            return Err(JournalError::BatchAborted);
        }
        let new_metadata = compute_put_artifact_sequence(
            self.staged_metadata_bytes,
            self.staged_bytes,
            self.byte_limit,
        )
        .map_err(|error| {
            self.aborted = true;
            error
        })?;
        self.staged_metadata_bytes = new_metadata;
        let counter_value = seq.to_event_seq().get().to_be_bytes();
        self.inner.insert(
            &self.journal.artifact_acceptance,
            LAST_ISSUED_KEY,
            counter_value.as_ref(),
        );
        Ok(())
    }
}
