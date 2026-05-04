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
    Warning,
    Info,
}

impl IncidentSeverity {
    /// Return the display color for this severity level as RGBA floats.
    /// Critical=#ff073a, Warning=#ffe600, Info=#00f5ff,
    /// Major=#ff8800, Minor=#888888.
    pub fn severity_color(&self) -> [f32; 4] {
        match self {
            Self::Critical => [1.0_f32, 0.027_f32, 0.227_f32, 1.0_f32],
            Self::Warning => [1.0_f32, 0.902_f32, 0.0_f32, 1.0_f32],
            Self::Info => [0.0_f32, 0.961_f32, 1.0_f32, 1.0_f32],
            Self::Major => [1.0_f32, 0.533_f32, 0.0_f32, 1.0_f32],
            Self::Minor => [0.533_f32, 0.533_f32, 0.533_f32, 1.0_f32],
        }
    }
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

impl FailureCode {
    /// Return a static string label for this failure code.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ActionTimeout => "ActionTimeout",
            Self::ActionFailed(_) => "ActionFailed",
            Self::BudgetExceeded => "StepBudgetExhausted",
            Self::StepPanicked => "StepPanicked",
            Self::ValidationError(_) => "ValidationError",
            Self::TaintLeak => "TaintViolation",
            Self::ReplayDivergence => "ReplayDivergence",
            Self::Unknown(_) => "InternalError",
        }
    }
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

/// Replay safety classification for an incident record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplaySafety {
    Safe,
    UnsafeSideEffect,
    Unknown,
}

impl ReplaySafety {
    /// Return true if replay is considered safe.
    pub fn is_safe(&self) -> bool {
        matches!(self, Self::Safe)
    }
}

/// Lightweight incident record for Phase 5A tracking.
#[derive(Debug, Clone)]
pub struct IncidentRecord {
    pub run_id: u64,
    pub shard_id: u32,
    pub step: u16,
    pub failure_code: FailureCode,
    pub severity: IncidentSeverity,
    pub replay_safety: ReplaySafety,
    pub timestamp_us: u64,
    pub detail: String,
}
