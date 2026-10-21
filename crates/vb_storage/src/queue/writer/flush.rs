//! Atomic bounded queue commits and explicit durability selection.
use super::group::selected_flush_len;
use super::stage::stage_queued_event;
use super::{
    DurabilityProfile, FjallJournal, JournalError, JournalWriterFlushReport, JournalWriterQueue,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum FlushDurability {
    ProfileDriven,
    SyncAll,
}

impl JournalWriterQueue {
    /// Flushes at most one configured batch to the journal.
    ///
    /// Atomicity: events drained in a single `flush_batch` either all
    /// become durable together or none do. Per master §49
    /// Crash-Consistency Rule, a partial prefix is forbidden: a process
    /// crash mid-flush must leave no durable-visible record set. We
    /// stage every event into one `fjall::OwnedWriteBatch` (acquired
    /// directly via `journal.database.batch()`) and commit the batch
    /// with `PersistMode::SyncAll` when any staged event requested the
    /// `Strict` durability profile. The non-strict branch leaves
    /// durability to Fjall's lazy WAL flush but is still atomic at the
    /// OwnedWriteBatch boundary.
    ///
    /// Idempotency: an event already present in the durable store at
    /// the same `(run, seq)` is treated as an idempotent retry — the
    /// queued bytes are compared to the existing value, and the event is
    /// skipped on match (so retries after a partial flush succeed) or
    /// surface `DuplicateEvent` on mismatch.
    ///
    /// ## Structural commit-before-drain ordering (master §49)
    ///
    /// The atomicity above is a sequential control-flow invariant, not
    /// an out-of-line predicate. The `owned_batch.commit()?` call MUST
    /// run to success before the `while drained < written` drain loop
    /// removes any item from the pending deque. Because the commit uses
    /// `?`, a failed durable commit propagates `JournalError` and
    /// returns early, so the deque is mutated only after the batch is
    /// durably committed. Reordering commit after drain — or making the
    /// flush non-atomic — would violate §49: a crash could leave durable
    /// records whose deque entries were already removed (or vice-versa),
    /// producing a partial durable-visible prefix. Any refactor of this
    /// function MUST preserve commit-then-drain ordering. (As of this
    /// commit the two sites are the `owned_batch.commit()?` call and the
    /// `while drained < written` loop just below it; line numbers may
    /// drift, the symbolic ordering is the binding contract.)
    pub fn flush_batch(
        &self,
        journal: &FjallJournal,
    ) -> Result<JournalWriterFlushReport, JournalError> {
        self.flush_batch_with_durability(journal, FlushDurability::ProfileDriven)
    }

    /// Commits one bounded batch with a synchronous durability barrier before
    /// removing queue entries, including entries admitted as journaled work.
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
        let batch_len = selected_flush_len(&state, self.batch_size)?;
        let mut has_strict = durability == FlushDurability::SyncAll;

        if batch_len == 0 {
            return Ok(JournalWriterFlushReport {
                drained: 0,
                written: 0,
            });
        }

        // Stage every event into a single OwnedWriteBatch so the commit
        // is atomic. Either all `written` events become durable or none
        // do. We accumulate errors into a single typed return without
        // touching the durable store, preserving the §49 rule that a
        // partial prefix is never observable.
        //
        // `staged_keys` is the per-flush dedup set: it tracks keys
        // inserted into `owned_batch` so two events with the same
        // `(run, seq)` queued back-to-back surface as
        // `DuplicateStagedKey` instead of silently overwriting one
        // another at commit time. The set is allocated fresh per
        // flush and dropped on return; cross-flush duplicates are
        // caught by the durable-store idempotency check inside
        // `stage_queued_event`.
        let mut owned_batch = journal.database.batch();
        let mut staged_keys = std::collections::HashSet::new();
        let mut written = 0usize;
        while written < batch_len {
            let Some(item) = state.pending.get(written) else {
                break;
            };
            if item.profile == DurabilityProfile::Strict {
                has_strict = true;
            }
            stage_queued_event(&mut owned_batch, journal, &item.event, &mut staged_keys)?;
            written = written.saturating_add(1);
        }

        // Apply durability: strict batches force SyncAll; journaled
        // batches rely on Fjall's lazy WAL flush but still commit
        // atomically through the OwnedWriteBatch.
        let owned_batch = if has_strict {
            owned_batch.durability(Some(fjall::PersistMode::SyncAll))
        } else {
            owned_batch.durability(None)
        };
        // Share the journal commit fault boundary with the strict batch tests.
        #[cfg(test)]
        if journal.consume_batch_commit_failure_for_test() {
            return Err(JournalError::Fjall(fjall::Error::Io(
                std::io::Error::other("injected queue batch commit failure (test hook)"),
            )));
        }
        // Fjall returns immediately for an empty batch. Idempotent entries
        // can be visible from an earlier unsynced append, so they still need
        // a barrier before this durable service may remove them from RAM.
        if has_strict && owned_batch.is_empty() {
            journal.persist_strict()?;
        }
        owned_batch.commit()?;

        let mut drained = 0usize;
        while drained < written {
            match state.pending.pop_front() {
                Some(_) => {
                    drained = drained.saturating_add(1);
                }
                // LOGIC INVARIANT: `written` counts items we just indexed via
                // `get(index)` on the same deque, so `pop_front` cannot return
                // None here unless an upstream bug corrupts the counts.  We
                // use WriteLockPoisoned only because no dedicated
                // queue-drain-inconsistent variant exists.
                None => return Err(JournalError::WriteLockPoisoned),
            }
        }

        Ok(JournalWriterFlushReport { drained, written })
    }
}
