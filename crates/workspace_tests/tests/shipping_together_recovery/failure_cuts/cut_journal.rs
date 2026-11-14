#![forbid(unsafe_code)]
//! Strict Fjall fault cut after ActionFailed and before handler entry.

use std::sync::{Arc, Mutex};

use vb_runtime::journal::{RuntimeJournal, RuntimeJournalEvent, StorageRuntimeJournal};
use vb_storage::{FjallJournal, JournalEvent};

enum CutStage {
    AwaitFailure,
    AwaitHandler,
    Fired,
}

pub(super) struct CutJournal {
    inner: StorageRuntimeJournal,
    stage: Mutex<CutStage>,
    handler: vb_core::StepIdx,
}

impl CutJournal {
    pub(super) fn new(journal: Arc<FjallJournal>, handler: vb_core::StepIdx) -> Self {
        Self {
            inner: StorageRuntimeJournal::strict(journal),
            stage: Mutex::new(CutStage::AwaitFailure),
            handler,
        }
    }

    fn has_failure(events: &[RuntimeJournalEvent]) -> bool {
        events
            .iter()
            .any(|event| matches!(event, RuntimeJournalEvent::ActionFailed { .. }))
    }

    fn has_handler_start(&self, events: &[RuntimeJournalEvent]) -> bool {
        events.iter().any(|event| {
            matches!(
                event,
                RuntimeJournalEvent::StepStarted { step, .. } if *step == self.handler
            )
        })
    }
}

impl RuntimeJournal for CutJournal {
    fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
        Some(vb_storage::DurabilityProfile::Strict)
    }

    fn append(&self, event: RuntimeJournalEvent) -> vb_runtime::RuntimeResult<()> {
        self.inner.append(event)
    }

    fn storage_journal(&self) -> Option<Arc<FjallJournal>> {
        self.inner.storage_journal()
    }

    fn append_sequenced(
        &self,
        event: RuntimeJournalEvent,
        seq: vb_storage::EventSeq,
    ) -> vb_runtime::RuntimeResult<()> {
        self.append_sequenced_batch(&[event], seq)
    }

    fn convert_to_storage_event(
        &self,
        event: RuntimeJournalEvent,
        seq: vb_storage::EventSeq,
    ) -> vb_runtime::RuntimeResult<JournalEvent> {
        self.inner.convert_to_storage_event(event, seq)
    }

    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        start_seq: vb_storage::EventSeq,
    ) -> vb_runtime::RuntimeResult<()> {
        let mut stage = self
            .stage
            .lock()
            .map_err(|_| vb_runtime::RuntimeError::JournalPoisoned)?;
        match *stage {
            CutStage::AwaitFailure => {
                self.inner.append_sequenced_batch(events, start_seq)?;
                if Self::has_failure(events) {
                    *stage = CutStage::AwaitHandler;
                }
            }
            CutStage::AwaitHandler if self.has_handler_start(events) => {
                *stage = CutStage::Fired;
                return Err(vb_runtime::RuntimeError::JournalPoisoned);
            }
            _ => self.inner.append_sequenced_batch(events, start_seq)?,
        }
        Ok(())
    }

    fn probe(&self) -> vb_runtime::RuntimeResult<()> {
        self.inner.probe()
    }
}
