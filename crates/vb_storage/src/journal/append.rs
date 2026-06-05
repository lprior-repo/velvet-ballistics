mod decision;
mod intent;

pub use self::decision::{
    Mrwe6DuplicateRetryDecision, Mrwe6RecoveryOutcome, Mrwe6ResolutionCommitDecision,
    mrwe6_duplicate_retry_decision, mrwe6_duplicate_retry_decision_from_facts,
    mrwe6_recovery_outcome, mrwe6_recovery_outcome_from_facts, mrwe6_resolution_commit_decision,
    mrwe6_resolution_commit_decision_from_facts,
};
pub use self::intent::{
    Mrwe6ActionIndexIntent, Mrwe6AtomKind, Mrwe6EventClass, Mrwe6IntentKind, Mrwe6SeamError,
    Mrwe6ValidatedAtom, mrwe6_action_index_intent, mrwe6_action_index_key_for_intent,
    mrwe6_event_class, mrwe6_event_intent_matches_class, mrwe6_intent_kind,
    mrwe6_intent_kind_matches_event_class, mrwe6_required_intent_kind_for_class,
    mrwe6_validated_atom, mrwe6_validated_atom_for_event,
};

#[cfg(kani)]
pub(crate) use self::decision::{
    VerificationDuplicateRetryDecision, VerificationRecoveryOutcome,
    VerificationResolutionCommitDecision, verification_duplicate_retry_decision,
    verification_recovery_outcome, verification_resolution_commit_decision,
    verification_resolution_marker_present_after_commit,
};
#[cfg(kani)]
pub(crate) use self::intent::{
    VerificationActionIndexIntent, verification_action_index_intent,
    verification_event_and_index_keys_exist,
};

use crate::{
    codec::{decode_record, encode_record},
    constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES},
    error::JournalError,
    events::JournalEvent,
    journal::FjallJournal,
    keys::{index_action_key, run_event_key},
};

use self::intent::ActionIndexIntent;

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
        let intent = ActionIndexIntent::for_event(event);
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
        _run: vb_core::RunId,
        seq: crate::EventSeq,
    ) -> Result<(), JournalError> {
        let intent = ActionIndexIntent::for_event(event);
        match intent {
            ActionIndexIntent::Put { action, run, step } => {
                self.require_index_state(action, run, step, true, seq)
            }
            ActionIndexIntent::Delete { action, run, step } => {
                self.require_index_state(action, run, step, false, seq)
            }
            ActionIndexIntent::None => Ok(()),
        }
    }

    fn require_index_state(
        &self,
        action: vb_core::ActionId,
        run: vb_core::RunId,
        step: vb_core::StepIdx,
        expected: bool,
        seq: crate::EventSeq,
    ) -> Result<(), JournalError> {
        let key = index_action_key(action, run, step)?;
        if self.index_action.contains_key(key)? == expected {
            Ok(())
        } else {
            Err(JournalError::DuplicateEvent { run, seq })
        }
    }

    fn stage_action_index_intent(
        &self,
        batch: &mut fjall::OwnedWriteBatch,
        intent: ActionIndexIntent,
    ) -> Result<(), JournalError> {
        match intent {
            ActionIndexIntent::None => Ok(()),
            ActionIndexIntent::Put { action, run, step } => {
                let key = index_action_key(action, run, step)?;
                batch.insert(&self.index_action, key.to_vec(), Vec::<u8>::new());
                Ok(())
            }
            ActionIndexIntent::Delete { action, run, step } => {
                let key = index_action_key(action, run, step)?;
                batch.remove(&self.index_action, key.to_vec());
                Ok(())
            }
        }
    }
}
