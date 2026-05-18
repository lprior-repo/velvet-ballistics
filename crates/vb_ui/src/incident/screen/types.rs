#![forbid(unsafe_code)]
//! Incident screen data types and color helpers.

use std::time::Instant;

use super::colors::{
    NEON_CYAN, NEON_MAGENTA, NEON_ORANGE, NEON_RED, NEON_TEAL, NEON_YELLOW, TEXT_SECONDARY,
};
use super::repair::{RepairSuggestion, SuggestionItem, suggest_repairs};

pub fn failure_kind_color(kind: &FailureKind) -> &'static str {
    match kind {
        FailureKind::ActionTimeout => NEON_ORANGE,
        FailureKind::ActionFailed => NEON_RED,
        FailureKind::RunFailed => NEON_RED,
        FailureKind::RetryExhausted => NEON_YELLOW,
        FailureKind::TaintViolation => NEON_MAGENTA,
        FailureKind::InternalError => TEXT_SECONDARY,
    }
}

pub fn severity_color_hex(severity: IncidentSeverity) -> &'static str {
    match severity {
        IncidentSeverity::Critical => NEON_RED,
        IncidentSeverity::Major => NEON_ORANGE,
        IncidentSeverity::Minor => NEON_YELLOW,
        IncidentSeverity::Warning => NEON_YELLOW,
        IncidentSeverity::Info => NEON_CYAN,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FailureKind {
    ActionTimeout,
    ActionFailed,
    RunFailed,
    RetryExhausted,
    TaintViolation,
    InternalError,
}

impl FailureKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ActionTimeout => "ACTION_TIMEOUT",
            Self::ActionFailed => "ACTION_FAILED",
            Self::RunFailed => "RUN_FAILED",
            Self::RetryExhausted => "RETRY_EXHAUSTED",
            Self::TaintViolation => "TAINT_VIOLATION",
            Self::InternalError => "INTERNAL_ERROR",
        }
    }

    pub fn is_replay_safe_default(&self) -> bool {
        matches!(self, Self::ActionTimeout | Self::RetryExhausted)
    }
}

#[derive(Debug, Clone)]
pub struct IncidentCard {
    pub run_id: u64,
    pub workflow_name: String,
    pub step_idx: u16,
    pub failure_kind: FailureKind,
    pub timestamp: u64,
    pub replay_safe: bool,
}

impl IncidentCard {
    #[must_use]
    pub fn placeholder() -> Self {
        Self {
            run_id: 8172,
            workflow_name: String::from("issue-triage"),
            step_idx: 3,
            failure_kind: FailureKind::ActionTimeout,
            timestamp: 1_714_745_600,
            replay_safe: true,
        }
    }

    #[must_use]
    pub fn badge_color(&self) -> &'static str {
        failure_kind_color(&self.failure_kind)
    }

    #[must_use]
    pub fn run_id_text(&self) -> String {
        format!("{}", self.run_id)
    }

    #[must_use]
    pub fn step_text(&self) -> String {
        format!("StepIdx {}", self.step_idx)
    }
}

#[derive(Debug, Clone)]
pub struct CausePanel {
    pub error_message: String,
    pub context: String,
    pub recommended_action: String,
}

impl CausePanel {
    #[must_use]
    pub fn placeholder() -> Self {
        Self {
            error_message: String::from("TIMEOUT"),
            context: String::from("5s exceeded"),
            recommended_action: String::from("retry with same ticket"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimelineChip {
    pub seq: u32,
    pub kind: String,
    pub step_idx: u16,
}

impl TimelineChip {
    #[must_use]
    pub fn label(&self) -> String {
        format!("[{}] {}", self.seq, self.kind)
    }
}

#[derive(Debug, Clone)]
pub struct TimelinePanel {
    pub events: Vec<TimelineChip>,
}

impl TimelinePanel {
    #[must_use]
    pub fn placeholder() -> Self {
        Self {
            events: vec![
                TimelineChip {
                    seq: 12,
                    kind: String::from("StepStarted"),
                    step_idx: 3,
                },
                TimelineChip {
                    seq: 13,
                    kind: String::from("ActionScheduled"),
                    step_idx: 3,
                },
                TimelineChip {
                    seq: 14,
                    kind: String::from("ActionFailed"),
                    step_idx: 3,
                },
                TimelineChip {
                    seq: 14,
                    kind: String::from("RunFailed"),
                    step_idx: 3,
                },
            ],
        }
    }

    #[must_use]
    pub fn chip_count(&self) -> usize {
        self.events.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct SlotDiff {
    pub slot_idx: u16,
    pub before: String,
    pub after: String,
    pub taint_changed: bool,
}

impl SlotDiff {
    #[must_use]
    pub fn display_label(&self) -> String {
        if self.taint_changed {
            format!(
                "Taint(SlotIdx({})): {} -> {}",
                self.slot_idx, self.before, self.after
            )
        } else {
            format!(
                "SlotIdx({}): {} -> {}",
                self.slot_idx, self.before, self.after
            )
        }
    }

    #[must_use]
    pub fn value_changed(&self) -> bool {
        self.before != self.after
    }

    #[must_use]
    pub fn diff_color(&self) -> &'static str {
        if self.taint_changed {
            return NEON_MAGENTA;
        }
        if self.value_changed() {
            return NEON_CYAN;
        }
        TEXT_SECONDARY
    }
}

#[derive(Debug, Clone)]
pub struct StateDiffPanel {
    pub diffs: Vec<SlotDiff>,
}

impl StateDiffPanel {
    #[must_use]
    pub fn placeholder() -> Self {
        Self {
            diffs: vec![
                SlotDiff {
                    slot_idx: 12,
                    before: String::from("null"),
                    after: String::from("ObjectId(\"issue-8472\")"),
                    taint_changed: false,
                },
                SlotDiff {
                    slot_idx: 5,
                    before: String::from("Clean"),
                    after: String::from("DerivedFromSecret"),
                    taint_changed: true,
                },
            ],
        }
    }

    #[must_use]
    pub fn diff_count(&self) -> usize {
        self.diffs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.diffs.is_empty()
    }

    #[must_use]
    pub fn has_taint_changes(&self) -> bool {
        self.diffs.iter().any(|d| d.taint_changed)
    }

    #[must_use]
    pub fn has_value_changes(&self) -> bool {
        self.diffs.iter().any(|d| d.value_changed())
    }
}

pub fn suggest_repairs_for_failure_kind(kind: &FailureKind) -> &'static str {
    match kind {
        FailureKind::ActionTimeout => "Increase timeout or check downstream service",
        FailureKind::ActionFailed => "Investigate action failure and retry if replay-safe",
        FailureKind::RunFailed => "Review run failure and check system state",
        FailureKind::RetryExhausted => "Check external service availability",
        FailureKind::TaintViolation => "Add sanitization step before Finish",
        FailureKind::InternalError => "Contact support for internal error investigation",
    }
}

pub fn build_suggestion_items(
    kind: &FailureKind,
    repair_suggestions: Vec<RepairSuggestion>,
) -> Vec<SuggestionItem> {
    let kind_summary = suggest_repairs_for_failure_kind(kind);
    let badge_color = failure_kind_color(kind);

    let mut items = Vec::new();

    if let Some(primary) = repair_suggestions.first() {
        items.push(SuggestionItem {
            suggestion: primary.clone(),
            action_label: String::from(kind.label()),
            badge_color,
            summary: String::from(kind_summary),
        });
    }

    for suggestion in repair_suggestions.iter().skip(1) {
        items.push(SuggestionItem {
            suggestion: suggestion.clone(),
            action_label: String::from(kind.label()),
            badge_color: NEON_TEAL,
            summary: suggestion.description.clone(),
        });
    }

    items
}

pub fn suggestion_items_for_failure_code(
    failure_code: &super::types::FailureCode,
) -> Vec<SuggestionItem> {
    let kind = match failure_code {
        super::types::FailureCode::ActionTimeout => FailureKind::ActionTimeout,
        super::types::FailureCode::ActionFailed(_) => FailureKind::ActionFailed,
        super::types::FailureCode::BudgetExceeded => FailureKind::RunFailed,
        super::types::FailureCode::StepPanicked => FailureKind::InternalError,
        super::types::FailureCode::ValidationError(_) => FailureKind::InternalError,
        super::types::FailureCode::TaintLeak => FailureKind::TaintViolation,
        super::types::FailureCode::ReplayDivergence => FailureKind::RunFailed,
        super::types::FailureCode::Unknown(_) => FailureKind::InternalError,
    };
    let dummy_incident = super::types::Incident {
        id: 0,
        incident_type: super::types::IncidentType::ActionFailure,
        severity: super::types::IncidentSeverity::Info,
        failure_code: failure_code.clone(),
        run_id: 0,
        workflow_name: String::new(),
        step_id: None,
        step_name: None,
        error_message: String::new(),
        replay_safe: true,
        side_effect_certainty: super::types::SideEffectCertainty::None,
        timestamp: Instant::now(),
        context: super::types::IncidentContext {
            slot_values_before: Vec::new(),
            taint_changes: Vec::new(),
            action_attempts: 0,
            last_action_idempotency_key: None,
        },
        timeline: Vec::new(),
    };
    let repairs = suggest_repairs(&dummy_incident);
    build_suggestion_items(&kind, repairs)
}
