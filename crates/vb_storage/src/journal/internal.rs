#[cfg(test)]
use crate::codec::decode_journal_event;
use crate::{
    codec::{EnforceKindParity, decode_record, encode_record},
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    error::JournalError,
    events::JournalEvent,
    journal::FjallJournal,
    keys::run_event_key,
    types::RecordEnvelope,
};
use serde::de::DeserializeOwned;

impl FjallJournal {
    #[allow(clippy::unused_self)]
    pub(crate) fn decode_optional_with<T, F>(
        &self,
        keyspace: &fjall::Keyspace,
        key: &[u8],
        magic: u32,
        max_bytes: u32,
        validate: F,
    ) -> Result<Option<T>, JournalError>
    where
        T: DeserializeOwned + EnforceKindParity,
        F: FnOnce(&RecordEnvelope, &T) -> Result<(), JournalError>,
    {
        let Some(value) = keyspace.get(key)? else {
            return Ok(None);
        };
        let (envelope, record) = decode_record(value.as_ref(), magic, max_bytes)?;
        validate(&envelope, &record)?;
        Ok(Some(record))
    }

    /// Appends one event to the LSM memtable without forcing a durability barrier.
    ///
    /// The event is committed to the WAL-backed LSM memtable and is visible
    /// to subsequent readers (replay / `events_for_run`) immediately on
    /// return.  "Unfsynced" is precise: the write survives process-level
    /// recovery (crash + restart reads back the memtable) but has not been
    /// force-flushed to stable storage.  Callers that require strict
    /// durability must invoke `persist_strict` after staging.
    ///
    /// vb-3wn7x: the runtime journal path uses this entry point for
    /// direct appends. To keep the `index_action` keyspace consistent
    /// with the durable event log, the action-lifecycle index mutation
    /// (insert for `ActionScheduled` / `ActionScheduledTicket`,
    /// tombstone for completion / failure / abandonment, no-op for
    /// every other variant) is staged into the SAME
    /// `fjall::OwnedWriteBatch` as the event write and committed
    /// atomically. A process crash between the event insert and the
    /// index mutation is impossible — they share one fsync'd batch.
    pub(crate) fn append_unfsynced(&self, event: &JournalEvent) -> Result<(), JournalError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        let key = run_event_key(event.run_id(), event.seq())?;
        if !event.is_valid() {
            return Err(JournalError::InvalidEvent);
        }
        if self.events.contains_key(key)? {
            return Err(JournalError::DuplicateEvent {
                run: event.run_id(),
                seq: event.seq(),
            });
        }
        let value = encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        // Stage event + pending-action-index update on one batch so the
        // index mutation succeeds or fails with the event write.
        let mut batch = self.database.batch();
        batch.insert(&self.events, key, value);
        self.stage_pending_action_index_op(&mut batch, event)?;
        batch.commit()?;
        Ok(())
    }

    /// Appends a queued event idempotently without forcing an fsync.
    ///
    /// The event is committed to the LSM memtable and is visible to
    /// subsequent readers (replay / `events_for_run`) immediately on
    /// return.  "Unfsynced" is the precise term: the write is durable
    /// against process-level recovery (crash + restart reads back the
    /// memtable) but has not been force-flushed to stable storage.
    /// Callers that require strict durability must invoke
    /// `persist_strict` after staging.
    #[cfg(test)]
    pub(crate) fn append_queued_unfsynced(&self, event: &JournalEvent) -> Result<(), JournalError> {
        match self.append_unfsynced(event) {
            Ok(()) => Ok(()),
            Err(JournalError::DuplicateEvent { run, seq }) => {
                let key = run_event_key(run, seq)?;
                let Some(value) = self.events.get(key)? else {
                    return Err(JournalError::DuplicateEvent { run, seq });
                };
                let (_, existing) = decode_journal_event(
                    value.as_ref(),
                    MAGIC_JOURNAL_EVENT,
                    MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
                )?;
                if existing == *event {
                    Ok(())
                } else {
                    Err(JournalError::DuplicateEvent { run, seq })
                }
            }
            Err(e) => Err(e),
        }
    }
}
