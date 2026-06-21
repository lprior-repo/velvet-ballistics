//! Incident analysis data types.

use crate::{EventSeq, RecordKind};

/// Terminal run-level failure classification derived from existing journal events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentFailureKind {
    /// `JournalEvent::RunFailedEvent` was observed.
    RunFailed,
    /// `JournalEvent::RunCancelled` was observed.
    RunCancelled,
    /// `JournalEvent::RunKilled` was observed.
    RunKilled,
}

impl IncidentFailureKind {
    /// Stable CLI-compatible code for this failure kind.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RunFailed => "RunFailed",
            Self::RunCancelled => "RunCancelled",
            Self::RunKilled => "RunKilled",
        }
    }
}

/// Action-side-effect evidence status derived from action journal events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideEffectDisposition {
    /// Action was durably scheduled but not yet resolved by completion/failure.
    Scheduled,
    /// Action completed durably.
    Completed,
    /// Action failed durably.
    Failed,
}

/// Durable action evidence with sequence and attempt context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideEffectEvidence {
    pub seq: EventSeq,
    pub step: u16,
    pub action: u16,
    pub attempt: u16,
    pub disposition: SideEffectDisposition,
}

/// Last durable journal checkpoint seen by incident analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncidentCheckpoint {
    pub seq: EventSeq,
    pub kind: RecordKind,
    pub step: Option<u16>,
    pub action: Option<u16>,
    pub slot: Option<u16>,
    pub attempt: Option<u16>,
}

/// Per-variant event counts useful for incident reports and CLI diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IncidentEventCounts {
    pub total: usize,
    pub run_accepted: usize,
    pub run_admission: usize,
    pub steps_started: usize,
    pub steps_succeeded: usize,
    pub actions_scheduled: usize,
    pub actions_completed: usize,
    pub actions_failed: usize,
    pub slot_writes: usize,
    pub waits_scheduled: usize,
    pub waits_cancelled: usize,
    pub asks_scheduled: usize,
    pub asks_answered: usize,
    pub asks_cancelled: usize,
    pub retries_scheduled: usize,
    pub run_cancelled: usize,
    pub run_killed: usize,
    pub run_finished: usize,
    pub run_failed: usize,
    pub run_resumed: usize,
    pub run_retried: usize,
    pub run_answered: usize,
}

/// Whether an action succeeded or failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideEffectCertainty {
    Confirmed,
    Failed,
}

/// Side effect recorded from an action event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideEffect {
    pub step: u16,
    pub action: u16,
    pub certainty: SideEffectCertainty,
}

/// Incident analysis result from scanning journal events.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IncidentAnalysis {
    pub failure_found: bool,
    pub failure_kind: Option<IncidentFailureKind>,
    pub failure_code: String,
    pub failed_at_step: Option<u16>,
    pub last_sequence: Option<EventSeq>,
    pub last_checkpoint: Option<IncidentCheckpoint>,
    pub counts: IncidentEventCounts,
    pub side_effects: Vec<SideEffect>,
    pub side_effect_evidence: Vec<SideEffectEvidence>,
    pub failed_action_evidence: Vec<SideEffectEvidence>,
    pub pending_scheduled_actions: Vec<SideEffectEvidence>,
}
