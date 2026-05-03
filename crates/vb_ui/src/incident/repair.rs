use super::types::{FailureCode, Incident, SideEffectCertainty};

#[derive(Debug, Clone)]
pub struct RepairSuggestion {
    pub action: RepairAction,
    pub description: String,
    pub confidence: f32,
}

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
                action: RepairAction::IncreaseTimeout,
                description: "Increase the action timeout to accommodate slower responses".into(),
                confidence: 0.9,
            });
            suggestions.push(RepairSuggestion {
                action: RepairAction::AddRetryBackoff,
                description: "Add exponential backoff to handle transient timeouts".into(),
                confidence: 0.7,
            });
        }
        FailureCode::ActionFailed(_) => {
            suggestions.push(RepairSuggestion {
                action: RepairAction::RestartRun,
                description: format!("Restart the run (replay safe: {})", incident.replay_safe),
                confidence: if incident.replay_safe { 0.95 } else { 0.3 },
            });
        }
        FailureCode::BudgetExceeded => {
            suggestions.push(RepairSuggestion {
                action: RepairAction::AdjustBudget,
                description: "Increase the step budget in the resource contract".into(),
                confidence: 0.8,
            });
        }
        FailureCode::StepPanicked => {
            suggestions.push(RepairSuggestion {
                action: RepairAction::ManualIntervention,
                description: "Step panicked — investigate the step logic and fix the bug".into(),
                confidence: 0.5,
            });
        }
        FailureCode::ValidationError(msg) => {
            suggestions.push(RepairSuggestion {
                action: RepairAction::ManualIntervention,
                description: format!("Validation error: {msg}"),
                confidence: 0.4,
            });
        }
        FailureCode::TaintLeak => {
            suggestions.push(RepairSuggestion {
                action: RepairAction::FixSecretLeak,
                description: "Secret data reached a public result — add taint barrier".into(),
                confidence: 0.85,
            });
        }
        FailureCode::ReplayDivergence => {
            suggestions.push(RepairSuggestion {
                action: RepairAction::ManualIntervention,
                description: "Replay diverged from original execution — investigate journal".into(),
                confidence: 0.3,
            });
        }
        FailureCode::Unknown(_) => {
            suggestions.push(RepairSuggestion {
                action: RepairAction::ManualIntervention,
                description: "Unknown failure — manual investigation required".into(),
                confidence: 0.1,
            });
        }
    }
    if incident.side_effect_certainty == SideEffectCertainty::Unknown {
        suggestions.push(RepairSuggestion {
            action: RepairAction::PinIdempotency,
            description: "Side effect certainty unknown — pin idempotency key before retry".into(),
            confidence: 0.6,
        });
    }
    suggestions
}
