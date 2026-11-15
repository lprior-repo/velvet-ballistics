use std::sync::{Arc, Mutex};

use vb_runtime::journal::{RuntimeJournal, RuntimeJournalEvent, StorageRuntimeJournal};
use vb_runtime::{RuntimeError, RuntimeResult};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

enum Cut {
    Completion,
    AfterCompletion,
    Resume,
    AfterResume,
    Finished,
}

pub(super) struct CutJournal {
    inner: StorageRuntimeJournal,
    cut: Mutex<Cut>,
}

impl CutJournal {
    pub(super) fn new(journal: Arc<FjallJournal>) -> Self {
        Self {
            inner: StorageRuntimeJournal::strict(journal),
            cut: Mutex::new(Cut::Completion),
        }
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

    fn append_admission_batch(
        &self,
        events: &[RuntimeJournalEvent],
        seq: EventSeq,
        inputs: vb_storage::RunInputsRecord,
    ) -> RuntimeResult<()> {
        self.inner.append_admission_batch(events, seq, inputs)
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

    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        let mut cut = self.cut.lock().map_err(|_| RuntimeError::JournalPoisoned)?;
        match *cut {
            Cut::AfterCompletion => {
                *cut = Cut::Resume;
                return Err(RuntimeError::JournalPoisoned);
            }
            Cut::AfterResume => {
                *cut = Cut::Finished;
                return Err(RuntimeError::JournalPoisoned);
            }
            _ => {}
        }
        self.inner.append_sequenced_batch(events, seq)?;
        match *cut {
            Cut::Completion
                if events.iter().any(|event| {
                    matches!(event, RuntimeJournalEvent::ActionCompletedEnvelope { .. })
                }) =>
            {
                *cut = Cut::AfterCompletion
            }
            Cut::Resume
                if events
                    .iter()
                    .any(|event| matches!(event, RuntimeJournalEvent::Resumed { .. })) =>
            {
                *cut = Cut::AfterResume
            }
            _ => {}
        }
        Ok(())
    }

    fn probe(&self) -> RuntimeResult<()> {
        self.inner.probe()
    }
}
