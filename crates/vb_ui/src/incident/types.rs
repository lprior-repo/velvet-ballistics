#![forbid(unsafe_code)]
use std::time::Instant;

use super::repair::RepairSuggestion;

/// Classification of the incident category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
#[non_exhaustive]
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

    /// Return true if this severity requires immediate operator action.
    /// Critical and Error (Major) are actionable; Minor, Warning, and Info
    /// are informational.
    pub fn is_actionable(&self) -> bool {
        matches!(self, Self::Critical | Self::Major)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
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

    /// Return a category string for grouping related failure codes.
    pub fn category(&self) -> &'static str {
        match self {
            Self::ActionTimeout | Self::ActionFailed(_) => "action",
            Self::BudgetExceeded | Self::StepPanicked => "execution",
            Self::ValidationError(_) => "validation",
            Self::TaintLeak => "security",
            Self::ReplayDivergence => "replay",
            Self::Unknown(_) => "internal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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

/// View model for the cause section of an incident detail panel.
/// Provides a structured breakdown of why an incident occurred.
#[derive(Debug, Clone)]
pub struct IncidentCauseView {
    /// Human-readable label for the failure code category.
    pub category: String,
    /// Structured failure code for programmatic access.
    pub failure_code: FailureCode,
    /// The error message associated with the incident.
    pub error_message: String,
    /// Severity of the incident.
    pub severity: IncidentSeverity,
    /// Optional step name where the failure occurred.
    pub step_name: Option<String>,
    /// Run identifier for the failed run.
    pub run_id: u64,
}

/// View model for a single entry in the incident detail timeline.
/// Derived from the base [`TimelineEntry`] but with display-oriented fields.
#[derive(Debug, Clone)]
pub struct IncidentTimelineEntry {
    /// Sequence number for ordering.
    pub seq: u32,
    /// Human-readable description of the event.
    pub description: String,
    /// Timestamp in microseconds since epoch.
    pub timestamp_micros: u64,
    /// Classification of the event kind.
    pub event_kind: TimelineEventKind,
}

/// View model representing the diff of a single slot value before/after the
/// incident. Used in the detail panel's state-diff section.
#[derive(Debug, Clone)]
pub struct IncidentSlotDiff {
    /// Slot index that changed.
    pub slot_index: u16,
    /// Value of the slot before the incident (empty string if unknown).
    pub value_before: String,
    /// Value of the slot after the incident (empty string if unknown).
    pub value_after: String,
    /// Label describing the nature of the change.
    pub change_label: String,
}

/// Aggregated detail sections for the currently selected incident.
/// Returned by `IncidentScreen::detail_sections` for tab-based rendering.
#[derive(Debug, Clone)]
pub struct IncidentDetailSections {
    /// Structured cause information, if an incident is selected.
    pub cause: Option<IncidentCauseView>,
    /// Chronological timeline of events for the incident.
    pub timeline: Vec<IncidentTimelineEntry>,
    /// Slot value changes observed during the incident.
    pub state_diff: Vec<IncidentSlotDiff>,
    /// Repair suggestions for the incident.
    pub repair_suggestions: Vec<RepairSuggestion>,
    /// Whether the incident is safe to replay.
    pub replay_safe: bool,
    /// Side-effect certainty classification.
    pub side_effect_certainty: SideEffectCertainty,
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
            slot_values_before: vec![
                (1_u16, String::from("alpha")),
                (2_u16, String::from("beta")),
            ],
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

    // ---------------------------------------------------------------------------
    // F. IncidentSeverity::is_actionable() — 5 tests
    // ---------------------------------------------------------------------------

    #[test]
    fn is_actionable_critical() {
        assert!(IncidentSeverity::Critical.is_actionable());
    }

    #[test]
    fn is_actionable_major() {
        assert!(IncidentSeverity::Major.is_actionable());
    }

    #[test]
    fn is_actionable_minor_not_actionable() {
        assert!(!IncidentSeverity::Minor.is_actionable());
    }

    #[test]
    fn is_actionable_warning_not_actionable() {
        assert!(!IncidentSeverity::Warning.is_actionable());
    }

    #[test]
    fn is_actionable_info_not_actionable() {
        assert!(!IncidentSeverity::Info.is_actionable());
    }

    // ---------------------------------------------------------------------------
    // G. FailureCode::category() — 8 tests
    // ---------------------------------------------------------------------------

    #[test]
    fn category_action_timeout() {
        assert_eq!(FailureCode::ActionTimeout.category(), "action");
    }

    #[test]
    fn category_action_failed() {
        assert_eq!(
            FailureCode::ActionFailed(String::from("boom")).category(),
            "action"
        );
    }

    #[test]
    fn category_budget_exceeded() {
        assert_eq!(FailureCode::BudgetExceeded.category(), "execution");
    }

    #[test]
    fn category_step_panicked() {
        assert_eq!(FailureCode::StepPanicked.category(), "execution");
    }

    #[test]
    fn category_validation_error() {
        assert_eq!(
            FailureCode::ValidationError(String::from("bad")).category(),
            "validation"
        );
    }

    #[test]
    fn category_taint_leak() {
        assert_eq!(FailureCode::TaintLeak.category(), "security");
    }

    #[test]
    fn category_replay_divergence() {
        assert_eq!(FailureCode::ReplayDivergence.category(), "replay");
    }

    #[test]
    fn category_unknown() {
        assert_eq!(
            FailureCode::Unknown(String::from("?")).category(),
            "internal"
        );
    }

    // ---------------------------------------------------------------------------
    // H. IncidentSeverity ordering via color channel dominance — 5 tests
    //     Verifies the logical ordering Critical > Major > Warning > Info by
    //     checking that higher-severity colors have stronger red channels and
    //     lower-severity colors shift toward green/blue.
    // ---------------------------------------------------------------------------

    #[test]
    fn severity_color_critical_has_strongest_red() {
        let [critical_r, ..] = IncidentSeverity::Critical.severity_color();
        let [major_r, ..] = IncidentSeverity::Major.severity_color();
        let [warning_r, ..] = IncidentSeverity::Warning.severity_color();
        let [minor_r, ..] = IncidentSeverity::Minor.severity_color();
        let [info_r, ..] = IncidentSeverity::Info.severity_color();
        // All severity colors should have red >= 0, and Critical should dominate
        assert!(
            critical_r >= major_r,
            "Critical red ({critical_r}) should be >= Major red ({major_r})"
        );
        assert!(
            major_r >= warning_r,
            "Major red ({major_r}) should be >= Warning red ({warning_r})"
        );
        assert!(
            warning_r > minor_r,
            "Warning red ({warning_r}) should be > Minor red ({minor_r})"
        );
        assert!(
            minor_r > info_r,
            "Minor red ({minor_r}) should be > Info red ({info_r})"
        );
    }

    #[test]
    fn severity_color_info_is_cyan_dominant() {
        let [r, g, b, ..] = IncidentSeverity::Info.severity_color();
        assert!(r < 0.1_f32, "Info should have near-zero red");
        assert!(g > 0.9_f32, "Info should have strong green");
        assert!(b > 0.9_f32, "Info should have strong blue");
    }

    #[test]
    fn severity_color_minor_is_gray_neutral() {
        let [r, g, b, ..] = IncidentSeverity::Minor.severity_color();
        let diff_rg = (r - g).abs();
        let diff_gb = (g - b).abs();
        assert!(diff_rg < 0.01_f32, "Minor should have equal R and G");
        assert!(diff_gb < 0.01_f32, "Minor should have equal G and B");
    }

    #[test]
    fn severity_color_warning_is_yellow_range() {
        let [r, g, b, ..] = IncidentSeverity::Warning.severity_color();
        assert!(r > 0.9_f32, "Warning red should be strong");
        assert!(g > 0.8_f32, "Warning green should be strong");
        assert!(b < 0.1_f32, "Warning blue should be near zero");
    }

    #[test]
    fn severity_color_major_is_orange_range() {
        let [r, g, b, ..] = IncidentSeverity::Major.severity_color();
        assert!(r > 0.9_f32, "Major red should be strong");
        assert!(
            g > 0.3_f32 && g < 0.7_f32,
            "Major green should be moderate (orange)"
        );
        assert!(b < 0.1_f32, "Major blue should be near zero");
    }

    // ---------------------------------------------------------------------------
    // I. FailureCode display formatting — boundary values — 3 tests
    // ---------------------------------------------------------------------------

    #[test]
    fn failure_code_action_failed_empty_string_label() {
        assert_eq!(
            FailureCode::ActionFailed(String::new()).as_str(),
            "ActionFailed"
        );
    }

    #[test]
    fn failure_code_validation_error_empty_string_label() {
        assert_eq!(
            FailureCode::ValidationError(String::new()).as_str(),
            "ValidationError"
        );
    }

    #[test]
    fn failure_code_unknown_empty_string_label() {
        assert_eq!(
            FailureCode::Unknown(String::new()).as_str(),
            "InternalError"
        );
    }

    // ---------------------------------------------------------------------------
    // J. ReplaySafety behavioral default — only Safe is safe — 1 test
    // ---------------------------------------------------------------------------

    #[test]
    fn replay_safety_only_safe_variant_passes_is_safe() {
        let all_variants = [
            ReplaySafety::Safe,
            ReplaySafety::UnsafeSideEffect,
            ReplaySafety::Unknown,
        ];
        let safe_count = all_variants.iter().filter(|v| v.is_safe()).count();
        assert_eq!(safe_count, 1, "exactly one variant should be safe");
        assert!(
            ReplaySafety::Safe.is_safe(),
            "the Safe variant must be the one that is safe"
        );
    }

    // ---------------------------------------------------------------------------
    // K. IncidentRecord field access patterns — 2 tests
    // ---------------------------------------------------------------------------

    #[test]
    fn incident_record_all_fields_accessible_after_construction() {
        let record = IncidentRecord {
            run_id: 55_u64,
            shard_id: 3_u32,
            step: 12_u16,
            failure_code: FailureCode::ActionTimeout,
            severity: IncidentSeverity::Major,
            replay_safety: ReplaySafety::Safe,
            timestamp_us: 7_777_777_u64,
            detail: String::from("field access test"),
        };
        assert_eq!(record.run_id, 55);
        assert_eq!(record.shard_id, 3);
        assert_eq!(record.step, 12);
        assert_eq!(record.failure_code.as_str(), "ActionTimeout");
        assert_eq!(record.severity, IncidentSeverity::Major);
        assert!(record.replay_safety.is_safe());
        assert_eq!(record.timestamp_us, 7_777_777);
        assert_eq!(record.detail, "field access test");
    }

    #[test]
    fn incident_record_failure_code_clone_preserves_category() {
        let record = IncidentRecord {
            run_id: 1_u64,
            shard_id: 0_u32,
            step: 0_u16,
            failure_code: FailureCode::ActionFailed(String::from("net")),
            severity: IncidentSeverity::Critical,
            replay_safety: ReplaySafety::Unknown,
            timestamp_us: 100_u64,
            detail: String::from("clone test"),
        };
        let cloned_code = record.failure_code.clone();
        assert_eq!(cloned_code.as_str(), "ActionFailed");
        assert_eq!(cloned_code.category(), "action");
        // Original still accessible
        assert_eq!(record.failure_code.as_str(), "ActionFailed");
    }

    // =========================================================================
    // BLACKHAT security review tests
    // =========================================================================

    /// FINDING: Incident struct contains a `replay_safe: bool` field but the
    /// same concept exists as `ReplaySafety` enum in IncidentRecord. The bool
    /// loses information (Unknown vs UnsafeSideEffect are both mapped to false).
    /// This is a design inconsistency between legacy and Phase 5A types.
    #[test]
    fn blackhat_incident_replay_safe_bool_loses_information_vs_replay_safety_enum() {
        // The bool can only represent safe/unsafe, losing the distinction between
        // UnsafeSideEffect and Unknown.
        let record_unsafe = IncidentRecord {
            run_id: 1,
            shard_id: 0,
            step: 0,
            failure_code: FailureCode::ActionTimeout,
            severity: IncidentSeverity::Critical,
            replay_safety: ReplaySafety::UnsafeSideEffect,
            timestamp_us: 0,
            detail: String::new(),
        };
        let record_unknown = IncidentRecord {
            run_id: 2,
            shard_id: 0,
            step: 0,
            failure_code: FailureCode::ActionTimeout,
            severity: IncidentSeverity::Critical,
            replay_safety: ReplaySafety::Unknown,
            timestamp_us: 0,
            detail: String::new(),
        };
        // Both map to the same boolean: not safe
        assert!(!record_unsafe.replay_safety.is_safe());
        assert!(!record_unknown.replay_safety.is_safe());
        // But they are distinct enum variants
        assert_ne!(record_unsafe.replay_safety, record_unknown.replay_safety);
    }

    /// FINDING: IncidentRecord has `timestamp_us: u64` which is microseconds
    /// since epoch. At u64::MAX (~18.4 exa-seconds), this represents ~584,942
    /// years. No overflow risk in practice. However, arithmetic on timestamps
    /// (subtraction for duration) should use saturating_sub.
    #[test]
    fn blackhat_incident_record_timestamp_no_overflow_risk() {
        let record = IncidentRecord {
            run_id: 1,
            shard_id: 0,
            step: 0,
            failure_code: FailureCode::ActionTimeout,
            severity: IncidentSeverity::Info,
            replay_safety: ReplaySafety::Safe,
            timestamp_us: u64::MAX,
            detail: String::new(),
        };
        assert_eq!(record.timestamp_us, u64::MAX);
        // Duration calculation with saturating_sub handles u64::MAX correctly
        let duration = record.timestamp_us.saturating_sub(0);
        assert_eq!(duration, u64::MAX);
    }

    /// FINDING: FailureCode::as_str() returns &'static str but some variants
    /// (ActionFailed, ValidationError, Unknown) contain String payloads that
    /// are silently ignored by as_str(). Callers needing the inner message
    /// must pattern-match on the enum rather than using as_str().
    #[test]
    fn blackhat_failure_code_as_str_loses_inner_message_for_string_variants() {
        let code_with_message =
            FailureCode::ActionFailed(String::from("database connection refused"));
        assert_eq!(code_with_message.as_str(), "ActionFailed");
        // The inner message "database connection refused" is not accessible via as_str()
        // Callers must destructure the enum to get the message

        let code_validation = FailureCode::ValidationError(String::from("missing field 'name'"));
        assert_eq!(code_validation.as_str(), "ValidationError");
        // Same: the validation message is lost
    }

    /// FINDING: FailureCode::as_str() returns inconsistent names for some variants.
    /// BudgetExceeded returns "StepBudgetExhausted" and TaintLeak returns
    /// "TaintViolation". These renames could confuse consumers who expect the
    /// as_str() output to match the variant name.
    #[test]
    fn blackhat_failure_code_as_str_name_mismatches() {
        // BudgetExceeded -> "StepBudgetExhausted" (different name)
        assert_ne!(
            FailureCode::BudgetExceeded.as_str(),
            "BudgetExceeded",
            "BudgetExceeded.as_str() returns a different name than the variant"
        );
        // TaintLeak -> "TaintViolation" (different name)
        assert_ne!(
            FailureCode::TaintLeak.as_str(),
            "TaintLeak",
            "TaintLeak.as_str() returns a different name than the variant"
        );
    }

    /// FINDING: IncidentContext::action_attempts is u32. If a run has more
    /// than u32::MAX action attempts, this would overflow. In practice this
    /// is unlikely (~4 billion attempts), but worth documenting.
    #[test]
    fn blackhat_incident_context_action_attempts_at_max_u32() {
        let ctx = IncidentContext {
            slot_values_before: Vec::new(),
            taint_changes: Vec::new(),
            action_attempts: u32::MAX,
            last_action_idempotency_key: None,
        };
        assert_eq!(ctx.action_attempts, u32::MAX);
        // If incremented with wrapping_add(1), would overflow to 0
        // If incremented with saturating_add(1), stays at u32::MAX
    }

    /// FINDING: TimelineEntry.seq is u32. If a single incident has more than
    /// u32::MAX timeline events, the sequence counter would overflow.
    #[test]
    fn blackhat_timeline_entry_seq_at_max_u32() {
        let entry = TimelineEntry {
            seq: u32::MAX,
            description: String::from("overflow boundary"),
            timestamp_micros: 0,
            event_kind: TimelineEventKind::FailureObserved,
            timestamp: Instant::now(),
        };
        assert_eq!(entry.seq, u32::MAX);
    }

    /// FINDING: IncidentSlotDiff always produces a String for change_label
    /// but the logic that sets it (in screen.rs detail_sections) only produces
    /// "unchanged" or "modified". This could miss other change types like
    /// "added" or "removed" for slots that appear in taint_changes but not
    /// in slot_values_before.
    #[test]
    fn blackhat_incident_slot_diff_change_label_only_two_states() {
        let diff_unchanged = IncidentSlotDiff {
            slot_index: 0,
            value_before: String::from("same"),
            value_after: String::from("same"),
            change_label: String::from("unchanged"),
        };
        let diff_modified = IncidentSlotDiff {
            slot_index: 1,
            value_before: String::from("old"),
            value_after: String::from("new"),
            change_label: String::from("modified"),
        };
        assert_eq!(diff_unchanged.change_label, "unchanged");
        assert_eq!(diff_modified.change_label, "modified");
        // No "added" or "removed" labels are produced by the current logic
    }

    /// FINDING: IncidentDetailSections sets replay_safe to false when no
    /// incident is selected (the default path). This is a safe default but
    /// could be misleading since no incident actually exists.
    #[test]
    fn blackhat_detail_sections_default_replay_safe_is_false() {
        let sections = IncidentDetailSections {
            cause: None,
            timeline: Vec::new(),
            state_diff: Vec::new(),
            repair_suggestions: Vec::new(),
            replay_safe: false,
            side_effect_certainty: SideEffectCertainty::None,
        };
        assert!(
            !sections.replay_safe,
            "default replay_safe is false even with no incident"
        );
        assert_eq!(sections.side_effect_certainty, SideEffectCertainty::None);
    }

    /// FINDING: SideEffectCertainty has a `None` variant which could conflict
    /// with `Option<SideEffectCertainty>::None`. This naming collision is a
    /// readability hazard.
    #[test]
    fn blackhat_side_effect_certainty_none_vs_option_none() {
        let certainty = SideEffectCertainty::None;
        let option_none: Option<SideEffectCertainty> = None;
        // These are different types but could be confused in code
        assert_eq!(certainty, SideEffectCertainty::None);
        assert!(option_none.is_none());
        // SideEffectCertainty::None != Option::None (different types entirely)
    }

    /// FINDING: IncidentType has no Unknown/Other fallback variant. If a new
    /// FailureCode is added without updating incident_type_for_code(), the
    /// match would fail to compile (which is actually good - exhaustive matching
    /// catches missing cases at compile time).
    #[test]
    fn blackhat_incident_type_has_no_unknown_fallback() {
        // IncidentType has exactly 4 variants - no Unknown fallback
        let variants = [
            IncidentType::ActionFailure,
            IncidentType::ReplayDivergence,
            IncidentType::BlockedReconciliation,
            IncidentType::SecretLeak,
        ];
        // All distinct
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    /// FINDING: ReplaySafety::Unknown is conservatively treated as not safe.
    /// This is the correct defensive posture for replay safety.
    #[test]
    fn blackhat_replay_safety_unknown_is_conservatively_unsafe() {
        assert!(
            !ReplaySafety::Unknown.is_safe(),
            "Unknown should be treated as unsafe"
        );
    }
}
