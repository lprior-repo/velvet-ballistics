#![forbid(unsafe_code)]
//! FjallJournal append implementation.

use crate::{
    codec::{decode_record, encode_record},
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    error::JournalError,
    events::JournalEvent,
    journal::FjallJournal,
    keys::{index_action_key, run_event_key},
};

use super::intent::ActionIndexIntent;

impl FjallJournal {
    /// Appends one event without forcing a durability barrier.
    pub fn append_journaled(&self, event: &JournalEvent) -> Result<(), JournalError> {
        self.append_indexed_unpersisted(event)
    }

    /// Appends one event and forces a strict durability barrier before returning.
    pub fn append_strict(&self, event: &JournalEvent) -> Result<(), JournalError> {
        self.append_indexed_unpersisted(event)?;
        self.persist_strict()
    }

    /// Appends multiple events with a single strict durability barrier.
    pub fn append_strict_batch(&self, events: &[JournalEvent]) -> Result<(), JournalError> {
        for event in events {
            self.append_indexed_unpersisted(event)?;
        }
        if !events.is_empty() {
            self.persist_strict()?;
        }
        Ok(())
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

    pub(crate) fn append_indexed_unpersisted(
        &self,
        event: &JournalEvent,
    ) -> Result<(), JournalError> {
        let intent = super::mrwe6_action_index_intent(event);
        if matches!(intent, ActionIndexIntent::None) {
            return self.append_unpersisted(event);
        }
        match self.append_event_and_index(event, intent) {
            Ok(()) => Ok(()),
            Err(JournalError::DuplicateEvent { run, seq }) => {
                self.accept_equal_duplicate(event, run, seq)
            }
            Err(e) => Err(e),
        }
    }

    pub(crate) fn append_queued_indexed_unpersisted(
        &self,
        event: &JournalEvent,
    ) -> Result<(), JournalError> {
        match self.append_indexed_unpersisted(event) {
            Ok(()) => Ok(()),
            Err(JournalError::DuplicateEvent { run, seq }) => {
                self.accept_equal_duplicate(event, run, seq)
            }
            Err(e) => Err(e),
        }
    }

    fn append_event_and_index(
        &self,
        event: &JournalEvent,
        intent: ActionIndexIntent,
    ) -> Result<(), JournalError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| JournalError::WriteLockPoisoned)?;
        let event_key = run_event_key(event.run_id(), event.seq())?;
        if self.events.contains_key(event_key)? {
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
        let mut batch = self.database.batch();
        batch.insert(&self.events, event_key.to_vec(), value);
        self.stage_action_index_intent(&mut batch, intent)?;
        batch.commit()?;
        Ok(())
    }

    fn accept_equal_duplicate(
        &self,
        event: &JournalEvent,
        run: vb_core::RunId,
        seq: crate::EventSeq,
    ) -> Result<(), JournalError> {
        let key = run_event_key(run, seq)?;
        let Some(value) = self.events.get(key)? else {
            return Err(JournalError::DuplicateEvent { run, seq });
        };
        let (_, existing) = decode_record::<JournalEvent>(
            value.as_ref(),
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        if existing != *event {
            return Err(JournalError::DuplicateEvent { run, seq });
        }
        self.verify_duplicate_index_state(event, run, seq)
    }

    fn verify_duplicate_index_state(
        &self,
        event: &JournalEvent,
        run: vb_core::RunId,
        seq: crate::EventSeq,
    ) -> Result<(), JournalError> {
        let intent = super::mrwe6_action_index_intent(event);
        match intent {
            ActionIndexIntent::Put { action, run, step } => {
                self.require_idempotent_put_duplicate(event, action, run, step, seq)
            }
            ActionIndexIntent::Delete { .. } => Err(JournalError::DuplicateEvent { run, seq }),
            ActionIndexIntent::None => Ok(()),
        }
    }

    fn require_idempotent_put_duplicate(
        &self,
        event: &JournalEvent,
        action: vb_core::ActionId,
        run: vb_core::RunId,
        step: vb_core::StepIdx,
        seq: crate::EventSeq,
    ) -> Result<(), JournalError> {
        let key = index_action_key(action, run, step)?;
        let marker_present = self.index_action.contains_key(key)?;
        super::mrwe6_idempotent_duplicate_retry_from_facts(
            true,
            super::mrwe6_event_class(event),
            marker_present,
        )
        .map(|_| ())
        .map_err(|_| JournalError::DuplicateEvent { run, seq })
    }

    fn stage_action_index_intent(
        &self,
        batch: &mut fjall::OwnedWriteBatch,
        intent: ActionIndexIntent,
    ) -> Result<(), JournalError> {
        match intent {
            ActionIndexIntent::None => Ok(()),
            ActionIndexIntent::Put { .. } => {
                let Some(key) = super::mrwe6_action_index_key_for_intent(intent)? else {
                    return Ok(());
                };
                batch.insert(&self.index_action, key.to_vec(), Vec::<u8>::new());
                Ok(())
            }
            ActionIndexIntent::Delete { .. } => {
                let Some(key) = super::mrwe6_action_index_key_for_intent(intent)? else {
                    return Ok(());
                };
                batch.remove(&self.index_action, key.to_vec());
                Ok(())
            }
        }
    }
}
