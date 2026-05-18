#![forbid(unsafe_code)]
//! Journal event types and record kind identifiers.

use crate::{EventSeq, JournalError, RecordKind};
use chrono::{DateTime, Utc};
use vb_core::{
    ActionId, CapabilitySet, ConstValue, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx,
    WorkflowDigest,
};

/// Compact binary journal event. JSONL is a projection, not this durable format.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
        value: Option<Vec<u8>>,
        /// Encoded frame extra data captured with this slot write, if any.
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
        /// When the run was resumed.
        timestamp: DateTime<Utc>,
    },
    /// Run was retried after failure.
    RunRetried {
        /// Run identifier.
        run: RunId,
        /// When the run was retried.
        timestamp: DateTime<Utc>,
    },
    /// Run received an answer to a waiting question.
    RunAnswered {
        /// Run identifier.
        run: RunId,
        /// Slot that received the answer.
        slot_idx: SlotIdx,
        /// The answer value.
        answer: ConstValue,
        /// When the answer was received.
        timestamp: DateTime<Utc>,
    },
}

impl JournalEvent {
    /// Run identifier carried by this event.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        match self {
            Self::RunAccepted { run, .. }
            | Self::RunAdmission { run, .. }
            | Self::StepStarted { run, .. }
            | Self::StepSucceeded { run, .. }
            | Self::ActionScheduled { run, .. }
            | Self::ActionCompletedEvent { run, .. }
            | Self::ActionFailedEvent { run, .. }
            | Self::SlotWrittenEvent { run, .. }
            | Self::WaitScheduledEvent { run, .. }
            | Self::AskScheduledEvent { run, .. }
            | Self::AskAnsweredEvent { run, .. }
            | Self::RetryScheduledEvent { run, .. }
            | Self::RunCancelled { run, .. }
            | Self::RunFinished { run, .. }
            | Self::RunFailedEvent { run, .. }
            | Self::RunResumed { run, .. }
            | Self::RunRetried { run, .. }
            | Self::RunAnswered { run, .. } => *run,
        }
    }

    /// Event sequence carried by this event.
    ///
    /// Lifecycle events (RunResumed, RunRetried, RunAnswered) do not carry sequence numbers
    /// as they are not part of the durable event log ordering.
    #[must_use]
    pub const fn seq(&self) -> EventSeq {
        match self {
            Self::RunAccepted { seq, .. }
            | Self::RunAdmission { seq, .. }
            | Self::StepStarted { seq, .. }
            | Self::StepSucceeded { seq, .. }
            | Self::ActionScheduled { seq, .. }
            | Self::ActionCompletedEvent { seq, .. }
            | Self::ActionFailedEvent { seq, .. }
            | Self::SlotWrittenEvent { seq, .. }
            | Self::WaitScheduledEvent { seq, .. }
            | Self::AskScheduledEvent { seq, .. }
            | Self::AskAnsweredEvent { seq, .. }
            | Self::RetryScheduledEvent { seq, .. }
            | Self::RunCancelled { seq, .. }
            | Self::RunFinished { seq, .. }
            | Self::RunFailedEvent { seq, .. } => *seq,
            Self::RunResumed { .. } | Self::RunRetried { .. } | Self::RunAnswered { .. } => {
                EventSeq::ZERO
            }
        }
    }

    /// Storage record kind for this event.
    #[must_use]
    pub const fn record_kind(&self) -> RecordKind {
        match self {
            Self::RunAccepted { .. } => RecordKind::RunAccepted,
            Self::RunAdmission { .. } => RecordKind::RunAdmission,
            Self::StepStarted { .. } => RecordKind::StepStarted,
            Self::StepSucceeded { .. } | Self::SlotWrittenEvent { .. } => RecordKind::SlotWritten,
            Self::ActionScheduled { .. } => RecordKind::ActionScheduled,
            Self::ActionCompletedEvent { .. } => RecordKind::ActionCompleted,
            Self::ActionFailedEvent { .. } => RecordKind::ActionFailed,
            Self::WaitScheduledEvent { .. } => RecordKind::WaitScheduled,
            Self::AskScheduledEvent { .. } => RecordKind::AskScheduled,
            Self::AskAnsweredEvent { .. } => RecordKind::AskAnswered,
            Self::RetryScheduledEvent { .. } => RecordKind::RetryScheduled,
            Self::RunCancelled { .. } => RecordKind::RunCancelled,
            Self::RunFinished { .. } => RecordKind::RunFinished,
            Self::RunFailedEvent { .. } => RecordKind::RunFailed,
            Self::RunResumed { .. } => RecordKind::RunResumed,
            Self::RunRetried { .. } => RecordKind::RunRetried,
            Self::RunAnswered { .. } => RecordKind::RunAnswered,
        }
    }

    /// Returns the slot value if this is a `SlotWrittenEvent` and a value was captured.
    ///
    /// Returns `Ok(None)` if no value was captured (absent optional payload).
    /// Returns `Ok(Some(slot_value))` if decoding succeeded.
    /// Returns `Err(JournalError::PostcardDecodeFailed)` if bytes are corrupt/truncated.
    /// Returns `Err(JournalError::PayloadTooLarge)` if bytes exceed the maximum allowed size.
    #[must_use = "slot_value returns a fallible result that must be handled"]
    pub fn slot_value(&self) -> Result<Option<SlotValue>, JournalError> {
        match self {
            Self::SlotWrittenEvent {
                value: Some(bytes), ..
            } => {
                // First check payload size bounds before attempting decode.
                let max_bytes = crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
                let len_u32 =
                    u32::try_from(bytes.len()).map_err(|_| JournalError::PayloadTooLarge {
                        len: u32::MAX,
                        max: max_bytes,
                    })?;
                if len_u32 > max_bytes {
                    return Err(JournalError::PayloadTooLarge {
                        len: len_u32,
                        max: max_bytes,
                    });
                }
                // Decode with typed error propagation instead of silent erasure.
                match postcard::from_bytes(bytes) {
                    Ok(value) => Ok(Some(value)),
                    Err(_) => Err(JournalError::PostcardDecodeFailed),
                }
            }
            // Explicit absent branch for optional payloads.
            Self::SlotWrittenEvent { value: None, .. } => Ok(None),
            // Non-SlotWrittenEvent variants have no slot value.
            _ => Ok(None),
        }
    }

    /// Returns the attempt number for this event.
    ///
    /// Events that carry attempt info return `Some(attempt)`.
    /// Events that don't carry attempt info (`RunAccepted`, `RunAdmission`,
    /// `StepSucceeded`) return `None`; these are treated as
    /// attempt 1 by the replay filtering logic (PRE-001).
    #[must_use]
    pub const fn attempt(&self) -> Option<u16> {
        match self {
            Self::ActionScheduled { attempt, .. }
            | Self::ActionCompletedEvent { attempt, .. }
            | Self::ActionFailedEvent { attempt, .. }
            | Self::SlotWrittenEvent { attempt, .. }
            | Self::WaitScheduledEvent { attempt, .. }
            | Self::AskScheduledEvent { attempt, .. }
            | Self::AskAnsweredEvent { attempt, .. }
            | Self::RetryScheduledEvent { attempt, .. }
            | Self::StepStarted { attempt, .. }
            | Self::RunCancelled { attempt, .. }
            | Self::RunFinished { attempt, .. }
            | Self::RunFailedEvent { attempt, .. } => Some(*attempt),
            Self::RunAccepted { .. }
            | Self::RunAdmission { .. }
            | Self::StepSucceeded { .. }
            | Self::RunResumed { .. }
            | Self::RunRetried { .. }
            | Self::RunAnswered { .. } => None,
        }
    }
}
