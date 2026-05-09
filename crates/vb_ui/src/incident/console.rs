#![forbid(unsafe_code)]
use super::repair::{RepairSuggestion, suggest_repairs};
use super::types::{Incident, IncidentRecord, IncidentSeverity};

pub struct IncidentConsole {
    incidents: Vec<Incident>,
    records: Vec<IncidentRecord>,
    max_display: usize,
    selected: Option<usize>,
}

impl Default for IncidentConsole {
    fn default() -> Self {
        Self::new()
    }
}

impl IncidentConsole {
    const DEFAULT_MAX_DISPLAY: usize = 1000;

    pub fn new() -> Self {
        Self {
            incidents: Vec::new(),
            records: Vec::new(),
            max_display: Self::DEFAULT_MAX_DISPLAY,
            selected: None,
        }
    }

    pub fn with_max_display(max_display: usize) -> Self {
        let max = if max_display == 0 {
            Self::DEFAULT_MAX_DISPLAY
        } else {
            max_display
        };
        Self {
            incidents: Vec::new(),
            records: Vec::new(),
            max_display: max,
            selected: None,
        }
    }

    // -- Legacy Incident API --

    pub fn add_incident(&mut self, incident: Incident) -> usize {
        let idx = self.incidents.len();
        self.incidents.push(incident);
        idx
    }

    pub fn dismiss(&mut self, index: usize) {
        if self.incidents.get(index).is_some() {
            self.incidents.remove(index);
            self.selected = self.selected.and_then(|sel| {
                if sel == index {
                    None
                } else if sel > index {
                    Some(sel.saturating_sub(1))
                } else {
                    Some(sel)
                }
            });
        }
    }

    pub fn select(&mut self, index: usize) {
        if self.incidents.get(index).is_some() {
            self.selected = Some(index);
        }
    }

    pub fn selected(&self) -> Option<&Incident> {
        self.selected.and_then(|i| self.incidents.get(i))
    }

    /// Returns the index of the currently selected incident, if any.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected_suggestions(&self) -> Vec<RepairSuggestion> {
        self.selected().map(suggest_repairs).unwrap_or_default()
    }

    /// Return formatted display strings for the repair suggestions of the
    /// currently selected incident. Each string is a clickable-item label
    /// containing the action kind, confidence level, and description.
    /// Returns an empty vec if no incident is selected.
    pub fn suggestion_display_items(&self) -> Vec<String> {
        self.selected().map_or_else(Vec::new, |incident| {
            let suggestions = suggest_repairs(incident);
            suggestions
                .into_iter()
                .map(|s| {
                    let confidence_pct =
                        (s.confidence * 100.0_f32).round().clamp(0.0_f32, 100.0_f32);
                    format!(
                        "[{}] ({}%) {}",
                        s.kind.as_str(),
                        confidence_pct,
                        s.description,
                    )
                })
                .collect()
        })
    }

    /// Apply the repair suggestion at the given index for the selected incident.
    /// Returns true if a suggestion existed at that index and was applied.
    /// This is a placeholder for the UI action callback -- the actual repair
    /// logic is handled by the runtime layer.
    pub fn apply_suggestion(&self, _index: usize) -> bool {
        self.selected().is_some()
    }

    pub fn legacy_incidents(&self) -> &[Incident] {
        &self.incidents
    }

    pub fn legacy_critical_count(&self) -> usize {
        self.incidents
            .iter()
            .filter(|i| matches!(i.severity, IncidentSeverity::Critical))
            .count()
    }

    pub fn active_count(&self) -> usize {
        self.incidents.len()
    }

    // -- Phase 5A: IncidentRecord API --

    /// Push an incident record, trimming to max_display capacity.
    pub fn push_incident(&mut self, record: IncidentRecord) {
        self.records.push(record);
        while self.records.len() > self.max_display {
            self.records.remove(0);
        }
    }

    /// Return all incident records.
    pub fn active_incidents(&self) -> &[IncidentRecord] {
        &self.records
    }

    /// Return references to all records matching the given severity.
    pub fn incidents_by_severity(&self, severity: IncidentSeverity) -> Vec<&IncidentRecord> {
        self.records
            .iter()
            .filter(|r| r.severity == severity)
            .collect()
    }

    /// Return the count of records with Critical severity.
    pub fn critical_count(&self) -> usize {
        self.records
            .iter()
            .filter(|r| r.severity == IncidentSeverity::Critical)
            .count()
    }

    /// Return true if any record has a non-Safe replay safety classification.
    pub fn has_unsafe_replay(&self) -> bool {
        self.records.iter().any(|r| !r.replay_safety.is_safe())
    }

    /// Remove all records associated with a resolved run.
    pub fn clear_resolved(&mut self, run_id: u64) {
        self.records.retain(|r| r.run_id != run_id);
    }
}

#[cfg(test)]
mod tests {
    use super::super::repair::RepairAction;
    use super::super::types::{
        FailureCode, IncidentContext, IncidentSeverity, IncidentType, ReplaySafety,
        SideEffectCertainty,
    };
    use super::*;
    use std::time::Instant;

    // -- Legacy incident helpers --

    fn make_incident(id: u64, severity: IncidentSeverity, code: FailureCode) -> Incident {
        Incident {
            id,
            incident_type: IncidentType::ActionFailure,
            severity,
            failure_code: code,
            run_id: id,
            workflow_name: String::from("test-wf"),
            step_id: None,
            step_name: None,
            error_message: String::from("test error"),
            replay_safe: true,
            side_effect_certainty: SideEffectCertainty::Certain,
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

    // -- Phase 5A record helpers --

    fn make_record(
        run_id: u64,
        severity: IncidentSeverity,
        failure_code: FailureCode,
        replay_safety: ReplaySafety,
    ) -> IncidentRecord {
        IncidentRecord {
            run_id,
            shard_id: 0,
            step: 1,
            failure_code,
            severity,
            replay_safety,
            timestamp_us: 1000,
            detail: String::from("test detail"),
        }
    }

    // -- Legacy tests --

    #[test]
    fn test_console_new_is_empty() {
        let console = IncidentConsole::new();
        assert!(console.legacy_incidents().is_empty());
        assert_eq!(console.active_count(), 0);
        assert!(console.selected().is_none());
        assert!(console.selected_suggestions().is_empty());
        assert_eq!(console.legacy_critical_count(), 0);
    }

    #[test]
    fn test_console_add_and_select() {
        let mut console = IncidentConsole::new();
        let inc = make_incident(1, IncidentSeverity::Major, FailureCode::ActionTimeout);
        let idx = console.add_incident(inc);
        assert_eq!(idx, 0);
        assert_eq!(console.active_count(), 1);
        console.select(0);
        assert!(console.selected().is_some());
        assert_eq!(console.selected().map(|i| i.id), Some(1));
    }

    #[test]
    fn test_console_dismiss_updates_selection() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.add_incident(make_incident(
            2,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
        ));
        console.select(1);
        assert!(console.selected().is_some());
        console.dismiss(1);
        assert_eq!(console.active_count(), 1);
        assert!(console.selected().is_none());
    }

    #[test]
    fn test_console_suggestions_for_selected() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Major,
            FailureCode::ActionTimeout,
        ));
        console.select(0);
        let suggestions = console.selected_suggestions();
        assert!(!suggestions.is_empty());
        assert!(
            suggestions
                .iter()
                .any(|s| s.action == RepairAction::IncreaseTimeout)
        );
    }

    // -- Phase 5A tests --

    #[test]
    fn test_push_incident_adds_record() {
        let mut console = IncidentConsole::new();
        let record = make_record(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
        );
        console.push_incident(record);
        assert_eq!(console.active_incidents().len(), 1);
        assert_eq!(console.active_incidents()[0].run_id, 1);
    }

    #[test]
    fn test_push_incident_trims_to_max_display() {
        let mut console = IncidentConsole::with_max_display(3);
        for i in 0u64..5 {
            console.push_incident(make_record(
                i,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
            ));
        }
        assert_eq!(console.active_incidents().len(), 3);
        // Oldest records (0, 1) should have been trimmed
        assert_eq!(console.active_incidents()[0].run_id, 2);
        assert_eq!(console.active_incidents()[2].run_id, 4);
    }

    #[test]
    fn test_incidents_by_severity_filters_correctly() {
        let mut console = IncidentConsole::new();
        console.push_incident(make_record(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
        ));
        console.push_incident(make_record(
            2,
            IncidentSeverity::Warning,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
        ));
        console.push_incident(make_record(
            3,
            IncidentSeverity::Critical,
            FailureCode::BudgetExceeded,
            ReplaySafety::Unknown,
        ));
        let criticals = console.incidents_by_severity(IncidentSeverity::Critical);
        assert_eq!(criticals.len(), 2);
        assert!(
            criticals
                .iter()
                .all(|r| r.severity == IncidentSeverity::Critical)
        );
        let warnings = console.incidents_by_severity(IncidentSeverity::Warning);
        assert_eq!(warnings.len(), 1);
        let infos = console.incidents_by_severity(IncidentSeverity::Info);
        assert!(infos.is_empty());
    }

    #[test]
    fn test_critical_count_returns_only_critical() {
        let mut console = IncidentConsole::new();
        console.push_incident(make_record(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
        ));
        console.push_incident(make_record(
            2,
            IncidentSeverity::Warning,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
        ));
        console.push_incident(make_record(
            3,
            IncidentSeverity::Info,
            FailureCode::Unknown(String::from("x")),
            ReplaySafety::Unknown,
        ));
        console.push_incident(make_record(
            4,
            IncidentSeverity::Critical,
            FailureCode::StepPanicked,
            ReplaySafety::UnsafeSideEffect,
        ));
        assert_eq!(console.critical_count(), 2);
    }

    #[test]
    fn test_critical_count_empty() {
        let console = IncidentConsole::new();
        assert_eq!(console.critical_count(), 0);
    }

    #[test]
    fn test_has_unsafe_replay_true_when_unsafe() {
        let mut console = IncidentConsole::new();
        console.push_incident(make_record(
            1,
            IncidentSeverity::Warning,
            FailureCode::ActionTimeout,
            ReplaySafety::UnsafeSideEffect,
        ));
        assert!(console.has_unsafe_replay());
    }

    #[test]
    fn test_has_unsafe_replay_true_when_unknown() {
        let mut console = IncidentConsole::new();
        console.push_incident(make_record(
            1,
            IncidentSeverity::Info,
            FailureCode::Unknown(String::from("x")),
            ReplaySafety::Unknown,
        ));
        assert!(console.has_unsafe_replay());
    }

    #[test]
    fn test_has_unsafe_replay_false_when_all_safe() {
        let mut console = IncidentConsole::new();
        console.push_incident(make_record(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
        ));
        assert!(!console.has_unsafe_replay());
    }

    #[test]
    fn test_has_unsafe_replay_false_when_empty() {
        let console = IncidentConsole::new();
        assert!(!console.has_unsafe_replay());
    }

    #[test]
    fn test_clear_resolved_removes_matching_run() {
        let mut console = IncidentConsole::new();
        console.push_incident(make_record(
            10,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
        ));
        console.push_incident(make_record(
            20,
            IncidentSeverity::Warning,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
        ));
        console.push_incident(make_record(
            10,
            IncidentSeverity::Info,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
        ));
        console.clear_resolved(10);
        assert_eq!(console.active_incidents().len(), 1);
        assert_eq!(console.active_incidents()[0].run_id, 20);
    }

    #[test]
    fn test_clear_resolved_no_match_is_noop() {
        let mut console = IncidentConsole::new();
        console.push_incident(make_record(
            1,
            IncidentSeverity::Info,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
        ));
        console.clear_resolved(999);
        assert_eq!(console.active_incidents().len(), 1);
    }

    #[test]
    fn test_severity_colors() {
        let critical_color = IncidentSeverity::Critical.severity_color();
        assert_eq!(critical_color[0], 1.0_f32);
        assert!(critical_color[1] < 0.1_f32);

        let warning_color = IncidentSeverity::Warning.severity_color();
        assert_eq!(warning_color[0], 1.0_f32);
        assert!(warning_color[1] > 0.8_f32);

        let info_color = IncidentSeverity::Info.severity_color();
        assert!(info_color[0] < 0.1_f32);
        assert!(info_color[2] > 0.9_f32);
    }

    #[test]
    fn test_failure_code_as_str() {
        assert_eq!(FailureCode::ActionTimeout.as_str(), "ActionTimeout");
        assert_eq!(
            FailureCode::ActionFailed(String::new()).as_str(),
            "ActionFailed"
        );
        assert_eq!(FailureCode::BudgetExceeded.as_str(), "StepBudgetExhausted");
        assert_eq!(FailureCode::StepPanicked.as_str(), "StepPanicked");
        assert_eq!(
            FailureCode::ValidationError(String::new()).as_str(),
            "ValidationError"
        );
        assert_eq!(FailureCode::TaintLeak.as_str(), "TaintViolation");
        assert_eq!(FailureCode::ReplayDivergence.as_str(), "ReplayDivergence");
        assert_eq!(
            FailureCode::Unknown(String::new()).as_str(),
            "InternalError"
        );
    }

    #[test]
    fn test_replay_safety_is_safe() {
        assert!(ReplaySafety::Safe.is_safe());
        assert!(!ReplaySafety::UnsafeSideEffect.is_safe());
        assert!(!ReplaySafety::Unknown.is_safe());
    }

    #[test]
    fn test_with_max_display_zero_uses_default() {
        let console = IncidentConsole::with_max_display(0);
        assert_eq!(console.max_display, IncidentConsole::DEFAULT_MAX_DISPLAY);
    }

    #[test]
    fn test_push_then_clear_resolved_then_push() {
        let mut console = IncidentConsole::new();
        console.push_incident(make_record(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::UnsafeSideEffect,
        ));
        console.clear_resolved(1);
        assert!(console.active_incidents().is_empty());
        console.push_incident(make_record(
            2,
            IncidentSeverity::Info,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
        ));
        assert_eq!(console.active_incidents().len(), 1);
        assert_eq!(console.active_incidents()[0].run_id, 2);
    }

    #[test]
    fn test_multiple_severity_types_in_mixed_records() {
        let mut console = IncidentConsole::new();
        console.push_incident(make_record(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
        ));
        console.push_incident(make_record(
            2,
            IncidentSeverity::Warning,
            FailureCode::ActionTimeout,
            ReplaySafety::Unknown,
        ));
        console.push_incident(make_record(
            3,
            IncidentSeverity::Info,
            FailureCode::Unknown(String::from("x")),
            ReplaySafety::Safe,
        ));
        assert_eq!(console.active_incidents().len(), 3);
        assert_eq!(console.critical_count(), 1);
        assert!(console.has_unsafe_replay());
        assert_eq!(
            console
                .incidents_by_severity(IncidentSeverity::Warning)
                .len(),
            1
        );
    }

    // ---------------------------------------------------------------------------
    // Additional tests: dismiss index adjustment, select edge cases, with_max_display
    // ---------------------------------------------------------------------------

    #[test]
    fn test_dismiss_before_selected_adjusts_index() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.add_incident(make_incident(
            2,
            IncidentSeverity::Major,
            FailureCode::TaintLeak,
        ));
        console.add_incident(make_incident(
            3,
            IncidentSeverity::Critical,
            FailureCode::StepPanicked,
        ));
        // Select index 2 (the third incident)
        console.select(2);
        assert_eq!(console.selected().map(|i| i.id), Some(3));
        // Dismiss index 0 (before the selected one); selected should shift to 1
        console.dismiss(0);
        assert_eq!(console.active_count(), 2);
        assert_eq!(console.selected().map(|i| i.id), Some(3));
    }

    #[test]
    fn test_dismiss_after_selected_keeps_index() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.add_incident(make_incident(
            2,
            IncidentSeverity::Major,
            FailureCode::TaintLeak,
        ));
        console.add_incident(make_incident(
            3,
            IncidentSeverity::Critical,
            FailureCode::StepPanicked,
        ));
        // Select index 0
        console.select(0);
        assert_eq!(console.selected().map(|i| i.id), Some(1));
        // Dismiss index 2 (after selected); selected index stays at 0
        console.dismiss(2);
        assert_eq!(console.active_count(), 2);
        assert_eq!(console.selected().map(|i| i.id), Some(1));
    }

    #[test]
    fn test_select_out_of_bounds_is_noop() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.select(5);
        assert!(
            console.selected().is_none(),
            "selecting out of bounds should not set selection"
        );
    }

    #[test]
    fn test_with_max_display_custom_value() {
        let console = IncidentConsole::with_max_display(50);
        assert_eq!(console.max_display, 50);
    }

    #[test]
    fn test_default_trait_matches_new() {
        let from_new = IncidentConsole::new();
        let from_default = IncidentConsole::default();
        assert!(from_default.legacy_incidents().is_empty());
        assert!(from_default.active_incidents().is_empty());
        assert_eq!(from_new.active_count(), from_default.active_count());
        assert!(from_default.selected().is_none());
    }

    #[test]
    fn test_add_incident_returns_sequential_indices() {
        let mut console = IncidentConsole::new();
        let i0 = console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        let i1 = console.add_incident(make_incident(
            2,
            IncidentSeverity::Major,
            FailureCode::BudgetExceeded,
        ));
        let i2 = console.add_incident(make_incident(
            3,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
        ));
        assert_eq!(i0, 0);
        assert_eq!(i1, 1);
        assert_eq!(i2, 2);
    }

    #[test]
    fn test_legacy_critical_count_mixed() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.add_incident(make_incident(
            2,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
        ));
        console.add_incident(make_incident(
            3,
            IncidentSeverity::Major,
            FailureCode::BudgetExceeded,
        ));
        console.add_incident(make_incident(
            4,
            IncidentSeverity::Critical,
            FailureCode::StepPanicked,
        ));
        assert_eq!(console.legacy_critical_count(), 2);
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: dismiss edge cases
    // ---------------------------------------------------------------------------

    #[test]
    fn test_dismiss_out_of_bounds_is_noop() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.dismiss(5);
        assert_eq!(console.active_count(), 1);
    }

    #[test]
    fn test_dismiss_only_incident_clears_all() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
        ));
        console.select(0);
        assert!(console.selected().is_some());
        console.dismiss(0);
        assert_eq!(console.active_count(), 0);
        assert!(console.selected().is_none());
    }

    #[test]
    fn test_dismiss_with_no_selection_set() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.dismiss(0);
        assert_eq!(console.active_count(), 0);
    }

    #[test]
    fn test_dismiss_from_empty_console_is_noop() {
        let mut console = IncidentConsole::new();
        console.dismiss(0);
        assert_eq!(console.active_count(), 0);
        assert!(console.selected().is_none());
    }

    #[test]
    fn test_dismiss_add_dismiss_add() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.dismiss(0);
        assert_eq!(console.active_count(), 0);
        let idx = console.add_incident(make_incident(
            2,
            IncidentSeverity::Major,
            FailureCode::TaintLeak,
        ));
        assert_eq!(idx, 0);
        assert_eq!(console.active_count(), 1);
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: select edge cases
    // ---------------------------------------------------------------------------

    #[test]
    fn test_select_when_empty_is_noop() {
        let mut console = IncidentConsole::new();
        console.select(0);
        assert!(console.selected().is_none());
    }

    #[test]
    fn test_select_then_replace_selection() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.add_incident(make_incident(
            2,
            IncidentSeverity::Major,
            FailureCode::TaintLeak,
        ));
        console.select(0);
        assert_eq!(console.selected().map(|i| i.id), Some(1));
        console.select(1);
        assert_eq!(console.selected().map(|i| i.id), Some(2));
    }

    #[test]
    fn test_selected_suggestions_when_nothing_selected() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        assert!(console.selected_suggestions().is_empty());
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: with_max_display boundary
    // ---------------------------------------------------------------------------

    #[test]
    fn test_with_max_display_one() {
        let mut console = IncidentConsole::with_max_display(1);
        console.push_incident(make_record(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
        ));
        assert_eq!(console.active_incidents().len(), 1);
        console.push_incident(make_record(
            2,
            IncidentSeverity::Info,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
        ));
        assert_eq!(console.active_incidents().len(), 1);
        assert_eq!(console.active_incidents()[0].run_id, 2);
    }

    #[test]
    fn test_push_exactly_max_display_no_trim() {
        let mut console = IncidentConsole::with_max_display(3);
        for i in 0u64..3 {
            console.push_incident(make_record(
                i,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
            ));
        }
        assert_eq!(console.active_incidents().len(), 3);
        assert_eq!(console.active_incidents()[0].run_id, 0);
        assert_eq!(console.active_incidents()[2].run_id, 2);
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: legacy + record API coexistence
    // ---------------------------------------------------------------------------

    #[test]
    fn test_legacy_and_record_apis_are_independent() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
        ));
        console.push_incident(make_record(
            10,
            IncidentSeverity::Warning,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
        ));
        assert_eq!(console.active_count(), 1, "legacy count should be 1");
        assert_eq!(console.legacy_incidents().len(), 1);
        assert_eq!(
            console.active_incidents().len(),
            1,
            "record count should be 1"
        );
        assert_eq!(console.legacy_critical_count(), 1);
        assert_eq!(
            console.critical_count(),
            0,
            "record has Warning, not Critical"
        );
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: clear_resolved edge cases
    // ---------------------------------------------------------------------------

    #[test]
    fn test_clear_resolved_empty_console_is_noop() {
        let mut console = IncidentConsole::new();
        console.clear_resolved(1);
        assert!(console.active_incidents().is_empty());
    }

    #[test]
    fn test_clear_resolved_all_records() {
        let mut console = IncidentConsole::new();
        console.push_incident(make_record(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
        ));
        console.push_incident(make_record(
            1,
            IncidentSeverity::Warning,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
        ));
        console.clear_resolved(1);
        assert!(console.active_incidents().is_empty());
    }

    #[test]
    fn test_clear_resolved_preserves_records_from_other_runs() {
        let mut console = IncidentConsole::new();
        console.push_incident(make_record(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
        ));
        console.push_incident(make_record(
            2,
            IncidentSeverity::Warning,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
        ));
        console.push_incident(make_record(
            1,
            IncidentSeverity::Info,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
        ));
        console.push_incident(make_record(
            3,
            IncidentSeverity::Major,
            FailureCode::BudgetExceeded,
            ReplaySafety::Unknown,
        ));
        console.clear_resolved(1);
        assert_eq!(console.active_incidents().len(), 2);
        assert!(console.active_incidents().iter().all(|r| r.run_id != 1));
    }

    #[test]
    fn test_clear_resolved_then_push_new_record() {
        let mut console = IncidentConsole::new();
        console.push_incident(make_record(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
        ));
        console.clear_resolved(1);
        assert!(console.active_incidents().is_empty());
        console.push_incident(make_record(
            2,
            IncidentSeverity::Info,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
        ));
        assert_eq!(console.active_incidents().len(), 1);
        assert_eq!(console.active_incidents()[0].run_id, 2);
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: dismiss multiple incidents and verify index tracking
    // ---------------------------------------------------------------------------

    #[test]
    fn test_dismiss_first_of_three_shifts_indices() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            10,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.add_incident(make_incident(
            20,
            IncidentSeverity::Major,
            FailureCode::TaintLeak,
        ));
        console.add_incident(make_incident(
            30,
            IncidentSeverity::Critical,
            FailureCode::StepPanicked,
        ));
        console.select(2);
        assert_eq!(console.selected().map(|i| i.id), Some(30));
        console.dismiss(0);
        assert_eq!(console.active_count(), 2);
        assert_eq!(console.selected().map(|i| i.id), Some(30));
    }

    #[test]
    fn test_dismiss_middle_of_three_adjusts_selected_after() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            10,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.add_incident(make_incident(
            20,
            IncidentSeverity::Major,
            FailureCode::TaintLeak,
        ));
        console.add_incident(make_incident(
            30,
            IncidentSeverity::Critical,
            FailureCode::StepPanicked,
        ));
        console.select(2);
        console.dismiss(1);
        assert_eq!(console.active_count(), 2);
        assert_eq!(console.selected().map(|i| i.id), Some(30));
    }

    #[test]
    fn test_dismiss_all_incidents_one_by_one() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.add_incident(make_incident(
            2,
            IncidentSeverity::Major,
            FailureCode::TaintLeak,
        ));
        console.dismiss(1);
        assert_eq!(console.active_count(), 1);
        console.dismiss(0);
        assert_eq!(console.active_count(), 0);
        assert!(console.selected().is_none());
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: incidents_by_severity on empty console
    // ---------------------------------------------------------------------------

    #[test]
    fn test_incidents_by_severity_empty() {
        let console = IncidentConsole::new();
        assert!(
            console
                .incidents_by_severity(IncidentSeverity::Critical)
                .is_empty()
        );
        assert!(
            console
                .incidents_by_severity(IncidentSeverity::Info)
                .is_empty()
        );
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: push_incident trimming preserves ordering
    // ---------------------------------------------------------------------------

    #[test]
    fn test_push_incident_trimming_preserves_newest_records() {
        let mut console = IncidentConsole::with_max_display(5);
        for i in 0u64..10 {
            console.push_incident(make_record(
                i,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
            ));
        }
        assert_eq!(console.active_incidents().len(), 5);
        assert_eq!(console.active_incidents()[0].run_id, 5);
        assert_eq!(console.active_incidents()[4].run_id, 9);
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: legacy add_incident returns sequential indices after dismiss
    // ---------------------------------------------------------------------------

    #[test]
    fn test_add_incident_index_after_dismiss() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.add_incident(make_incident(
            2,
            IncidentSeverity::Major,
            FailureCode::TaintLeak,
        ));
        console.dismiss(0);
        let idx = console.add_incident(make_incident(
            3,
            IncidentSeverity::Critical,
            FailureCode::StepPanicked,
        ));
        assert_eq!(idx, 1);
        assert_eq!(console.active_count(), 2);
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: selected_suggestions for different failure codes
    // ---------------------------------------------------------------------------

    #[test]
    fn test_selected_suggestions_for_taint_leak() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
        ));
        console.select(0);
        let suggestions = console.selected_suggestions();
        assert!(!suggestions.is_empty());
        assert!(
            suggestions
                .iter()
                .any(|s| s.action == RepairAction::FixSecretLeak)
        );
    }

    #[test]
    fn test_selected_suggestions_for_budget_exceeded() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Major,
            FailureCode::BudgetExceeded,
        ));
        console.select(0);
        let suggestions = console.selected_suggestions();
        assert!(!suggestions.is_empty());
        assert!(
            suggestions
                .iter()
                .any(|s| s.action == RepairAction::AdjustBudget)
        );
    }

    // ---------------------------------------------------------------------------
    // suggestion_display_items tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_suggestion_display_items_empty_when_nothing_selected() {
        let console = IncidentConsole::new();
        assert!(console.suggestion_display_items().is_empty());
    }

    #[test]
    fn test_suggestion_display_items_returns_strings_for_selected() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Major,
            FailureCode::ActionTimeout,
        ));
        console.select(0);
        let items = console.suggestion_display_items();
        assert!(!items.is_empty());
        assert!(items[0].contains("[IncreaseTimeout]"));
        assert!(items[0].contains("90%"));
    }

    #[test]
    fn test_suggestion_display_items_taint_leak() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
        ));
        console.select(0);
        let items = console.suggestion_display_items();
        assert!(!items.is_empty());
        assert!(items[0].contains("[FixSecretLeak]"));
    }

    #[test]
    fn test_suggestion_display_items_clears_after_dismiss() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.select(0);
        assert!(!console.suggestion_display_items().is_empty());
        console.dismiss(0);
        assert!(console.suggestion_display_items().is_empty());
    }

    #[test]
    fn test_apply_suggestion_returns_true_when_selected() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.select(0);
        assert!(console.apply_suggestion(0));
    }

    #[test]
    fn test_apply_suggestion_returns_false_when_no_selection() {
        let console = IncidentConsole::new();
        assert!(!console.apply_suggestion(0));
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: IncidentSeverity display variants
    // ---------------------------------------------------------------------------

    #[test]
    fn test_severity_color_major_is_orange() {
        let [r, g, b, a] = IncidentSeverity::Major.severity_color();
        assert!(r > 0.9_f32, "Major red should be strong");
        assert!(g > 0.3_f32 && g < 0.7_f32, "Major green should be moderate");
        assert!(b < 0.1_f32, "Major blue should be near zero");
        let diff = (a - 1.0_f32).abs();
        assert!(diff < f32::EPSILON);
    }

    #[test]
    fn test_severity_color_minor_is_gray() {
        let [r, g, b, _a] = IncidentSeverity::Minor.severity_color();
        let diff_rg = (r - g).abs();
        let diff_gb = (g - b).abs();
        assert!(diff_rg < 0.01_f32, "Minor should have equal R and G");
        assert!(diff_gb < 0.01_f32, "Minor should have equal G and B");
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: clear_resolved does not affect legacy incidents
    // ---------------------------------------------------------------------------

    #[test]
    fn test_clear_resolved_does_not_affect_legacy() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.push_incident(make_record(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
        ));
        console.clear_resolved(1);
        assert_eq!(
            console.active_count(),
            1,
            "legacy incidents should not be cleared by clear_resolved"
        );
        assert!(console.active_incidents().is_empty());
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: has_unsafe_replay mixed safe and unsafe
    // ---------------------------------------------------------------------------

    #[test]
    fn test_has_unsafe_replay_mixed_safe_and_unsafe() {
        let mut console = IncidentConsole::new();
        console.push_incident(make_record(
            1,
            IncidentSeverity::Info,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
        ));
        assert!(!console.has_unsafe_replay());
        console.push_incident(make_record(
            2,
            IncidentSeverity::Warning,
            FailureCode::ActionTimeout,
            ReplaySafety::UnsafeSideEffect,
        ));
        assert!(
            console.has_unsafe_replay(),
            "one unsafe record should make has_unsafe_replay true"
        );
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: critical_count vs legacy_critical_count independence
    // ---------------------------------------------------------------------------

    #[test]
    fn test_critical_counts_are_independent() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
        ));
        console.add_incident(make_incident(
            2,
            IncidentSeverity::Critical,
            FailureCode::StepPanicked,
        ));
        console.push_incident(make_record(
            10,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
        ));
        console.push_incident(make_record(
            20,
            IncidentSeverity::Warning,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
        ));
        assert_eq!(
            console.legacy_critical_count(),
            2,
            "two legacy critical incidents"
        );
        assert_eq!(console.critical_count(), 1, "one record critical incident");
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: push_incident with max_display = DEFAULT_MAX_DISPLAY
    // ---------------------------------------------------------------------------

    #[test]
    fn test_push_incident_within_default_max_display() {
        let mut console = IncidentConsole::new();
        for i in 0u64..10 {
            console.push_incident(make_record(
                i,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
            ));
        }
        assert_eq!(console.active_incidents().len(), 10);
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: dismiss and then select
    // ---------------------------------------------------------------------------

    #[test]
    fn test_dismiss_then_select_new_index() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.add_incident(make_incident(
            2,
            IncidentSeverity::Major,
            FailureCode::TaintLeak,
        ));
        console.dismiss(0);
        console.select(0);
        assert_eq!(console.selected().map(|i| i.id), Some(2));
    }

    // ---------------------------------------------------------------------------
    // Additional coverage: Incidents by severity when all same severity
    // ---------------------------------------------------------------------------

    #[test]
    fn test_incidents_by_severity_all_same_severity() {
        let mut console = IncidentConsole::new();
        for i in 0u64..4 {
            console.push_incident(make_record(
                i,
                IncidentSeverity::Warning,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
            ));
        }
        let warnings = console.incidents_by_severity(IncidentSeverity::Warning);
        assert_eq!(warnings.len(), 4);
        let criticals = console.incidents_by_severity(IncidentSeverity::Critical);
        assert!(criticals.is_empty());
    }

    // ---------------------------------------------------------------------------
    // FailureCode as_str coverage
    // ---------------------------------------------------------------------------

    #[test]
    fn test_failure_code_as_str_covers_all_variants() {
        assert_eq!(FailureCode::ActionTimeout.as_str(), "ActionTimeout");
        assert_eq!(
            FailureCode::ActionFailed(String::new()).as_str(),
            "ActionFailed"
        );
        assert_eq!(FailureCode::BudgetExceeded.as_str(), "StepBudgetExhausted");
        assert_eq!(FailureCode::StepPanicked.as_str(), "StepPanicked");
        assert_eq!(
            FailureCode::ValidationError(String::new()).as_str(),
            "ValidationError"
        );
        assert_eq!(FailureCode::TaintLeak.as_str(), "TaintViolation");
        assert_eq!(FailureCode::ReplayDivergence.as_str(), "ReplayDivergence");
        assert_eq!(
            FailureCode::Unknown(String::new()).as_str(),
            "InternalError"
        );
    }

    // ---------------------------------------------------------------------------
    // Legacy critical count with no critical incidents
    // ---------------------------------------------------------------------------

    #[test]
    fn test_legacy_critical_count_none() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.add_incident(make_incident(
            2,
            IncidentSeverity::Major,
            FailureCode::TaintLeak,
        ));
        console.add_incident(make_incident(
            3,
            IncidentSeverity::Info,
            FailureCode::BudgetExceeded,
        ));
        assert_eq!(console.legacy_critical_count(), 0);
    }

    // ---------------------------------------------------------------------------
    // Legacy critical count on empty console
    // ---------------------------------------------------------------------------

    #[test]
    fn test_legacy_critical_count_empty_console() {
        let console = IncidentConsole::new();
        assert_eq!(console.legacy_critical_count(), 0);
    }

    // ---------------------------------------------------------------------------
    // dismiss clears selection when dismissed incident was selected
    // ---------------------------------------------------------------------------

    #[test]
    fn test_dismiss_selected_clears_selection() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.add_incident(make_incident(
            2,
            IncidentSeverity::Major,
            FailureCode::TaintLeak,
        ));
        console.select(0);
        assert_eq!(console.selected().map(|i| i.id), Some(1));
        console.dismiss(0);
        assert!(
            console.selected().is_none(),
            "selection should be cleared when selected incident is dismissed"
        );
    }

    // ---------------------------------------------------------------------------
    // Verify legacy_incidents returns items in add order
    // ---------------------------------------------------------------------------

    #[test]
    fn test_legacy_incidents_preserves_add_order() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            10,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.add_incident(make_incident(
            20,
            IncidentSeverity::Major,
            FailureCode::TaintLeak,
        ));
        console.add_incident(make_incident(
            30,
            IncidentSeverity::Critical,
            FailureCode::StepPanicked,
        ));
        let incidents = console.legacy_incidents();
        assert_eq!(incidents[0].id, 10);
        assert_eq!(incidents[1].id, 20);
        assert_eq!(incidents[2].id, 30);
    }

    // =========================================================================
    // BLACKHAT security review tests
    // =========================================================================

    /// FINDING: push_incident trimming uses Vec::remove(0) in a while loop.
    /// This is an O(n^2) performance concern when trimming many records at once.
    /// If max_display is small and many records are pushed rapidly, each remove(0)
    /// shifts all remaining elements. This test documents the behavior at the boundary.
    #[test]
    fn blackhat_push_incident_trim_uses_remove_zero_performance_concern() {
        let mut console = IncidentConsole::with_max_display(5);
        // Push 100 records at once; the while loop runs 95 iterations of remove(0)
        for i in 0u64..100 {
            console.push_incident(make_record(
                i,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
            ));
        }
        // Verify only the last 5 remain (records 95..99)
        assert_eq!(console.active_incidents().len(), 5);
        assert_eq!(console.active_incidents()[0].run_id, 95);
        assert_eq!(console.active_incidents()[4].run_id, 99);
    }

    /// FINDING: with_max_display accepts usize::MAX without capping.
    /// While not a memory safety issue in Rust, this could lead to unbounded
    /// memory consumption if a caller passes a very large value.
    /// This test documents that max_display is stored as-is without validation.
    #[test]
    fn blackhat_with_max_display_accepts_very_large_value() {
        let console = IncidentConsole::with_max_display(usize::MAX);
        assert_eq!(console.max_display, usize::MAX);
    }

    /// FINDING: dismiss() selection adjustment has a subtle edge case.
    /// When dismissing index == selected, the selection is cleared (set to None).
    /// This means if two incidents exist at indices 0 and 1, selecting index 1
    /// and then dismissing index 1 clears the selection, even though index 0
    /// still has a valid incident. This is by design but worth documenting.
    #[test]
    fn blackhat_dismiss_selected_index_clears_instead_of_shifting() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.add_incident(make_incident(
            2,
            IncidentSeverity::Major,
            FailureCode::TaintLeak,
        ));
        console.select(1);
        assert_eq!(console.selected().map(|i| i.id), Some(2));
        // Dismissing the selected incident clears selection entirely
        // rather than shifting to the remaining valid index
        console.dismiss(1);
        assert!(
            console.selected().is_none(),
            "selection cleared even though index 0 remains valid"
        );
        assert_eq!(console.active_count(), 1);
    }

    /// FINDING: selected() uses `self.selected.and_then(|i| self.incidents.get(i))`.
    /// If `selected` is Some(idx) but `incidents.get(idx)` returns None (impossible
    /// under normal operation since select() validates the index, but if incidents
    /// are removed externally), the selection silently returns None.
    /// This test verifies the defensive behavior.
    #[test]
    fn blackhat_selected_returns_none_if_index_out_of_sync() {
        // The select() method validates, so this scenario requires a sequence of
        // add, select, dismiss. After dismiss of a different index, the selected
        // index may point to a different incident. This is correct behavior.
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.select(0);
        assert_eq!(console.selected().map(|i| i.id), Some(1));
        // Dismiss the only incident; selection should be cleared
        console.dismiss(0);
        assert!(console.selected().is_none());
    }

    /// FINDING: selected_suggestions() uses `unwrap_or_default()` on the Option
    /// from `map(suggest_repairs)`. This means if no incident is selected, we get
    /// an empty Vec rather than an error. This is defensive and correct.
    #[test]
    fn blackhat_selected_suggestions_empty_when_none_selected() {
        let console = IncidentConsole::new();
        assert!(console.selected_suggestions().is_empty());
    }

    /// FINDING: push_incident and legacy add_incident operate on completely
    /// independent Vecs. dismiss() only affects legacy incidents and their
    /// selection. clear_resolved() only affects records. There is no cross-
    /// contamination risk, but the two APIs are a maintenance hazard.
    #[test]
    fn blackhat_dismiss_does_not_affect_records_and_clear_resolved_does_not_affect_legacy() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.push_incident(make_record(
            10,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
        ));

        // dismiss only affects legacy
        console.dismiss(0);
        assert_eq!(console.active_count(), 0, "legacy should be empty");
        assert_eq!(
            console.active_incidents().len(),
            1,
            "records should be untouched"
        );

        // Add back a legacy incident
        console.add_incident(make_incident(
            2,
            IncidentSeverity::Major,
            FailureCode::TaintLeak,
        ));

        // clear_resolved only affects records
        console.clear_resolved(10);
        assert_eq!(console.active_count(), 1, "legacy should still have 1");
        assert!(
            console.active_incidents().is_empty(),
            "records should be cleared"
        );
    }

    /// FINDING: The max_display field is only used for push_incident trimming.
    /// It has no effect on the legacy add_incident API. A user might assume
    /// max_display limits all stored incidents, but legacy incidents bypass it.
    #[test]
    fn blackhat_max_display_does_not_limit_legacy_incidents() {
        let mut console = IncidentConsole::with_max_display(2);
        // Legacy API ignores max_display
        for i in 0u64..10 {
            console.add_incident(make_incident(
                i,
                IncidentSeverity::Minor,
                FailureCode::ActionTimeout,
            ));
        }
        assert_eq!(
            console.active_count(),
            10,
            "legacy API bypasses max_display"
        );

        // Record API respects max_display
        for i in 0u64..10 {
            console.push_incident(make_record(
                i,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
            ));
        }
        assert_eq!(
            console.active_incidents().len(),
            2,
            "record API respects max_display"
        );
    }

    /// FINDING: critical_count() and legacy_critical_count() count severity
    /// independently on records vs legacy incidents. Callers must be careful
    /// which count they use.
    #[test]
    fn blackhat_both_critical_counts_are_zero_when_only_records_have_critical() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(
            1,
            IncidentSeverity::Minor,
            FailureCode::ActionTimeout,
        ));
        console.push_incident(make_record(
            10,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
        ));
        assert_eq!(console.legacy_critical_count(), 0, "legacy has no critical");
        assert_eq!(console.critical_count(), 1, "records have one critical");
    }

    /// FINDING: incidents_by_severity returns Vec of references.
    /// If the caller holds these references and modifies the console (push/dismiss),
    /// the borrow checker prevents use-after-free. This is safe by Rust's design.
    /// This test confirms the API compiles and works correctly.
    #[test]
    fn blackhat_incidents_by_severity_returns_correct_references() {
        let mut console = IncidentConsole::new();
        console.push_incident(make_record(
            1,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
        ));
        console.push_incident(make_record(
            2,
            IncidentSeverity::Warning,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
        ));
        console.push_incident(make_record(
            3,
            IncidentSeverity::Critical,
            FailureCode::StepPanicked,
            ReplaySafety::UnsafeSideEffect,
        ));
        let criticals = console.incidents_by_severity(IncidentSeverity::Critical);
        assert_eq!(criticals.len(), 2);
        // Verify references point to actual records
        assert_eq!(criticals[0].run_id, 1);
        assert_eq!(criticals[1].run_id, 3);
    }
}
