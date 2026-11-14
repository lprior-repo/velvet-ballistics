use crate::{JournalError, JournalWriteBatch};

impl JournalWriteBatch<'_> {
    pub(crate) fn put_artifact_sequence(&mut self, next: u64) -> Result<(), JournalError> {
        let result = self.stage_artifact_sequence(next);
        if result.is_err() {
            self.aborted = true;
        }
        result
    }

    fn stage_artifact_sequence(&mut self, next: u64) -> Result<(), JournalError> {
        let key = crate::journal::artifact_acceptance::LAST_ISSUED_KEY;
        let value = next.to_be_bytes();
        let bytes = u64::try_from(key.len().saturating_add(value.len()))
            .map_err(|_| JournalError::SequenceOverflow)?;
        let attempted = self
            .staged_bytes
            .checked_add(bytes)
            .ok_or(JournalError::SequenceOverflow)?;
        if let Some(limit) = self.byte_limit
            && attempted > limit
        {
            return Err(JournalError::JournalBatchBytesExceeded { attempted, limit });
        }
        self.inner
            .insert(&self.journal.artifact_acceptance, key, value);
        self.staged_bytes = attempted;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn counter_byte_limit_refusal_aborts_without_publication()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let journal = crate::FjallJournal::open(directory.path(), None)?;
        let mut batch = journal.batch();
        batch.byte_limit = Some(18);
        assert!(matches!(
            batch.put_artifact_sequence(1),
            Err(crate::JournalError::JournalBatchBytesExceeded {
                attempted: 19,
                limit: 18
            })
        ));
        assert!(batch.is_aborted());
        assert!(matches!(
            batch.commit(),
            Err(crate::JournalError::BatchAborted)
        ));
        assert!(journal.artifact_acceptance.is_empty()?);
        Ok(())
    }
}
