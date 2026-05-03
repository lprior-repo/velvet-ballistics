use std::time::Instant;

/// Classification of the incident category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentType {
    ActionFailure,
    ReplayDivergence,
    BlockedReconciliation,
    SecretLeak,
}

#[derive(Debug, Clone)]
pub struct Incident {
    pub id: u64,
    pub incident_type: IncidentType,
    pub severity: IncidentSeverity,
    pub failure_code: FailureCode,
    pub run_id: u64,
    pub workflow_name: String,
    pub step_id: Option<u16>,
    pub step_name: Option<String>,
    pub error_message: String,
    pub replay_safe: bool,
    pub side_effect_certainty: SideEffectCertainty,
    pub timestamp: Instant,
    pub context: IncidentContext,
    pub timeline: Vec<TimelineEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentSeverity {
    Critical,
    Major,
    Minor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureCode {
    ActionTimeout,
    ActionFailed(String),
    BudgetExceeded,
    StepPanicked,
    ValidationError(String),
    TaintLeak,
    ReplayDivergence,
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideEffectCertainty {
    Certain,
    Unknown,
    None,
}

#[derive(Debug, Clone)]
pub struct IncidentContext {
    pub slot_values_before: Vec<(u16, String)>,
    pub taint_changes: Vec<(u16, String)>,
    pub action_attempts: u32,
    pub last_action_idempotency_key: Option<String>,
}

/// Structured failure detail returned when querying an incident.
#[derive(Debug, Clone)]
pub struct FailureDetail {
    pub error_code: String,
    pub step_id: Option<u16>,
    pub run_id: u64,
    pub workflow_name: String,
    pub replay_safe: bool,
    pub timeline: Vec<TimelineEntry>,
    /// Original failure code for callers that need structured access.
    pub failure_code: FailureCode,
    /// Step name for display purposes.
    pub step_name: Option<String>,
    /// Side-effect certainty classification.
    pub side_effect_certainty: SideEffectCertainty,
    /// Incident context with slot and taint information.
    pub error_context: IncidentContext,
}

/// A single chronological event in the incident timeline.
#[derive(Debug, Clone)]
pub struct TimelineEntry {
    pub seq: u32,
    pub description: String,
    pub timestamp_micros: u64,
    /// Classification of the timeline event kind.
    pub event_kind: TimelineEventKind,
    /// Original instant for callers that need monotonic time.
    pub timestamp: Instant,
}

/// Classification of a timeline event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineEventKind {
    FailureObserved,
    RetryAttempted,
    SideEffectDetected,
    ReplayDivergence,
    RepairApplied,
    IncidentDismissed,
}
