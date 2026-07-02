use crate::{
    error::JournalError, events::JournalEvent, journal::FjallJournal, keys::run_event_key,
};

impl FjallJournal {
    /// Appends one event without forcing a durability barrier.
    ///
    /// # Next-sequence-at-write guard (vb-r8oso)
    ///
    /// Delegates to [`FjallJournal::append_unfsynced`], which enforces
    /// `event.seq() == next_sequence_at_write(event.run_id())` before
    /// staging. A mismatch is rejected with
    /// `JournalError::SequenceMismatch { run, expected, actual }` and
    /// the durable log is unchanged.
    pub fn append_journaled(&self, event: &JournalEvent) -> Result<(), JournalError> {
        self.append_unfsynced(event)
    }

    /// Appends one event and forces a strict durability barrier before returning.
    ///
    /// Durability contract: this function returns `Ok(())` only after both the
    /// staged insert into the events keyspace AND the strict fsync barrier
    /// (`fjall::PersistMode::SyncAll`) have succeeded atomically through a
    /// single `JournalWriteBatch::commit()`. The event is therefore never
    /// visible to readers without being durable on success.
    ///
    /// Previous behaviour staged the event into the LSM memtable via
    /// `append_unfsynced` (visible to subsequent readers) and only then
    /// invoked `persist_strict`. If `persist_strict` failed the event was
    /// visible-but-not-durable; a retry then observed
    /// `events.contains_key` and returned `DuplicateEvent`, preventing the
    /// caller from cleanly retrying the strict durability barrier. The
    /// batched implementation closes that window: the event is staged and
    /// committed (with `SyncAll`) atomically, and a failed commit surfaces
    /// as `JournalError::StrictDurabilityFailed` (or a Fjall error) rather
    /// than as `DuplicateEvent`.
    ///
    /// Idempotency on retry: if the commit fails after staging but before
    /// returning, the event is *not* visible because the entire batch
    /// commit was rejected. A retry re-stages and re-commits cleanly
    /// (returning `Ok`) — no `DuplicateEvent` from a previously-visible
    /// but undelivered state.
    ///
    /// # Next-sequence-at-write guard (vb-r8oso)
    ///
    /// `event.seq()` must equal `next_sequence_at_write(event.run_id())`
    /// at the moment of write. The check is performed inside
    /// `JournalWriteBatch::append_event` so the comparison and the
    /// durable insert share one batch boundary. A mismatch is rejected
    /// with `JournalError::SequenceMismatch { run, expected, actual }`
    /// and no state is mutated.
    pub fn append_strict(&self, event: &JournalEvent) -> Result<(), JournalError> {
        // Validate first so an invalid event is rejected before any
        // allocation; `append_event` repeats this check defensively.
        if !event.is_valid() {
            return Err(JournalError::InvalidEvent);
        }
        // vb-r8oso: next-sequence-at-write guard fires before the
        // durable duplicate pre-check. A retry whose seq no longer
        // matches the expected next seq is rejected with
        // `SequenceMismatch` (a typed caller-fix error) before the
        // `DuplicateEvent` path can fire. This aligns `append_strict`
        // with the C-4.2 guard precedence.
        let expected = self.next_sequence_at_write(event.run_id())?;
        if event.seq() != expected {
            return Err(JournalError::SequenceMismatch {
                run: event.run_id(),
                expected,
                actual: event.seq(),
            });
        }
        // Pre-check the durable duplicate key so a retry after a
        // StrictDurabilityFailed does not race against a partially
        // committed prior attempt. `append_event` repeats this check
        // inside the batch boundary; both are needed for the new
        // atomic guarantee (the second check inside the batch commits
        // with the key, the first check guards the caller's intent).
        let key = run_event_key(event.run_id(), event.seq())?;
        if self.events.contains_key(key)? {
            return Err(JournalError::DuplicateEvent {
                run: event.run_id(),
                seq: event.seq(),
            });
        }
        let mut batch = self.batch();
        batch.append_event(event)?;
        batch.strict().commit()
    }

    /// Appends multiple events with a single strict durability barrier.
    ///
    /// Atomicity: every event in `events` either becomes durable together or
    /// no event is made durable. Implemented via a `fjall::OwnedWriteBatch`
    /// wrapped through [`crate::batch::JournalWriteBatch`]: events are
    /// staged into a single cross-event batch and committed with
    /// `PersistMode::SyncAll`, so a process crash mid-batch leaves no
    /// partial, durable-visible record set. Master §49 Crash-Consistency
    /// Rule requires this single-barrier semantic; the previous
    /// per-event `append_unfsynced` loop violated it.
    ///
    /// # Next-sequence-at-write guard (vb-r8oso)
    ///
    /// Each event in `events` is required to satisfy
    /// `event.seq() == next_sequence_at_write(event.run_id())` at the
    /// moment its `append_event` runs. The first element whose seq
    /// diverges is rejected with
    /// `JournalError::SequenceMismatch { run, expected, actual }` and
    /// the batch is aborted (`self.aborted = true`) so subsequent
    /// `append_event` calls return `BatchAborted`. The whole batch
    /// must be retried; no partial durable commit is performed.
    pub fn append_strict_batch(&self, events: &[JournalEvent]) -> Result<(), JournalError> {
        if events.is_empty() {
            return Ok(());
        }
        let mut batch = self.batch();
        for event in events {
            batch.append_event(event)?;
        }
        batch.strict().commit()
    }

    /// Forces a strict durability barrier.
    pub fn persist_strict(&self) -> Result<(), JournalError> {
        #[cfg(test)]
        if self.consume_persist_failure_for_test() {
            return Err(JournalError::StrictDurabilityFailed);
        }
        self.database.persist(fjall::PersistMode::SyncAll)?;
        Ok(())
    }
}
