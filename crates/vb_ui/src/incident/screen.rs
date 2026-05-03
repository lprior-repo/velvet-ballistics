use std::time::Instant;

use super::console::IncidentConsole;
use super::repair::{suggest_repairs, RepairSuggestion};
use super::types::{
    FailureCode, FailureDetail, Incident, IncidentContext, IncidentSeverity, SideEffectCertainty,
    TimelineEntry, TimelineEventKind,
};

/// Screen orchestrator that wraps an [`IncidentConsole`] and provides
/// high-level operations for processing failures and querying incident data.
pub struct IncidentScreen {
    console: IncidentConsole,
    next_incident_id: u64,
}

impl Default for IncidentScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl IncidentScreen {
    /// Create a new, empty incident screen.
    pub fn new() -> Self {
        Self {
            console: IncidentConsole::new(),
            next_incident_id: 1,
        }
    }

    /// Allocate a unique incident ID and advance the counter.
    fn allocate_id(&mut self) -> u64 {
        let id = self.next_incident_id;
        self.next_incident_id = self.next_incident_id.saturating_add(1);
        id
    }

    /// Derive incident severity from a failure code.
    fn severity_for_code(code: &FailureCode) -> IncidentSeverity {
        match code {
            FailureCode::TaintLeak
            | FailureCode::StepPanicked
            | FailureCode::ReplayDivergence => IncidentSeverity::Critical,
            FailureCode::ActionTimeout
            | FailureCode::ActionFailed(_)
            | FailureCode::BudgetExceeded => IncidentSeverity::Major,
            FailureCode::ValidationError(_) | FailureCode::Unknown(_) => IncidentSeverity::Minor,
        }
    }

    /// Derive side-effect certainty from a failure code.
    fn certainty_for_code(code: &FailureCode) -> SideEffectCertainty {
        match code {
            FailureCode::ActionFailed(_) | FailureCode::StepPanicked => {
                SideEffectCertainty::Unknown
            }
            FailureCode::TaintLeak => SideEffectCertainty::Certain,
            _ => SideEffectCertainty::None,
        }
    }

    /// Determine whether a failure code is replay-safe.
    fn replay_safe_for_code(code: &FailureCode) -> bool {
        matches!(
            code,
            FailureCode::ActionTimeout
                | FailureCode::ValidationError(_)
                | FailureCode::BudgetExceeded
        )
    }

    /// Build the initial timeline for a newly-created incident.
    fn initial_timeline(
        code: &FailureCode,
        error_context: &str,
        timestamp: Instant,
    ) -> Vec<TimelineEntry> {
        vec![TimelineEntry {
            seq_no: 0,
            event_kind: TimelineEventKind::FailureObserved,
            timestamp,
            description: format!(
                "Failure observed: {} — {error_context}",
                format_failure_code(code)
            ),
        }]
    }

    /// Process a run failure and register it as an incident.
    ///
    /// Returns the index of the new incident within the console.
    pub fn process_run_failure(
        &mut self,
        run_id: u64,
        step: Option<&str>,
        error_code: FailureCode,
        error_context: &str,
    ) -> usize {
        let id = self.allocate_id();
        let now = Instant::now();
        let timeline = Self::initial_timeline(&error_code, error_context, now);

        let incident = Incident {
            id,
            severity: Self::severity_for_code(&error_code),
            failure_code: error_code.clone(),
            run_id,
            workflow_name: String::new(),
            step_id: None,
            step_name: step.map(String::from),
            error_message: String::from(error_context),
            replay_safe: Self::replay_safe_for_code(&error_code),
            side_effect_certainty: Self::certainty_for_code(&error_code),
            timestamp: now,
            context: IncidentContext {
                slot_values_before: Vec::new(),
                taint_changes: Vec::new(),
                action_attempts: 0,
                last_action_idempotency_key: None,
            },
            timeline,
        };
        self.console.add_incident(incident)
    }

    /// Process a replay divergence and register it as a critical incident.
    ///
    /// Returns the index of the new incident within the console.
    pub fn process_replay_divergence(
        &mut self,
        run_id: u64,
        expected: &str,
        actual: &str,
    ) -> usize {
        let id = self.allocate_id();
        let now = Instant::now();
        let description =
            format!("Replay divergence: expected {expected}, actual {actual}");

        let timeline = vec![
            TimelineEntry {
                seq_no: 0,
                event_kind: TimelineEventKind::FailureObserved,
                timestamp: now,
                description: description.clone(),
            },
            TimelineEntry {
                seq_no: 1,
                event_kind: TimelineEventKind::ReplayDivergence,
                timestamp: now,
                description: format!("Divergence detail — expected: {expected}, actual: {actual}"),
            },
        ];

        let incident = Incident {
            id,
            severity: IncidentSeverity::Critical,
            failure_code: FailureCode::ReplayDivergence,
            run_id,
            workflow_name: String::new(),
            step_id: None,
            step_name: None,
            error_message: description,
            replay_safe: false,
            side_effect_certainty: SideEffectCertainty::Unknown,
            timestamp: now,
            context: IncidentContext {
                slot_values_before: Vec::new(),
                taint_changes: Vec::new(),
                action_attempts: 0,
                last_action_idempotency_key: None,
            },
            timeline,
        };
        self.console.add_incident(incident)
    }

    /// Retrieve structured failure detail for the incident at the given index.
    pub fn get_failure_detail(&self, incident_id: usize) -> Option<FailureDetail> {
        let incidents = self.console.active_incidents();
        let incident = incidents.get(incident_id)?;
        Some(FailureDetail {
            error_code: incident.failure_code.clone(),
            step_name: incident.step_name.clone(),
            error_context: incident.context.clone(),
            replay_safe: incident.replay_safe,
            side_effect_certainty: incident.side_effect_certainty,
            timeline_events: incident.timeline.clone(),
        })
    }

    /// Get repair suggestions for the incident at the given index.
    pub fn repair_suggestions(&self, incident_id: usize) -> Vec<RepairSuggestion> {
        let incidents = self.console.active_incidents();
        match incidents.get(incident_id) {
            Some(incident) => suggest_repairs(incident),
            None => Vec::new(),
        }
    }

    /// Delegate: select an incident by index.
    pub fn select(&mut self, index: usize) {
        self.console.select(index);
    }

    /// Delegate: dismiss an incident by index.
    pub fn dismiss(&mut self, index: usize) {
        self.console.dismiss(index);
    }

    /// Delegate: return the number of active incidents.
    pub fn active_count(&self) -> usize {
        self.console.active_count()
    }

    /// Delegate: return the number of critical incidents.
    pub fn critical_count(&self) -> usize {
        self.console.critical_count()
    }

    /// Delegate: return the currently selected incident, if any.
    pub fn selected(&self) -> Option<&Incident> {
        self.console.selected()
    }

    /// Delegate: return repair suggestions for the currently selected incident.
    pub fn selected_suggestions(&self) -> Vec<RepairSuggestion> {
        self.console.selected_suggestions()
    }
}

/// Helper to format a [`FailureCode`] for display in timeline descriptions.
fn format_failure_code(code: &FailureCode) -> String {
    match code {
        FailureCode::ActionTimeout => String::from("ActionTimeout"),
        FailureCode::ActionFailed(msg) => format!("ActionFailed({msg})"),
        FailureCode::BudgetExceeded => String::from("BudgetExceeded"),
        FailureCode::StepPanicked => String::from("StepPanicked"),
        FailureCode::ValidationError(msg) => format!("ValidationError({msg})"),
        FailureCode::TaintLeak => String::from("TaintLeak"),
        FailureCode::ReplayDivergence => String::from("ReplayDivergence"),
        FailureCode::Unknown(msg) => format!("Unknown({msg})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::incident::repair::RepairAction;

    // ---------------------------------------------------------------
    // process_run_failure
    // ---------------------------------------------------------------

    #[test]
    fn test_process_run_failure_creates_incident() {
        let mut screen = IncidentScreen::new();
        let idx = screen.process_run_failure(
            42,
            Some("step-fetch"),
            FailureCode::ActionTimeout,
            "timed out after 30s",
        );
        assert_eq!(idx, 0);
        assert_eq!(screen.active_count(), 1);

        let detail = screen.get_failure_detail(0);
        assert!(detail.is_some());
        let detail = detail.unwrap();
        assert_eq!(detail.error_code, FailureCode::ActionTimeout);
        assert_eq!(detail.step_name.as_deref(), Some("step-fetch"));
        assert!(detail.replay_safe);
        assert_eq!(detail.side_effect_certainty, SideEffectCertainty::None);
        assert_eq!(detail.timeline_events.len(), 1);
        assert_eq!(
            detail.timeline_events[0].event_kind,
            TimelineEventKind::FailureObserved
        );
    }

    #[test]
    fn test_process_run_failure_assigns_incrementing_ids() {
        let mut screen = IncidentScreen::new();
        let i1 = screen.process_run_failure(1, None, FailureCode::ActionTimeout, "a");
        let i2 = screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        assert_eq!(i1, 0);
        assert_eq!(i2, 1);

        let d1 = screen.get_failure_detail(0).unwrap();
        let d2 = screen.get_failure_detail(1).unwrap();
        assert_ne!(d1.error_code, d2.error_code);
    }

    #[test]
    fn test_process_run_failure_taint_leak_is_critical() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(10, None, FailureCode::TaintLeak, "secret leaked");
        assert_eq!(screen.critical_count(), 1);
    }

    #[test]
    fn test_process_run_failure_step_panicked_is_critical() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(11, None, FailureCode::StepPanicked, "panic");
        assert_eq!(screen.critical_count(), 1);
    }

    #[test]
    fn test_process_run_failure_action_failed_is_unknown_certainty() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(12, None, FailureCode::ActionFailed("db".into()), "db error");
        let detail = screen.get_failure_detail(0).unwrap();
        assert_eq!(detail.side_effect_certainty, SideEffectCertainty::Unknown);
    }

    #[test]
    fn test_process_run_failure_validation_error_is_minor() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(
            13,
            None,
            FailureCode::ValidationError("bad input".into()),
            "bad input",
        );
        assert_eq!(screen.critical_count(), 0);
        let detail = screen.get_failure_detail(0).unwrap();
        assert_eq!(
            detail.side_effect_certainty,
            SideEffectCertainty::None
        );
    }

    #[test]
    fn test_process_run_failure_no_step() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(99, None, FailureCode::ActionTimeout, "timeout");
        let detail = screen.get_failure_detail(0).unwrap();
        assert!(detail.step_name.is_none());
    }

    // ---------------------------------------------------------------
    // process_replay_divergence
    // ---------------------------------------------------------------

    #[test]
    fn test_process_replay_divergence_creates_critical_incident() {
        let mut screen = IncidentScreen::new();
        let idx = screen.process_replay_divergence(100, "value-A", "value-B");
        assert_eq!(idx, 0);
        assert_eq!(screen.active_count(), 1);
        assert_eq!(screen.critical_count(), 1);
    }

    #[test]
    fn test_process_replay_divergence_has_two_timeline_entries() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(100, "value-A", "value-B");
        let detail = screen.get_failure_detail(0).unwrap();
        assert_eq!(detail.timeline_events.len(), 2);
        assert_eq!(
            detail.timeline_events[0].event_kind,
            TimelineEventKind::FailureObserved
        );
        assert_eq!(
            detail.timeline_events[1].event_kind,
            TimelineEventKind::ReplayDivergence
        );
    }

    #[test]
    fn test_process_replay_divergence_not_replay_safe() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(100, "a", "b");
        let detail = screen.get_failure_detail(0).unwrap();
        assert!(!detail.replay_safe);
        assert_eq!(
            detail.side_effect_certainty,
            SideEffectCertainty::Unknown
        );
    }

    #[test]
    fn test_process_replay_divergence_error_message() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(50, "expected-val", "actual-val");
        let selected = screen.selected();
        assert!(selected.is_none());
        screen.select(0);
        let selected = screen.selected().unwrap();
        assert!(selected.error_message.contains("expected-val"));
        assert!(selected.error_message.contains("actual-val"));
    }

    // ---------------------------------------------------------------
    // get_failure_detail
    // ---------------------------------------------------------------

    #[test]
    fn test_get_failure_detail_returns_none_for_missing() {
        let screen = IncidentScreen::new();
        assert!(screen.get_failure_detail(0).is_none());
    }

    #[test]
    fn test_get_failure_detail_returns_context() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, Some("step-1"), FailureCode::BudgetExceeded, "over");
        let detail = screen.get_failure_detail(0).unwrap();
        assert_eq!(detail.error_code, FailureCode::BudgetExceeded);
        assert_eq!(detail.step_name.as_deref(), Some("step-1"));
        assert!(detail.replay_safe);
    }

    // ---------------------------------------------------------------
    // repair_suggestions
    // ---------------------------------------------------------------

    #[test]
    fn test_repair_suggestions_for_timeout() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        let suggestions = screen.repair_suggestions(0);
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.action == RepairAction::IncreaseTimeout));
    }

    #[test]
    fn test_repair_suggestions_for_missing_incident() {
        let screen = IncidentScreen::new();
        let suggestions = screen.repair_suggestions(999);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_repair_suggestions_for_taint_leak() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        let suggestions = screen.repair_suggestions(0);
        assert!(suggestions.iter().any(|s| s.action == RepairAction::FixSecretLeak));
    }

    // ---------------------------------------------------------------
    // delegate methods
    // ---------------------------------------------------------------

    #[test]
    fn test_select_and_selected() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");

        assert!(screen.selected().is_none());
        screen.select(1);
        assert!(screen.selected().is_some());
        assert_eq!(screen.selected().unwrap().run_id, 2);

        // Out-of-bounds selection is a no-op.
        screen.select(99);
        // Selection should remain unchanged (still index 1).
        assert_eq!(screen.selected().unwrap().run_id, 2);
    }

    #[test]
    fn test_dismiss_reduces_count() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        assert_eq!(screen.active_count(), 2);

        screen.dismiss(0);
        assert_eq!(screen.active_count(), 1);
    }

    #[test]
    fn test_selected_suggestions() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        assert!(screen.selected_suggestions().is_empty());

        screen.select(0);
        let suggestions = screen.selected_suggestions();
        assert!(!suggestions.is_empty());
    }

    // ---------------------------------------------------------------
    // format_failure_code
    // ---------------------------------------------------------------

    #[test]
    fn test_format_failure_code_variants() {
        assert_eq!(
            format_failure_code(&FailureCode::ActionTimeout),
            "ActionTimeout"
        );
        assert_eq!(
            format_failure_code(&FailureCode::ActionFailed(String::from("db"))),
            "ActionFailed(db)"
        );
        assert_eq!(
            format_failure_code(&FailureCode::BudgetExceeded),
            "BudgetExceeded"
        );
        assert_eq!(
            format_failure_code(&FailureCode::StepPanicked),
            "StepPanicked"
        );
        assert_eq!(
            format_failure_code(&FailureCode::ValidationError(String::from("bad"))),
            "ValidationError(bad)"
        );
        assert_eq!(format_failure_code(&FailureCode::TaintLeak), "TaintLeak");
        assert_eq!(
            format_failure_code(&FailureCode::ReplayDivergence),
            "ReplayDivergence"
        );
        assert_eq!(
            format_failure_code(&FailureCode::Unknown(String::from("x"))),
            "Unknown(x)"
        );
    }
}
