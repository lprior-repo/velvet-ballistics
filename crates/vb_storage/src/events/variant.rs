#![forbid(unsafe_code)]
//! Compact binary journal event types.
//!
//! JSONL is a projection, not this durable format.

use crate::EventSeq;
use chrono::{DateTime, Utc};
use vb_core::{
    ActionId, ActionTicket, CapabilitySet, ConstValue, RunId, RuntimePolicy, SlotIdx, StepIdx,
    Taint, WorkflowDigest,
};

/// Compact binary journal event. JSONL is a projection, not this durable format.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum JournalEvent {
    /// Run was accepted after input mapping.
    RunAccepted {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Compiled workflow digest.
        workflow: WorkflowDigest,
    },
    /// Run admission metadata persisted after admission control succeeds.
    RunAdmission {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Compiled artifact digest admitted for this run.
        artifact_digest: WorkflowDigest,
        /// Capabilities granted for this run.
        granted_capabilities: CapabilitySet,
        /// Policy used to admit this run.
        policy: RuntimePolicy,
    },
    /// Step began execution.
    StepStarted {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Step completed and wrote an output slot.
    StepSucceeded {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Output slot index.
        output: SlotIdx,
    },
    /// Action was scheduled.
    ActionScheduled {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Action identifier.
        action: ActionId,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Action completed successfully.
    ActionCompletedEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Action identifier.
        action: ActionId,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Action was scheduled with the full replay ticket preserved.
    ActionScheduledTicket {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Full action ticket issued by the runtime.
        ticket: ActionTicket,
        /// Input slot consumed by the action.
        input: SlotIdx,
        /// Output slot expected to receive the result.
        output: SlotIdx,
    },
    /// Action completed successfully with an atomic durable envelope.
    ActionCompletedEnvelope {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Full action ticket completed by the runtime.
        ticket: ActionTicket,
        /// Output slot written by the action.
        output: SlotIdx,
        /// Terminal outcome discriminant for this completion.
        outcome: super::outcome::DurableActionOutcome,
        /// Encoded output value bytes.
        value: Vec<u8>,
        /// Encoded output byte length validated before persistence.
        encoded_len: u32,
        /// Taint written with the output value.
        taint: Taint,
        /// BLAKE3 digest of `value` used to reject divergent duplicate evidence.
        value_digest: [u8; 32],
    },
    /// Action failed.
    ActionFailedEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Action identifier.
        action: ActionId,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Slot was written during execution.
    SlotWrittenEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Slot index.
        slot: SlotIdx,
        /// Encoded slot value bytes (postcard-encoded `SlotValue`), if captured.
        #[serde(default)]
        value: Option<Vec<u8>>,
        /// Versioned slot-write extra envelope, or legacy encoded frame extra data.
        #[serde(default)]
        extra: Option<Vec<u8>>,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Wait was scheduled.
    WaitScheduledEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
        /// Duration from wait scheduling to timer fire, in milliseconds.
        #[serde(default)]
        deadline_ms: u64,
    },
    /// Ask was scheduled.
    AskScheduledEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
        /// Duration from ask scheduling to timeout, in milliseconds.
        #[serde(default)]
        deadline_ms: u64,
    },
    /// Ask was answered.
    AskAnsweredEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Wait was cancelled before timer fired.
    WaitCancelledEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Ask was cancelled before timer fired.
    AskCancelledEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Retry was scheduled.
    RetryScheduledEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Run cancelled.
    RunCancelled {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Attempt number (1-based).
        attempt: u16,
        /// Optional cancellation reason.
        reason: Option<String>,
    },
    /// Run killed.
    RunKilled {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Attempt number (1-based).
        attempt: u16,
        /// Optional kill reason.
        #[serde(default)]
        reason: Option<String>,
    },
    /// Run completed.
    RunFinished {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Result slot index.
        result: SlotIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Run failed.
    RunFailedEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Attempt number (1-based).
        attempt: u16,
    },
    /// Run was resumed from a waiting state.
    RunResumed {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// When the run was resumed.
        timestamp: DateTime<Utc>,
    },
    /// Run was retried after failure.
    RunRetried {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// When the run was retried.
        timestamp: DateTime<Utc>,
    },
    /// Run received an answer to a waiting question.
    RunAnswered {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Slot that received the answer.
        slot_idx: SlotIdx,
        /// The answer value.
        answer: ConstValue,
        /// When the answer was received.
        timestamp: DateTime<Utc>,
    },
}
