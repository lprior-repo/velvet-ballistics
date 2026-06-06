mod decision;
mod intent;
pub(crate) mod mrwe6_kernel;

pub use self::decision::{
    Mrwe6DuplicateRetryDecision, Mrwe6RecoveryOutcome, Mrwe6ResolutionCommitDecision,
    mrwe6_committed_resolution_from_facts, mrwe6_duplicate_retry_decision,
    mrwe6_duplicate_retry_decision_from_facts, mrwe6_idempotent_duplicate_retry_from_facts,
    mrwe6_pending_inventory_from_facts, mrwe6_recovery_outcome, mrwe6_recovery_outcome_from_facts,
    mrwe6_resolution_commit_decision, mrwe6_resolution_commit_decision_from_facts,
};
pub use self::intent::{
    Mrwe6ActionIndexIntent, Mrwe6AtomKind, Mrwe6EventClass, Mrwe6IntentKind, Mrwe6SeamError,
    Mrwe6ValidatedAtom, mrwe6_action_index_intent, mrwe6_action_index_key_for_intent,
    mrwe6_event_class, mrwe6_event_intent_matches_class, mrwe6_intent_kind,
    mrwe6_intent_kind_matches_event_class, mrwe6_required_intent_kind_for_class,
    mrwe6_valid_queued_relevant_intent, mrwe6_valid_scheduled_atom, mrwe6_validated_atom,
    mrwe6_validated_atom_for_event,
};
pub use self::mrwe6_kernel::{
    atom_kind_for_intent_kind as mrwe6_kernel_atom_kind_for_intent_kind,
    checked_atom_kind as mrwe6_kernel_checked_atom_kind,
    checked_queued_relevant_atom_kind as mrwe6_kernel_checked_queued_relevant_atom_kind,
    checked_scheduled_atom_kind as mrwe6_kernel_checked_scheduled_atom_kind,
    duplicate_retry_decision_from_facts as mrwe6_kernel_duplicate_retry_decision_from_facts,
    intent_kind_matches_event_class as mrwe6_kernel_intent_kind_matches_event_class,
    recovery_outcome_from_facts as mrwe6_kernel_recovery_outcome_from_facts,
    required_intent_kind_for_class as mrwe6_kernel_required_intent_kind_for_class,
    resolution_commit_decision_from_facts as mrwe6_kernel_resolution_commit_decision_from_facts,
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
        let intent = mrwe6_action_index_intent(event);
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
        let intent = mrwe6_action_index_intent(event);
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
        mrwe6_idempotent_duplicate_retry_from_facts(true, mrwe6_event_class(event), marker_present)
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
                let Some(key) = mrwe6_action_index_key_for_intent(intent)? else {
                    return Ok(());
                };
                batch.insert(&self.index_action, key.to_vec(), Vec::<u8>::new());
                Ok(())
            }
            ActionIndexIntent::Delete { .. } => {
                let Some(key) = mrwe6_action_index_key_for_intent(intent)? else {
                    return Ok(());
                };
                batch.remove(&self.index_action, key.to_vec());
                Ok(())
            }
        }
    }
}
