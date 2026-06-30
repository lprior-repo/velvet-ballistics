use std::num::NonZeroUsize;
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
#[non_exhaustive]
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
    /// Run was killed by the runtime.
    RunKilled {
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
    /// Action was scheduled with the full ticket preserved for durable replay.
    ActionScheduledTicket {
        /// Full ticket issued for the action.
        ticket: vb_core::action::ActionTicket,
        /// Input slot consumed by the action.
        input: SlotIdx,
        /// Output slot expected to receive the result.
        output: SlotIdx,
        /// Digest of the persisted action ABI contract used for this action.
        action_abi_digest: WorkflowDigest,
    },
    /// Action completed successfully with an atomic durable envelope.
    ActionCompletedEnvelope {
        /// Full ticket completed by the action boundary.
        ticket: vb_core::action::ActionTicket,
        /// Output slot written by the action.
        output: SlotIdx,
        /// Encoded output value bytes.
        value: Vec<u8>,
        /// Encoded output byte length validated before persistence.
        encoded_len: u32,
        /// Taint written with the output value.
        taint: Taint,
        /// BLAKE3 digest of `value` used to reject divergent duplicate evidence.
        value_digest: [u8; 32],
        /// Digest of the persisted action ABI contract used for this action.
        action_abi_digest: WorkflowDigest,
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
    /// Action was abandoned because the run was cancelled or killed
    /// before the action boundary completed. Carries the full ticket
    /// so recovery can deterministically finalize the step without
    /// re-executing the external action. Master §45 Do-node "Resume"
    /// sub-row requires this event.
    ActionAbandoned {
        /// Full ticket that was abandoned. All seven required
        /// `ActionTicket` fields are preserved across the durability
        /// boundary.
        ticket: vb_core::action::ActionTicket,
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
    /// Ask timed out and resumed along the timeout path.
    AskTimedOut {
        /// Run identifier.
        run: RunId,
        /// Step that scheduled the ask.
        step: StepIdx,
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
            | Self::RunKilled { run, .. }
            | Self::ActionScheduled { run, .. }
            | Self::ActionCompleted { run, .. }
            | Self::ActionFailed { run, .. }
            | Self::WaitScheduled { run, .. }
            | Self::WaitResolved { run, .. }
            | Self::AskScheduled { run, .. }
            | Self::AskAnswered { run, .. }
            | Self::AskTimedOut { run, .. }
            | Self::SlotWritten { run, .. }
            | Self::StepStarted { run, .. }
            | Self::StepSucceeded { run, .. }
            | Self::Resumed { run, .. } => *run,
            Self::ActionScheduledTicket { ticket, .. }
            | Self::ActionCompletedEnvelope { ticket, .. }
            | Self::ActionAbandoned { ticket } => ticket.run,
            Self::RunAdmission { admission } => admission.run_id(),
        }
    }
}

/// Append-only port used by runtime shards for lifecycle journaling.
pub trait RuntimeJournal: Send + Sync {
    /// Appends a lifecycle event.
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()>;

    /// Returns the underlying Fjall journal if this is a storage-backed journal.
    ///
    /// Returns `Some(Arc<FjallJournal>)` for `StorageRuntimeJournal`, or `None` for
    /// noop/volatile journals. This allows callers to construct a
    /// `StorageArtifactStore` from a storage-backed journal for strict/journaled
    /// artifact admission.
    fn storage_journal(&self) -> Option<std::sync::Arc<vb_storage::FjallJournal>> {
        None
    }

    /// Appends a lifecycle event whose per-run sequence is owned by the shard.
    fn append_sequenced(&self, event: RuntimeJournalEvent, _seq: EventSeq) -> RuntimeResult<()> {
        self.append(event)
    }

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
    /// Creates a shared noop journal for explicitly non-durable tests or benchmarks.
    #[must_use]
    pub fn shared_for_tests_and_benchmarks() -> SharedRuntimeJournal {
        Arc::new(Self)
    }

    /// Creates a shared noop journal for callers that explicitly select no durability.
    #[must_use]
    pub fn shared() -> SharedRuntimeJournal {
        Self::shared_for_tests_and_benchmarks()
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
#[derive(Debug)]
pub struct VolatileRuntimeJournal {
    events: Mutex<Vec<RuntimeJournalEvent>>,
    capacity: usize,
}

impl VolatileRuntimeJournal {
    /// Default maximum number of in-memory journal events retained by a volatile journal.
    pub const DEFAULT_CAPACITY: usize = 65_536;

    /// Creates an empty volatile journal.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            capacity: Self::DEFAULT_CAPACITY,
        }
    }

    /// Creates an empty volatile journal with an explicit event capacity.
    #[must_use]
    pub const fn with_capacity(capacity: NonZeroUsize) -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            capacity: capacity.get(),
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

    fn reserve_one_event(
        events: &mut Vec<RuntimeJournalEvent>,
        capacity: usize,
    ) -> RuntimeResult<()> {
        events
            .try_reserve(1)
            .map_err(|_| crate::RuntimeError::JournalFull { capacity })
    }
}

impl Default for VolatileRuntimeJournal {
    fn default() -> Self {
        Self::new()
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
    ///
    /// Returns `Err(RuntimeError::UnsupportedDurabilityProfile { .. })`
    /// for any future `DurabilityProfile` variant the runtime does not
    /// yet implement. This replaces the prior silent downgrade to
    /// `Volatile` (master §45 contract: missing transitions return a
    /// typed error rather than silently absorbing into the
    /// least-durable profile).
    pub fn shared_journal(
        self,
        journal: Arc<FjallJournal>,
        queue: Arc<JournalWriterQueue>,
    ) -> RuntimeResult<SharedRuntimeJournal> {
        match self.profile {
            DurabilityProfile::Volatile => Ok(VolatileRuntimeJournal::shared()),
            DurabilityProfile::Journaled => Ok(QueuedStorageRuntimeJournal::shared_journaled(
                journal, queue,
            )),
            DurabilityProfile::Strict => Ok(StorageRuntimeJournal::shared_strict(journal)),
            #[allow(unreachable_patterns)]
            _ => Err(RuntimeError::UnsupportedDurabilityProfile {
                profile_debug: format!("{:?}", self.profile),
            }),
        }
    }

//    /// Compile-time exhaustiveness assertion for `shared_journal`.
//    #[allow(dead_code, unreachable_patterns)]
//    const _: () = {
//        fn _check_exhaustiveness(
//            profile: DurabilityProfile,
//            journal: std::sync::Arc<FjallJournal>,
//            queue: std::sync::Arc<JournalWriterQueue>,
//        ) -> RuntimeResult<SharedRuntimeJournal> {
//            match profile {
//                DurabilityProfile::Volatile => Ok(VolatileRuntimeJournal::shared()),
//                DurabilityProfile::Journaled => Ok(QueuedStorageRuntimeJournal::shared_journaled(
//                    journal, queue,
//                )),
//                DurabilityProfile::Strict => Ok(StorageRuntimeJournal::shared_strict(journal)),
//                _ => Err(RuntimeError::UnsupportedDurabilityProfile {
//                    profile_debug: String::new(),
//                }),
//            }
//        }
//// };
//}

}

impl RuntimeJournal for VolatileRuntimeJournal {
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        let mut events = self
            .events
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;
        if events.len() >= self.capacity {
            return Err(RuntimeError::JournalFull {
                capacity: self.capacity,
            });
        }
        Self::reserve_one_event(&mut events, self.capacity)?;
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
    profile: DurabilityProfile,
}
