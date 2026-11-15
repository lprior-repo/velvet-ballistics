use super::TestResult;
use std::sync::{Arc, Mutex};
use vb_runtime::{
    RuntimeError, RuntimeResult,
    journal::{RuntimeJournal, RuntimeJournalEvent, StorageRuntimeJournal},
};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

type Rows = Vec<(String, Vec<u8>, Vec<u8>)>;
#[derive(Clone, Copy)]
pub(super) enum Mode {
    Reject,
    Conflict,
    Observe,
}

pub(super) struct CutJournal {
    inner: StorageRuntimeJournal,
    journal: Arc<FjallJournal>,
    mode: Mode,
    batch: Mutex<Option<(EventSeq, Vec<RuntimeJournalEvent>)>>,
    observed: Mutex<Option<(EventSeq, Vec<RuntimeJournalEvent>)>>,
    expected: Mutex<Rows>,
}

impl CutJournal {
    pub(super) fn new(journal: Arc<FjallJournal>, mode: Mode, original_cold_rows: Rows) -> Self {
        Self {
            inner: StorageRuntimeJournal::strict(journal.clone()),
            journal,
            mode,
            batch: Mutex::new(None),
            observed: Mutex::new(None),
            expected: Mutex::new(original_cold_rows),
        }
    }
    pub(super) fn journal(&self) -> Arc<FjallJournal> {
        self.journal.clone()
    }
    pub(super) fn captured(&self) -> TestResult<(EventSeq, Vec<RuntimeJournalEvent>)> {
        self.batch
            .lock()
            .map_err(|_| "batch capture poisoned")?
            .clone()
            .ok_or_else(|| "failure batch absent".into())
    }
    pub(super) fn observed_facts(&self) -> TestResult<(EventSeq, Vec<RuntimeJournalEvent>)> {
        self.observed
            .lock()
            .map_err(|_| "fact capture poisoned")?
            .clone()
            .ok_or_else(|| "facts absent".into())
    }
    pub(super) fn expected_after_injection(&self) -> TestResult<Rows> {
        Ok(self
            .expected
            .lock()
            .map_err(|_| "row capture poisoned")?
            .clone())
    }
    fn insert_hostile_collision(
        &self,
        events: &[RuntimeJournalEvent],
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        let third = events.get(2).ok_or(RuntimeError::UnsupportedOperation {
            operation: "fixture_atomic_retry_third_record_absent",
        })?;
        let third_seq = EventSeq::new(seq.get().checked_add(2).ok_or(RuntimeError::EncodeFailed)?);
        let collision = self
            .inner
            .convert_to_storage_event(third.clone(), third_seq)?;
        let JournalEvent::ActionScheduledTicket { ticket, .. } = &collision else {
            return Err(RuntimeError::InvalidActionCompletion);
        };
        let index = vb_storage::keys::index_action_key(ticket.action, ticket.run, ticket.step)?;
        assert!(self.journal.has_action_index_entry(&index)?);
        assert_eq!(self.journal.get_event_bytes(ticket.run, third_seq)?, None);
        // Hostile fixture delta only; not a partial successful retry commit.
        self.journal.append_strict(&collision)?;
        let value = self
            .journal
            .get_event_bytes(ticket.run, third_seq)?
            .ok_or(RuntimeError::RunNotFound)?;
        let key = vb_storage::keys::run_event_key(ticket.run, third_seq)?;
        let mut expected = self
            .expected
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;
        let original_index: Vec<_> = expected
            .iter()
            .filter(|(name, _, _)| name == "index_action")
            .collect();
        assert_eq!(
            original_index,
            vec![&(String::from("index_action"), index.to_vec(), Vec::new())]
        );
        expected.push((String::from("run_event"), key.to_vec(), value));
        expected.sort();
        assert!(self.journal.has_action_index_entry(&index)?);
        Ok(())
    }
}

impl RuntimeJournal for CutJournal {
    fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
        self.inner.durability_profile()
    }
    fn storage_journal(&self) -> Option<Arc<FjallJournal>> {
        self.inner.storage_journal()
    }
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        self.inner.append(event)
    }
    fn append_sequenced(&self, event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<()> {
        self.append_sequenced_batch(&[event], seq)
    }
    fn convert_to_storage_event(
        &self,
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<JournalEvent> {
        self.inner.convert_to_storage_event(event, seq)
    }
    fn append_admission_batch(
        &self,
        events: &[RuntimeJournalEvent],
        seq: EventSeq,
        inputs: vb_storage::RunInputsRecord,
    ) -> RuntimeResult<()> {
        self.inner.append_admission_batch(events, seq, inputs)
    }
    fn probe(&self) -> RuntimeResult<()> {
        self.inner.probe()
    }
    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        let mut observed = self
            .observed
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;
        let (_, facts) = observed.get_or_insert_with(|| (seq, Vec::new()));
        facts.extend_from_slice(events);
        drop(observed);
        if events.iter().any(|event| {
            matches!(
                event,
                RuntimeJournalEvent::ActionFailed { .. } | RuntimeJournalEvent::RunFailed { .. }
            )
        }) {
            *self
                .batch
                .lock()
                .map_err(|_| RuntimeError::JournalPoisoned)? = Some((seq, events.to_vec()));
            match self.mode {
                Mode::Reject => return Err(RuntimeError::JournalPoisoned),
                Mode::Conflict => self.insert_hostile_collision(events, seq)?,
                Mode::Observe => {}
            }
        }
        self.inner.append_sequenced_batch(events, seq)
    }
}
