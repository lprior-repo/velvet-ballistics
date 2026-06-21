#![forbid(unsafe_code)]
//! Accessor methods on `JournalEvent`.
//!
//! Extracts run identifiers, sequence numbers, record kinds, and attempt metadata.

use super::variant::JournalEvent;
use crate::{EventSeq, RecordKind};
use vb_core::RunId;

/// Match-arm fragment covering every `JournalEvent` variant and binding the
/// named common field (`run` or `seq`).
///
/// Invoked at match-arm position; the OR-pattern it produces is followed by
/// `=>` and a body at the call site. Used by `run_id` and `seq` so that
/// adding a new variant only requires appending one line here.
macro_rules! event_variants {
    ($field:ident) => {
        Self::RunAccepted { $field, .. }
            | Self::RunAdmission { $field, .. }
            | Self::StepStarted { $field, .. }
            | Self::StepSucceeded { $field, .. }
            | Self::ActionScheduled { $field, .. }
            | Self::ActionCompletedEvent { $field, .. }
            | Self::ActionScheduledTicket { $field, .. }
            | Self::ActionCompletedEnvelope { $field, .. }
            | Self::ActionFailedEvent { $field, .. }
            | Self::SlotWrittenEvent { $field, .. }
            | Self::WaitScheduledEvent { $field, .. }
            | Self::WaitCancelledEvent { $field, .. }
            | Self::AskScheduledEvent { $field, .. }
            | Self::AskAnsweredEvent { $field, .. }
            | Self::AskCancelledEvent { $field, .. }
            | Self::RetryScheduledEvent { $field, .. }
            | Self::RunCancelled { $field, .. }
            | Self::RunKilled { $field, .. }
            | Self::RunFinished { $field, .. }
            | Self::RunFailedEvent { $field, .. }
            | Self::RunResumed { $field, .. }
            | Self::RunRetried { $field, .. }
            | Self::RunAnswered { $field, .. }
    };
}

/// Match expression for `attempt()`, emitted as a complete expression body.
/// Three categories are recognised: direct `attempt: u16` field, `ticket.attempt`
/// via the enclosed `ActionTicket`, and absent (returns `None`).
///
/// Invoked at expression position; macro_rules! cannot expand directly to
/// match arms, so the entire match expression lives here.
macro_rules! attempt_match {
    ($self:expr) => {
        match $self {
            Self::ActionScheduled { attempt, .. }
            | Self::ActionCompletedEvent { attempt, .. }
            | Self::ActionFailedEvent { attempt, .. }
            | Self::SlotWrittenEvent { attempt, .. }
            | Self::WaitScheduledEvent { attempt, .. }
            | Self::WaitCancelledEvent { attempt, .. }
            | Self::AskScheduledEvent { attempt, .. }
            | Self::AskAnsweredEvent { attempt, .. }
            | Self::AskCancelledEvent { attempt, .. }
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
    };
}

impl JournalEvent {
    /// Run identifier carried by this event.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        match self {
            event_variants!(run) => *run,
        }
    }

    /// Event sequence carried by this event.
    ///
    /// Lifecycle events (RunResumed, RunRetried, RunAnswered) now carry sequence numbers
    /// to enable deduplication and ordering in the journal.
    #[must_use]
    pub const fn seq(&self) -> EventSeq {
        match self {
            event_variants!(seq) => *seq,
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
            Self::WaitCancelledEvent { .. } => RecordKind::WaitCancelled,
            Self::AskScheduledEvent { .. } => RecordKind::AskScheduled,
            Self::AskAnsweredEvent { .. } => RecordKind::AskAnswered,
            Self::AskCancelledEvent { .. } => RecordKind::AskCancelled,
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

    /// Returns true when the envelope kind is the canonical kind for this event.
    #[must_use]
    pub const fn has_canonical_envelope_kind(&self, envelope_kind: u16) -> bool {
        envelope_kind == self.record_kind_id()
    }

    /// Returns the attempt number for this event.
    ///
    /// Events that carry attempt info return `Some(attempt)`.
    /// Events that don't carry attempt info (`RunAccepted`, `RunAdmission`,
    /// `StepSucceeded`) return `None`; these are treated as
    /// attempt 1 by the replay filtering logic (PRE-001).
    #[must_use]
    pub const fn attempt(&self) -> Option<u16> {
        attempt_match!(self)
    }

    /// Returns the deadline duration in milliseconds for wait/ask events.
    ///
    /// Returns `Some(deadline_ms)` for `WaitScheduledEvent` and
    /// `AskScheduledEvent`. Returns `None` for other event types.
    #[must_use]
    pub const fn deadline_ms(&self) -> Option<u64> {
        match self {
            Self::WaitScheduledEvent { deadline_ms, .. }
            | Self::AskScheduledEvent { deadline_ms, .. } => Some(*deadline_ms),
            _ => None,
        }
    }
}
