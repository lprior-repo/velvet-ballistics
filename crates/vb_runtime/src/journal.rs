//! Runtime-local journal append port.

use indexmap::IndexMap;
use std::sync::{Arc, Mutex};
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::{
    DurabilityProfile, EventSeq, FjallJournal, JournalEvent, JournalWriterFlushReport,
    JournalWriterQueue,
};

use crate::{RuntimeError, RuntimeResult};

/// Minimal lifecycle event emitted by the runtime before a durable store is wired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeJournalEvent {
    /// Run was accepted by the runtime.
    RunSubmitted {
        /// Run identifier.
        run: RunId,
        /// Compiled workflow digest admitted for this run.
        workflow: WorkflowDigest,
    },
    /// Run admission metadata was recorded after admission control succeeded.
    RunAdmission {
        /// Admission record for this run.
        admission: crate::admission::RunAdmission,
    },
    /// Run reached a successful terminal state.
    RunFinished {
        /// Run identifier.
        run: RunId,
        /// Result slot selected by the terminal finish node.
        result: SlotIdx,
    },
    /// Run reached a failed terminal state.
    RunFailed {
        /// Run identifier.
        run: RunId,
    },
    /// Run was cancelled by the caller.
    RunCancelled {
        /// Run identifier.
        run: RunId,
    },
    /// Action was scheduled and handed to the external action boundary.
    ActionScheduled {
        /// Run identifier.
        run: RunId,
        /// Step that scheduled the action.
        step: StepIdx,
        /// Action identifier.
        action: ActionId,
    },
    /// Action completed successfully.
    ActionCompleted {
        /// Run identifier.
        run: RunId,
        /// Step that received completion.
        step: StepIdx,
        /// Action identifier.
        action: ActionId,
    },
    /// Action failed at the external action boundary.
    ActionFailed {
        /// Run identifier.
        run: RunId,
        /// Step that received failure.
        step: StepIdx,
        /// Action identifier.
        action: ActionId,
    },
    /// Wait was scheduled and the run suspended.
    WaitScheduled {
        /// Run identifier.
        run: RunId,
        /// Step that scheduled the wait.
        step: StepIdx,
    },
    /// Wait was resolved by an external timer.
    WaitResolved {
        /// Run identifier.
        run: RunId,
        /// Step that resumed from waiting.
        step: StepIdx,
    },
    /// Ask was scheduled and the run suspended.
    AskScheduled {
        /// Run identifier.
        run: RunId,
        /// Step that scheduled the ask.
        step: StepIdx,
    },
    /// Ask was answered and wrote a slot.
    AskAnswered {
        /// Run identifier.
        run: RunId,
        /// Step that scheduled the ask.
        step: StepIdx,
        /// Slot that received the answer.
        slot: SlotIdx,
    },
    /// Slot was written by an external runtime boundary.
    SlotWritten {
        /// Run identifier.
        run: RunId,
        /// Slot written by the event.
        slot: SlotIdx,
        /// Encoded slot value bytes (postcard-encoded `SlotValue`).
        value: Vec<u8>,
        /// Encoded frame extra data captured with this slot write.
        extra: Option<Vec<u8>>,
    },
    /// Deterministic step began execution.
    StepStarted {
        /// Run identifier.
        run: RunId,
        /// Step index.
        step: StepIdx,
    },
    /// Deterministic step completed and wrote an output slot.
    StepSucceeded {
        /// Run identifier.
        run: RunId,
        /// Step index.
        step: StepIdx,
        /// Output slot index.
        output: SlotIdx,
    },
}

impl RuntimeJournalEvent {
    /// Run identifier carried by this runtime event.
    #[must_use]
    pub fn run_id(&self) -> RunId {
        match self {
            Self::RunSubmitted { run, .. }
            | Self::RunFinished { run, .. }
            | Self::RunFailed { run }
            | Self::RunCancelled { run }
            | Self::ActionScheduled { run, .. }
            | Self::ActionCompleted { run, .. }
            | Self::ActionFailed { run, .. }
            | Self::WaitScheduled { run, .. }
            | Self::WaitResolved { run, .. }
            | Self::AskScheduled { run, .. }
            | Self::AskAnswered { run, .. }
            | Self::SlotWritten { run, .. }
            | Self::StepStarted { run, .. }
            | Self::StepSucceeded { run, .. } => *run,
            Self::RunAdmission { admission } => admission.run_id(),
        }
    }
}

/// Append-only port used by runtime shards for lifecycle journaling.
pub trait RuntimeJournal: Send + Sync {
    /// Appends a lifecycle event.
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()>;

    /// Drains queued durable writes during graceful shutdown.
    fn drain_for_shutdown(&self) -> RuntimeResult<JournalWriterFlushReport> {
        Ok(JournalWriterFlushReport {
            drained: 0,
            written: 0,
        })
    }
}

/// Shared journal trait object.
pub type SharedRuntimeJournal = Arc<dyn RuntimeJournal>;

/// Journal implementation that intentionally drops all events.
#[derive(Debug, Default)]
pub struct NoopRuntimeJournal;

impl NoopRuntimeJournal {
    /// Creates a shared noop journal.
    #[must_use]
    pub fn shared() -> SharedRuntimeJournal {
        Arc::new(Self)
    }
}

impl RuntimeJournal for NoopRuntimeJournal {
    fn append(&self, _event: RuntimeJournalEvent) -> RuntimeResult<()> {
        Ok(())
    }
}

/// In-memory journal useful for tests and volatile embeddings.
#[derive(Debug, Default)]
pub struct VolatileRuntimeJournal {
    events: Mutex<Vec<RuntimeJournalEvent>>,
}

impl VolatileRuntimeJournal {
    /// Creates an empty volatile journal.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    /// Creates a shared volatile journal.
    #[must_use]
    pub fn shared() -> SharedRuntimeJournal {
        Arc::new(Self::new())
    }

    /// Returns a point-in-time copy of appended events.
    pub fn snapshot(&self) -> RuntimeResult<Vec<RuntimeJournalEvent>> {
        let events = self
            .events
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;
        Ok(events.clone())
    }
}

/// Runtime journal selection using explicit storage durability profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeJournalConfig {
    profile: DurabilityProfile,
}

impl RuntimeJournalConfig {
    /// Creates a runtime journal configuration for a durability profile.
    #[must_use]
    pub const fn new(profile: DurabilityProfile) -> Self {
        Self { profile }
    }

    /// Selected durability profile.
    #[must_use]
    pub const fn profile(self) -> DurabilityProfile {
        self.profile
    }

    /// Builds a shared runtime journal for this profile.
    #[must_use]
    pub fn shared_journal(
        self,
        journal: Arc<FjallJournal>,
        queue: Arc<JournalWriterQueue>,
    ) -> SharedRuntimeJournal {
        match self.profile {
            DurabilityProfile::Volatile => VolatileRuntimeJournal::shared(),
            DurabilityProfile::Journaled => {
                QueuedStorageRuntimeJournal::shared_journaled(journal, queue)
            }
            DurabilityProfile::Strict => StorageRuntimeJournal::shared_strict(journal),
        }
    }
}

impl RuntimeJournal for VolatileRuntimeJournal {
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        let mut events = self
            .events
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;
        events.push(event);
        Ok(())
    }
}

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
    pub fn shared_journaled(journal: Arc<FjallJournal>) -> SharedRuntimeJournal {
        Arc::new(Self::journaled(journal))
    }

    /// Creates a shared strict adapter for direct runtime construction.
    #[must_use]
    pub fn shared_strict(journal: Arc<FjallJournal>) -> SharedRuntimeJournal {
        Arc::new(Self::strict(journal))
    }

    fn append_storage_event(&self, event: &JournalEvent) -> RuntimeResult<()> {
        let result = if self.profile == DurabilityProfile::Strict {
            self.journal.append_strict(event)
        } else {
            self.journal.append_journaled(event)
        };
        result.map_err(RuntimeError::from)
    }

    fn run_storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
        match event {
            RuntimeJournalEvent::RunSubmitted { run, workflow } => {
                Some(JournalEvent::RunAccepted { run, seq, workflow })
            }
            RuntimeJournalEvent::RunAdmission { admission } => Some(JournalEvent::RunAdmission {
                run: admission.run_id(),
                seq,
                artifact_digest: admission.artifact_digest(),
                granted_capabilities: admission.granted_capabilities().clone(),
                policy: admission.policy(),
            }),
            RuntimeJournalEvent::RunFinished { run, result } => {
                Some(JournalEvent::RunFinished { run, seq, result })
            }
            RuntimeJournalEvent::RunFailed { run } => {
                Some(JournalEvent::RunFailedEvent { run, seq })
            }
            RuntimeJournalEvent::RunCancelled { run } => {
                Some(JournalEvent::RunCancelled { run, seq })
            }
            RuntimeJournalEvent::StepStarted { run, step } => {
                Some(JournalEvent::StepStarted { run, seq, step })
            }
            RuntimeJournalEvent::StepSucceeded { run, step, output } => {
                Some(JournalEvent::StepSucceeded {
                    run,
                    seq,
                    step,
                    output,
                })
            }
            RuntimeJournalEvent::ActionScheduled { .. }
            | RuntimeJournalEvent::ActionCompleted { .. }
            | RuntimeJournalEvent::ActionFailed { .. }
            | RuntimeJournalEvent::WaitScheduled { .. }
            | RuntimeJournalEvent::WaitResolved { .. }
            | RuntimeJournalEvent::AskScheduled { .. }
            | RuntimeJournalEvent::AskAnswered { .. }
            | RuntimeJournalEvent::SlotWritten { .. } => None,
        }
    }

    fn action_storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
        match event {
            RuntimeJournalEvent::ActionScheduled { run, step, action } => {
                Some(JournalEvent::ActionScheduled {
                    run,
                    seq,
                    step,
                    action,
                })
            }
            RuntimeJournalEvent::ActionCompleted { run, step, action } => {
                Some(JournalEvent::ActionCompletedEvent {
                    run,
                    seq,
                    step,
                    action,
                })
            }
            RuntimeJournalEvent::ActionFailed { run, step, action } => {
                Some(JournalEvent::ActionFailedEvent {
                    run,
                    seq,
                    step,
                    action,
                })
            }
            RuntimeJournalEvent::RunSubmitted { .. }
            | RuntimeJournalEvent::RunAdmission { .. }
            | RuntimeJournalEvent::RunFinished { .. }
            | RuntimeJournalEvent::RunFailed { .. }
            | RuntimeJournalEvent::RunCancelled { .. }
            | RuntimeJournalEvent::WaitScheduled { .. }
            | RuntimeJournalEvent::WaitResolved { .. }
            | RuntimeJournalEvent::AskScheduled { .. }
            | RuntimeJournalEvent::AskAnswered { .. }
            | RuntimeJournalEvent::SlotWritten { .. }
            | RuntimeJournalEvent::StepStarted { .. }
            | RuntimeJournalEvent::StepSucceeded { .. } => None,
        }
    }

    fn boundary_storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> Option<JournalEvent> {
        match event {
            RuntimeJournalEvent::WaitScheduled { run, step } => {
                Some(JournalEvent::WaitScheduledEvent { run, seq, step })
            }
            RuntimeJournalEvent::WaitResolved { run, step } => {
                Some(JournalEvent::RetryScheduledEvent { run, seq, step })
            }
            RuntimeJournalEvent::AskScheduled { run, step } => {
                Some(JournalEvent::AskScheduledEvent { run, seq, step })
            }
            RuntimeJournalEvent::AskAnswered { run, step, .. } => {
                Some(JournalEvent::AskAnsweredEvent { run, seq, step })
            }
            RuntimeJournalEvent::SlotWritten {
                run,
                slot,
                value,
                extra,
            } => Some(JournalEvent::SlotWrittenEvent {
                run,
                seq,
                slot,
                value: Some(value),
                extra,
            }),
            RuntimeJournalEvent::RunSubmitted { .. }
            | RuntimeJournalEvent::RunAdmission { .. }
            | RuntimeJournalEvent::RunFinished { .. }
            | RuntimeJournalEvent::RunFailed { .. }
            | RuntimeJournalEvent::RunCancelled { .. }
            | RuntimeJournalEvent::ActionScheduled { .. }
            | RuntimeJournalEvent::ActionCompleted { .. }
            | RuntimeJournalEvent::ActionFailed { .. }
            | RuntimeJournalEvent::StepStarted { .. }
            | RuntimeJournalEvent::StepSucceeded { .. } => None,
        }
    }

    fn storage_event(event: RuntimeJournalEvent, seq: EventSeq) -> JournalEvent {
        if let Some(storage_event) = Self::run_storage_event(event.clone(), seq) {
            return storage_event;
        }
        if let Some(storage_event) = Self::action_storage_event(event.clone(), seq) {
            return storage_event;
        }
        match Self::boundary_storage_event(event.clone(), seq) {
            Some(storage_event) => storage_event,
            None => JournalEvent::RunFailedEvent {
                run: event.run_id(),
                seq,
            },
        }
    }
}

impl RuntimeJournal for StorageRuntimeJournal {
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        let run_id = event.run_id();
        let mut sequences = self
            .next_seq_by_run
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;
        let seq = current_seq(&sequences, run_id);
        let next = next_seq(seq)?;
        let storage_event = Self::storage_event(event, seq);
        self.append_storage_event(&storage_event)?;
        sequences.insert(run_id, next);
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
    ) -> SharedRuntimeJournal {
        Arc::new(Self::journaled(journal, queue))
    }

    /// Creates a shared queued strict adapter for direct runtime construction.
    #[must_use]
    pub fn shared_strict(
        journal: Arc<FjallJournal>,
        queue: Arc<JournalWriterQueue>,
    ) -> SharedRuntimeJournal {
        Arc::new(Self::strict(journal, queue))
    }

    /// Flushes a bounded batch from the queue into Fjall.
    pub fn flush_batch(&self) -> RuntimeResult<JournalWriterFlushReport> {
        self.queue
            .flush_batch(&self.journal)
            .map_err(RuntimeError::from)
    }

    /// Drains all queued journal writes into Fjall.
    pub fn drain_all(&self) -> RuntimeResult<JournalWriterFlushReport> {
        self.queue
            .drain_all(&self.journal)
            .map_err(RuntimeError::from)
    }
}

impl RuntimeJournal for QueuedStorageRuntimeJournal {
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        if self.profile == DurabilityProfile::Strict {
            return Err(RuntimeError::UnsupportedAsyncStrictAck);
        }
        let run_id = event.run_id();
        let mut sequences = self
            .next_seq_by_run
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;
        let seq = current_seq(&sequences, run_id);
        let next = next_seq(seq)?;
        let storage_event = StorageRuntimeJournal::storage_event(event, seq);
        let result = self.queue.enqueue_journaled(storage_event);
        result.map_err(RuntimeError::from)?;
        sequences.insert(run_id, next);
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
        .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))
}

#[cfg(test)]
mod tests {
    use super::{
        QueuedStorageRuntimeJournal, RuntimeJournal, RuntimeJournalConfig, RuntimeJournalEvent,
        StorageRuntimeJournal,
    };
    use crate::runtime::Runtime;
    use crate::shard::ShardConfig;
    use std::num::NonZeroUsize;
    use std::sync::Arc;
    use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
    };
    use vb_storage::{
        DurabilityProfile, EventSeq, FjallJournal, JournalEvent, JournalWriterQueue, StorageLimits,
    };

    fn single_finish_workflow(workflow: WorkflowDigest) -> Result<CompiledWorkflow, String> {
        let node = CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: Box::from("single_finish"),
            digest: workflow,
            nodes: Box::from([node]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::ZERO,
            step_names: Box::from([]),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
    }

    fn temp_journal() -> Result<(tempfile::TempDir, Arc<FjallJournal>), String> {
        let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let journal = FjallJournal::open(dir.path(), None).map_err(|error| error.to_string())?;
        Ok((dir, Arc::new(journal)))
    }

    fn journal_queue(
        capacity: usize,
        batch_size: usize,
    ) -> Result<Arc<JournalWriterQueue>, String> {
        JournalWriterQueue::new(capacity, batch_size, StorageLimits::DEFAULT)
            .map(Arc::new)
            .map_err(|error| error.to_string())
    }

    fn require_ok<T>(result: Result<T, String>, context: &'static str) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                assert!(false, "{context}: {error}");
                None
            }
        }
    }

    #[test]
    fn storage_runtime_journal_maps_lifecycle_events_in_sequence() {
        let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
            return;
        };
        let adapter = StorageRuntimeJournal::journaled(journal.clone());
        let run = RunId::new(41);
        let workflow = WorkflowDigest::from_bytes([7; 32]);

        assert_eq!(
            adapter.append(RuntimeJournalEvent::RunSubmitted { run, workflow }),
            Ok(())
        );
        assert_eq!(
            adapter.append(RuntimeJournalEvent::RunFinished {
                run,
                result: SlotIdx::new(3),
            }),
            Ok(())
        );

        let Some(events) = require_ok(
            journal
                .events_for_run(run)
                .map_err(|error| error.to_string()),
            "events read",
        ) else {
            return;
        };
        assert_eq!(
            events,
            vec![
                JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow,
                },
                JournalEvent::RunFinished {
                    run,
                    seq: EventSeq::new(1),
                    result: SlotIdx::new(3),
                },
            ]
        );
    }

    #[test]
    fn storage_runtime_journal_maps_run_admission_event() {
        let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
            return;
        };
        let adapter = StorageRuntimeJournal::journaled(journal.clone());
        let run = RunId::new(45);
        let workflow = WorkflowDigest::from_bytes([9; 32]);
        let admission = crate::admission::RunAdmission::new(
            workflow,
            run,
            vb_core::capability::CapabilitySet::empty(),
            vb_core::policy::RuntimePolicy::Relaxed,
        );

        assert_eq!(
            adapter.append(RuntimeJournalEvent::RunAdmission { admission }),
            Ok(())
        );

        let Some(events) = require_ok(
            journal
                .events_for_run(run)
                .map_err(|error| error.to_string()),
            "admission events read",
        ) else {
            return;
        };
        assert_eq!(
            events,
            vec![vb_storage::JournalEvent::RunAdmission {
                run,
                seq: EventSeq::new(0),
                artifact_digest: workflow,
                granted_capabilities: vb_core::capability::CapabilitySet::empty(),
                policy: vb_core::policy::RuntimePolicy::Relaxed,
            }]
        );
    }

    #[test]
    fn storage_runtime_journal_maps_cancelled_and_failed_events() {
        let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
            return;
        };
        let adapter = StorageRuntimeJournal::journaled(journal.clone());
        let run = RunId::new(42);
        let workflow = WorkflowDigest::from_bytes([8; 32]);

        assert_eq!(
            adapter.append(RuntimeJournalEvent::RunSubmitted { run, workflow }),
            Ok(())
        );
        assert_eq!(
            adapter.append(RuntimeJournalEvent::RunCancelled { run }),
            Ok(())
        );

        let Some(events) = require_ok(
            journal
                .events_for_run(run)
                .map_err(|error| error.to_string()),
            "cancelled events read",
        ) else {
            return;
        };
        assert_eq!(
            events,
            vec![
                JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow,
                },
                JournalEvent::RunCancelled {
                    run,
                    seq: EventSeq::new(1),
                },
            ]
        );

        let failed_run = RunId::new(43);
        assert_eq!(
            adapter.append(RuntimeJournalEvent::RunFailed { run: failed_run }),
            Ok(())
        );
        let Some(failed_events) = require_ok(
            journal
                .events_for_run(failed_run)
                .map_err(|error| error.to_string()),
            "failed events read",
        ) else {
            return;
        };
        assert_eq!(
            failed_events,
            vec![JournalEvent::RunFailedEvent {
                run: failed_run,
                seq: EventSeq::new(0),
            }]
        );
    }

    #[test]
    fn storage_runtime_journal_maps_action_wait_and_ask_events() {
        let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
            return;
        };
        let adapter = StorageRuntimeJournal::journaled(journal.clone());
        let run = RunId::new(44);

        assert_eq!(
            adapter.append(RuntimeJournalEvent::ActionScheduled {
                run,
                step: StepIdx::new(1),
                action: ActionId::new(2),
            }),
            Ok(())
        );
        assert_eq!(
            adapter.append(RuntimeJournalEvent::ActionCompleted {
                run,
                step: StepIdx::new(1),
                action: ActionId::new(2),
            }),
            Ok(())
        );
        assert_eq!(
            adapter.append(RuntimeJournalEvent::WaitScheduled {
                run,
                step: StepIdx::new(3),
            }),
            Ok(())
        );
        assert_eq!(
            adapter.append(RuntimeJournalEvent::WaitResolved {
                run,
                step: StepIdx::new(3),
            }),
            Ok(())
        );
        assert_eq!(
            adapter.append(RuntimeJournalEvent::AskScheduled {
                run,
                step: StepIdx::new(4),
            }),
            Ok(())
        );
        assert_eq!(
            adapter.append(RuntimeJournalEvent::AskAnswered {
                run,
                step: StepIdx::new(4),
                slot: SlotIdx::new(5),
            }),
            Ok(())
        );
        assert_eq!(
            adapter.append(RuntimeJournalEvent::SlotWritten {
                run,
                slot: SlotIdx::new(5),
                value: Vec::new(),
                extra: None,
            }),
            Ok(())
        );

        let Some(events) = require_ok(
            journal
                .events_for_run(run)
                .map_err(|error| error.to_string()),
            "action/wait/ask events read",
        ) else {
            return;
        };
        assert_eq!(
            events,
            vec![
                JournalEvent::ActionScheduled {
                    run,
                    seq: EventSeq::new(0),
                    step: StepIdx::new(1),
                    action: ActionId::new(2),
                },
                JournalEvent::ActionCompletedEvent {
                    run,
                    seq: EventSeq::new(1),
                    step: StepIdx::new(1),
                    action: ActionId::new(2),
                },
                JournalEvent::WaitScheduledEvent {
                    run,
                    seq: EventSeq::new(2),
                    step: StepIdx::new(3),
                },
                JournalEvent::RetryScheduledEvent {
                    run,
                    seq: EventSeq::new(3),
                    step: StepIdx::new(3),
                },
                JournalEvent::AskScheduledEvent {
                    run,
                    seq: EventSeq::new(4),
                    step: StepIdx::new(4),
                },
                JournalEvent::AskAnsweredEvent {
                    run,
                    seq: EventSeq::new(5),
                    step: StepIdx::new(4),
                },
                JournalEvent::SlotWrittenEvent {
                    run,
                    seq: EventSeq::new(6),
                    slot: SlotIdx::new(5),
                    value: Some(Vec::new()),
                    extra: None,
                },
            ]
        );
    }

    #[test]
    fn queued_storage_runtime_journal_flushes_mapped_events_to_fjall() {
        let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
            return;
        };
        let Some(queue) = require_ok(journal_queue(4, 2), "journal queue opens") else {
            return;
        };
        let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue);
        let run = RunId::new(45);
        let workflow = WorkflowDigest::from_bytes([9; 32]);

        assert_eq!(
            adapter.append(RuntimeJournalEvent::RunSubmitted { run, workflow }),
            Ok(())
        );
        assert_eq!(
            adapter.append(RuntimeJournalEvent::ActionScheduled {
                run,
                step: StepIdx::new(1),
                action: ActionId::new(2),
            }),
            Ok(())
        );
        assert_eq!(
            adapter.append(RuntimeJournalEvent::RunFinished {
                run,
                result: SlotIdx::new(3),
            }),
            Ok(())
        );

        assert!(matches!(journal.events_for_run(run), Ok(events) if events.is_empty()));
        assert!(
            matches!(adapter.flush_batch(), Ok(report) if report.drained == 2 && report.written == 2)
        );
        assert!(
            matches!(adapter.flush_batch(), Ok(report) if report.drained == 1 && report.written == 1)
        );

        let Some(events) = require_ok(
            journal
                .events_for_run(run)
                .map_err(|error| error.to_string()),
            "queued events read",
        ) else {
            return;
        };
        assert_eq!(
            events,
            vec![
                JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow,
                },
                JournalEvent::ActionScheduled {
                    run,
                    seq: EventSeq::new(1),
                    step: StepIdx::new(1),
                    action: ActionId::new(2),
                },
                JournalEvent::RunFinished {
                    run,
                    seq: EventSeq::new(2),
                    result: SlotIdx::new(3),
                },
            ]
        );
    }

    #[test]
    fn runtime_journal_config_maps_profiles_to_volatile_journaled_and_strict_behavior() {
        let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
            return;
        };
        let Some(volatile_queue) = require_ok(journal_queue(4, 4), "volatile queue opens") else {
            return;
        };
        let run = RunId::new(47);
        let workflow = WorkflowDigest::from_bytes([10; 32]);

        let volatile = RuntimeJournalConfig::new(DurabilityProfile::Volatile)
            .shared_journal(journal.clone(), volatile_queue.clone());
        assert_eq!(
            volatile.append(RuntimeJournalEvent::RunSubmitted { run, workflow }),
            Ok(())
        );
        assert!(matches!(journal.events_for_run(run), Ok(events) if events.is_empty()));
        assert!(matches!(
            volatile_queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 0 && counts.strict == 0
        ));

        let Some(journaled_queue) = require_ok(journal_queue(4, 4), "journaled queue opens") else {
            return;
        };
        let journaled = RuntimeJournalConfig::new(DurabilityProfile::Journaled)
            .shared_journal(journal.clone(), journaled_queue.clone());
        assert_eq!(
            journaled.append(RuntimeJournalEvent::RunCancelled { run }),
            Ok(())
        );
        assert!(matches!(
            journaled_queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 1 && counts.strict == 0
        ));

        let Some(strict_queue) = require_ok(journal_queue(4, 4), "strict queue opens") else {
            return;
        };
        let strict_run = RunId::new(48);
        let strict = RuntimeJournalConfig::new(DurabilityProfile::Strict)
            .shared_journal(journal.clone(), strict_queue.clone());
        assert_eq!(
            strict.append(RuntimeJournalEvent::RunFailed { run: strict_run }),
            Ok(())
        );
        assert!(matches!(
            strict_queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 0 && counts.strict == 0
        ));
        assert!(matches!(
            journal.events_for_run(strict_run),
            Ok(events) if matches!(events.as_slice(), [JournalEvent::RunFailedEvent { seq, .. }] if *seq == EventSeq::new(0))
        ));
    }

    #[test]
    fn queued_storage_runtime_journal_drain_all_flushes_past_batch_size() {
        let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
            return;
        };
        let Some(queue) = require_ok(journal_queue(8, 2), "journal queue opens") else {
            return;
        };
        let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue.clone());
        let run = RunId::new(48);
        let workflow = WorkflowDigest::from_bytes([11; 32]);

        assert_eq!(
            adapter.append(RuntimeJournalEvent::RunSubmitted { run, workflow }),
            Ok(())
        );
        assert_eq!(
            adapter.append(RuntimeJournalEvent::RunCancelled { run }),
            Ok(())
        );
        assert_eq!(
            adapter.append(RuntimeJournalEvent::RunFailed { run }),
            Ok(())
        );
        assert!(matches!(
            queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 3 && counts.strict == 0
        ));

        assert!(matches!(
            adapter.drain_all(),
            Ok(report) if report.drained == 3 && report.written == 3
        ));
        assert!(matches!(
            queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 0 && counts.strict == 0
        ));
        assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() == 3));
    }

    #[test]
    fn runtime_shutdown_graceful_drains_owned_queued_journal() {
        let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
            return;
        };
        let Some(queue) = require_ok(journal_queue(4, 1), "journal queue opens") else {
            return;
        };
        let runtime_journal = Arc::new(QueuedStorageRuntimeJournal::journaled(
            journal.clone(),
            queue.clone(),
        ));
        let run = RunId::new(49);
        let workflow = WorkflowDigest::from_bytes([12; 32]);
        let Some(shard_count) = NonZeroUsize::new(1) else {
            assert!(false, "invalid shard count");
            return;
        };
        let runtime =
            Runtime::new_with_journal(shard_count, ShardConfig::default(), runtime_journal);

        let Some(compiled) = require_ok(single_finish_workflow(workflow), "workflow compiles")
        else {
            return;
        };
        assert_eq!(runtime.submit_direct(run, compiled), Ok(()));
        assert!(matches!(
            queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 0 && counts.strict == 0
        ));

        let mut runtime = runtime;
        assert_eq!(runtime.tick_all(), Ok(true));
        // Evidence chain adds StepStarted + StepSucceeded per step.
        // Single Finish step: RunSubmitted + StepStarted(0) + StepSucceeded(0) + RunFinished
        assert!(matches!(
            queue.pending_profile_counts(),
            Ok(ref c) if c.journaled >= 3 && c.strict == 0
        ));
        assert_eq!(runtime.shutdown_graceful(), Ok(()));
        assert!(matches!(
            queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 0 && counts.strict == 0
        ));
        // At minimum RunSubmitted + StepSucceeded + RunFinished stored after drain
        assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() >= 3));
    }

    #[test]
    fn queued_storage_runtime_journal_maps_queue_full_to_runtime_error() {
        let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
            return;
        };
        let Some(queue) = require_ok(journal_queue(1, 1), "journal queue opens") else {
            return;
        };
        let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue);
        let run = RunId::new(46);

        assert_eq!(
            adapter.append(RuntimeJournalEvent::RunCancelled { run }),
            Ok(())
        );
        assert!(matches!(
            adapter.append(RuntimeJournalEvent::RunFailed { run }),
            Err(crate::RuntimeError::StorageJournalAppend { source })
                if matches!(source.as_ref(), vb_storage::JournalError::QueueFull)
        ));
        assert!(
            matches!(adapter.flush_batch(), Ok(report) if report.drained == 1 && report.written == 1)
        );
        assert_eq!(
            adapter.append(RuntimeJournalEvent::RunFailed { run }),
            Ok(())
        );
        assert!(
            matches!(adapter.flush_batch(), Ok(report) if report.drained == 1 && report.written == 1)
        );

        let Some(events) = require_ok(
            journal
                .events_for_run(run)
                .map_err(|error| error.to_string()),
            "queue-full events read",
        ) else {
            return;
        };
        assert_eq!(
            events,
            vec![
                JournalEvent::RunCancelled {
                    run,
                    seq: EventSeq::new(0),
                },
                JournalEvent::RunFailedEvent {
                    run,
                    seq: EventSeq::new(1),
                },
            ]
        );
    }
}
