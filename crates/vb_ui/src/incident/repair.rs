#![forbid(unsafe_code)]
use super::types::{FailureCode, Incident, IncidentRecord, SideEffectCertainty};

// ---------------------------------------------------------------------------
// Cyberpunk color palette (Phase 5B display constants)
// ---------------------------------------------------------------------------

/// Neon cyan (#00f5ff) — primary accent, applied/active state.
pub const NEON_CYAN: [f32; 4] = [0.0_f32, 0.961_f32, 1.0_f32, 1.0_f32];
/// Neon green (#39ff14) — success, applied confirmation.
pub const NEON_GREEN: [f32; 4] = [0.224_f32, 1.0_f32, 0.078_f32, 1.0_f32];
/// Neon red (#ff073a) — failure, dismissed/error state.
pub const NEON_RED: [f32; 4] = [1.0_f32, 0.027_f32, 0.227_f32, 1.0_f32];
/// Neon yellow (#ffe600) — pending/attention state.
pub const NEON_YELLOW: [f32; 4] = [1.0_f32, 0.902_f32, 0.0_f32, 1.0_f32];
/// Neon orange (#ff6b00) — warning/external action state.
pub const NEON_ORANGE: [f32; 4] = [1.0_f32, 0.420_f32, 0.0_f32, 1.0_f32];

/// Primary repair kind as specified by the Phase 5A contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RepairKind {
    IncreaseTimeout,
    AddRetryBackoff,
    ReducePayload,
    PinIdempotency,
    FixSecretLeak,
    ManualInvestigation,
}

impl RepairKind {
    /// Return a static display label for this repair kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::IncreaseTimeout => "IncreaseTimeout",
            Self::AddRetryBackoff => "AddRetryBackoff",
            Self::ReducePayload => "ReducePayload",
            Self::PinIdempotency => "PinIdempotency",
            Self::FixSecretLeak => "FixSecretLeak",
            Self::ManualInvestigation => "ManualInvestigation",
        }
    }
}

/// Confidence level for a repair suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RepairConfidence {
    High,
    Medium,
    Low,
}

impl RepairConfidence {
    /// Return a static display label for this confidence level.
    pub fn display_str(&self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

/// A single repair suggestion tied to a [`RepairKind`].
#[derive(Debug, Clone)]
pub struct RepairSuggestion {
    pub kind: RepairKind,
    pub description: String,
    /// Legacy action field for backward compatibility.
    pub action: RepairAction,
    /// Confidence score between 0.0 and 1.0.
    pub confidence: f32,
    /// Structured confidence level for record-based suggestions.
    pub confidence_level: RepairConfidence,
    /// Rationale explaining why this suggestion was chosen.
    pub rationale: String,
}

/// Extended repair actions for additional failure modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RepairAction {
    IncreaseTimeout,
    ReducePayload,
    AddRetryBackoff,
    PinIdempotency,
    FixSecretLeak,
    AdjustBudget,
    RestartRun,
    ManualIntervention,
}

impl RepairAction {
    /// Return a static display label for this repair action.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::IncreaseTimeout => "IncreaseTimeout",
            Self::ReducePayload => "ReducePayload",
            Self::AddRetryBackoff => "AddRetryBackoff",
            Self::PinIdempotency => "PinIdempotency",
            Self::FixSecretLeak => "FixSecretLeak",
            Self::AdjustBudget => "AdjustBudget",
            Self::RestartRun => "RestartRun",
            Self::ManualIntervention => "ManualIntervention",
        }
    }
}

pub fn suggest_repairs(incident: &Incident) -> Vec<RepairSuggestion> {
    let mut suggestions = Vec::new();
    match &incident.failure_code {
        FailureCode::ActionTimeout => {
            suggestions.push(RepairSuggestion {
                kind: RepairKind::IncreaseTimeout,
                action: RepairAction::IncreaseTimeout,
                description: "Increase the action timeout to accommodate slower responses".into(),
                confidence: 0.9,
                confidence_level: RepairConfidence::High,
                rationale: String::from(
                    "Timeout failures are typically resolved by allowing more time",
                ),
            });
            suggestions.push(RepairSuggestion {
                kind: RepairKind::AddRetryBackoff,
                action: RepairAction::AddRetryBackoff,
                description: "Add exponential backoff to handle transient timeouts".into(),
                confidence: 0.7,
                confidence_level: RepairConfidence::Medium,
                rationale: String::from("Backoff handles transient latency spikes"),
            });
        }
        FailureCode::ActionFailed(_) => {
            suggestions.push(RepairSuggestion {
                kind: RepairKind::AddRetryBackoff,
                action: RepairAction::RestartRun,
                description: format!("Restart the run (replay safe: {})", incident.replay_safe),
                confidence: if incident.replay_safe { 0.95 } else { 0.3 },
                confidence_level: if incident.replay_safe {
                    RepairConfidence::High
                } else {
                    RepairConfidence::Low
                },
                rationale: String::from(
                    "Action failure may be transient; replay safety determines retry viability",
                ),
            });
        }
        FailureCode::BudgetExceeded => {
            suggestions.push(RepairSuggestion {
                kind: RepairKind::IncreaseTimeout,
                action: RepairAction::AdjustBudget,
                description: "Increase the step budget in the resource contract".into(),
                confidence: 0.8,
                confidence_level: RepairConfidence::Medium,
                rationale: String::from(
                    "Budget exhaustion indicates the step needs more resource headroom",
                ),
            });
        }
        FailureCode::StepPanicked => {
            suggestions.push(RepairSuggestion {
                kind: RepairKind::ManualInvestigation,
                action: RepairAction::ManualIntervention,
                description: "Step panicked - investigate the step logic and fix the bug".into(),
                confidence: 0.5,
                confidence_level: RepairConfidence::Medium,
                rationale: String::from("Panics indicate logic bugs that require code changes"),
            });
        }
        FailureCode::ValidationError(msg) => {
            suggestions.push(RepairSuggestion {
                kind: RepairKind::ReducePayload,
                action: RepairAction::ManualIntervention,
                description: format!("Validation error: {msg}"),
                confidence: 0.4,
                confidence_level: RepairConfidence::Low,
                rationale: String::from("Validation errors require correcting the input data"),
            });
        }
        FailureCode::TaintLeak => {
            suggestions.push(RepairSuggestion {
                kind: RepairKind::FixSecretLeak,
                action: RepairAction::FixSecretLeak,
                description: "Secret data reached a public result - add taint barrier".into(),
                confidence: 0.85,
                confidence_level: RepairConfidence::High,
                rationale: String::from(
                    "Taint leaks require blocking data flow from secret to public outputs",
                ),
            });
        }
        FailureCode::ReplayDivergence => {
            suggestions.push(RepairSuggestion {
                kind: RepairKind::PinIdempotency,
                action: RepairAction::ManualIntervention,
                description: "Replay diverged from original execution - investigate journal".into(),
                confidence: 0.3,
                confidence_level: RepairConfidence::Low,
                rationale: String::from(
                    "Replay divergence indicates non-deterministic behavior that must be traced",
                ),
            });
        }
        FailureCode::Unknown(_) => {
            suggestions.push(RepairSuggestion {
                kind: RepairKind::PinIdempotency,
                action: RepairAction::ManualIntervention,
                description: "Unknown failure - manual investigation required".into(),
                confidence: 0.1,
                confidence_level: RepairConfidence::Low,
                rationale: String::from("Unknown failures cannot be automatically diagnosed"),
            });
        }
    }
    if incident.side_effect_certainty == SideEffectCertainty::Unknown {
        suggestions.push(RepairSuggestion {
            kind: RepairKind::PinIdempotency,
            action: RepairAction::PinIdempotency,
            description: "Side effect certainty unknown - pin idempotency key before retry".into(),
            confidence: 0.6,
            confidence_level: RepairConfidence::Medium,
            rationale: String::from(
                "Unknown side effects require pinning to ensure idempotent retries",
            ),
        });
    }
    suggestions
}

/// Generate repair suggestions for an [`IncidentRecord`], mapping each
/// [`FailureCode`] variant to a targeted suggestion with confidence and rationale.
pub fn suggest_repairs_for_record(record: &IncidentRecord) -> Vec<RepairSuggestion> {
    let suggestion = match &record.failure_code {
        FailureCode::TaintLeak => RepairSuggestion {
            kind: RepairKind::FixSecretLeak,
            action: RepairAction::FixSecretLeak,
            description: String::from("review data flow"),
            confidence: 0.9,
            confidence_level: RepairConfidence::High,
            rationale: format!(
                "Taint leak detected in run {} step {}: secret data may have reached a public output",
                record.run_id, record.step
            ),
        },
        FailureCode::BudgetExceeded => RepairSuggestion {
            kind: RepairKind::IncreaseTimeout,
            action: RepairAction::AdjustBudget,
            description: String::from("increase step budget"),
            confidence: 0.7,
            confidence_level: RepairConfidence::Medium,
            rationale: format!(
                "Run {} step {} exhausted its step budget; consider raising the limit",
                record.run_id, record.step
            ),
        },
        FailureCode::ReplayDivergence => RepairSuggestion {
            kind: RepairKind::PinIdempotency,
            action: RepairAction::ManualIntervention,
            description: String::from("investigate state divergence"),
            confidence: 0.85,
            confidence_level: RepairConfidence::High,
            rationale: format!(
                "Run {} step {} diverged from the original replay journal; non-deterministic behavior suspected",
                record.run_id, record.step
            ),
        },
        FailureCode::ActionTimeout => RepairSuggestion {
            kind: RepairKind::IncreaseTimeout,
            action: RepairAction::IncreaseTimeout,
            description: String::from("increase timeout"),
            confidence: 0.7,
            confidence_level: RepairConfidence::Medium,
            rationale: format!(
                "Run {} step {} timed out; the action may need a longer deadline",
                record.run_id, record.step
            ),
        },
        FailureCode::StepPanicked => RepairSuggestion {
            kind: RepairKind::ManualInvestigation,
            action: RepairAction::ManualIntervention,
            description: String::from("investigate panic cause"),
            confidence: 0.9,
            confidence_level: RepairConfidence::High,
            rationale: format!(
                "Run {} step {} panicked; this indicates a logic bug that must be fixed before retry",
                record.run_id, record.step
            ),
        },
        FailureCode::ActionFailed(_) => RepairSuggestion {
            kind: RepairKind::AddRetryBackoff,
            action: RepairAction::RestartRun,
            description: String::from("retry with backoff"),
            confidence: 0.3,
            confidence_level: RepairConfidence::Low,
            rationale: format!(
                "Run {} step {} reported an action failure; retry with exponential backoff may resolve transient issues",
                record.run_id, record.step
            ),
        },
        FailureCode::ValidationError(msg) => RepairSuggestion {
            kind: RepairKind::ReducePayload,
            action: RepairAction::ManualIntervention,
            description: String::from("fix validation input"),
            confidence: 0.6,
            confidence_level: RepairConfidence::Medium,
            rationale: format!(
                "Run {} step {} failed validation: {msg}",
                record.run_id, record.step
            ),
        },
        FailureCode::Unknown(inner) => RepairSuggestion {
            kind: RepairKind::PinIdempotency,
            action: RepairAction::ManualIntervention,
            description: String::from("contact support"),
            confidence: 0.1,
            confidence_level: RepairConfidence::Low,
            rationale: format!(
                "Run {} step {} has an unrecognized failure ({inner}); manual investigation required",
                record.run_id, record.step
            ),
        },
    };

    let mut results = vec![suggestion];

    // If replay safety is not Safe, add an extra idempotency-pinning suggestion.
    if !record.replay_safety.is_safe() {
        results.push(RepairSuggestion {
            kind: RepairKind::PinIdempotency,
            action: RepairAction::PinIdempotency,
            description: String::from("pin idempotency key before retry"),
            confidence: 0.6,
            confidence_level: RepairConfidence::Medium,
            rationale: format!(
                "Run {} has replay safety={:?}; pin idempotency to prevent duplicate side effects",
                record.run_id, record.replay_safety
            ),
        });
    }

    results
}

// ---------------------------------------------------------------------------
// Phase 5B: RepairActionState — per-suggestion applied/dismissed tracking
// ---------------------------------------------------------------------------

/// Tracks the resolution state of a single [`RepairSuggestion`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RepairActionState {
    /// The suggestion has not been acted upon yet.
    Pending,
    /// The operator has applied the suggestion.
    Applied,
    /// The operator has dismissed the suggestion.
    Dismissed,
}

impl RepairActionState {
    /// Return a static display label for this state.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Applied => "Applied",
            Self::Dismissed => "Dismissed",
        }
    }

    /// Return the cyberpunk display color for this state.
    pub fn display_color(&self) -> [f32; 4] {
        match self {
            Self::Pending => NEON_YELLOW,
            Self::Applied => NEON_GREEN,
            Self::Dismissed => NEON_RED,
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 5B: RepairPanel — manages a collection of repair suggestions
// ---------------------------------------------------------------------------

/// An entry in the repair panel, pairing a suggestion with its resolution state.
#[derive(Debug, Clone)]
pub struct RepairEntry {
    /// The repair suggestion.
    pub suggestion: RepairSuggestion,
    /// Whether the suggestion has been applied, dismissed, or is still pending.
    pub state: RepairActionState,
}

/// A panel of repair suggestions for a single incident. Each suggestion tracks
/// whether the operator has applied it, dismissed it, or left it pending.
#[derive(Debug, Clone)]
pub struct RepairPanel {
    entries: Vec<RepairEntry>,
}

impl Default for RepairPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl RepairPanel {
    /// Create a new, empty repair panel.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Create a repair panel pre-populated with suggestions for the given incident.
    pub fn from_incident(incident: &Incident) -> Self {
        let suggestions = suggest_repairs(incident);
        Self::from_suggestions(suggestions)
    }

    /// Create a repair panel pre-populated with suggestions for the given record.
    pub fn from_record(record: &IncidentRecord) -> Self {
        let suggestions = suggest_repairs_for_record(record);
        Self::from_suggestions(suggestions)
    }

    /// Build a panel from an existing list of suggestions, all starting as Pending.
    pub fn from_suggestions(suggestions: Vec<RepairSuggestion>) -> Self {
        let entries = suggestions
            .into_iter()
            .map(|suggestion| RepairEntry {
                suggestion,
                state: RepairActionState::Pending,
            })
            .collect();
        Self { entries }
    }

    /// Add a single repair suggestion to the panel (starts as Pending).
    pub fn add_suggestion(&mut self, suggestion: RepairSuggestion) {
        self.entries.push(RepairEntry {
            suggestion,
            state: RepairActionState::Pending,
        });
    }

    /// Mark the suggestion at the given index as dismissed.
    /// Returns true if the index was valid and the entry was pending.
    /// Returns false if the index is out of bounds or the entry was already
    /// resolved.
    pub fn dismiss(&mut self, index: usize) -> bool {
        let entry = match self.entries.get(index) {
            Some(e) => e,
            None => return false,
        };
        if entry.state != RepairActionState::Pending {
            return false;
        }
        // Safe: index is valid and we just verified the entry exists.
        if let Some(e) = self.entries.get_mut(index) {
            e.state = RepairActionState::Dismissed;
        }
        true
    }

    /// Mark the suggestion at the given index as applied.
    /// Returns true if the index was valid and the entry was pending.
    /// Returns false if the index is out of bounds or the entry was already
    /// resolved.
    pub fn apply(&mut self, index: usize) -> bool {
        let entry = match self.entries.get(index) {
            Some(e) => e,
            None => return false,
        };
        if entry.state != RepairActionState::Pending {
            return false;
        }
        if let Some(e) = self.entries.get_mut(index) {
            e.state = RepairActionState::Applied;
        }
        true
    }

    /// Mark all pending suggestions as applied.
    /// Returns the number of suggestions that were transitioned from Pending to Applied.
    pub fn apply_all(&mut self) -> usize {
        let mut count: usize = 0;
        for entry in &mut self.entries {
            if entry.state == RepairActionState::Pending {
                entry.state = RepairActionState::Applied;
                count = count.saturating_add(1);
            }
        }
        count
    }

    /// Return the number of pending suggestions.
    pub fn pending_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.state == RepairActionState::Pending)
            .count()
    }

    /// Return the number of applied suggestions.
    pub fn applied_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.state == RepairActionState::Applied)
            .count()
    }

    /// Return the number of dismissed suggestions.
    pub fn dismissed_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.state == RepairActionState::Dismissed)
            .count()
    }

    /// Return the total number of suggestions.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return true if the panel has no suggestions.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return a reference to all entries.
    pub fn entries(&self) -> &[RepairEntry] {
        &self.entries
    }

    /// Return the entry at the given index, if it exists.
    pub fn get(&self, index: usize) -> Option<&RepairEntry> {
        self.entries.get(index)
    }

    /// Return true if all suggestions are in a resolved state (Applied or Dismissed).
    pub fn is_resolved(&self) -> bool {
        self.entries
            .iter()
            .all(|e| e.state != RepairActionState::Pending)
    }

    /// Return the highlight color for the panel based on its overall state.
    /// Returns NEON_GREEN if all resolved, NEON_YELLOW if pending items remain,
    /// or NEON_RED if all were dismissed.
    pub fn panel_color(&self) -> [f32; 4] {
        if self.is_empty() {
            return NEON_CYAN;
        }
        if self.pending_count() > 0 {
            return NEON_YELLOW;
        }
        if self.applied_count() > 0 {
            return NEON_GREEN;
        }
        NEON_RED
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::{
        FailureCode, Incident, IncidentContext, IncidentRecord, IncidentSeverity, IncidentType,
        ReplaySafety, SideEffectCertainty,
    };
    use super::*;
    use std::time::Instant;

    fn make_incident(code: FailureCode, certainty: SideEffectCertainty) -> Incident {
        Incident {
            id: 1,
            incident_type: IncidentType::ActionFailure,
            severity: IncidentSeverity::Major,
            failure_code: code,
            run_id: 1,
            workflow_name: String::from("test"),
            step_id: None,
            step_name: None,
            error_message: String::from("error"),
            replay_safe: true,
            side_effect_certainty: certainty,
            timestamp: Instant::now(),
            context: IncidentContext {
                slot_values_before: Vec::new(),
                taint_changes: Vec::new(),
                action_attempts: 0,
                last_action_idempotency_key: None,
            },
            timeline: Vec::new(),
        }
    }

    fn make_record(
        run_id: u64,
        step: u16,
        failure_code: FailureCode,
        replay_safety: ReplaySafety,
    ) -> IncidentRecord {
        IncidentRecord {
            run_id,
            shard_id: 0,
            step,
            failure_code,
            severity: IncidentSeverity::Critical,
            replay_safety,
            timestamp_us: 1000,
            detail: String::from("test detail"),
        }
    }

    // -- Legacy suggest_repairs tests --

    #[test]
    fn test_action_timeout_suggests_increase_timeout() {
        let incident = make_incident(FailureCode::ActionTimeout, SideEffectCertainty::Certain);
        let suggestions = suggest_repairs(&incident);
        assert!(
            suggestions
                .iter()
                .any(|s| s.action == RepairAction::IncreaseTimeout)
        );
        assert!(
            suggestions
                .iter()
                .any(|s| s.kind == RepairKind::IncreaseTimeout)
        );
    }

    #[test]
    fn test_taint_leak_suggests_fix_secret_leak() {
        let incident = make_incident(FailureCode::TaintLeak, SideEffectCertainty::Certain);
        let suggestions = suggest_repairs(&incident);
        assert!(
            suggestions
                .iter()
                .any(|s| s.action == RepairAction::FixSecretLeak)
        );
        assert!(
            suggestions
                .iter()
                .any(|s| s.kind == RepairKind::FixSecretLeak)
        );
    }

    #[test]
    fn test_unknown_certainty_adds_pin_idempotency() {
        let incident = make_incident(
            FailureCode::Unknown("x".into()),
            SideEffectCertainty::Unknown,
        );
        let suggestions = suggest_repairs(&incident);
        assert!(
            suggestions
                .iter()
                .any(|s| s.action == RepairAction::PinIdempotency)
        );
        assert!(
            suggestions
                .iter()
                .any(|s| s.action == RepairAction::ManualIntervention)
        );
        assert!(
            suggestions
                .iter()
                .any(|s| s.kind == RepairKind::PinIdempotency)
        );
    }

    // -- RepairConfidence tests --

    #[test]
    fn test_repair_confidence_display_str() {
        assert_eq!(RepairConfidence::High.display_str(), "high");
        assert_eq!(RepairConfidence::Medium.display_str(), "medium");
        assert_eq!(RepairConfidence::Low.display_str(), "low");
    }

    // -- suggest_repairs_for_record tests --

    #[test]
    fn test_record_taint_leak_high_confidence() {
        let record = make_record(100, 5, FailureCode::TaintLeak, ReplaySafety::Safe);
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::High);
        assert_eq!(suggestions[0].kind, RepairKind::FixSecretLeak);
        assert_eq!(suggestions[0].description, "review data flow");
        assert!(suggestions[0].rationale.contains("100"));
        assert!(suggestions[0].rationale.contains("5"));
    }

    #[test]
    fn test_record_budget_exceeded_medium_confidence() {
        let record = make_record(200, 3, FailureCode::BudgetExceeded, ReplaySafety::Safe);
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::Medium);
        assert_eq!(suggestions[0].description, "increase step budget");
        assert!(suggestions[0].rationale.contains("200"));
    }

    #[test]
    fn test_record_replay_divergence_high_confidence() {
        let record = make_record(300, 7, FailureCode::ReplayDivergence, ReplaySafety::Safe);
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::High);
        assert_eq!(suggestions[0].description, "investigate state divergence");
        assert!(suggestions[0].rationale.contains("300"));
    }

    #[test]
    fn test_record_action_timeout_medium_confidence() {
        let record = make_record(400, 2, FailureCode::ActionTimeout, ReplaySafety::Safe);
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::Medium);
        assert_eq!(suggestions[0].description, "increase timeout");
        assert!(suggestions[0].rationale.contains("400"));
    }

    #[test]
    fn test_record_step_panicked_high_confidence() {
        let record = make_record(500, 9, FailureCode::StepPanicked, ReplaySafety::Safe);
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::High);
        assert_eq!(suggestions[0].description, "investigate panic cause");
        assert!(suggestions[0].rationale.contains("500"));
    }

    #[test]
    fn test_record_action_failed_low_confidence() {
        let record = make_record(
            600,
            4,
            FailureCode::ActionFailed(String::from("connection refused")),
            ReplaySafety::Safe,
        );
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::Low);
        assert_eq!(suggestions[0].description, "retry with backoff");
        assert!(suggestions[0].rationale.contains("600"));
    }

    #[test]
    fn test_record_validation_error_medium_confidence() {
        let record = make_record(
            700,
            1,
            FailureCode::ValidationError(String::from("field missing")),
            ReplaySafety::Safe,
        );
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::Medium);
        assert_eq!(suggestions[0].description, "fix validation input");
        assert!(suggestions[0].rationale.contains("field missing"));
        assert!(suggestions[0].rationale.contains("700"));
    }

    #[test]
    fn test_record_unknown_low_confidence() {
        let record = make_record(
            800,
            6,
            FailureCode::Unknown(String::from("internal")),
            ReplaySafety::Safe,
        );
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::Low);
        assert_eq!(suggestions[0].description, "contact support");
        assert!(suggestions[0].rationale.contains("internal"));
        assert!(suggestions[0].rationale.contains("800"));
    }

    #[test]
    fn test_record_unsafe_replay_adds_idempotency_suggestion() {
        let record = make_record(
            900,
            2,
            FailureCode::ActionTimeout,
            ReplaySafety::UnsafeSideEffect,
        );
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 2);
        let idempotency = suggestions
            .iter()
            .find(|s| s.kind == RepairKind::PinIdempotency);
        assert!(idempotency.is_some());
        assert_eq!(
            idempotency.map(|s| s.confidence_level),
            Some(RepairConfidence::Medium)
        );
        assert!(idempotency.map_or(false, |s| s.rationale.contains("900")));
    }

    #[test]
    fn test_record_unknown_replay_safety_adds_idempotency_suggestion() {
        let record = make_record(950, 3, FailureCode::TaintLeak, ReplaySafety::Unknown);
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 2);
        assert!(
            suggestions
                .iter()
                .any(|s| s.kind == RepairKind::PinIdempotency)
        );
    }

    #[test]
    fn test_record_safe_replay_no_extra_suggestion() {
        let record = make_record(999, 1, FailureCode::TaintLeak, ReplaySafety::Safe);
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].kind, RepairKind::FixSecretLeak);
    }

    #[test]
    fn test_legacy_suggestions_include_confidence_level() {
        let incident = make_incident(FailureCode::TaintLeak, SideEffectCertainty::Certain);
        let suggestions = suggest_repairs(&incident);
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().all(|s| !s.rationale.is_empty()));
        assert!(
            suggestions
                .iter()
                .all(|s| !s.confidence_level.display_str().is_empty())
        );
    }

    // ---------------------------------------------------------------------------
    // Additional tests: RepairKind::as_str, legacy variants, RepairConfidence
    // ---------------------------------------------------------------------------

    #[test]
    fn test_repair_kind_as_str_all_variants() {
        assert_eq!(RepairKind::IncreaseTimeout.as_str(), "IncreaseTimeout");
        assert_eq!(RepairKind::AddRetryBackoff.as_str(), "AddRetryBackoff");
        assert_eq!(RepairKind::ReducePayload.as_str(), "ReducePayload");
        assert_eq!(RepairKind::PinIdempotency.as_str(), "PinIdempotency");
        assert_eq!(RepairKind::FixSecretLeak.as_str(), "FixSecretLeak");
        assert_eq!(
            RepairKind::ManualInvestigation.as_str(),
            "ManualInvestigation"
        );
    }

    #[test]
    fn test_suggest_repairs_budget_exceeded() {
        let incident = make_incident(FailureCode::BudgetExceeded, SideEffectCertainty::Certain);
        let suggestions = suggest_repairs(&incident);
        assert!(
            suggestions
                .iter()
                .any(|s| s.kind == RepairKind::IncreaseTimeout)
        );
        assert!(
            suggestions
                .iter()
                .any(|s| s.action == RepairAction::AdjustBudget)
        );
    }

    #[test]
    fn test_suggest_repairs_step_panicked() {
        let incident = make_incident(FailureCode::StepPanicked, SideEffectCertainty::Unknown);
        let suggestions = suggest_repairs(&incident);
        assert!(
            suggestions
                .iter()
                .any(|s| s.kind == RepairKind::ManualInvestigation)
        );
        assert!(
            suggestions
                .iter()
                .any(|s| s.action == RepairAction::ManualIntervention)
        );
        // StepPanicked maps to Unknown certainty, so PinIdempotency should also appear
        assert!(
            suggestions
                .iter()
                .any(|s| s.action == RepairAction::PinIdempotency)
        );
    }

    #[test]
    fn test_suggest_repairs_replay_divergence() {
        let incident = make_incident(FailureCode::ReplayDivergence, SideEffectCertainty::Certain);
        let suggestions = suggest_repairs(&incident);
        assert!(
            suggestions
                .iter()
                .any(|s| s.kind == RepairKind::PinIdempotency)
        );
        assert!(
            suggestions
                .iter()
                .any(|s| s.action == RepairAction::ManualIntervention)
        );
    }

    #[test]
    fn test_repair_confidence_all_variants_are_distinct() {
        let variants = [
            RepairConfidence::High,
            RepairConfidence::Medium,
            RepairConfidence::Low,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: legacy suggest_repairs for remaining FailureCode variants
    // ---------------------------------------------------------------------------

    #[test]
    fn test_suggest_repairs_action_failed_replay_safe_high_confidence() {
        let incident = make_incident(
            FailureCode::ActionFailed(String::from("connection refused")),
            SideEffectCertainty::Certain,
        );
        let suggestions = suggest_repairs(&incident);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].action, RepairAction::RestartRun);
        assert_eq!(suggestions[0].kind, RepairKind::AddRetryBackoff);
        let diff = (suggestions[0].confidence - 0.95_f32).abs();
        assert!(
            diff < f32::EPSILON,
            "expected 0.95 confidence for replay-safe failure"
        );
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::High);
    }

    #[test]
    fn test_suggest_repairs_action_failed_replay_unsafe_low_confidence() {
        let mut incident = make_incident(
            FailureCode::ActionFailed(String::from("network")),
            SideEffectCertainty::Certain,
        );
        incident.replay_safe = false;
        let suggestions = suggest_repairs(&incident);
        assert_eq!(suggestions.len(), 1);
        let diff = (suggestions[0].confidence - 0.3_f32).abs();
        assert!(
            diff < f32::EPSILON,
            "expected 0.3 confidence for replay-unsafe failure"
        );
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::Low);
    }

    #[test]
    fn test_suggest_repairs_action_failed_description_contains_replay_safe() {
        let incident = make_incident(
            FailureCode::ActionFailed(String::from("timeout")),
            SideEffectCertainty::Certain,
        );
        let suggestions = suggest_repairs(&incident);
        assert!(!suggestions.is_empty());
        assert!(
            suggestions[0].description.contains("replay safe: true"),
            "description should mention replay_safe status"
        );
    }

    #[test]
    fn test_suggest_repairs_validation_error() {
        let incident = make_incident(
            FailureCode::ValidationError(String::from("missing field 'name'")),
            SideEffectCertainty::Certain,
        );
        let suggestions = suggest_repairs(&incident);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].kind, RepairKind::ReducePayload);
        assert_eq!(suggestions[0].action, RepairAction::ManualIntervention);
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::Low);
        assert!(
            suggestions[0].description.contains("missing field 'name'"),
            "description should include the validation message"
        );
    }

    #[test]
    fn test_suggest_repairs_unknown_without_unknown_certainty() {
        let incident = make_incident(
            FailureCode::Unknown(String::from("mystery error")),
            SideEffectCertainty::Certain,
        );
        let suggestions = suggest_repairs(&incident);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].kind, RepairKind::PinIdempotency);
        assert_eq!(suggestions[0].action, RepairAction::ManualIntervention);
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::Low);
    }

    #[test]
    fn test_suggest_repairs_side_effect_certainty_none_no_pin() {
        let incident = make_incident(FailureCode::ActionTimeout, SideEffectCertainty::None);
        let suggestions = suggest_repairs(&incident);
        assert!(
            !suggestions
                .iter()
                .any(|s| s.action == RepairAction::PinIdempotency),
            "PinIdempotency should only be added for Unknown certainty, not None"
        );
        assert!(
            suggestions
                .iter()
                .any(|s| s.kind == RepairKind::IncreaseTimeout)
        );
    }

    #[test]
    fn test_suggest_repairs_side_effect_certainty_unknown_adds_pin() {
        let incident = make_incident(FailureCode::ActionTimeout, SideEffectCertainty::Unknown);
        let suggestions = suggest_repairs(&incident);
        assert!(
            suggestions
                .iter()
                .any(|s| s.action == RepairAction::PinIdempotency)
        );
        assert!(
            suggestions
                .iter()
                .any(|s| s.kind == RepairKind::PinIdempotency)
        );
    }

    #[test]
    fn test_suggest_repairs_side_effect_certainty_certain_no_pin() {
        let incident = make_incident(FailureCode::TaintLeak, SideEffectCertainty::Certain);
        let suggestions = suggest_repairs(&incident);
        assert!(
            !suggestions
                .iter()
                .any(|s| s.action == RepairAction::PinIdempotency),
            "Certain certainty should not add PinIdempotency"
        );
    }

    #[test]
    fn test_suggest_repairs_action_timeout_returns_two_suggestions() {
        let incident = make_incident(FailureCode::ActionTimeout, SideEffectCertainty::Certain);
        let suggestions = suggest_repairs(&incident);
        assert_eq!(suggestions.len(), 2);
        assert!(
            suggestions
                .iter()
                .any(|s| s.kind == RepairKind::IncreaseTimeout)
        );
        assert!(
            suggestions
                .iter()
                .any(|s| s.kind == RepairKind::AddRetryBackoff)
        );
    }

    // ---------------------------------------------------------------------------
    // RepairKind Copy + PartialEq
    // ---------------------------------------------------------------------------

    #[test]
    fn test_repair_kind_copy_trait() {
        let kind = RepairKind::FixSecretLeak;
        let copied = kind;
        assert_eq!(kind, copied);
    }

    #[test]
    fn test_repair_kind_equality() {
        assert_eq!(RepairKind::IncreaseTimeout, RepairKind::IncreaseTimeout);
        assert_ne!(RepairKind::IncreaseTimeout, RepairKind::ManualInvestigation);
    }

    // ---------------------------------------------------------------------------
    // RepairConfidence Copy + PartialEq
    // ---------------------------------------------------------------------------

    #[test]
    fn test_repair_confidence_copy_trait() {
        let conf = RepairConfidence::High;
        let copied = conf;
        assert_eq!(conf, copied);
    }

    // ---------------------------------------------------------------------------
    // RepairAction::as_str
    // ---------------------------------------------------------------------------

    #[test]
    fn test_repair_action_as_str_all_variants() {
        assert_eq!(RepairAction::IncreaseTimeout.as_str(), "IncreaseTimeout");
        assert_eq!(RepairAction::ReducePayload.as_str(), "ReducePayload");
        assert_eq!(RepairAction::AddRetryBackoff.as_str(), "AddRetryBackoff");
        assert_eq!(RepairAction::PinIdempotency.as_str(), "PinIdempotency");
        assert_eq!(RepairAction::FixSecretLeak.as_str(), "FixSecretLeak");
        assert_eq!(RepairAction::AdjustBudget.as_str(), "AdjustBudget");
        assert_eq!(RepairAction::RestartRun.as_str(), "RestartRun");
        assert_eq!(
            RepairAction::ManualIntervention.as_str(),
            "ManualIntervention"
        );
    }

    // ---------------------------------------------------------------------------
    // RepairAction equality checks (all variants distinct)
    // ---------------------------------------------------------------------------

    #[test]
    fn test_repair_action_variants_are_distinct() {
        let variants = [
            RepairAction::IncreaseTimeout,
            RepairAction::ReducePayload,
            RepairAction::AddRetryBackoff,
            RepairAction::PinIdempotency,
            RepairAction::FixSecretLeak,
            RepairAction::AdjustBudget,
            RepairAction::RestartRun,
            RepairAction::ManualIntervention,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(
                    variants[i], variants[j],
                    "RepairAction variants must be distinct"
                );
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Confidence values within [0.0, 1.0] for all legacy failure codes
    // ---------------------------------------------------------------------------

    #[test]
    fn test_suggest_repairs_all_confidence_values_in_range() {
        let failure_codes = [
            FailureCode::ActionTimeout,
            FailureCode::ActionFailed(String::from("err")),
            FailureCode::BudgetExceeded,
            FailureCode::StepPanicked,
            FailureCode::ValidationError(String::from("bad")),
            FailureCode::TaintLeak,
            FailureCode::ReplayDivergence,
            FailureCode::Unknown(String::from("x")),
        ];
        for code in &failure_codes {
            let incident = make_incident(code.clone(), SideEffectCertainty::Unknown);
            let suggestions = suggest_repairs(&incident);
            for s in &suggestions {
                assert!(
                    (0.0_f32..=1.0_f32).contains(&s.confidence),
                    "confidence {} out of range for {:?}",
                    s.confidence,
                    code
                );
            }
        }
    }

    // ---------------------------------------------------------------------------
    // suggest_repairs_for_record: confidence values in range for all codes
    // ---------------------------------------------------------------------------

    #[test]
    fn test_suggest_repairs_for_record_all_confidence_values_in_range() {
        let failure_codes = [
            FailureCode::ActionTimeout,
            FailureCode::ActionFailed(String::from("err")),
            FailureCode::BudgetExceeded,
            FailureCode::StepPanicked,
            FailureCode::ValidationError(String::from("bad")),
            FailureCode::TaintLeak,
            FailureCode::ReplayDivergence,
            FailureCode::Unknown(String::from("x")),
        ];
        for code in &failure_codes {
            let record = make_record(1, 1, code.clone(), ReplaySafety::Safe);
            let suggestions = suggest_repairs_for_record(&record);
            assert_eq!(
                suggestions.len(),
                1,
                "safe replay should yield exactly 1 suggestion for {:?}",
                code
            );
            assert!(
                (0.0_f32..=1.0_f32).contains(&suggestions[0].confidence),
                "confidence {} out of range for record with {:?}",
                suggestions[0].confidence,
                code
            );
        }
    }

    // ---------------------------------------------------------------------------
    // suggest_repairs_for_record: rationale always contains run_id and step
    // ---------------------------------------------------------------------------

    #[test]
    fn test_suggest_repairs_for_record_rationale_contains_run_and_step() {
        let record = make_record(4242, 13, FailureCode::TaintLeak, ReplaySafety::Safe);
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 1);
        assert!(
            suggestions[0].rationale.contains("4242"),
            "rationale must contain run_id"
        );
        assert!(
            suggestions[0].rationale.contains("13"),
            "rationale must contain step"
        );
    }

    // ---------------------------------------------------------------------------
    // suggest_repairs_for_record: all failure codes produce non-empty description and rationale
    // ---------------------------------------------------------------------------

    #[test]
    fn test_suggest_repairs_for_record_all_non_empty_fields() {
        let failure_codes = [
            FailureCode::ActionTimeout,
            FailureCode::ActionFailed(String::from("err")),
            FailureCode::BudgetExceeded,
            FailureCode::StepPanicked,
            FailureCode::ValidationError(String::from("msg")),
            FailureCode::TaintLeak,
            FailureCode::ReplayDivergence,
            FailureCode::Unknown(String::from("x")),
        ];
        for code in &failure_codes {
            let record = make_record(1, 1, code.clone(), ReplaySafety::Safe);
            let suggestions = suggest_repairs_for_record(&record);
            assert!(
                !suggestions.is_empty(),
                "must produce suggestions for {:?}",
                code
            );
            for s in &suggestions {
                assert!(
                    !s.description.is_empty(),
                    "description empty for {:?}",
                    code
                );
                assert!(!s.rationale.is_empty(), "rationale empty for {:?}", code);
            }
        }
    }

    // ---------------------------------------------------------------------------
    // suggest_repairs_for_record: failure code to kind mapping
    // ---------------------------------------------------------------------------

    #[test]
    fn test_record_failure_code_to_kind_mapping() {
        let mappings: &[(FailureCode, RepairKind)] = &[
            (FailureCode::TaintLeak, RepairKind::FixSecretLeak),
            (FailureCode::BudgetExceeded, RepairKind::IncreaseTimeout),
            (FailureCode::ReplayDivergence, RepairKind::PinIdempotency),
            (FailureCode::ActionTimeout, RepairKind::IncreaseTimeout),
            (FailureCode::StepPanicked, RepairKind::ManualInvestigation),
            (
                FailureCode::ActionFailed(String::from("x")),
                RepairKind::AddRetryBackoff,
            ),
            (
                FailureCode::ValidationError(String::from("x")),
                RepairKind::ReducePayload,
            ),
            (
                FailureCode::Unknown(String::from("x")),
                RepairKind::PinIdempotency,
            ),
        ];
        for (code, expected_kind) in mappings {
            let record = make_record(1, 1, code.clone(), ReplaySafety::Safe);
            let suggestions = suggest_repairs_for_record(&record);
            assert!(
                suggestions.iter().any(|s| s.kind == *expected_kind),
                "expected kind {:?} for failure code {:?}",
                expected_kind,
                code
            );
        }
    }

    // ---------------------------------------------------------------------------
    // suggest_repairs_for_record: action mapping
    // ---------------------------------------------------------------------------

    #[test]
    fn test_record_failure_code_to_action_mapping() {
        let mappings: &[(FailureCode, RepairAction)] = &[
            (FailureCode::TaintLeak, RepairAction::FixSecretLeak),
            (FailureCode::BudgetExceeded, RepairAction::AdjustBudget),
            (
                FailureCode::ReplayDivergence,
                RepairAction::ManualIntervention,
            ),
            (FailureCode::ActionTimeout, RepairAction::IncreaseTimeout),
            (FailureCode::StepPanicked, RepairAction::ManualIntervention),
            (
                FailureCode::ActionFailed(String::from("x")),
                RepairAction::RestartRun,
            ),
            (
                FailureCode::ValidationError(String::from("x")),
                RepairAction::ManualIntervention,
            ),
            (
                FailureCode::Unknown(String::from("x")),
                RepairAction::ManualIntervention,
            ),
        ];
        for (code, expected_action) in mappings {
            let record = make_record(1, 1, code.clone(), ReplaySafety::Safe);
            let suggestions = suggest_repairs_for_record(&record);
            assert!(
                suggestions.iter().any(|s| s.action == *expected_action),
                "expected action {:?} for failure code {:?}",
                expected_action,
                code
            );
        }
    }

    // ---------------------------------------------------------------------------
    // suggest_repairs_for_record: all failure codes with UnsafeSideEffect yield 2 suggestions
    // ---------------------------------------------------------------------------

    #[test]
    fn test_record_unsafe_replay_yields_two_suggestions_for_all_codes() {
        let failure_codes = [
            FailureCode::ActionTimeout,
            FailureCode::ActionFailed(String::from("err")),
            FailureCode::BudgetExceeded,
            FailureCode::StepPanicked,
            FailureCode::ValidationError(String::from("bad")),
            FailureCode::TaintLeak,
            FailureCode::ReplayDivergence,
            FailureCode::Unknown(String::from("x")),
        ];
        for code in &failure_codes {
            let record = make_record(1, 1, code.clone(), ReplaySafety::UnsafeSideEffect);
            let suggestions = suggest_repairs_for_record(&record);
            assert_eq!(
                suggestions.len(),
                2,
                "unsafe replay should yield 2 suggestions for {:?}",
                code
            );
            assert!(
                suggestions
                    .iter()
                    .any(|s| s.kind == RepairKind::PinIdempotency),
                "second suggestion should be PinIdempotency for {:?}",
                code
            );
        }
    }

    // ---------------------------------------------------------------------------
    // RepairSuggestion Clone
    // ---------------------------------------------------------------------------

    #[test]
    fn test_repair_suggestion_clone() {
        let original = RepairSuggestion {
            kind: RepairKind::FixSecretLeak,
            description: String::from("fix the leak"),
            action: RepairAction::FixSecretLeak,
            confidence: 0.85,
            confidence_level: RepairConfidence::High,
            rationale: String::from("secret data leaked"),
        };
        let cloned = original.clone();
        assert_eq!(cloned.kind, original.kind);
        assert_eq!(cloned.description, original.description);
        assert_eq!(cloned.action, original.action);
        assert_eq!(cloned.confidence_level, original.confidence_level);
        assert_eq!(cloned.rationale, original.rationale);
    }

    // ---------------------------------------------------------------------------
    // Legacy: every suggestion has non-empty description and rationale
    // ---------------------------------------------------------------------------

    #[test]
    fn test_legacy_all_suggestions_have_description_and_rationale() {
        let failure_codes = [
            FailureCode::ActionTimeout,
            FailureCode::ActionFailed(String::from("err")),
            FailureCode::BudgetExceeded,
            FailureCode::StepPanicked,
            FailureCode::ValidationError(String::from("bad")),
            FailureCode::TaintLeak,
            FailureCode::ReplayDivergence,
            FailureCode::Unknown(String::from("x")),
        ];
        for code in &failure_codes {
            let incident = make_incident(code.clone(), SideEffectCertainty::Certain);
            let suggestions = suggest_repairs(&incident);
            assert!(
                !suggestions.is_empty(),
                "must produce suggestions for {:?}",
                code
            );
            for s in &suggestions {
                assert!(
                    !s.description.is_empty(),
                    "description empty for {:?}",
                    code
                );
                assert!(!s.rationale.is_empty(), "rationale empty for {:?}", code);
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Legacy: specific confidence and level checks per failure code
    // ---------------------------------------------------------------------------

    #[test]
    fn test_suggest_repairs_step_panicked_confidence_and_level() {
        let incident = make_incident(FailureCode::StepPanicked, SideEffectCertainty::Certain);
        let suggestions = suggest_repairs(&incident);
        assert_eq!(suggestions.len(), 1);
        let diff = (suggestions[0].confidence - 0.5_f32).abs();
        assert!(diff < f32::EPSILON, "StepPanicked confidence should be 0.5");
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::Medium);
    }

    #[test]
    fn test_suggest_repairs_budget_exceeded_confidence_and_level() {
        let incident = make_incident(FailureCode::BudgetExceeded, SideEffectCertainty::Certain);
        let suggestions = suggest_repairs(&incident);
        assert_eq!(suggestions.len(), 1);
        let diff = (suggestions[0].confidence - 0.8_f32).abs();
        assert!(
            diff < f32::EPSILON,
            "BudgetExceeded confidence should be 0.8"
        );
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::Medium);
    }

    #[test]
    fn test_suggest_repairs_taint_leak_confidence_and_level() {
        let incident = make_incident(FailureCode::TaintLeak, SideEffectCertainty::Certain);
        let suggestions = suggest_repairs(&incident);
        assert_eq!(suggestions.len(), 1);
        let diff = (suggestions[0].confidence - 0.85_f32).abs();
        assert!(diff < f32::EPSILON, "TaintLeak confidence should be 0.85");
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::High);
    }

    #[test]
    fn test_suggest_repairs_replay_divergence_confidence_and_level() {
        let incident = make_incident(FailureCode::ReplayDivergence, SideEffectCertainty::Certain);
        let suggestions = suggest_repairs(&incident);
        assert_eq!(suggestions.len(), 1);
        let diff = (suggestions[0].confidence - 0.3_f32).abs();
        assert!(
            diff < f32::EPSILON,
            "ReplayDivergence confidence should be 0.3"
        );
        assert_eq!(suggestions[0].confidence_level, RepairConfidence::Low);
    }

    #[test]
    fn test_suggest_repairs_action_timeout_confidence_values() {
        let incident = make_incident(FailureCode::ActionTimeout, SideEffectCertainty::Certain);
        let suggestions = suggest_repairs(&incident);
        assert_eq!(suggestions.len(), 2);
        let increase_timeout = suggestions
            .iter()
            .find(|s| s.kind == RepairKind::IncreaseTimeout);
        assert!(increase_timeout.is_some());
        let inc = increase_timeout.map_or(false, |s| {
            let diff = (s.confidence - 0.9_f32).abs();
            diff < f32::EPSILON
        });
        assert!(inc, "IncreaseTimeout confidence should be 0.9");
        let add_backoff = suggestions
            .iter()
            .find(|s| s.kind == RepairKind::AddRetryBackoff);
        assert!(add_backoff.is_some());
        let backoff = add_backoff.map_or(false, |s| {
            let diff = (s.confidence - 0.7_f32).abs();
            diff < f32::EPSILON
        });
        assert!(backoff, "AddRetryBackoff confidence should be 0.7");
    }

    // ---------------------------------------------------------------------------
    // suggest_repairs_for_record: ValidationError includes message in rationale
    // ---------------------------------------------------------------------------

    #[test]
    fn test_record_validation_error_rationale_contains_message() {
        let record = make_record(
            100,
            3,
            FailureCode::ValidationError(String::from("missing required field")),
            ReplaySafety::Safe,
        );
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 1);
        assert!(
            suggestions[0].rationale.contains("missing required field"),
            "rationale should contain the validation message"
        );
    }

    // ---------------------------------------------------------------------------
    // suggest_repairs_for_record: Unknown includes inner string in rationale
    // ---------------------------------------------------------------------------

    #[test]
    fn test_record_unknown_rationale_contains_inner() {
        let record = make_record(
            200,
            5,
            FailureCode::Unknown(String::from("weird-error")),
            ReplaySafety::Safe,
        );
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 1);
        assert!(
            suggestions[0].rationale.contains("weird-error"),
            "rationale should contain the unknown failure string"
        );
    }

    // ---------------------------------------------------------------------------
    // suggest_repairs_for_record: ActionFailed rationale
    // ---------------------------------------------------------------------------

    #[test]
    fn test_record_action_failed_rationale() {
        let record = make_record(
            300,
            7,
            FailureCode::ActionFailed(String::from("conn refused")),
            ReplaySafety::Safe,
        );
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 1);
        assert!(
            suggestions[0].rationale.contains("300"),
            "must contain run_id"
        );
        assert!(suggestions[0].rationale.contains("7"), "must contain step");
    }

    // ---------------------------------------------------------------------------
    // Edge: legacy suggest_repairs returns at least one suggestion for every code
    // ---------------------------------------------------------------------------

    #[test]
    fn test_legacy_min_one_suggestion_per_failure_code() {
        let failure_codes = [
            FailureCode::ActionTimeout,
            FailureCode::ActionFailed(String::from("err")),
            FailureCode::BudgetExceeded,
            FailureCode::StepPanicked,
            FailureCode::ValidationError(String::from("bad")),
            FailureCode::TaintLeak,
            FailureCode::ReplayDivergence,
            FailureCode::Unknown(String::from("x")),
        ];
        for code in &failure_codes {
            let incident = make_incident(code.clone(), SideEffectCertainty::Certain);
            let suggestions = suggest_repairs(&incident);
            assert!(
                !suggestions.is_empty(),
                "every failure code must produce at least one suggestion: {:?}",
                code
            );
        }
    }

    // ---------------------------------------------------------------------------
    // Edge: legacy suggest_repairs with Unknown certainty may return 2+ suggestions
    // ---------------------------------------------------------------------------

    #[test]
    fn test_legacy_unknown_certainty_doubles_for_non_panicked() {
        let incident = make_incident(FailureCode::ActionTimeout, SideEffectCertainty::Unknown);
        let suggestions = suggest_repairs(&incident);
        assert_eq!(suggestions.len(), 3);
        let pin_count = suggestions
            .iter()
            .filter(|s| s.action == RepairAction::PinIdempotency)
            .count();
        assert_eq!(pin_count, 1, "exactly one PinIdempotency suggestion");
    }

    // =========================================================================
    // BLACKHAT security review tests
    // =========================================================================

    /// FINDING: suggest_repairs_for_record() does NOT check side_effect_certainty
    /// unlike suggest_repairs() which adds a PinIdempotency suggestion when
    /// certainty is Unknown. IncidentRecord has no side_effect_certainty field,
    /// so this check cannot be ported. However, this creates a behavioral
    /// inconsistency between the two suggestion functions.
    #[test]
    fn blackhat_suggest_repairs_for_record_misses_side_effect_certainty_check() {
        // suggest_repairs adds PinIdempotency for Unknown certainty
        let incident = make_incident(FailureCode::ActionTimeout, SideEffectCertainty::Unknown);
        let legacy_suggestions = suggest_repairs(&incident);
        assert!(
            legacy_suggestions
                .iter()
                .any(|s| s.kind == RepairKind::PinIdempotency),
            "legacy adds PinIdempotency for Unknown certainty"
        );

        // suggest_repairs_for_record has no equivalent check since
        // IncidentRecord lacks side_effect_certainty
        let record = make_record(1, 1, FailureCode::ActionTimeout, ReplaySafety::Safe);
        let record_suggestions = suggest_repairs_for_record(&record);
        assert!(
            !record_suggestions
                .iter()
                .any(|s| s.kind == RepairKind::PinIdempotency),
            "record-based suggestions miss the side_effect_certainty check"
        );
    }

    /// FINDING: RepairSuggestion.confidence is f32, which has precision issues.
    /// 0.85_f32 may not be exactly 0.85 due to floating-point representation.
    /// The code uses hardcoded f32 literals which are fine for comparison with
    /// epsilon, but exact equality checks (==) would be unreliable.
    #[test]
    fn blackhat_confidence_f32_precision_at_common_values() {
        // 0.85 cannot be exactly represented in f32
        let val = 0.85_f32;
        // It should be very close but may not be exact
        let diff = (val - 0.85_f32).abs();
        assert!(
            diff < 0.001_f32,
            "f32 precision for 0.85 is acceptable but not exact"
        );

        // Verify all hardcoded confidence values are in [0.0, 1.0]
        let hardcoded_values = [
            0.9_f32, 0.7_f32, 0.95_f32, 0.3_f32, 0.8_f32, 0.5_f32, 0.4_f32, 0.85_f32, 0.6_f32,
            0.1_f32,
        ];
        for v in &hardcoded_values {
            assert!(
                (0.0_f32..=1.0_f32).contains(v),
                "confidence {v} out of [0,1] range"
            );
        }
    }

    /// FINDING: suggest_repairs for ActionFailed bases confidence on
    /// incident.replay_safe (0.95 for true, 0.3 for false) but does not
    /// consider other factors like severity or failure count. This is a
    /// simplistic heuristic that could mislead operators.
    #[test]
    fn blackhat_action_failed_confidence_binary_based_on_replay_safe() {
        let incident_safe = make_incident(
            FailureCode::ActionFailed("err".into()),
            SideEffectCertainty::Certain,
        );
        assert!(incident_safe.replay_safe);
        let suggestions_safe = suggest_repairs(&incident_safe);
        let safe_diff = (suggestions_safe[0].confidence - 0.95_f32).abs();
        assert!(
            safe_diff < f32::EPSILON,
            "replay_safe=true gives 0.95 confidence"
        );

        let mut incident_unsafe = make_incident(
            FailureCode::ActionFailed("err".into()),
            SideEffectCertainty::Certain,
        );
        incident_unsafe.replay_safe = false;
        let suggestions_unsafe = suggest_repairs(&incident_unsafe);
        let unsafe_diff = (suggestions_unsafe[0].confidence - 0.3_f32).abs();
        assert!(
            unsafe_diff < f32::EPSILON,
            "replay_safe=false gives 0.3 confidence"
        );
    }

    /// FINDING: RepairKind has no Unknown/Default variant. If a new FailureCode
    /// is added and suggest_repairs is not updated, the match will fail at compile
    /// time (good - exhaustive matching). But suggest_repairs_for_record would
    /// also fail. This is actually a safety feature.
    #[test]
    fn blackhat_repair_kind_covers_all_failure_codes() {
        // Verify every FailureCode maps to at least one RepairKind
        let failure_codes = [
            FailureCode::ActionTimeout,
            FailureCode::ActionFailed(String::from("err")),
            FailureCode::BudgetExceeded,
            FailureCode::StepPanicked,
            FailureCode::ValidationError(String::from("bad")),
            FailureCode::TaintLeak,
            FailureCode::ReplayDivergence,
            FailureCode::Unknown(String::from("x")),
        ];
        for code in &failure_codes {
            let incident = make_incident(code.clone(), SideEffectCertainty::Certain);
            let suggestions = suggest_repairs(&incident);
            assert!(
                !suggestions.is_empty(),
                "every FailureCode must produce at least one suggestion"
            );
            // Every suggestion must have a valid kind
            for s in &suggestions {
                let label = s.kind.as_str();
                assert!(
                    !label.is_empty(),
                    "RepairKind.as_str() must return non-empty for {:?}",
                    s.kind
                );
            }
        }
    }

    /// FINDING: suggest_repairs_for_record for ValidationError includes the raw
    /// message in the rationale via format!(). If the validation message contains
    /// user-controlled data, this could lead to log injection or misleading output.
    #[test]
    fn blackhat_validation_error_rationale_includes_raw_message() {
        let record = make_record(
            1,
            1,
            FailureCode::ValidationError(String::from(
                "field 'name' contains <script>alert(1)</script>",
            )),
            ReplaySafety::Safe,
        );
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(suggestions.len(), 1);
        // The raw message is included verbatim in the rationale
        assert!(
            suggestions[0]
                .rationale
                .contains("<script>alert(1)</script>"),
            "raw user message included in rationale without sanitization"
        );
    }

    /// FINDING: suggest_repairs_for_record for Unknown includes the inner string
    /// in the rationale. Same injection concern as ValidationError.
    #[test]
    fn blackhat_unknown_failure_inner_string_in_rationale() {
        let record = make_record(
            1,
            1,
            FailureCode::Unknown(String::from("error\nwith\nnewlines")),
            ReplaySafety::Safe,
        );
        let suggestions = suggest_repairs_for_record(&record);
        assert!(
            suggestions[0].rationale.contains("error\nwith\nnewlines"),
            "raw inner string with newlines included in rationale"
        );
    }

    /// FINDING: RepairAction has both PinIdempotency and ManualIntervention
    /// which overlap in purpose for some failure codes. ReplayDivergence maps
    /// to PinIdempotency kind but ManualIntervention action, which is confusing.
    #[test]
    fn blackhat_replay_divergence_kind_vs_action_mismatch() {
        let record = make_record(1, 1, FailureCode::ReplayDivergence, ReplaySafety::Safe);
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(
            suggestions[0].kind,
            RepairKind::PinIdempotency,
            "kind is PinIdempotency"
        );
        assert_eq!(
            suggestions[0].action,
            RepairAction::ManualIntervention,
            "action is ManualIntervention"
        );
        // The kind says "pin idempotency" but the action says "manual intervention"
        // This is semantically inconsistent
    }

    /// FINDING: RepairAction::AdjustBudget is used only for BudgetExceeded in
    /// both legacy and record paths, but there's no corresponding RepairKind
    /// (it maps to RepairKind::IncreaseTimeout instead). This means the "kind"
    /// label says "IncreaseTimeout" while the "action" says "AdjustBudget".
    #[test]
    fn blackhat_budget_exceeded_kind_increase_timeout_vs_action_adjust_budget() {
        let record = make_record(1, 1, FailureCode::BudgetExceeded, ReplaySafety::Safe);
        let suggestions = suggest_repairs_for_record(&record);
        assert_eq!(
            suggestions[0].kind,
            RepairKind::IncreaseTimeout,
            "kind says IncreaseTimeout"
        );
        assert_eq!(
            suggestions[0].action,
            RepairAction::AdjustBudget,
            "action says AdjustBudget"
        );
        // Kind and action disagree on the repair strategy
    }

    /// FINDING: suggest_repairs adds PinIdempotency when side_effect_certainty
    /// is Unknown, regardless of the failure code. This means StepPanicked with
    /// Unknown certainty gets both ManualInvestigation AND PinIdempotency,
    /// which may be contradictory (if the panic was in the idempotency logic).
    #[test]
    fn blackhat_step_panicked_unknown_certainty_adds_contradictory_pin_idempotency() {
        let incident = make_incident(FailureCode::StepPanicked, SideEffectCertainty::Unknown);
        let suggestions = suggest_repairs(&incident);
        assert!(
            suggestions
                .iter()
                .any(|s| s.kind == RepairKind::ManualInvestigation)
        );
        assert!(
            suggestions
                .iter()
                .any(|s| s.kind == RepairKind::PinIdempotency)
        );
        // Both suggestions are present, which may be contradictory if the panic
        // occurred in the idempotency pinning logic itself
    }

    /// FINDING: suggest_repairs_for_record with ReplaySafety::Unknown adds a
    /// PinIdempotency suggestion with confidence 0.6. This is the same as
    /// UnsafeSideEffect, which means Unknown and UnsafeSideEffect are treated
    /// identically in the record-based suggestion system.
    #[test]
    fn blackhat_record_unknown_and_unsafe_replay_safety_same_extra_suggestion() {
        let record_unknown = make_record(1, 1, FailureCode::ActionTimeout, ReplaySafety::Unknown);
        let record_unsafe = make_record(
            1,
            1,
            FailureCode::ActionTimeout,
            ReplaySafety::UnsafeSideEffect,
        );

        let suggestions_unknown = suggest_repairs_for_record(&record_unknown);
        let suggestions_unsafe = suggest_repairs_for_record(&record_unsafe);

        // Both should have 2 suggestions
        assert_eq!(suggestions_unknown.len(), 2);
        assert_eq!(suggestions_unsafe.len(), 2);

        // The extra PinIdempotency suggestion has the same confidence and level for both
        let pin_unknown = suggestions_unknown
            .iter()
            .find(|s| s.kind == RepairKind::PinIdempotency);
        let pin_unsafe = suggestions_unsafe
            .iter()
            .find(|s| s.kind == RepairKind::PinIdempotency);
        assert!(pin_unknown.is_some());
        assert!(pin_unsafe.is_some());
        assert_eq!(
            pin_unknown.map(|s| s.confidence),
            pin_unsafe.map(|s| s.confidence)
        );
    }

    // =========================================================================
    // Phase 5B: RepairActionState tests
    // =========================================================================

    #[test]
    fn test_repair_action_state_variants_are_distinct() {
        let variants = [
            RepairActionState::Pending,
            RepairActionState::Applied,
            RepairActionState::Dismissed,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    #[test]
    fn test_repair_action_state_as_str() {
        assert_eq!(RepairActionState::Pending.as_str(), "Pending");
        assert_eq!(RepairActionState::Applied.as_str(), "Applied");
        assert_eq!(RepairActionState::Dismissed.as_str(), "Dismissed");
    }

    #[test]
    fn test_repair_action_state_display_color_pending_is_yellow() {
        let color = RepairActionState::Pending.display_color();
        assert_eq!(color, NEON_YELLOW);
    }

    #[test]
    fn test_repair_action_state_display_color_applied_is_green() {
        let color = RepairActionState::Applied.display_color();
        assert_eq!(color, NEON_GREEN);
    }

    #[test]
    fn test_repair_action_state_display_color_dismissed_is_red() {
        let color = RepairActionState::Dismissed.display_color();
        assert_eq!(color, NEON_RED);
    }

    #[test]
    fn test_repair_action_state_copy_trait() {
        let state = RepairActionState::Applied;
        let copied = state;
        assert_eq!(state, copied);
    }

    // =========================================================================
    // Phase 5B: Cyberpunk color constants tests
    // =========================================================================

    #[test]
    fn test_neon_cyan_values() {
        let [r, g, b, a] = NEON_CYAN;
        assert!((0.0_f32..=1.0_f32).contains(&r));
        assert!(g > 0.9_f32);
        assert!(b > 0.9_f32);
        let diff = (a - 1.0_f32).abs();
        assert!(diff < f32::EPSILON);
    }

    #[test]
    fn test_neon_green_values() {
        let [r, g, b, a] = NEON_GREEN;
        assert!(r > 0.1_f32);
        assert!(g > 0.9_f32);
        assert!((0.0_f32..=1.0_f32).contains(&b));
        let diff = (a - 1.0_f32).abs();
        assert!(diff < f32::EPSILON);
    }

    #[test]
    fn test_neon_red_values() {
        let [r, g, b, a] = NEON_RED;
        assert!(r > 0.9_f32);
        assert!(g < 0.1_f32);
        assert!((0.0_f32..=1.0_f32).contains(&b));
        let diff = (a - 1.0_f32).abs();
        assert!(diff < f32::EPSILON);
    }

    #[test]
    fn test_neon_yellow_values() {
        let [r, g, b, a] = NEON_YELLOW;
        assert!(r > 0.9_f32);
        assert!(g > 0.8_f32);
        assert!(b < 0.1_f32);
        let diff = (a - 1.0_f32).abs();
        assert!(diff < f32::EPSILON);
    }

    #[test]
    fn test_neon_orange_values() {
        let [r, g, b, a] = NEON_ORANGE;
        assert!(r > 0.9_f32);
        assert!(g > 0.3_f32 && g < 0.6_f32);
        assert!(b < 0.1_f32);
        let diff = (a - 1.0_f32).abs();
        assert!(diff < f32::EPSILON);
    }

    #[test]
    fn test_all_neon_colors_have_full_alpha() {
        let colors = [NEON_CYAN, NEON_GREEN, NEON_RED, NEON_YELLOW, NEON_ORANGE];
        for (i, color) in colors.iter().enumerate() {
            let diff = (color[3] - 1.0_f32).abs();
            assert!(
                diff < f32::EPSILON,
                "color at index {i} must have alpha=1.0"
            );
        }
    }

    // =========================================================================
    // Phase 5B: RepairPanel tests
    // =========================================================================

    fn make_suggestion(kind: RepairKind, action: RepairAction) -> RepairSuggestion {
        RepairSuggestion {
            kind,
            description: String::from("test suggestion"),
            action,
            confidence: 0.5,
            confidence_level: RepairConfidence::Medium,
            rationale: String::from("test rationale"),
        }
    }

    #[test]
    fn test_repair_panel_new_is_empty() {
        let panel = RepairPanel::new();
        assert!(panel.is_empty());
        assert_eq!(panel.len(), 0);
        assert_eq!(panel.pending_count(), 0);
        assert_eq!(panel.applied_count(), 0);
        assert_eq!(panel.dismissed_count(), 0);
        assert!(panel.entries().is_empty());
        assert!(
            panel.is_resolved(),
            "empty panel should be considered resolved"
        );
    }

    #[test]
    fn test_repair_panel_default_matches_new() {
        let from_new = RepairPanel::new();
        let from_default = RepairPanel::default();
        assert_eq!(from_new.len(), from_default.len());
        assert!(from_default.is_empty());
    }

    #[test]
    fn test_repair_panel_add_suggestion_increments_len() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        assert_eq!(panel.len(), 1);
        assert_eq!(panel.pending_count(), 1);
        assert!(!panel.is_empty());

        panel.add_suggestion(make_suggestion(
            RepairKind::FixSecretLeak,
            RepairAction::FixSecretLeak,
        ));
        assert_eq!(panel.len(), 2);
        assert_eq!(panel.pending_count(), 2);
    }

    #[test]
    fn test_repair_panel_add_suggestion_starts_as_pending() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::PinIdempotency,
            RepairAction::PinIdempotency,
        ));
        let entry = panel.get(0);
        assert!(entry.is_some());
        assert_eq!(entry.map(|e| e.state), Some(RepairActionState::Pending));
    }

    #[test]
    fn test_repair_panel_dismiss_pending() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        assert!(panel.dismiss(0));
        assert_eq!(panel.pending_count(), 0);
        assert_eq!(panel.dismissed_count(), 1);
        assert_eq!(panel.applied_count(), 0);
        assert!(panel.is_resolved());
    }

    #[test]
    fn test_repair_panel_dismiss_out_of_bounds_returns_false() {
        let mut panel = RepairPanel::new();
        assert!(!panel.dismiss(0));
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        assert!(!panel.dismiss(5));
        assert_eq!(
            panel.pending_count(),
            1,
            "out-of-bounds dismiss should not change state"
        );
    }

    #[test]
    fn test_repair_panel_dismiss_already_applied_returns_false() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        assert!(panel.apply(0));
        assert!(
            !panel.dismiss(0),
            "cannot dismiss an already-applied suggestion"
        );
        assert_eq!(panel.applied_count(), 1);
        assert_eq!(panel.dismissed_count(), 0);
    }

    #[test]
    fn test_repair_panel_dismiss_already_dismissed_returns_false() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        assert!(panel.dismiss(0));
        assert!(
            !panel.dismiss(0),
            "cannot dismiss an already-dismissed suggestion"
        );
    }

    #[test]
    fn test_repair_panel_apply_pending() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::ReducePayload,
            RepairAction::ReducePayload,
        ));
        assert!(panel.apply(0));
        assert_eq!(panel.pending_count(), 0);
        assert_eq!(panel.applied_count(), 1);
        assert!(panel.is_resolved());
    }

    #[test]
    fn test_repair_panel_apply_out_of_bounds_returns_false() {
        let mut panel = RepairPanel::new();
        assert!(!panel.apply(0));
        panel.add_suggestion(make_suggestion(
            RepairKind::ReducePayload,
            RepairAction::ReducePayload,
        ));
        assert!(!panel.apply(99));
    }

    #[test]
    fn test_repair_panel_apply_already_applied_returns_false() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::ReducePayload,
            RepairAction::ReducePayload,
        ));
        assert!(panel.apply(0));
        assert!(
            !panel.apply(0),
            "cannot apply an already-applied suggestion"
        );
    }

    #[test]
    fn test_repair_panel_apply_already_dismissed_returns_false() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::ReducePayload,
            RepairAction::ReducePayload,
        ));
        assert!(panel.dismiss(0));
        assert!(!panel.apply(0), "cannot apply a dismissed suggestion");
    }

    #[test]
    fn test_repair_panel_apply_all() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        panel.add_suggestion(make_suggestion(
            RepairKind::AddRetryBackoff,
            RepairAction::AddRetryBackoff,
        ));
        panel.add_suggestion(make_suggestion(
            RepairKind::ReducePayload,
            RepairAction::ReducePayload,
        ));
        let applied = panel.apply_all();
        assert_eq!(applied, 3);
        assert_eq!(panel.pending_count(), 0);
        assert_eq!(panel.applied_count(), 3);
        assert!(panel.is_resolved());
    }

    #[test]
    fn test_repair_panel_apply_all_skips_resolved() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        panel.add_suggestion(make_suggestion(
            RepairKind::AddRetryBackoff,
            RepairAction::AddRetryBackoff,
        ));
        // Dismiss the first one
        assert!(panel.dismiss(0));
        // apply_all should only apply the remaining pending one
        let applied = panel.apply_all();
        assert_eq!(applied, 1);
        assert_eq!(panel.dismissed_count(), 1);
        assert_eq!(panel.applied_count(), 1);
        assert_eq!(panel.pending_count(), 0);
    }

    #[test]
    fn test_repair_panel_apply_all_empty_returns_zero() {
        let mut panel = RepairPanel::new();
        let applied = panel.apply_all();
        assert_eq!(applied, 0);
    }

    #[test]
    fn test_repair_panel_pending_count_mixed_states() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        panel.add_suggestion(make_suggestion(
            RepairKind::AddRetryBackoff,
            RepairAction::AddRetryBackoff,
        ));
        panel.add_suggestion(make_suggestion(
            RepairKind::ReducePayload,
            RepairAction::ReducePayload,
        ));
        panel.add_suggestion(make_suggestion(
            RepairKind::FixSecretLeak,
            RepairAction::FixSecretLeak,
        ));
        // Apply 0, dismiss 1, leave 2 and 3 pending
        assert!(panel.apply(0));
        assert!(panel.dismiss(1));
        assert_eq!(panel.pending_count(), 2);
        assert_eq!(panel.applied_count(), 1);
        assert_eq!(panel.dismissed_count(), 1);
        assert!(!panel.is_resolved());
    }

    #[test]
    fn test_repair_panel_from_incident() {
        let incident = make_incident(FailureCode::ActionTimeout, SideEffectCertainty::Certain);
        let panel = RepairPanel::from_incident(&incident);
        // ActionTimeout produces 2 suggestions
        assert_eq!(panel.len(), 2);
        assert_eq!(panel.pending_count(), 2);
        assert!(
            panel
                .entries()
                .iter()
                .all(|e| e.state == RepairActionState::Pending)
        );
    }

    #[test]
    fn test_repair_panel_from_record() {
        let record = make_record(1, 1, FailureCode::TaintLeak, ReplaySafety::Safe);
        let panel = RepairPanel::from_record(&record);
        assert_eq!(panel.len(), 1);
        assert_eq!(panel.pending_count(), 1);
        assert_eq!(
            panel.get(0).map(|e| e.suggestion.kind),
            Some(RepairKind::FixSecretLeak)
        );
    }

    #[test]
    fn test_repair_panel_from_record_unsafe_replay() {
        let record = make_record(
            1,
            1,
            FailureCode::ActionTimeout,
            ReplaySafety::UnsafeSideEffect,
        );
        let panel = RepairPanel::from_record(&record);
        assert_eq!(panel.len(), 2, "unsafe replay adds PinIdempotency");
        assert_eq!(panel.pending_count(), 2);
    }

    #[test]
    fn test_repair_panel_from_suggestions() {
        let suggestions = vec![
            make_suggestion(RepairKind::IncreaseTimeout, RepairAction::IncreaseTimeout),
            make_suggestion(RepairKind::FixSecretLeak, RepairAction::FixSecretLeak),
        ];
        let panel = RepairPanel::from_suggestions(suggestions);
        assert_eq!(panel.len(), 2);
        assert_eq!(panel.pending_count(), 2);
    }

    #[test]
    fn test_repair_panel_from_suggestions_empty() {
        let panel = RepairPanel::from_suggestions(Vec::new());
        assert!(panel.is_empty());
        assert_eq!(panel.pending_count(), 0);
    }

    #[test]
    fn test_repair_panel_get_valid_index() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        let entry = panel.get(0);
        assert!(entry.is_some());
        assert_eq!(
            entry.map(|e| e.suggestion.kind),
            Some(RepairKind::IncreaseTimeout)
        );
    }

    #[test]
    fn test_repair_panel_get_invalid_index_returns_none() {
        let panel = RepairPanel::new();
        assert!(panel.get(0).is_none());
    }

    #[test]
    fn test_repair_panel_is_resolved_false_with_pending() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        assert!(!panel.is_resolved());
    }

    #[test]
    fn test_repair_panel_is_resolved_true_after_apply_all() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        panel.add_suggestion(make_suggestion(
            RepairKind::FixSecretLeak,
            RepairAction::FixSecretLeak,
        ));
        panel.apply_all();
        assert!(panel.is_resolved());
    }

    #[test]
    fn test_repair_panel_is_resolved_true_after_dismiss_all() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        panel.dismiss(0);
        assert!(panel.is_resolved());
    }

    #[test]
    fn test_repair_panel_panel_color_empty_is_cyan() {
        let panel = RepairPanel::new();
        assert_eq!(panel.panel_color(), NEON_CYAN);
    }

    #[test]
    fn test_repair_panel_panel_color_pending_is_yellow() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        assert_eq!(panel.panel_color(), NEON_YELLOW);
    }

    #[test]
    fn test_repair_panel_panel_color_applied_is_green() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        panel.apply(0);
        assert_eq!(panel.panel_color(), NEON_GREEN);
    }

    #[test]
    fn test_repair_panel_panel_color_all_dismissed_is_red() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        panel.dismiss(0);
        assert_eq!(panel.panel_color(), NEON_RED);
    }

    #[test]
    fn test_repair_panel_panel_color_mixed_applied_and_dismissed_is_green() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        panel.add_suggestion(make_suggestion(
            RepairKind::FixSecretLeak,
            RepairAction::FixSecretLeak,
        ));
        panel.apply(0);
        panel.dismiss(1);
        // Has at least one applied, so green
        assert_eq!(panel.panel_color(), NEON_GREEN);
    }

    #[test]
    fn test_repair_panel_clone() {
        let mut panel = RepairPanel::new();
        panel.add_suggestion(make_suggestion(
            RepairKind::IncreaseTimeout,
            RepairAction::IncreaseTimeout,
        ));
        panel.apply(0);
        let cloned = panel.clone();
        assert_eq!(cloned.len(), panel.len());
        assert_eq!(cloned.applied_count(), panel.applied_count());
        assert_eq!(cloned.pending_count(), panel.pending_count());
    }

    #[test]
    fn test_repair_entry_clone() {
        let entry = RepairEntry {
            suggestion: make_suggestion(RepairKind::FixSecretLeak, RepairAction::FixSecretLeak),
            state: RepairActionState::Applied,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.state, entry.state);
        assert_eq!(cloned.suggestion.kind, entry.suggestion.kind);
        assert_eq!(cloned.suggestion.action, entry.suggestion.action);
    }

    #[test]
    fn test_repair_panel_large_number_of_suggestions() {
        let mut panel = RepairPanel::new();
        for i in 0..100usize {
            panel.add_suggestion(RepairSuggestion {
                kind: RepairKind::ManualInvestigation,
                description: format!("suggestion {i}"),
                action: RepairAction::ManualIntervention,
                confidence: 0.5,
                confidence_level: RepairConfidence::Medium,
                rationale: String::from("test"),
            });
        }
        assert_eq!(panel.len(), 100);
        assert_eq!(panel.pending_count(), 100);
        let applied = panel.apply_all();
        assert_eq!(applied, 100);
        assert_eq!(panel.pending_count(), 0);
        assert_eq!(panel.applied_count(), 100);
        assert!(panel.is_resolved());
    }
}
