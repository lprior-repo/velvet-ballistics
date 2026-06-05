#![forbid(unsafe_code)]
//! Journal event types and record kind identifiers.

use crate::mrwe5_contract::{Mrwe5PayloadClass, mrwe5_canonical_kind_id};
use crate::{EventSeq, JournalError, RecordKind};
use chrono::{DateTime, Utc};
use vb_core::{
    ActionId, ActionTicket, CapabilitySet, ConstValue, RunId, RuntimePolicy, SlotIdx, SlotValue,
    StepIdx, Taint, WorkflowDigest,
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
        outcome: DurableActionOutcome,
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
}

/// Verus-friendly class for the two MRWE5 payloads whose record kinds must stay
/// separated. `Other` deliberately carries no compatibility privilege.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum JournalEventKindClass {
    /// `JournalEvent::StepSucceeded` payloads use `RecordKind::StepSucceeded`.
    StepSucceeded = 1,
    /// `JournalEvent::SlotWrittenEvent` payloads use `RecordKind::SlotWritten`.
    SlotWrittenEvent = 2,
    /// Any journal payload outside the MRWE5 separation pair.
    Other = 3,
}

impl JournalEventKindClass {
    /// Canonical record kind for the named MRWE5 payload classes.
    #[must_use]
    pub const fn canonical_record_kind(self) -> Option<RecordKind> {
        match self {
            Self::StepSucceeded => Some(RecordKind::StepSucceeded),
            Self::SlotWrittenEvent => Some(RecordKind::SlotWritten),
            Self::Other => None,
        }
    }

    /// Canonical record-kind id for the named MRWE5 payload classes.
    #[must_use]
    pub const fn canonical_record_kind_id(self) -> Option<u16> {
        mrwe5_canonical_kind_id(self.mrwe5_payload_class())
    }

    /// Primitive production-bound class used by the shared MRWE5 contract kernel.
    #[must_use]
    pub const fn mrwe5_payload_class(self) -> Mrwe5PayloadClass {
        match self {
            Self::StepSucceeded => Mrwe5PayloadClass::StepSucceeded,
            Self::SlotWrittenEvent => Mrwe5PayloadClass::SlotWrittenEvent,
            Self::Other => Mrwe5PayloadClass::Other,
        }
    }
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
            | Self::ActionScheduledTicket { run, .. }
            | Self::ActionCompletedEnvelope { run, .. }
            | Self::ActionFailedEvent { run, .. }
            | Self::SlotWrittenEvent { run, .. }
            | Self::WaitScheduledEvent { run, .. }
            | Self::AskScheduledEvent { run, .. }
            | Self::AskAnsweredEvent { run, .. }
            | Self::RetryScheduledEvent { run, .. }
            | Self::RunCancelled { run, .. }
            | Self::RunKilled { run, .. }
            | Self::RunFinished { run, .. }
            | Self::RunFailedEvent { run, .. }
            | Self::RunResumed { run, .. }
            | Self::RunRetried { run, .. }
            | Self::RunAnswered { run, .. } => *run,
        }
    }

    /// Event sequence carried by this event.
    ///
    /// Lifecycle events (RunResumed, RunRetried, RunAnswered) now carry sequence numbers
    /// to enable deduplication and ordering in the journal.
    #[must_use]
    pub const fn seq(&self) -> EventSeq {
        match self {
            Self::RunAccepted { seq, .. }
            | Self::RunAdmission { seq, .. }
            | Self::StepStarted { seq, .. }
            | Self::StepSucceeded { seq, .. }
            | Self::ActionScheduled { seq, .. }
            | Self::ActionCompletedEvent { seq, .. }
            | Self::ActionScheduledTicket { seq, .. }
            | Self::ActionCompletedEnvelope { seq, .. }
            | Self::ActionFailedEvent { seq, .. }
            | Self::SlotWrittenEvent { seq, .. }
            | Self::WaitScheduledEvent { seq, .. }
            | Self::AskScheduledEvent { seq, .. }
            | Self::AskAnsweredEvent { seq, .. }
            | Self::RetryScheduledEvent { seq, .. }
            | Self::RunCancelled { seq, .. }
            | Self::RunKilled { seq, .. }
            | Self::RunFinished { seq, .. }
            | Self::RunFailedEvent { seq, .. }
            | Self::RunResumed { seq, .. }
            | Self::RunRetried { seq, .. }
            | Self::RunAnswered { seq, .. } => *seq,
        }
    }

    /// Storage record kind for this event.
    #[must_use]
    pub const fn record_kind(&self) -> RecordKind {
        match self {
            Self::RunAccepted { .. } => RecordKind::RunAccepted,
            Self::RunAdmission { .. } => RecordKind::RunAdmission,
            Self::StepStarted { .. } => RecordKind::StepStarted,
            Self::StepSucceeded { .. } => RecordKind::StepSucceeded,
            Self::SlotWrittenEvent { .. } => RecordKind::SlotWritten,
            Self::ActionScheduled { .. } | Self::ActionScheduledTicket { .. } => {
                RecordKind::ActionScheduled
            }
            Self::ActionCompletedEvent { .. } | Self::ActionCompletedEnvelope { .. } => {
                RecordKind::ActionCompleted
            }
            Self::ActionFailedEvent { .. } => RecordKind::ActionFailed,
            Self::WaitScheduledEvent { .. } => RecordKind::WaitScheduled,
            Self::AskScheduledEvent { .. } => RecordKind::AskScheduled,
            Self::AskAnsweredEvent { .. } => RecordKind::AskAnswered,
            Self::RetryScheduledEvent { .. } => RecordKind::RetryScheduled,
            Self::RunCancelled { .. } => RecordKind::RunCancelled,
            Self::RunKilled { .. } => RecordKind::RunKilled,
            Self::RunFinished { .. } => RecordKind::RunFinished,
            Self::RunFailedEvent { .. } => RecordKind::RunFailed,
            Self::RunResumed { .. } => RecordKind::RunResumed,
            Self::RunRetried { .. } => RecordKind::RunRetried,
            Self::RunAnswered { .. } => RecordKind::RunAnswered,
        }
    }

    /// Storage record-kind id for this event.
    #[must_use]
    pub const fn record_kind_id(&self) -> u16 {
        match self.kind_class().canonical_record_kind_id() {
            Some(id) => id,
            None => self.record_kind().id(),
        }
    }

    /// MRWE5 proof seam exposing the payload class used for kind separation.
    #[must_use]
    pub const fn kind_class(&self) -> JournalEventKindClass {
        match self {
            Self::StepSucceeded { .. } => JournalEventKindClass::StepSucceeded,
            Self::SlotWrittenEvent { .. } => JournalEventKindClass::SlotWrittenEvent,
            _ => JournalEventKindClass::Other,
        }
    }

    /// Returns true when the envelope kind is the canonical kind for this event.
    #[must_use]
    pub const fn has_canonical_envelope_kind(&self, envelope_kind: u16) -> bool {
        envelope_kind == self.record_kind_id()
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
            | Self::RunKilled { attempt, .. }
            | Self::RunFinished { attempt, .. }
            | Self::RunFailedEvent { attempt, .. } => Some(*attempt),
            Self::ActionScheduledTicket { ticket, .. }
            | Self::ActionCompletedEnvelope { ticket, .. } => Some(ticket.attempt),
            Self::RunAccepted { .. }
            | Self::RunAdmission { .. }
            | Self::StepSucceeded { .. }
            | Self::RunResumed { .. }
            | Self::RunRetried { .. }
            | Self::RunAnswered { .. } => None,
        }
    }

    /// Returns true if this event passes basic structural validity checks.
    ///
    /// A valid event has:
    /// - A non-zero run identifier (zero is the null/placeholder value)
    /// - A sequence number within reasonable bounds (not u64::MAX)
    /// - Attempt numbers, when present, are non-zero (zero is ambiguous in replay)
    ///
    /// Note: This does NOT validate cryptographic digests or semantic consistency.
    /// Those are enforced by the codec round-trip and replay validation.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        // RunId(0) is the zero/placeholder value - valid events must have a real run
        if self.run_id().get() == 0 {
            return false;
        }
        // Sequence must not be at the max value (overflow sentinel)
        if self.seq().get() == u64::MAX {
            return false;
        }
        // Attempt numbers must be non-zero when present (zero is ambiguous)
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
            | Self::RunKilled { attempt, .. }
            | Self::RunFinished { attempt, .. }
            | Self::RunFailedEvent { attempt, .. } => *attempt != 0,
            Self::ActionScheduledTicket { ticket, .. }
            | Self::ActionCompletedEnvelope { ticket, .. } => ticket.attempt != 0,
            Self::RunAccepted { .. }
            | Self::RunAdmission { .. }
            | Self::StepSucceeded { .. }
            | Self::RunResumed { .. }
            | Self::RunRetried { .. }
            | Self::RunAnswered { .. } => true,
        }
    }
}
