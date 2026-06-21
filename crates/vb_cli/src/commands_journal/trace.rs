#![forbid(unsafe_code)]
//! Trace inspection types and pure analysis logic.
//!
//! Produces structured [`TraceEntry`] items from journal events, classifies
//! events into [`TraceStatus`] categories, and applies optional filters.

use crate::args::EventStatus;
use vb_storage::JournalEvent;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Structured trace entry produced by [`build_trace`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TraceEntry {
    pub index: usize,
    pub event_type: &'static str,
    pub step: Option<u16>,
    pub status: Option<TraceStatus>,
    pub action: Option<u16>,
    pub seq: u64,
    /// Extra key-value pairs for JSON output (variant-specific fields).
    pub extra_json: Vec<(&'static str, serde_json::Value)>,
}

/// Trace status categories used by CLI filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TraceStatus {
    Pending,
    Active,
    WaitingAnswer,
    Cancelled,
    Completed,
    Failed,
}

impl TraceStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::WaitingAnswer => "waiting_answer",
            Self::Cancelled => "cancelled",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

impl std::str::FromStr for TraceStatus {
    type Err = crate::args::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "active" => Ok(Self::Active),
            "waiting_answer" => Ok(Self::WaitingAnswer),
            "cancelled" => Ok(Self::Cancelled),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            other => Err(crate::args::ParseError::InvalidTraceArgument(other.into())),
        }
    }
}

/// Convert a CLI `EventStatus` filter into the canonical `TraceStatus` used by
/// the in-memory filter. The two enums share the same variant set, so this is
/// a direct structural mapping.
impl From<EventStatus> for TraceStatus {
    fn from(value: EventStatus) -> Self {
        match value {
            EventStatus::Pending => Self::Pending,
            EventStatus::Active => Self::Active,
            EventStatus::WaitingAnswer => Self::WaitingAnswer,
            EventStatus::Cancelled => Self::Cancelled,
            EventStatus::Completed => Self::Completed,
            EventStatus::Failed => Self::Failed,
        }
    }
}

/// Optional trace filters. All populated filters must match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct TraceFilters {
    pub(crate) step: Option<u16>,
    pub(crate) action: Option<u16>,
    pub(crate) status: Option<TraceStatus>,
    pub(crate) since_seq: Option<u64>,
    pub(crate) until_seq: Option<u64>,
    pub(crate) limit: Option<usize>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Scan a slice of journal events and return one structured entry per event.
pub(crate) fn build_trace(events: &[JournalEvent]) -> Vec<TraceEntry> {
    events
        .iter()
        .enumerate()
        .map(|(idx, event)| trace_one(idx, event))
        .collect()
}

/// Map a single journal event to its trace status category. Returns `None` for
/// event variants that have no meaningful status (e.g. internal envelopes).
/// This is the canonical single source of truth for status classification; both
/// [`trace_one`] and the `events --status` filter use it.
pub(crate) fn event_status(event: &JournalEvent) -> Option<TraceStatus> {
    match event {
        JournalEvent::RunAccepted { .. } | JournalEvent::RunAdmission { .. } => {
            Some(TraceStatus::Pending)
        }
        JournalEvent::StepStarted { .. }
        | JournalEvent::ActionScheduled { .. }
        | JournalEvent::ActionScheduledTicket { .. }
        | JournalEvent::WaitScheduledEvent { .. }
        | JournalEvent::RetryScheduledEvent { .. }
        | JournalEvent::RunResumed { .. }
        | JournalEvent::RunRetried { .. } => Some(TraceStatus::Active),
        JournalEvent::StepSucceeded { .. }
        | JournalEvent::ActionCompletedEvent { .. }
        | JournalEvent::ActionCompletedEnvelope { .. }
        | JournalEvent::SlotWrittenEvent { .. }
        | JournalEvent::AskAnsweredEvent { .. }
        | JournalEvent::RunFinished { .. }
        | JournalEvent::RunAnswered { .. } => Some(TraceStatus::Completed),
        JournalEvent::ActionFailedEvent { .. } | JournalEvent::RunFailedEvent { .. } => {
            Some(TraceStatus::Failed)
        }
        JournalEvent::AskScheduledEvent { .. } => Some(TraceStatus::WaitingAnswer),
        JournalEvent::RunCancelled { .. } | JournalEvent::RunKilled { .. } => {
            Some(TraceStatus::Cancelled)
        }
        _ => None,
    }
}

/// Apply trace filters while preserving the surviving entries' original order and index.
pub(crate) fn filter_trace(entries: Vec<TraceEntry>, filters: TraceFilters) -> Vec<TraceEntry> {
    let filtered = entries
        .into_iter()
        .filter(|entry| trace_entry_matches_filters(entry, filters));
    match filters.limit {
        Some(limit) => filtered.take(limit).collect(),
        None => filtered.collect(),
    }
}

/// Apply the `events --status` and `events --limit` filters to a raw event list.
///
/// - `status` selects events whose [`event_status`] matches the given value.
///   `None` preserves every event.
/// - `limit` truncates the result to at most `limit` events AFTER the status
///   filter has been applied. `None` or a non-positive value is treated as
///   unbounded.
///
/// The original ordering of the journal is preserved.
pub(crate) fn filter_events(
    events: Vec<JournalEvent>,
    status: Option<EventStatus>,
    limit: Option<i64>,
) -> Vec<JournalEvent> {
    let status_filter: Option<TraceStatus> = status.map(TraceStatus::from);
    let filtered: Vec<JournalEvent> = events
        .into_iter()
        .filter(|event| match status_filter {
            Some(expected) => event_status(event) == Some(expected),
            None => true,
        })
        .collect();
    let limit_usize = match limit {
        Some(n) => usize::try_from(n).unwrap_or(0),
        None => usize::MAX,
    };
    filtered.into_iter().take(limit_usize).collect()
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn trace_entry_matches_filters(entry: &TraceEntry, filters: TraceFilters) -> bool {
    let step_matches = filters
        .step
        .is_none_or(|expected_step| entry.step == Some(expected_step));
    let action_matches = filters
        .action
        .is_none_or(|expected_action| entry.action == Some(expected_action));
    let status_matches = filters
        .status
        .is_none_or(|expected_status| entry.status == Some(expected_status));
    let since_seq_matches = filters
        .since_seq
        .is_none_or(|minimum_seq| entry.seq >= minimum_seq);
    let until_seq_matches = filters
        .until_seq
        .is_none_or(|maximum_seq| entry.seq <= maximum_seq);
    step_matches && action_matches && status_matches && since_seq_matches && until_seq_matches
}

fn trace_one(idx: usize, event: &JournalEvent) -> TraceEntry {
    let meta = trace_event_metadata(event);
    TraceEntry {
        index: idx,
        event_type: meta.event_type,
        step: meta.step,
        status: event_status(event),
        action: meta.action,
        seq: trace_seq(event),
        extra_json: meta.extra_json,
    }
}

/// Structured metadata extracted from a single journal event variant.
struct TraceEventMetadata {
    event_type: &'static str,
    step: Option<u16>,
    action: Option<u16>,
    extra_json: Vec<(&'static str, serde_json::Value)>,
}

/// Extract the variant label plus variant-specific structured metadata.
fn trace_event_metadata(event: &JournalEvent) -> TraceEventMetadata {
    match event {
        JournalEvent::RunAccepted { run, workflow, .. } => TraceEventMetadata {
            event_type: "RunAccepted",
            step: None,
            action: None,
            extra_json: vec![
                ("run", serde_json::Value::from(run.get())),
                ("workflow", serde_json::Value::from(format!("{workflow:?}"))),
            ],
        },
        JournalEvent::RunAdmission {
            artifact_digest,
            granted_capabilities,
            policy,
            ..
        } => TraceEventMetadata {
            event_type: "RunAdmission",
            step: None,
            action: None,
            extra_json: vec![
                (
                    "artifact_digest",
                    serde_json::Value::from(format!("{artifact_digest:?}")),
                ),
                (
                    "granted_capabilities",
                    serde_json::Value::from(format!("{granted_capabilities:?}")),
                ),
                ("policy", serde_json::Value::from(format!("{policy:?}"))),
            ],
        },
        JournalEvent::StepStarted { step, .. } => TraceEventMetadata {
            event_type: "StepStarted",
            step: Some(step.get()),
            action: None,
            extra_json: vec![],
        },
        JournalEvent::StepSucceeded { step, output, .. } => TraceEventMetadata {
            event_type: "StepSucceeded",
            step: Some(step.get()),
            action: None,
            extra_json: vec![("output", serde_json::Value::from(output.get()))],
        },
        JournalEvent::ActionScheduled { step, action, .. } => TraceEventMetadata {
            event_type: "ActionScheduled",
            step: Some(step.get()),
            action: Some(action.get()),
            extra_json: vec![("action", serde_json::Value::from(action.get()))],
        },
        JournalEvent::ActionCompletedEvent { step, action, .. } => TraceEventMetadata {
            event_type: "ActionCompleted",
            step: Some(step.get()),
            action: Some(action.get()),
            extra_json: vec![("action", serde_json::Value::from(action.get()))],
        },
        JournalEvent::ActionFailedEvent { step, action, .. } => TraceEventMetadata {
            event_type: "ActionFailed",
            step: Some(step.get()),
            action: Some(action.get()),
            extra_json: vec![("action", serde_json::Value::from(action.get()))],
        },
        JournalEvent::SlotWrittenEvent { slot, .. } => TraceEventMetadata {
            event_type: "SlotWritten",
            step: None,
            action: None,
            extra_json: vec![("slot", serde_json::Value::from(slot.get()))],
        },
        JournalEvent::WaitScheduledEvent { step, .. } => TraceEventMetadata {
            event_type: "WaitScheduled",
            step: Some(step.get()),
            action: None,
            extra_json: vec![],
        },
        JournalEvent::AskScheduledEvent { step, .. } => TraceEventMetadata {
            event_type: "AskScheduled",
            step: Some(step.get()),
            action: None,
            extra_json: vec![],
        },
        JournalEvent::AskAnsweredEvent { step, .. } => TraceEventMetadata {
            event_type: "AskAnswered",
            step: Some(step.get()),
            action: None,
            extra_json: vec![],
        },
        JournalEvent::RetryScheduledEvent { step, .. } => TraceEventMetadata {
            event_type: "RetryScheduled",
            step: Some(step.get()),
            action: None,
            extra_json: vec![],
        },
        JournalEvent::RunCancelled { .. } => TraceEventMetadata {
            event_type: "RunCancelled",
            step: None,
            action: None,
            extra_json: vec![],
        },
        JournalEvent::RunFinished { result, .. } => TraceEventMetadata {
            event_type: "RunFinished",
            step: None,
            action: None,
            extra_json: vec![("result", serde_json::Value::from(result.get()))],
        },
        JournalEvent::RunFailedEvent { .. } => TraceEventMetadata {
            event_type: "RunFailed",
            step: None,
            action: None,
            extra_json: vec![],
        },
        JournalEvent::RunResumed { run, .. } => TraceEventMetadata {
            event_type: "RunResumed",
            step: None,
            action: None,
            extra_json: vec![("run", serde_json::Value::from(run.get()))],
        },
        JournalEvent::RunRetried { run, .. } => TraceEventMetadata {
            event_type: "RunRetried",
            step: None,
            action: None,
            extra_json: vec![("run", serde_json::Value::from(run.get()))],
        },
        JournalEvent::RunAnswered {
            run,
            slot_idx,
            answer,
            ..
        } => TraceEventMetadata {
            event_type: "RunAnswered",
            step: None,
            action: None,
            extra_json: vec![
                ("run", serde_json::Value::from(run.get())),
                ("slot_idx", serde_json::Value::from(slot_idx.get())),
                ("answer", serde_json::Value::from(format!("{:?}", answer))),
            ],
        },
        // The ticket and envelope variants were added after the trace surface
        // was defined. Map them to the same labels as their non-ticket
        // counterparts so the trace remains a single label-per-variant surface.
        // They carry `output` slot indices (not step indices) so the trace
        // surfaces the output slot rather than a step.
        JournalEvent::ActionScheduledTicket { output, .. } => TraceEventMetadata {
            event_type: "ActionScheduled",
            step: None,
            action: None,
            extra_json: vec![("output", serde_json::Value::from(output.get()))],
        },
        JournalEvent::ActionCompletedEnvelope { output, .. } => TraceEventMetadata {
            event_type: "ActionCompleted",
            step: None,
            action: None,
            extra_json: vec![("output", serde_json::Value::from(output.get()))],
        },
        JournalEvent::RunKilled { .. } => TraceEventMetadata {
            event_type: "RunKilled",
            step: None,
            action: None,
            extra_json: vec![],
        },
        _ => TraceEventMetadata {
            event_type: "Unknown",
            step: None,
            action: None,
            extra_json: vec![],
        },
    }
}

fn trace_seq(event: &JournalEvent) -> u64 {
    if let Some(seq) = event_seq(event) {
        seq.get()
    } else {
        0
    }
}

fn event_seq(event: &JournalEvent) -> Option<vb_storage::EventSeq> {
    match event {
        JournalEvent::RunAccepted { seq, .. }
        | JournalEvent::RunAdmission { seq, .. }
        | JournalEvent::StepStarted { seq, .. }
        | JournalEvent::StepSucceeded { seq, .. }
        | JournalEvent::ActionScheduled { seq, .. }
        | JournalEvent::ActionCompletedEvent { seq, .. }
        | JournalEvent::ActionScheduledTicket { seq, .. }
        | JournalEvent::ActionCompletedEnvelope { seq, .. }
        | JournalEvent::ActionFailedEvent { seq, .. }
        | JournalEvent::SlotWrittenEvent { seq, .. }
        | JournalEvent::WaitScheduledEvent { seq, .. }
        | JournalEvent::AskScheduledEvent { seq, .. }
        | JournalEvent::AskAnsweredEvent { seq, .. }
        | JournalEvent::RetryScheduledEvent { seq, .. }
        | JournalEvent::RunCancelled { seq, .. }
        | JournalEvent::RunKilled { seq, .. }
        | JournalEvent::RunFinished { seq, .. }
        | JournalEvent::RunFailedEvent { seq, .. } => Some(*seq),
        JournalEvent::RunResumed { .. }
        | JournalEvent::RunRetried { .. }
        | JournalEvent::RunAnswered { .. } => None,
        _ => None,
    }
}
