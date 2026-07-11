#![forbid(unsafe_code)]
//! Journal event enum types and outcome discriminant.
//!
//! Extracted from the parent `events` module so the impl-block host stays
//! below the production source-length limit while preserving the public
//! API contract (`pub use events::{JournalEvent, DurableActionOutcome}`).

use crate::EventSeq;
use chrono::{DateTime, Utc};
use vb_core::{
    ActionId, ActionTicket, CapabilitySet, ConstValue, RunId, RuntimePolicy, SlotIdx, StepIdx,
    Taint, WorkflowDigest,
};

/// Terminal action outcome captured by durable completion envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum DurableActionOutcome {
    /// Action completed successfully and wrote an output slot.
    Ready = 1,
}

/// Compact binary journal event. JSONL is a projection, not this durable format.
///
/// The postcard payload wire format is a stable newtype variant whose encoded
/// variant index is the unique `RecordKind` tag, followed by that event
/// variant's fields in documented order. Persisted schema-1 legacy ordinal
/// payloads are decoded as an exact-consumption compatibility fallback while
/// new writes use these stable record-kind tags. For schema-1 records written
/// before the split, `StepSucceeded`, `ActionScheduledTicket`, and
/// `ActionCompletedEnvelope` may still appear under their old shared envelope
/// kinds; those are accepted only through the named schema-one shared-envelope
/// compatibility path.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Step failed and recovery must retain the failed step identity.
    StepFailed {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
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
        /// Digest of the persisted action ABI contract used for this action.
        action_abi_digest: WorkflowDigest,
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
        outcome: DurableActionOutcome,
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
    /// Action was abandoned because the run was cancelled or killed
    /// before the action boundary completed. Distinct from
    /// `ActionFailedEvent` because no `ActionFailureCode` was ever
    /// produced by the action — the suspension itself was terminated
    /// by run-level cancellation. Master §45 Do-node "Resume" sub-row
    /// requires this event so recovery can finalize the step without
    /// re-running the external action.
    ActionAbandoned {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Full action ticket that was abandoned. Preserves all seven
        /// required `ActionTicket` fields plus the journal position so
        /// recovery can deterministically drop the pending action from
        /// the resume queue without re-executing it.
        ticket: ActionTicket,
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
        value: Option<Vec<u8>>,
        /// Versioned slot-write extra envelope, or legacy encoded frame extra data.
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
    /// Wait was resolved by an external timer.
    ///
    /// Distinct from `RetryScheduledEvent` because a wait resolution is not a
    /// retry: the suspended run resumes from a satisfied external condition
    /// rather than from a bounded retry attempt. See bug-hunt RE-009.
    WaitResolvedEvent {
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
    /// Ask timed out and resumed along the ask timeout path.
    AskTimedOutEvent {
        /// Run identifier.
        run: RunId,
        /// Per-run sequence.
        seq: EventSeq,
        /// Step index.
        step: StepIdx,
        /// Attempt number (1-based).
        attempt: u16,
    },
}
