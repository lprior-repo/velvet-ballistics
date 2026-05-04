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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    // ---------------------------------------------------------------------------
    // A. IncidentSeverity::severity_color() — 7 tests
    // ---------------------------------------------------------------------------

    #[test]
    fn severity_color_critical_returns_valid_rgba() {
        let [r, g, b, a] = IncidentSeverity::Critical.severity_color();
        assert!((0.0..=1.0).contains(&r), "red out of range");
        assert!((0.0..=1.0).contains(&g), "green out of range");
        assert!((0.0..=1.0).contains(&b), "blue out of range");
        assert!((0.0..=1.0).contains(&a), "alpha out of range");
    }

    #[test]
    fn severity_color_major_returns_valid_rgba() {
        let [r, g, b, a] = IncidentSeverity::Major.severity_color();
        assert!((0.0..=1.0).contains(&r));
        assert!((0.0..=1.0).contains(&g));
        assert!((0.0..=1.0).contains(&b));
        assert!((0.0..=1.0).contains(&a));
    }

    #[test]
    fn severity_color_minor_returns_valid_rgba() {
        let [r, g, b, a] = IncidentSeverity::Minor.severity_color();
        assert!((0.0..=1.0).contains(&r));
        assert!((0.0..=1.0).contains(&g));
        assert!((0.0..=1.0).contains(&b));
        assert!((0.0..=1.0).contains(&a));
    }

    #[test]
    fn severity_color_warning_returns_valid_rgba() {
        let [r, g, b, a] = IncidentSeverity::Warning.severity_color();
        assert!((0.0..=1.0).contains(&r));
        assert!((0.0..=1.0).contains(&g));
        assert!((0.0..=1.0).contains(&b));
        assert!((0.0..=1.0).contains(&a));
    }

    #[test]
    fn severity_color_info_returns_valid_rgba() {
        let [r, g, b, a] = IncidentSeverity::Info.severity_color();
        assert!((0.0..=1.0).contains(&r));
        assert!((0.0..=1.0).contains(&g));
        assert!((0.0..=1.0).contains(&b));
        assert!((0.0..=1.0).contains(&a));
    }

    #[test]
    fn severity_color_all_variants_have_alpha_one() {
        let variants = [
            IncidentSeverity::Critical,
            IncidentSeverity::Major,
            IncidentSeverity::Minor,
            IncidentSeverity::Warning,
            IncidentSeverity::Info,
        ];
        for v in &variants {
            let [.., a] = v.severity_color();
            let diff = (a - 1.0_f32).abs();
            assert!(diff < f32::EPSILON, "alpha must be exactly 1.0 for {v:?}");
        }
    }

    #[test]
    fn severity_color_all_variants_are_distinct() {
        let colors: Vec<[f32; 4]> = [
            IncidentSeverity::Critical,
            IncidentSeverity::Major,
            IncidentSeverity::Minor,
            IncidentSeverity::Warning,
            IncidentSeverity::Info,
        ]
        .iter()
        .map(|v| v.severity_color())
        .collect();

        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                let differs = colors[i][0] != colors[j][0]
                    || colors[i][1] != colors[j][1]
                    || colors[i][2] != colors[j][2];
                assert!(differs, "colors[{i}] and colors[{j}] must differ");
            }
        }
    }

    // ---------------------------------------------------------------------------
    // B. FailureCode::as_str() — 8 tests
    // ---------------------------------------------------------------------------

    #[test]
    fn failure_code_action_timeout_label() {
        assert_eq!(FailureCode::ActionTimeout.as_str(), "ActionTimeout");
    }

    #[test]
    fn failure_code_action_failed_label() {
        assert_eq!(
            FailureCode::ActionFailed(String::from("boom")).as_str(),
            "ActionFailed"
        );
    }

    #[test]
    fn failure_code_budget_exceeded_label() {
        assert_eq!(FailureCode::BudgetExceeded.as_str(), "StepBudgetExhausted");
    }

    #[test]
    fn failure_code_step_panicked_label() {
        assert_eq!(FailureCode::StepPanicked.as_str(), "StepPanicked");
    }

    #[test]
    fn failure_code_validation_error_label() {
        assert_eq!(
            FailureCode::ValidationError(String::from("bad")).as_str(),
            "ValidationError"
        );
    }

    #[test]
    fn failure_code_taint_leak_label() {
        assert_eq!(FailureCode::TaintLeak.as_str(), "TaintViolation");
    }

    #[test]
    fn failure_code_replay_divergence_label() {
        assert_eq!(FailureCode::ReplayDivergence.as_str(), "ReplayDivergence");
    }

    #[test]
    fn failure_code_unknown_label() {
        assert_eq!(
            FailureCode::Unknown(String::from("mystery")).as_str(),
            "InternalError"
        );
    }

    // ---------------------------------------------------------------------------
    // C. ReplaySafety::is_safe() — 3 tests
    // ---------------------------------------------------------------------------

    #[test]
    fn replay_safety_safe_is_safe() {
        assert!(ReplaySafety::Safe.is_safe());
    }

    #[test]
    fn replay_safety_unsafe_side_effect_is_not_safe() {
        assert!(!ReplaySafety::UnsafeSideEffect.is_safe());
    }

    #[test]
    fn replay_safety_unknown_is_not_safe() {
        assert!(!ReplaySafety::Unknown.is_safe());
    }

    // ---------------------------------------------------------------------------
    // D. Enum distinctness — 5 tests
    // ---------------------------------------------------------------------------

    #[test]
    fn incident_severity_variants_are_distinct() {
        let variants = [
            IncidentSeverity::Critical,
            IncidentSeverity::Major,
            IncidentSeverity::Minor,
            IncidentSeverity::Warning,
            IncidentSeverity::Info,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    #[test]
    fn incident_type_variants_are_distinct() {
        let variants = [
            IncidentType::ActionFailure,
            IncidentType::ReplayDivergence,
            IncidentType::BlockedReconciliation,
            IncidentType::SecretLeak,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    #[test]
    fn replay_safety_variants_are_distinct() {
        let variants = [
            ReplaySafety::Safe,
            ReplaySafety::UnsafeSideEffect,
            ReplaySafety::Unknown,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    #[test]
    fn side_effect_certainty_variants_are_distinct() {
        let variants = [
            SideEffectCertainty::Certain,
            SideEffectCertainty::Unknown,
            SideEffectCertainty::None,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    #[test]
    fn timeline_event_kind_variants_are_distinct() {
        let variants = [
            TimelineEventKind::FailureObserved,
            TimelineEventKind::RetryAttempted,
            TimelineEventKind::SideEffectDetected,
            TimelineEventKind::ReplayDivergence,
            TimelineEventKind::RepairApplied,
            TimelineEventKind::IncidentDismissed,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    // ---------------------------------------------------------------------------
    // E. Struct construction — 6 tests
    // ---------------------------------------------------------------------------

    #[test]
    fn incident_context_construction() {
        let ctx = IncidentContext {
            slot_values_before: vec![(1_u16, String::from("alpha")), (2_u16, String::from("beta"))],
            taint_changes: vec![(3_u16, String::from("gamma"))],
            action_attempts: 7_u32,
            last_action_idempotency_key: Some(String::from("key-42")),
        };
        assert_eq!(ctx.slot_values_before.len(), 2);
        assert_eq!(ctx.taint_changes.len(), 1);
        assert_eq!(ctx.action_attempts, 7);
        let Some(ref k) = ctx.last_action_idempotency_key else {
            assert!(false, "idempotency key must be Some");
            return;
        };
        assert_eq!(k, "key-42");
    }

    #[test]
    fn timeline_entry_construction() {
        let now = Instant::now();
        let entry = TimelineEntry {
            seq: 10_u32,
            description: String::from("step failed"),
            timestamp_micros: 1_000_000_u64,
            event_kind: TimelineEventKind::FailureObserved,
            timestamp: now,
        };
        assert_eq!(entry.seq, 10);
        assert_eq!(entry.description, "step failed");
        assert_eq!(entry.timestamp_micros, 1_000_000);
        assert_eq!(entry.event_kind, TimelineEventKind::FailureObserved);
    }

    #[test]
    fn failure_detail_construction() {
        let now = Instant::now();
        let detail = FailureDetail {
            error_code: String::from("E001"),
            step_id: Some(5_u16),
            run_id: 99_u64,
            workflow_name: String::from("ci-pipeline"),
            replay_safe: true,
            timeline: vec![TimelineEntry {
                seq: 1_u32,
                description: String::from("retry"),
                timestamp_micros: 500_u64,
                event_kind: TimelineEventKind::RetryAttempted,
                timestamp: now,
            }],
            failure_code: FailureCode::ActionTimeout,
            step_name: Some(String::from("build")),
            side_effect_certainty: SideEffectCertainty::None,
            error_context: IncidentContext {
                slot_values_before: vec![],
                taint_changes: vec![],
                action_attempts: 0_u32,
                last_action_idempotency_key: None,
            },
        };
        assert_eq!(detail.error_code, "E001");
        let Some(sid) = detail.step_id else {
            assert!(false, "step_id must be Some");
            return;
        };
        assert_eq!(sid, 5);
        assert_eq!(detail.run_id, 99);
        assert!(detail.replay_safe);
        assert_eq!(detail.failure_code, FailureCode::ActionTimeout);
    }

    #[test]
    fn incident_record_construction() {
        let record = IncidentRecord {
            run_id: 42_u64,
            shard_id: 3_u32,
            step: 7_u16,
            failure_code: FailureCode::BudgetExceeded,
            severity: IncidentSeverity::Critical,
            replay_safety: ReplaySafety::UnsafeSideEffect,
            timestamp_us: 9_999_999_u64,
            detail: String::from("budget blown"),
        };
        assert_eq!(record.run_id, 42);
        assert_eq!(record.shard_id, 3);
        assert_eq!(record.step, 7);
        assert_eq!(record.failure_code, FailureCode::BudgetExceeded);
        assert_eq!(record.severity, IncidentSeverity::Critical);
        assert_eq!(record.replay_safety, ReplaySafety::UnsafeSideEffect);
        assert_eq!(record.timestamp_us, 9_999_999);
        assert_eq!(record.detail, "budget blown");
    }

    #[test]
    fn incident_construction() {
        let now = Instant::now();
        let incident = Incident {
            id: 1_u64,
            incident_type: IncidentType::ActionFailure,
            severity: IncidentSeverity::Major,
            failure_code: FailureCode::ActionFailed(String::from("network")),
            run_id: 10_u64,
            workflow_name: String::from("deploy"),
            step_id: Some(2_u16),
            step_name: Some(String::from("push")),
            error_message: String::from("connection refused"),
            replay_safe: false,
            side_effect_certainty: SideEffectCertainty::Unknown,
            timestamp: now,
            context: IncidentContext {
                slot_values_before: vec![(0_u16, String::from("init"))],
                taint_changes: vec![],
                action_attempts: 3_u32,
                last_action_idempotency_key: None,
            },
            timeline: vec![],
        };
        assert_eq!(incident.id, 1);
        assert_eq!(incident.incident_type, IncidentType::ActionFailure);
        assert_eq!(incident.severity, IncidentSeverity::Major);
        assert!(!incident.replay_safe);
        let Some(ref sn) = incident.step_name else {
            assert!(false, "step_name must be Some");
            return;
        };
        assert_eq!(sn, "push");
    }

    #[test]
    fn incident_record_minimal_fields() {
        let record = IncidentRecord {
            run_id: 0_u64,
            shard_id: 0_u32,
            step: 0_u16,
            failure_code: FailureCode::Unknown(String::from("?")),
            severity: IncidentSeverity::Info,
            replay_safety: ReplaySafety::Unknown,
            timestamp_us: 0_u64,
            detail: String::new(),
        };
        assert_eq!(record.run_id, 0);
        assert_eq!(record.shard_id, 0);
        assert_eq!(record.step, 0);
        assert_eq!(record.timestamp_us, 0);
        assert!(record.detail.is_empty());
        assert!(!record.replay_safety.is_safe());
    }
}
