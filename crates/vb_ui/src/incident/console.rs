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

    pub fn selected_suggestions(&self) -> Vec<RepairSuggestion> {
        self.selected().map(suggest_repairs).unwrap_or_default()
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
        assert!(suggestions.iter().any(|s| s.action == RepairAction::IncreaseTimeout));
    }

    // -- Phase 5A tests --

    #[test]
    fn test_push_incident_adds_record() {
        let mut console = IncidentConsole::new();
        let record = make_record(1, IncidentSeverity::Critical, FailureCode::TaintLeak, ReplaySafety::Safe);
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
        assert!(criticals.iter().all(|r| r.severity == IncidentSeverity::Critical));
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
        assert_eq!(FailureCode::ActionFailed(String::new()).as_str(), "ActionFailed");
        assert_eq!(FailureCode::BudgetExceeded.as_str(), "StepBudgetExhausted");
        assert_eq!(FailureCode::StepPanicked.as_str(), "StepPanicked");
        assert_eq!(
            FailureCode::ValidationError(String::new()).as_str(),
            "ValidationError"
        );
        assert_eq!(FailureCode::TaintLeak.as_str(), "TaintViolation");
        assert_eq!(FailureCode::ReplayDivergence.as_str(), "ReplayDivergence");
        assert_eq!(FailureCode::Unknown(String::new()).as_str(), "InternalError");
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
        assert_eq!(console.incidents_by_severity(IncidentSeverity::Warning).len(), 1);
    }

    // ---------------------------------------------------------------------------
    // Additional tests: dismiss index adjustment, select edge cases, with_max_display
    // ---------------------------------------------------------------------------

    #[test]
    fn test_dismiss_before_selected_adjusts_index() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(1, IncidentSeverity::Minor, FailureCode::ActionTimeout));
        console.add_incident(make_incident(2, IncidentSeverity::Major, FailureCode::TaintLeak));
        console.add_incident(make_incident(3, IncidentSeverity::Critical, FailureCode::StepPanicked));
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
        console.add_incident(make_incident(1, IncidentSeverity::Minor, FailureCode::ActionTimeout));
        console.add_incident(make_incident(2, IncidentSeverity::Major, FailureCode::TaintLeak));
        console.add_incident(make_incident(3, IncidentSeverity::Critical, FailureCode::StepPanicked));
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
        console.add_incident(make_incident(1, IncidentSeverity::Minor, FailureCode::ActionTimeout));
        console.select(5);
        assert!(console.selected().is_none(), "selecting out of bounds should not set selection");
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
        let i0 = console.add_incident(make_incident(1, IncidentSeverity::Minor, FailureCode::ActionTimeout));
        let i1 = console.add_incident(make_incident(2, IncidentSeverity::Major, FailureCode::BudgetExceeded));
        let i2 = console.add_incident(make_incident(3, IncidentSeverity::Critical, FailureCode::TaintLeak));
        assert_eq!(i0, 0);
        assert_eq!(i1, 1);
        assert_eq!(i2, 2);
    }

    #[test]
    fn test_legacy_critical_count_mixed() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(1, IncidentSeverity::Minor, FailureCode::ActionTimeout));
        console.add_incident(make_incident(2, IncidentSeverity::Critical, FailureCode::TaintLeak));
        console.add_incident(make_incident(3, IncidentSeverity::Major, FailureCode::BudgetExceeded));
        console.add_incident(make_incident(4, IncidentSeverity::Critical, FailureCode::StepPanicked));
        assert_eq!(console.legacy_critical_count(), 2);
    }
}
