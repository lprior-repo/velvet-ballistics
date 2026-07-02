#![forbid(unsafe_code)]
//! Canonical `JournalEvent` name lookup.
//!
//! Owns [`KnownVariant::name`], [`KnownVariant::try_from_event`], and the
//! top-level [`event_name`] free function. The closed-enum contract is
//! enforced here: every new variant requires a `name()` arm (compile
//! error otherwise) and a `try_from_event` arm (compile error otherwise).

use vb_storage::events::JournalEvent;

use super::schema::KnownVariant;

impl KnownVariant {
    /// Canonical variant name used by [`event_name`] and the JSON
    /// `"type"` field of [`super::diff::diff_event_summary`]. This match
    /// is exhaustive on `KnownVariant`; adding a new variant without
    /// updating it fails at compile time.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::RunAccepted => "RunAccepted",
            Self::RunAdmission => "RunAdmission",
            Self::StepStarted => "StepStarted",
            Self::StepSucceeded => "StepSucceeded",
            Self::ActionScheduled => "ActionScheduled",
            Self::ActionCompletedEvent => "ActionCompleted",
            Self::ActionScheduledTicket => "ActionScheduledTicket",
            Self::ActionCompletedEnvelope => "ActionCompletedEnvelope",
            Self::ActionFailedEvent => "ActionFailed",
            Self::ActionAbandoned => "ActionAbandoned",
            Self::SlotWrittenEvent => "SlotWritten",
            Self::WaitScheduledEvent => "WaitScheduled",
            Self::AskScheduledEvent => "AskScheduled",
            Self::AskAnsweredEvent => "AskAnswered",
            Self::WaitResolvedEvent => "WaitResolved",
            Self::RetryScheduledEvent => "RetryScheduled",
            Self::RunCancelled => "RunCancelled",
            Self::RunKilled => "RunKilled",
            Self::RunFinished => "RunFinished",
            Self::RunFailedEvent => "RunFailed",
            Self::RunResumed => "RunResumed",
            Self::RunRetried => "RunRetried",
            Self::RunAnswered => "RunAnswered",
            Self::AskTimedOutEvent => "AskTimedOut",
        }
    }

    /// Attempt to classify an event as one of the known variants.
    ///
    /// Returns `None` for genuinely-new `JournalEvent` variants added after
    /// this snapshot. The `#[non_exhaustive]` upstream attribute forces a
    /// wildcard arm even for a fully-exhaustive local list; that arm is
    /// the only path that returns `None`.
    pub(crate) fn try_from_event(event: &JournalEvent) -> Option<Self> {
        Some(match event {
            JournalEvent::RunAccepted { .. } => Self::RunAccepted,
            JournalEvent::RunAdmission { .. } => Self::RunAdmission,
            JournalEvent::StepStarted { .. } => Self::StepStarted,
            JournalEvent::StepSucceeded { .. } => Self::StepSucceeded,
            JournalEvent::ActionScheduled { .. } => Self::ActionScheduled,
            JournalEvent::ActionCompletedEvent { .. } => Self::ActionCompletedEvent,
            JournalEvent::ActionScheduledTicket { .. } => Self::ActionScheduledTicket,
            JournalEvent::ActionCompletedEnvelope { .. } => Self::ActionCompletedEnvelope,
            JournalEvent::ActionFailedEvent { .. } => Self::ActionFailedEvent,
            JournalEvent::ActionAbandoned { .. } => Self::ActionAbandoned,
            JournalEvent::SlotWrittenEvent { .. } => Self::SlotWrittenEvent,
            JournalEvent::WaitScheduledEvent { .. } => Self::WaitScheduledEvent,
            JournalEvent::AskScheduledEvent { .. } => Self::AskScheduledEvent,
            JournalEvent::AskAnsweredEvent { .. } => Self::AskAnsweredEvent,
            JournalEvent::WaitResolvedEvent { .. } => Self::WaitResolvedEvent,
            JournalEvent::RetryScheduledEvent { .. } => Self::RetryScheduledEvent,
            JournalEvent::RunCancelled { .. } => Self::RunCancelled,
            JournalEvent::RunKilled { .. } => Self::RunKilled,
            JournalEvent::RunFinished { .. } => Self::RunFinished,
            JournalEvent::RunFailedEvent { .. } => Self::RunFailedEvent,
            JournalEvent::RunResumed { .. } => Self::RunResumed,
            JournalEvent::RunRetried { .. } => Self::RunRetried,
            JournalEvent::RunAnswered { .. } => Self::RunAnswered,
            JournalEvent::AskTimedOutEvent { .. } => Self::AskTimedOutEvent,
            _ => return None,
        })
    }
}

/// Return the static name string for an event variant.
///
/// For known variants the name is taken from [`KnownVariant::name`]; for
/// future `#[non_exhaustive]` additions the literal `"Unknown"` is
/// returned. The companion test
/// `every_known_variant_maps_to_a_non_unknown_name` enforces that no
/// current variant falls through.
pub fn event_name(event: &JournalEvent) -> &'static str {
    KnownVariant::try_from_event(event).map_or("Unknown", KnownVariant::name)
}
