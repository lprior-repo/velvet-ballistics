use super::types::{FailureCode, Incident, IncidentRecord, SideEffectCertainty};

/// Primary repair kind as specified by the Phase 5A contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
                rationale: String::from("Timeout failures are typically resolved by allowing more time"),
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
                rationale: String::from("Action failure may be transient; replay safety determines retry viability"),
            });
        }
        FailureCode::BudgetExceeded => {
            suggestions.push(RepairSuggestion {
                kind: RepairKind::IncreaseTimeout,
                action: RepairAction::AdjustBudget,
                description: "Increase the step budget in the resource contract".into(),
                confidence: 0.8,
                confidence_level: RepairConfidence::Medium,
                rationale: String::from("Budget exhaustion indicates the step needs more resource headroom"),
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
                rationale: String::from("Taint leaks require blocking data flow from secret to public outputs"),
            });
        }
        FailureCode::ReplayDivergence => {
            suggestions.push(RepairSuggestion {
                kind: RepairKind::PinIdempotency,
                action: RepairAction::ManualIntervention,
                description: "Replay diverged from original execution - investigate journal".into(),
                confidence: 0.3,
                confidence_level: RepairConfidence::Low,
                rationale: String::from("Replay divergence indicates non-deterministic behavior that must be traced"),
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
            rationale: String::from("Unknown side effects require pinning to ensure idempotent retries"),
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
        assert!(suggestions.iter().any(|s| s.action == RepairAction::IncreaseTimeout));
        assert!(suggestions.iter().any(|s| s.kind == RepairKind::IncreaseTimeout));
    }

    #[test]
    fn test_taint_leak_suggests_fix_secret_leak() {
        let incident = make_incident(FailureCode::TaintLeak, SideEffectCertainty::Certain);
        let suggestions = suggest_repairs(&incident);
        assert!(suggestions.iter().any(|s| s.action == RepairAction::FixSecretLeak));
        assert!(suggestions.iter().any(|s| s.kind == RepairKind::FixSecretLeak));
    }

    #[test]
    fn test_unknown_certainty_adds_pin_idempotency() {
        let incident = make_incident(FailureCode::Unknown("x".into()), SideEffectCertainty::Unknown);
        let suggestions = suggest_repairs(&incident);
        assert!(suggestions.iter().any(|s| s.action == RepairAction::PinIdempotency));
        assert!(suggestions.iter().any(|s| s.action == RepairAction::ManualIntervention));
        assert!(suggestions.iter().any(|s| s.kind == RepairKind::PinIdempotency));
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
        let record = make_record(900, 2, FailureCode::ActionTimeout, ReplaySafety::UnsafeSideEffect);
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
        assert!(suggestions
            .iter()
            .any(|s| s.kind == RepairKind::PinIdempotency));
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
        assert!(suggestions.iter().all(|s| !s.confidence_level.display_str().is_empty()));
    }
}
