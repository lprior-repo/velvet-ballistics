#![forbid(unsafe_code)]
//! Journal event types and record kind identifiers.
//!
//! `JournalEvent` and `DurableActionOutcome` live in `event.rs`; this
//! module hosts the impl block plus the wire-format re-exports. The
//! wire-format sub-modules (`wire*`) and their `decode_*` /
//! `is_schema_one_shared_envelope_compatible` re-exports are unchanged.

use crate::{JournalError, RecordKind};
use vb_core::{RunId, SlotValue};

mod event;
mod wire;

pub(crate) use wire::{
    decode_journal_event_payload_for_envelope, is_schema_one_shared_envelope_compatible,
};

pub use event::{DurableActionOutcome, JournalEvent};

impl JournalEvent {
    /// Run identifier carried by this event.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        match self {
            Self::RunAccepted { run, .. }
            | Self::RunAdmission { run, .. }
            | Self::StepStarted { run, .. }
            | Self::StepSucceeded { run, .. }
            | Self::StepFailed { run, .. }
            | Self::ActionScheduled { run, .. }
            | Self::ActionCompletedEvent { run, .. }
            | Self::ActionScheduledTicket { run, .. }
            | Self::ActionCompletedEnvelope { run, .. }
            | Self::ActionFailedEvent { run, .. }
            | Self::ActionAbandoned { run, .. }
            | Self::SlotWrittenEvent { run, .. }
            | Self::WaitScheduledEvent { run, .. }
            | Self::AskScheduledEvent { run, .. }
            | Self::AskAnsweredEvent { run, .. }
            | Self::WaitResolvedEvent { run, .. }
            | Self::RetryScheduledEvent { run, .. }
            | Self::RunCancelled { run, .. }
            | Self::RunKilled { run, .. }
            | Self::RunFinished { run, .. }
            | Self::RunFailedEvent { run, .. }
            | Self::RunResumed { run, .. }
            | Self::RunRetried { run, .. }
            | Self::RunAnswered { run, .. }
            | Self::AskTimedOutEvent { run, .. } => *run,
        }
    }

    /// Event sequence carried by this event.
    ///
    /// Lifecycle events (RunResumed, RunRetried, RunAnswered) now carry sequence numbers
    /// to enable deduplication and ordering in the journal.
    #[must_use]
    pub const fn seq(&self) -> crate::EventSeq {
        match self {
            Self::RunAccepted { seq, .. }
            | Self::RunAdmission { seq, .. }
            | Self::StepStarted { seq, .. }
            | Self::StepSucceeded { seq, .. }
            | Self::StepFailed { seq, .. }
            | Self::ActionScheduled { seq, .. }
            | Self::ActionCompletedEvent { seq, .. }
            | Self::ActionScheduledTicket { seq, .. }
            | Self::ActionCompletedEnvelope { seq, .. }
            | Self::ActionFailedEvent { seq, .. }
            | Self::ActionAbandoned { seq, .. }
            | Self::SlotWrittenEvent { seq, .. }
            | Self::WaitScheduledEvent { seq, .. }
            | Self::AskScheduledEvent { seq, .. }
            | Self::AskAnsweredEvent { seq, .. }
            | Self::WaitResolvedEvent { seq, .. }
            | Self::RetryScheduledEvent { seq, .. }
            | Self::RunCancelled { seq, .. }
            | Self::RunKilled { seq, .. }
            | Self::RunFinished { seq, .. }
            | Self::RunFailedEvent { seq, .. }
            | Self::RunResumed { seq, .. }
            | Self::RunRetried { seq, .. }
            | Self::RunAnswered { seq, .. }
            | Self::AskTimedOutEvent { seq, .. } => *seq,
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
            Self::StepFailed { .. } => RecordKind::StepFailed,
            Self::ActionScheduled { .. } => RecordKind::ActionScheduled,
            Self::ActionCompletedEvent { .. } => RecordKind::ActionCompleted,
            Self::ActionScheduledTicket { .. } => RecordKind::ActionScheduledTicket,
            Self::ActionCompletedEnvelope { .. } => RecordKind::ActionCompletedEnvelope,
            Self::ActionFailedEvent { .. } => RecordKind::ActionFailed,
            Self::ActionAbandoned { .. } => RecordKind::ActionAbandoned,
            Self::SlotWrittenEvent { .. } => RecordKind::SlotWritten,
            Self::WaitScheduledEvent { .. } => RecordKind::WaitScheduled,
            Self::AskScheduledEvent { .. } => RecordKind::AskScheduled,
            Self::AskAnsweredEvent { .. } => RecordKind::AskAnswered,
            Self::WaitResolvedEvent { .. } => RecordKind::WaitResolved,
            Self::RetryScheduledEvent { .. } => RecordKind::RetryScheduled,
            Self::RunCancelled { .. } => RecordKind::RunCancelled,
            Self::RunKilled { .. } => RecordKind::RunKilled,
            Self::RunFinished { .. } => RecordKind::RunFinished,
            Self::RunFailedEvent { .. } => RecordKind::RunFailed,
            Self::RunResumed { .. } => RecordKind::RunResumed,
            Self::RunRetried { .. } => RecordKind::RunRetried,
            Self::RunAnswered { .. } => RecordKind::RunAnswered,
            Self::AskTimedOutEvent { .. } => RecordKind::AskTimedOut,
        }
    }

    /// Returns the slot value if this is a `SlotWrittenEvent` and a value was captured.
    ///
    /// Returns `Ok(None)` if no value was captured (absent optional payload).
    /// Returns `Ok(Some(slot_value))` if decoding succeeded.
    /// Returns `Err(JournalError::PostcardDecodeFailed(_))` if bytes are corrupt/truncated.
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
                postcard::from_bytes(bytes)
                    .map(Some)
                    .map_err(JournalError::PostcardDecodeFailed)
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
            | Self::WaitResolvedEvent { attempt, .. }
            | Self::RetryScheduledEvent { attempt, .. }
            | Self::StepStarted { attempt, .. }
            | Self::StepFailed { attempt, .. }
            | Self::RunCancelled { attempt, .. }
            | Self::RunKilled { attempt, .. }
            | Self::RunFinished { attempt, .. }
            | Self::RunFailedEvent { attempt, .. }
            | Self::AskTimedOutEvent { attempt, .. } => Some(*attempt),
            Self::ActionScheduledTicket { ticket, .. }
            | Self::ActionCompletedEnvelope { ticket, .. }
            | Self::ActionAbandoned { ticket, .. } => Some(ticket.attempt),
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
            | Self::WaitResolvedEvent { attempt, .. }
            | Self::RetryScheduledEvent { attempt, .. }
            | Self::StepStarted { attempt, .. }
            | Self::StepFailed { attempt, .. }
            | Self::RunCancelled { attempt, .. }
            | Self::RunKilled { attempt, .. }
            | Self::RunFinished { attempt, .. }
            | Self::RunFailedEvent { attempt, .. }
            | Self::AskTimedOutEvent { attempt, .. } => *attempt != 0,
            Self::ActionScheduledTicket { ticket, .. }
            | Self::ActionCompletedEnvelope { ticket, .. }
            | Self::ActionAbandoned { ticket, .. } => ticket.attempt != 0,
            Self::RunAccepted { .. }
            | Self::RunAdmission { .. }
            | Self::StepSucceeded { .. }
            | Self::RunResumed { .. }
            | Self::RunRetried { .. }
            | Self::RunAnswered { .. } => true,
        }
    }
}
