use indexmap::IndexMap;
use std::sync::{Arc, Mutex};
use vb_core::Taint;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::{
    DurabilityProfile, EventSeq, FjallJournal, JournalEvent, JournalWriterFlushReport,
    JournalWriterQueue,
};

use crate::{RuntimeError, RuntimeResult};

/// Minimal lifecycle event emitted by the runtime before a durable store is wired.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
        /// Optional cancellation reason.
        reason: Option<String>,
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
        /// Execution attempt number for this action.
        attempt: u16,
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
        /// Taint written with this slot value.
        taint: Taint,
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
        /// Execution attempt number for this step.
        attempt: u16,
    },
    /// Run was resumed from a suspended state.
    Resumed {
        /// Run identifier.
        run: RunId,
        /// Monotonic timestamp in seconds since epoch.
        timestamp: u64,
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
            | Self::RunCancelled { run, .. }
            | Self::ActionScheduled { run, .. }
            | Self::ActionCompleted { run, .. }
            | Self::ActionFailed { run, .. }
            | Self::WaitScheduled { run, .. }
            | Self::WaitResolved { run, .. }
            | Self::AskScheduled { run, .. }
            | Self::AskAnswered { run, .. }
            | Self::SlotWritten { run, .. }
            | Self::StepStarted { run, .. }
            | Self::StepSucceeded { run, .. }
            | Self::Resumed { run, .. } => *run,
            Self::RunAdmission { admission } => admission.run_id(),
        }
    }
}

/// Append-only port used by runtime shards for lifecycle journaling.
pub trait RuntimeJournal: Send + Sync {
    /// Appends a lifecycle event.
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()>;

    /// Probes journal health without side effects.
    /// Returns `JournalPoisoned` if the underlying storage is unavailable.
    fn probe(&self) -> RuntimeResult<()>;

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
    fn probe(&self) -> RuntimeResult<()> {
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
    fn probe(&self) -> RuntimeResult<()> {
        // Verify the mutex is not poisoned.
        let _guard = self
            .events
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;
        Ok(())
    }
}

/// Runtime journal adapter that appends lifecycle events into `vb_storage`.
pub struct StorageRuntimeJournal {
    journal: Arc<FjallJournal>,
    next_seq_by_run: Mutex<IndexMap<RunId, EventSeq>>,
    profile: DurabilityProfile,
}
