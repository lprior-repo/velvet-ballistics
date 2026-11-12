use chrono::{DateTime, Utc};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use vb_core::Taint;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, TenantId, WorkflowDigest};
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
    /// §77.17 compatibility decision journaled with the admission header
    /// for an admitted run. Carries the full decision record; replay
    /// treats it as informational and live dispatch re-checks the
    /// journaled decision before authorizing new live effects.
    CompatibilityDecision {
        /// Run identifier.
        run: RunId,
        /// Compatibility decision recorded at admission.
        decision: vb_storage::compat::CompatDecision,
    },
    /// §77.18 tenant admission receipt journaled with the admission header
    /// for a tenant-scoped run. Carries the tenant the run is charged to
    /// plus the admitted cost units, so recovery can rebuild the per-tenant
    /// shard usage snapshot from the journal. Replay treats it as
    /// informational: it authorizes no live effect.
    TenantAdmission {
        /// Run identifier.
        run: RunId,
        /// Tenant the run is charged to.
        tenant: TenantId,
        /// Admitted cost units charged to the tenant.
        cost_units: u64,
    },
    /// Full typed checkpoint and next-attempt authority for a retry.
    RetryCheckpoint {
        /// Run identifier.
        run: RunId,
        /// New run attempt authorized by this checkpoint.
        attempt: u16,
        /// Failed step selected for re-execution.
        step: StepIdx,
        /// Digest of the serialized checkpoint payload.
        payload_digest: [u8; 32],
        /// Serialized versioned checkpoint payload.
        payload: Vec<u8>,
    },
    /// Run reached a successful terminal state.
    RunFinished {
        /// Run identifier.
        run: RunId,
        /// Result slot selected by the terminal finish node.
        result: SlotIdx,
        /// Run attempt that reached the terminal state.
        attempt: u16,
    },
    /// Run reached a failed terminal state.
    RunFailed {
        /// Run identifier.
        run: RunId,
        /// Run attempt that failed.
        attempt: u16,
    },
    /// Run was cancelled by the caller.
    RunCancelled {
        /// Run identifier.
        run: RunId,
        /// Optional cancellation reason.
        reason: Option<String>,
        /// Run attempt that was cancelled.
        attempt: u16,
    },
    /// Run was killed by the runtime.
    RunKilled {
        /// Run identifier.
        run: RunId,
        /// Run attempt that was killed.
        attempt: u16,
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
        /// Run attempt in which the step started.
        attempt: u16,
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
    /// A deterministic step failed before entering its handler.
    StepFailed {
        /// Run identifier.
        run: RunId,
        /// Failed step.
        step: StepIdx,
        /// Execution attempt.
        attempt: u16,
    },
    /// Run was resumed from a suspended state.
    Resumed {
        /// Run identifier.
        run: RunId,
        /// Monotonic timestamp in seconds since epoch.
        timestamp: u64,
    },
    /// Complete failed-action state and exact durable retry timer authority.
    RetryTimerScheduled {
        /// Owning run.
        run: RunId,
        /// Failed action step.
        step: StepIdx,
        /// Exact versioned deadline and checkpoint payload.
        timer: vb_storage::retry_timer::RetryTimerV1,
    },
}
