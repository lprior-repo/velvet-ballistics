use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Incident {
    pub id: u64,
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
