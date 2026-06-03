use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use vb_core::Taint;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::{DurabilityProfile, EventSeq, FjallJournal, JournalEvent, JournalWriterFlushReport, JournalWriterQueue};

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
            | Self::RunKilled { run, .. }
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
            Self::ActionScheduledTicket { ticket, .. }
            | Self::ActionCompletedEnvelope { ticket, .. } => ticket.run,
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
            // Handle any future DurabilityProfile variants as Volatile (safest fallback).
            #[allow(unreachable_code)]
            _ => VolatileRuntimeJournal::shared(),
        }
    }
}

/// Runtime journal adapter that appends lifecycle events into `vb_storage`.
pub struct StorageRuntimeJournal {
    journal: Arc<FjallJournal>,
    profile: DurabilityProfile,
}

include!("chunk_001_noop.rs");
include!("chunk_001_volatile.rs");
