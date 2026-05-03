//! Storage-backed journal adapters.

use indexmap::IndexMap;
use std::sync::{Arc, Mutex};
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::{
    DurabilityProfile, EventSeq, FjallJournal, JournalEvent, JournalWriterFlushReport,
    JournalWriterQueue,
};

use crate::{RuntimeError, RuntimeResult};

/// Runtime journal adapter that appends lifecycle events into `vb_storage`.
pub struct StorageRuntimeJournal {
    journal: Arc<FjallJournal>,
    next_seq_by_run: Mutex<IndexMap<RunId, EventSeq>>,
    profile: DurabilityProfile,
}

impl StorageRuntimeJournal {
    /// Creates an adapter that uses journaled appends without a per-event sync barrier.
    #[must_use]
    pub fn journaled(journal: Arc<FjallJournal>) -> Self {
        Self {
            journal,
            next_seq_by_run: Mutex::new(IndexMap::new()),
            profile: DurabilityProfile::Journaled,
        }
    }

    /// Creates an adapter that forces a strict durability barrier for every append.
    #[must_use]
    pub fn strict(journal: Arc<FjallJournal>) -> Self {
        Self {
            journal,
            next_seq_by_run: Mutex::new(IndexMap::new()),
            profile: DurabilityProfile::Strict,
        }
    }

    /// Creates a shared journaled adapter for direct runtime construction.
    #[must_use]
    pub fn shared_journaled(journal: Arc<FjallJournal>) -> crate::SharedRuntimeJournal {
        Arc::new(Self::journaled(journal))
    }

    /// Creates a shared strict adapter for direct runtime construction.
    #[must_use]
    pub fn shared_strict(journal: Arc<FjallJournal>) -> crate::SharedRuntimeJournal {
        Arc::new(Self::strict(journal))
    }

    fn append_storage_event(&self, event: &JournalEvent) -> RuntimeResult<()> {
        let result = if self.profile == DurabilityProfile::Strict {
            self.journal.append_strict(event)
        } else {
            self.journal.append_journaled(event)
        };
        result.map_err(|_| RuntimeError::StorageJournalAppendFailed)
    }

    fn run_storage_event(event: crate::RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
        match event {
            crate::RuntimeJournalEvent::RunSubmitted { run, workflow } => {
                Some(JournalEvent::RunAccepted { run, seq, workflow })
            }
            crate::RuntimeJournalEvent::RunFinished { run, result } => {
                Some(JournalEvent::RunFinished { run, seq, result })
            }
            crate::RuntimeJournalEvent::RunFailed { run } => {
                Some(JournalEvent::RunFailedEvent { run, seq })
            }
            crate::RuntimeJournalEvent::RunCancelled { run } => {
                Some(JournalEvent::RunCancelled { run, seq })
            }
            crate::RuntimeJournalEvent::StepStarted { run, step } => {
                Some(JournalEvent::StepStarted { run, seq, step })
            }
            crate::RuntimeJournalEvent::StepSucceeded { run, step, output } => {
                Some(JournalEvent::StepSucceeded { run, seq, step, output })
            }
            crate::RuntimeJournalEvent::ActionScheduled { .. }
            | crate::RuntimeJournalEvent::ActionCompleted { .. }
            | crate::RuntimeJournalEvent::ActionFailed { .. }
            | crate::RuntimeJournalEvent::WaitScheduled { .. }
            | crate::RuntimeJournalEvent::WaitResolved { .. }
            | crate::RuntimeJournalEvent::AskScheduled { .. }
            | crate::RuntimeJournalEvent::AskAnswered { .. }
            | crate::RuntimeJournalEvent::SlotWritten { .. } => None,
        }
    }

    fn action_storage_event(
        event: crate::RuntimeJournalEvent,
        seq: EventSeq,
    ) -> Option<JournalEvent> {
        match event {
            crate::RuntimeJournalEvent::ActionScheduled { run, step, action } => {
                Some(JournalEvent::ActionScheduled {
                    run,
                    seq,
                    step,
                    action,
                })
            }
            crate::RuntimeJournalEvent::ActionCompleted { run, step, action } => {
                Some(JournalEvent::ActionCompletedEvent {
                    run,
                    seq,
                    step,
                    action,
                })
            }
            crate::RuntimeJournalEvent::ActionFailed { run, step, action } => {
                Some(JournalEvent::ActionFailedEvent {
                    run,
                    seq,
                    step,
                    action,
                })
            }
            crate::RuntimeJournalEvent::RunSubmitted { .. }
            | crate::RuntimeJournalEvent::RunFinished { .. }
            | crate::RuntimeJournalEvent::RunFailed { .. }
            | crate::RuntimeJournalEvent::RunCancelled { .. }
            | crate::RuntimeJournalEvent::WaitScheduled { .. }
            | crate::RuntimeJournalEvent::WaitResolved { .. }
            | crate::RuntimeJournalEvent::AskScheduled { .. }
            | crate::RuntimeJournalEvent::AskAnswered { .. }
            | crate::RuntimeJournalEvent::SlotWritten { .. }
            | crate::RuntimeJournalEvent::StepStarted { .. }
            | crate::RuntimeJournalEvent::StepSucceeded { .. } => None,
        }
    }

    fn boundary_storage_event(
        event: crate::RuntimeJournalEvent,
        seq: EventSeq,
    ) -> Option<JournalEvent> {
        match event {
            crate::RuntimeJournalEvent::WaitScheduled { run, step } => {
                Some(JournalEvent::WaitScheduledEvent { run, seq, step })
            }
            crate::RuntimeJournalEvent::WaitResolved { run, step } => {
                Some(JournalEvent::RetryScheduledEvent { run, seq, step })
            }
            crate::RuntimeJournalEvent::AskScheduled { run, step } => {
                Some(JournalEvent::AskScheduledEvent { run, seq, step })
            }
            crate::RuntimeJournalEvent::AskAnswered { run, step, .. } => {
                Some(JournalEvent::AskAnsweredEvent { run, seq, step })
            }
            crate::RuntimeJournalEvent::SlotWritten { run, slot } => {
                Some(JournalEvent::SlotWrittenEvent { run, seq, slot })
            }
            crate::RuntimeJournalEvent::RunSubmitted { .. }
            | crate::RuntimeJournalEvent::RunFinished { .. }
            | crate::RuntimeJournalEvent::RunFailed { .. }
            | crate::RuntimeJournalEvent::RunCancelled { .. }
            | crate::RuntimeJournalEvent::ActionScheduled { .. }
            | crate::RuntimeJournalEvent::ActionCompleted { .. }
            | crate::RuntimeJournalEvent::ActionFailed { .. }
            | crate::RuntimeJournalEvent::StepStarted { .. }
            | crate::RuntimeJournalEvent::StepSucceeded { .. } => None,
        }
    }

    /// Maps a runtime journal event to a storage journal event.
    pub fn storage_event(event: crate::RuntimeJournalEvent, seq: EventSeq) -> JournalEvent {
        if let Some(storage_event) = Self::run_storage_event(event, seq) {
            return storage_event;
        }
        if let Some(storage_event) = Self::action_storage_event(event, seq) {
            return storage_event;
        }
        match Self::boundary_storage_event(event, seq) {
            Some(storage_event) => storage_event,
            None => JournalEvent::RunFailedEvent {
                run: event.run_id(),
                seq,
            },
        }
    }
}

impl crate::RuntimeJournal for StorageRuntimeJournal {
    fn append(&self, event: crate::RuntimeJournalEvent) -> RuntimeResult<()> {
        let mut sequences = self
            .next_seq_by_run
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;
        let seq = current_seq(&sequences, event.run_id());
        let next = next_seq(seq)?;
        let storage_event = Self::storage_event(event, seq);
        self.append_storage_event(&storage_event)?;
        sequences.insert(event.run_id(), next);
        Ok(())
    }
}

/// Runtime journal adapter that stages lifecycle events through `JournalWriterQueue`.
pub struct QueuedStorageRuntimeJournal {
    journal: Arc<FjallJournal>,
    queue: Arc<JournalWriterQueue>,
    next_seq_by_run: Mutex<IndexMap<RunId, EventSeq>>,
    profile: DurabilityProfile,
}

impl QueuedStorageRuntimeJournal {
    /// Creates a queued adapter that enqueues journaled requests.
    #[must_use]
    pub fn journaled(journal: Arc<FjallJournal>, queue: Arc<JournalWriterQueue>) -> Self {
        Self {
            journal,
            queue,
            next_seq_by_run: Mutex::new(IndexMap::new()),
            profile: DurabilityProfile::Journaled,
        }
    }

    /// Creates a queued adapter that enqueues strict requests.
    #[must_use]
    pub fn strict(journal: Arc<FjallJournal>, queue: Arc<JournalWriterQueue>) -> Self {
        Self {
            journal,
            queue,
            next_seq_by_run: Mutex::new(IndexMap::new()),
            profile: DurabilityProfile::Strict,
        }
    }

    /// Creates a shared queued journaled adapter for direct runtime construction.
    #[must_use]
    pub fn shared_journaled(
        journal: Arc<FjallJournal>,
        queue: Arc<JournalWriterQueue>,
    ) -> crate::SharedRuntimeJournal {
        Arc::new(Self::journaled(journal, queue))
    }

    /// Creates a shared queued strict adapter for direct runtime construction.
    #[must_use]
    pub fn shared_strict(
        journal: Arc<FjallJournal>,
        queue: Arc<JournalWriterQueue>,
    ) -> crate::SharedRuntimeJournal {
        Arc::new(Self::strict(journal, queue))
    }

    /// Flushes a bounded batch from the queue into Fjall.
    pub fn flush_batch(&self) -> RuntimeResult<JournalWriterFlushReport> {
        self.queue
            .flush_batch(&self.journal)
            .map_err(|_| RuntimeError::StorageJournalAppendFailed)
    }

    /// Drains all queued journal writes into Fjall.
    pub fn drain_all(&self) -> RuntimeResult<JournalWriterFlushReport> {
        self.queue
            .drain_all(&self.journal)
            .map_err(|_| RuntimeError::StorageJournalAppendFailed)
    }
}

impl crate::RuntimeJournal for QueuedStorageRuntimeJournal {
    fn append(&self, event: crate::RuntimeJournalEvent) -> RuntimeResult<()> {
        if self.profile == DurabilityProfile::Strict {
            return Err(RuntimeError::UnsupportedAsyncStrictAck);
        }
        let mut sequences = self
            .next_seq_by_run
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;
        let seq = current_seq(&sequences, event.run_id());
        let next = next_seq(seq)?;
        let storage_event = StorageRuntimeJournal::storage_event(event, seq);
        let result = self.queue.enqueue_journaled(storage_event);
        result.map_err(|_| RuntimeError::StorageJournalAppendFailed)?;
        sequences.insert(event.run_id(), next);
        Ok(())
    }

    fn drain_for_shutdown(&self) -> RuntimeResult<JournalWriterFlushReport> {
        self.drain_all()
    }
}

fn current_seq(sequences: &IndexMap<RunId, EventSeq>, run: RunId) -> EventSeq {
    match sequences.get(&run).copied() {
        Some(value) => value,
        None => EventSeq::new(0),
    }
}

fn next_seq(seq: EventSeq) -> RuntimeResult<EventSeq> {
    seq.get()
        .checked_add(1)
        .map(EventSeq::new)
        .ok_or(RuntimeError::StorageJournalAppendFailed)
}
