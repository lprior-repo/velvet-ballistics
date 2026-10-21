//! Atomic bounded queue commits and explicit durability selection.
use super::group::selected_flush_len;
use super::stage::stage_queued_event;
use super::{
    DurabilityProfile, FjallJournal, JournalError, JournalWriterFlushReport, JournalWriterQueue,
    JournalWriterQueueState,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum FlushDurability {
    ProfileDriven,
    SyncAll,
}

struct PreparedBatch {
    batch: fjall::OwnedWriteBatch,
    written: usize,
    has_strict: bool,
}

impl JournalWriterQueue {
    /// Flushes at most one configured batch atomically. Strict entries select
    /// SyncAll; journaled-only entries use Fjall's lazy WAL durability.
    /// Matching durable events are idempotent; divergent duplicates are errors.
    /// The queue mutex covers selection, staging, commit and retirement.
    ///
    /// Master49 ordering: commit_prepared MUST succeed before retire_committed
    /// removes any deque entry. A failed staging or commit leaves the batch owned
    /// by the queue. Atomic same-run batches are selected without splitting.
    pub fn flush_batch(
        &self,
        journal: &FjallJournal,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        self.flush_batch_with_durability(journal, FlushDurability::ProfileDriven)
    }

    /// Commits one bounded batch with a synchronous durability barrier before
    /// removing entries, including those admitted as journaled work.
    pub fn flush_batch_durable(
        &self,
        journal: &FjallJournal,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        self.flush_batch_with_durability(journal, FlushDurability::SyncAll)
    }

    fn flush_batch_with_durability(
        &self,
        journal: &FjallJournal,
        durability: FlushDurability,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        let length = selected_flush_len(&state, self.batch_size)?;
        if length == 0 {
            return Ok(JournalWriterFlushReport {
                drained: 0,
                written: 0,
            });
        }
        let prepared = prepare_batch(&state, journal, length, durability)?;
        let written = commit_prepared(prepared, journal)?;
        retire_committed(&mut state, written)
    }
}

fn prepare_batch(
    state: &JournalWriterQueueState,
    journal: &FjallJournal,
    length: usize,
    durability: FlushDurability,
) -> Result<PreparedBatch, JournalError> {
    let mut prepared = PreparedBatch {
        batch: journal.database.batch(),
        written: 0,
        has_strict: durability == FlushDurability::SyncAll,
    };
    let mut staged_keys = std::collections::HashSet::new();
    for item in state.pending.iter().take(length) {
        prepared.has_strict |= item.profile == DurabilityProfile::Strict;
        stage_queued_event(&mut prepared.batch, journal, &item.event, &mut staged_keys)?;
        prepared.written = prepared.written.saturating_add(1);
    }
    Ok(prepared)
}

fn commit_prepared(prepared: PreparedBatch, journal: &FjallJournal) -> Result<usize, JournalError> {
    let PreparedBatch {
        batch,
        written,
        has_strict,
    } = prepared;
    let mode = if has_strict {
        Some(fjall::PersistMode::SyncAll)
    } else {
        None
    };
    let batch = batch.durability(mode);
    #[cfg(test)]
    inject_commit_failure(journal)?;
    // Fjall3.1.4 short-circuits an empty batch. Matching visible entries may
    // come from an earlier unsynced append, so their retirement needs a barrier.
    if has_strict && batch.is_empty() {
        journal.persist_strict()?;
    }
    batch.commit()?;
    Ok(written)
}

fn retire_committed(
    state: &mut JournalWriterQueueState,
    written: usize,
) -> Result<JournalWriterFlushReport, JournalError> {
    let mut drained = 0usize;
    while drained < written {
        match state.pending.pop_front() {
            Some(_) => drained = drained.saturating_add(1),
            None => return Err(JournalError::WriteLockPoisoned),
        }
    }
    Ok(JournalWriterFlushReport { drained, written })
}

#[cfg(test)]
fn inject_commit_failure(journal: &FjallJournal) -> Result<(), JournalError> {
    if journal.consume_batch_commit_failure_for_test() {
        return Err(JournalError::Fjall(fjall::Error::Io(
            std::io::Error::other("injected queue batch commit failure (test hook)"),
        )));
    }
    Ok(())
}
