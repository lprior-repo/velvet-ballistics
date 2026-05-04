use std::time::Instant;

use super::console::IncidentConsole;
use super::repair::{suggest_repairs, RepairSuggestion};
use super::types::{
    FailureCode, FailureDetail, Incident, IncidentCauseView, IncidentContext,
    IncidentDetailSections, IncidentSeverity, IncidentSlotDiff, IncidentTimelineEntry,
    IncidentType, SideEffectCertainty, TimelineEntry, TimelineEventKind,
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

    fn allocate_id(&mut self) -> u64 {
        let id = self.next_incident_id;
        self.next_incident_id = self.next_incident_id.saturating_add(1);
        id
    }

    fn instant_to_micros(instant: Instant) -> u64 {
        instant.elapsed().as_micros().try_into().unwrap_or(u64::MAX)
    }

    fn severity_for_code(code: &FailureCode) -> IncidentSeverity {
        match code {
            FailureCode::TaintLeak | FailureCode::StepPanicked | FailureCode::ReplayDivergence => IncidentSeverity::Critical,
            FailureCode::ActionTimeout | FailureCode::ActionFailed(_) | FailureCode::BudgetExceeded => IncidentSeverity::Major,
            FailureCode::ValidationError(_) | FailureCode::Unknown(_) => IncidentSeverity::Minor,
        }
    }

    fn incident_type_for_code(code: &FailureCode) -> IncidentType {
        match code {
            FailureCode::ActionTimeout | FailureCode::ActionFailed(_) | FailureCode::BudgetExceeded
            | FailureCode::StepPanicked | FailureCode::ValidationError(_) => IncidentType::ActionFailure,
            FailureCode::ReplayDivergence => IncidentType::ReplayDivergence,
            FailureCode::TaintLeak => IncidentType::SecretLeak,
            FailureCode::Unknown(_) => IncidentType::BlockedReconciliation,
        }
    }

    fn certainty_for_code(code: &FailureCode) -> SideEffectCertainty {
        match code {
            FailureCode::ActionFailed(_) | FailureCode::StepPanicked => SideEffectCertainty::Unknown,
            FailureCode::TaintLeak => SideEffectCertainty::Certain,
            _ => SideEffectCertainty::None,
        }
    }

    fn replay_safe_for_code(code: &FailureCode) -> bool {
        matches!(code, FailureCode::ActionTimeout | FailureCode::ValidationError(_) | FailureCode::BudgetExceeded)
    }

    fn initial_timeline(code: &FailureCode, error_context: &str, timestamp: Instant) -> Vec<TimelineEntry> {
        vec![TimelineEntry {
            seq: 0,
            description: format!("Failure observed: {} - {}", format_failure_code(code), error_context),
            timestamp_micros: Self::instant_to_micros(timestamp),
            event_kind: TimelineEventKind::FailureObserved,
            timestamp,
        }]
    }

    /// Process a [`vb_storage::JournalEvent`] and convert it into an [`Incident`]
    /// if it represents a failure (ActionFailedEvent or RunFailedEvent).
    pub fn process_failure(event: &vb_storage::JournalEvent) -> Option<Incident> {
        match event {
            vb_storage::JournalEvent::ActionFailedEvent { run, seq, step, action } => {
                let now = Instant::now();
                let failure_code = FailureCode::ActionFailed(format!(
                    "action {} failed in step {}", action.get(), step.get()
                ));
                let timeline = vec![TimelineEntry {
                    seq: 0,
                    description: format!("ActionFailed: action {} in step {} at seq {}", action.get(), step.get(), seq.get()),
                    timestamp_micros: Self::instant_to_micros(now),
                    event_kind: TimelineEventKind::FailureObserved,
                    timestamp: now,
                }];
                Some(Incident {
                    id: 0,
                    incident_type: IncidentType::ActionFailure,
                    severity: Self::severity_for_code(&failure_code),
                    failure_code,
                    run_id: run.get(),
                    workflow_name: String::new(),
                    step_id: Some(step.get()),
                    step_name: None,
                    error_message: format!("Action {} failed in step {} for run {}", action.get(), step.get(), run.get()),
                    replay_safe: false,
                    side_effect_certainty: SideEffectCertainty::Unknown,
                    timestamp: now,
                    context: IncidentContext {
                        slot_values_before: Vec::new(), taint_changes: Vec::new(),
                        action_attempts: 0, last_action_idempotency_key: None,
                    },
                    timeline,
                })
            }
            vb_storage::JournalEvent::RunFailedEvent { run, seq } => {
                let now = Instant::now();
                let failure_code = FailureCode::Unknown(format!("Run {} failed at seq {}", run.get(), seq.get()));
                let timeline = vec![TimelineEntry {
                    seq: 0,
                    description: format!("RunFailed: run {} at seq {}", run.get(), seq.get()),
                    timestamp_micros: Self::instant_to_micros(now),
                    event_kind: TimelineEventKind::FailureObserved,
                    timestamp: now,
                }];
                Some(Incident {
                    id: 0,
                    incident_type: IncidentType::BlockedReconciliation,
                    severity: IncidentSeverity::Critical,
                    failure_code,
                    run_id: run.get(),
                    workflow_name: String::new(),
                    step_id: None, step_name: None,
                    error_message: format!("Run {} failed", run.get()),
                    replay_safe: false,
                    side_effect_certainty: SideEffectCertainty::Unknown,
                    timestamp: now,
                    context: IncidentContext {
                        slot_values_before: Vec::new(), taint_changes: Vec::new(),
                        action_attempts: 0, last_action_idempotency_key: None,
                    },
                    timeline,
                })
            }
            _ => None,
        }
    }

    /// Register an externally-created incident into this screen, assigning a unique ID.
    pub fn register_incident(&mut self, mut incident: Incident) -> usize {
        incident.id = self.allocate_id();
        self.console.add_incident(incident)
    }

    /// Process a run failure and register it as an incident.
    pub fn process_run_failure(&mut self, run_id: u64, step: Option<&str>, error_code: FailureCode, error_context: &str) -> usize {
        let id = self.allocate_id();
        let now = Instant::now();
        let timeline = Self::initial_timeline(&error_code, error_context, now);
        let incident = Incident {
            id, incident_type: Self::incident_type_for_code(&error_code),
            severity: Self::severity_for_code(&error_code), failure_code: error_code.clone(),
            run_id, workflow_name: String::new(), step_id: None,
            step_name: step.map(String::from), error_message: String::from(error_context),
            replay_safe: Self::replay_safe_for_code(&error_code),
            side_effect_certainty: Self::certainty_for_code(&error_code),
            timestamp: now,
            context: IncidentContext { slot_values_before: Vec::new(), taint_changes: Vec::new(), action_attempts: 0, last_action_idempotency_key: None },
            timeline,
        };
        self.console.add_incident(incident)
    }

    /// Process a replay divergence and register it as a critical incident.
    pub fn process_replay_divergence(&mut self, run_id: u64, expected: &str, actual: &str) -> usize {
        let id = self.allocate_id();
        let now = Instant::now();
        let description = format!("Replay divergence: expected {}, actual {}", expected, actual);
        let timeline = vec![
            TimelineEntry { seq: 0, description: description.clone(), timestamp_micros: Self::instant_to_micros(now), event_kind: TimelineEventKind::FailureObserved, timestamp: now },
            TimelineEntry { seq: 1, description: format!("Divergence detail - expected: {}, actual: {}", expected, actual), timestamp_micros: Self::instant_to_micros(now), event_kind: TimelineEventKind::ReplayDivergence, timestamp: now },
        ];
        let incident = Incident {
            id, incident_type: IncidentType::ReplayDivergence, severity: IncidentSeverity::Critical,
            failure_code: FailureCode::ReplayDivergence, run_id, workflow_name: String::new(),
            step_id: None, step_name: None, error_message: description, replay_safe: false,
            side_effect_certainty: SideEffectCertainty::Unknown, timestamp: now,
            context: IncidentContext { slot_values_before: Vec::new(), taint_changes: Vec::new(), action_attempts: 0, last_action_idempotency_key: None },
            timeline,
        };
        self.console.add_incident(incident)
    }

    /// Retrieve structured failure detail for the incident at the given index.
    pub fn get_failure_detail(&self, incident_index: usize) -> Option<FailureDetail> {
        let incidents = self.console.legacy_incidents();
        let incident = incidents.get(incident_index)?;
        Some(FailureDetail {
            error_code: format_failure_code(&incident.failure_code),
            step_id: incident.step_id, run_id: incident.run_id,
            workflow_name: incident.workflow_name.clone(), replay_safe: incident.replay_safe,
            timeline: incident.timeline.clone(), failure_code: incident.failure_code.clone(),
            step_name: incident.step_name.clone(), side_effect_certainty: incident.side_effect_certainty,
            error_context: incident.context.clone(),
        })
    }

    /// Get repair suggestions for the incident at the given index.
    pub fn repair_suggestions(&self, incident_index: usize) -> Vec<RepairSuggestion> {
        let incidents = self.console.legacy_incidents();
        match incidents.get(incident_index) {
            Some(incident) => suggest_repairs(incident),
            None => Vec::new(),
        }
    }

    /// Return the list of all active incidents.
    pub fn incidents(&self) -> &[Incident] { self.console.legacy_incidents() }

    /// Select an incident by index.
    pub fn select(&mut self, index: usize) { self.console.select(index); }

    /// Dismiss an incident by index.
    pub fn dismiss(&mut self, index: usize) { self.console.dismiss(index); }

    /// Return the number of active incidents.
    pub fn active_count(&self) -> usize { self.console.active_count() }

    /// Return the number of critical incidents.
    pub fn critical_count(&self) -> usize { self.console.legacy_critical_count() }

    /// Return the currently selected incident, if any.
    pub fn selected(&self) -> Option<&Incident> { self.console.selected() }

    /// Return repair suggestions for the currently selected incident.
    pub fn selected_suggestions(&self) -> Vec<RepairSuggestion> { self.console.selected_suggestions() }

    /// Return a human-readable summary of all incidents, e.g.
    /// "3 incidents: 1 Critical, 1 Error, 1 Warning".
    pub fn summary_text(&self) -> String {
        let incidents = self.console.legacy_incidents();
        let total = incidents.len();
        if total == 0 {
            return String::from("0 incidents");
        }
        let mut critical: usize = 0;
        let mut major: usize = 0;
        let mut minor: usize = 0;
        let mut warning: usize = 0;
        let mut info: usize = 0;
        for inc in incidents {
            match inc.severity {
                IncidentSeverity::Critical => critical = critical.saturating_add(1),
                IncidentSeverity::Major => major = major.saturating_add(1),
                IncidentSeverity::Minor => minor = minor.saturating_add(1),
                IncidentSeverity::Warning => warning = warning.saturating_add(1),
                IncidentSeverity::Info => info = info.saturating_add(1),
            }
        }
        let mut parts: Vec<String> = Vec::new();
        if critical > 0 {
            parts.push(format!("{} Critical", critical));
        }
        if major > 0 {
            parts.push(format!("{} Error", major));
        }
        if minor > 0 {
            parts.push(format!("{} Minor", minor));
        }
        if warning > 0 {
            parts.push(format!("{} Warning", warning));
        }
        if info > 0 {
            parts.push(format!("{} Info", info));
        }
        format!("{} incidents: {}", total, parts.join(", "))
    }

    /// Return true if any incident has Critical severity.
    pub fn has_critical(&self) -> bool {
        self.console.legacy_incidents()
            .iter()
            .any(|inc| inc.severity == IncidentSeverity::Critical)
    }

    /// Return references to all incidents matching the given severity.
    pub fn filter_by_severity(&self, severity: IncidentSeverity) -> Vec<&Incident> {
        self.console.legacy_incidents()
            .iter()
            .filter(|inc| inc.severity == severity)
            .collect()
    }

    // -----------------------------------------------------------------------
    // Click-handler and tab-switching methods
    // -----------------------------------------------------------------------

    /// Select an incident by index and return a reference to it.
    /// Returns `None` if the index is out of bounds.
    pub fn select_incident(&mut self, index: usize) -> Option<&Incident> {
        let incidents = self.console.legacy_incidents();
        if incidents.get(index).is_some() {
            self.console.select(index);
            return self.console.selected();
        }
        None
    }

    /// Return a reference to the currently selected incident, if any.
    pub fn selected_incident(&self) -> Option<&Incident> {
        self.console.selected()
    }

    /// Dismiss the currently selected incident and return true if something
    /// was actually dismissed. Clears the selection.
    pub fn dismiss_selected(&mut self) -> bool {
        let selected_index = self.console.selected_index();
        match selected_index {
            Some(idx) => {
                let had_incident = self.console.legacy_incidents().get(idx).is_some();
                if had_incident {
                    self.console.dismiss(idx);
                    return true;
                }
                false
            }
            None => false,
        }
    }

    /// Return the aggregated detail sections for the currently selected
    /// incident. If no incident is selected, all fields are empty/None.
    pub fn detail_sections(&self) -> IncidentDetailSections {
        let selected = match self.console.selected() {
            Some(inc) => inc,
            None => {
                return IncidentDetailSections {
                    cause: None,
                    timeline: Vec::new(),
                    state_diff: Vec::new(),
                    repair_suggestions: Vec::new(),
                    replay_safe: false,
                    side_effect_certainty: SideEffectCertainty::None,
                };
            }
        };

        let cause = IncidentCauseView {
            category: String::from(selected.failure_code.category()),
            failure_code: selected.failure_code.clone(),
            error_message: selected.error_message.clone(),
            severity: selected.severity,
            step_name: selected.step_name.clone(),
            run_id: selected.run_id,
        };

        let timeline: Vec<IncidentTimelineEntry> = selected
            .timeline
            .iter()
            .map(|entry| IncidentTimelineEntry {
                seq: entry.seq,
                description: entry.description.clone(),
                timestamp_micros: entry.timestamp_micros,
                event_kind: entry.event_kind,
            })
            .collect();

        let state_diff: Vec<IncidentSlotDiff> = selected
            .context
            .slot_values_before
            .iter()
            .map(|(slot_index, value_before)| {
                let matching_taint = selected
                    .context
                    .taint_changes
                    .iter()
                    .find(|(idx, _)| *idx == *slot_index);
                let value_after = matching_taint
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                let change_label = if value_before == &value_after {
                    String::from("unchanged")
                } else {
                    String::from("modified")
                };
                IncidentSlotDiff {
                    slot_index: *slot_index,
                    value_before: value_before.clone(),
                    value_after,
                    change_label,
                }
            })
            .collect();

        let repair_suggestions = suggest_repairs(selected);

        IncidentDetailSections {
            cause: Some(cause),
            timeline,
            state_diff,
            repair_suggestions,
            replay_safe: selected.replay_safe,
            side_effect_certainty: selected.side_effect_certainty,
        }
    }
}

fn format_failure_code(code: &FailureCode) -> String {
    match code {
        FailureCode::ActionTimeout => String::from("ActionTimeout"),
        FailureCode::ActionFailed(msg) => format!("ActionFailed({})", msg),
        FailureCode::BudgetExceeded => String::from("BudgetExceeded"),
        FailureCode::StepPanicked => String::from("StepPanicked"),
        FailureCode::ValidationError(msg) => format!("ValidationError({})", msg),
        FailureCode::TaintLeak => String::from("TaintLeak"),
        FailureCode::ReplayDivergence => String::from("ReplayDivergence"),
        FailureCode::Unknown(msg) => format!("Unknown({})", msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::incident::repair::{RepairAction, RepairKind};

    #[test]
    fn test_process_failure_action_failed_event() {
        use vb_core::{ActionId, RunId, StepIdx};
        use vb_storage::EventSeq;
        let event = vb_storage::JournalEvent::ActionFailedEvent { run: RunId::new(42), seq: EventSeq::new(5), step: StepIdx::new(3), action: ActionId::new(10) };
        let inc = IncidentScreen::process_failure(&event).unwrap();
        assert_eq!(inc.run_id, 42);
        assert_eq!(inc.step_id, Some(3));
        assert_eq!(inc.incident_type, IncidentType::ActionFailure);
        assert_eq!(inc.severity, IncidentSeverity::Major);
        assert!(!inc.replay_safe);
        assert_eq!(inc.side_effect_certainty, SideEffectCertainty::Unknown);
        assert_eq!(inc.timeline.len(), 1);
        assert_eq!(inc.timeline.first().map(|t| t.event_kind), Some(TimelineEventKind::FailureObserved));
        assert_eq!(inc.timeline.first().map(|t| t.seq), Some(0));
        assert!(inc.error_message.contains("Action 10"));
    }

    #[test]
    fn test_process_failure_run_failed_event() {
        use vb_core::RunId;
        use vb_storage::EventSeq;
        let event = vb_storage::JournalEvent::RunFailedEvent { run: RunId::new(99), seq: EventSeq::new(7) };
        let inc = IncidentScreen::process_failure(&event).unwrap();
        assert_eq!(inc.run_id, 99);
        assert!(inc.step_id.is_none());
        assert_eq!(inc.incident_type, IncidentType::BlockedReconciliation);
        assert_eq!(inc.severity, IncidentSeverity::Critical);
        assert!(!inc.replay_safe);
        assert_eq!(inc.timeline.len(), 1);
        assert!(inc.error_message.contains("99"));
    }

    #[test]
    fn test_process_failure_non_failure_event_returns_none() {
        use vb_core::{RunId, WorkflowDigest};
        use vb_storage::EventSeq;
        let event = vb_storage::JournalEvent::RunAccepted { run: RunId::new(1), seq: EventSeq::new(0), workflow: WorkflowDigest::from_bytes([0u8; 32]) };
        assert!(IncidentScreen::process_failure(&event).is_none());
    }

    #[test]
    fn test_process_failure_step_started_returns_none() {
        use vb_core::{RunId, StepIdx};
        use vb_storage::EventSeq;
        let event = vb_storage::JournalEvent::StepStarted { run: RunId::new(1), seq: EventSeq::new(1), step: StepIdx::new(0) };
        assert!(IncidentScreen::process_failure(&event).is_none());
    }

    #[test]
    fn test_process_failure_run_finished_returns_none() {
        use vb_core::{RunId, SlotIdx};
        use vb_storage::EventSeq;
        let event = vb_storage::JournalEvent::RunFinished { run: RunId::new(1), seq: EventSeq::new(10), result: SlotIdx::new(0) };
        assert!(IncidentScreen::process_failure(&event).is_none());
    }

    #[test]
    fn test_process_failure_action_completed_returns_none() {
        use vb_core::{ActionId, RunId, StepIdx};
        use vb_storage::EventSeq;
        let event = vb_storage::JournalEvent::ActionCompletedEvent { run: RunId::new(1), seq: EventSeq::new(2), step: StepIdx::new(0), action: ActionId::new(5) };
        assert!(IncidentScreen::process_failure(&event).is_none());
    }

    #[test]
    fn test_process_run_failure_creates_incident() {
        let mut screen = IncidentScreen::new();
        let idx = screen.process_run_failure(42, Some("step-fetch"), FailureCode::ActionTimeout, "timed out after 30s");
        assert_eq!(idx, 0);
        assert_eq!(screen.active_count(), 1);
        let detail = screen.get_failure_detail(0).unwrap();
        assert_eq!(detail.failure_code, FailureCode::ActionTimeout);
        assert_eq!(detail.step_name.as_deref(), Some("step-fetch"));
        assert!(detail.replay_safe);
        assert_eq!(detail.side_effect_certainty, SideEffectCertainty::None);
        assert_eq!(detail.timeline.len(), 1);
        assert_eq!(detail.timeline.first().map(|t| t.event_kind), Some(TimelineEventKind::FailureObserved));
        assert_eq!(detail.timeline.first().map(|t| t.seq), Some(0));
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
        assert_ne!(d1.failure_code, d2.failure_code);
    }

    #[test]
    fn test_process_run_failure_taint_leak_is_critical() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(10, None, FailureCode::TaintLeak, "secret leaked");
        assert_eq!(screen.critical_count(), 1);
        assert_eq!(screen.incidents().first().unwrap().incident_type, IncidentType::SecretLeak);
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
        assert_eq!(screen.get_failure_detail(0).unwrap().side_effect_certainty, SideEffectCertainty::Unknown);
    }

    #[test]
    fn test_process_run_failure_validation_error_is_minor() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(13, None, FailureCode::ValidationError("bad input".into()), "bad input");
        assert_eq!(screen.critical_count(), 0);
        assert_eq!(screen.get_failure_detail(0).unwrap().side_effect_certainty, SideEffectCertainty::None);
    }

    #[test]
    fn test_process_run_failure_no_step() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(99, None, FailureCode::ActionTimeout, "timeout");
        assert!(screen.get_failure_detail(0).unwrap().step_name.is_none());
    }

    #[test]
    fn test_process_replay_divergence_creates_critical_incident() {
        let mut screen = IncidentScreen::new();
        let idx = screen.process_replay_divergence(100, "value-A", "value-B");
        assert_eq!(idx, 0);
        assert_eq!(screen.active_count(), 1);
        assert_eq!(screen.critical_count(), 1);
        assert_eq!(screen.incidents().first().unwrap().incident_type, IncidentType::ReplayDivergence);
    }

    #[test]
    fn test_process_replay_divergence_has_two_timeline_entries() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(100, "value-A", "value-B");
        let detail = screen.get_failure_detail(0).unwrap();
        assert_eq!(detail.timeline.len(), 2);
        assert_eq!(detail.timeline.first().map(|t| t.event_kind), Some(TimelineEventKind::FailureObserved));
        assert_eq!(detail.timeline.get(1).map(|t| t.event_kind), Some(TimelineEventKind::ReplayDivergence));
        assert_eq!(detail.timeline.first().map(|t| t.seq), Some(0));
        assert_eq!(detail.timeline.get(1).map(|t| t.seq), Some(1));
    }

    #[test]
    fn test_process_replay_divergence_not_replay_safe() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(100, "a", "b");
        let detail = screen.get_failure_detail(0).unwrap();
        assert!(!detail.replay_safe);
        assert_eq!(detail.side_effect_certainty, SideEffectCertainty::Unknown);
    }

    #[test]
    fn test_process_replay_divergence_error_message() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(50, "expected-val", "actual-val");
        screen.select(0);
        let selected = screen.selected().unwrap();
        assert!(selected.error_message.contains("expected-val"));
        assert!(selected.error_message.contains("actual-val"));
    }

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
        assert_eq!(detail.failure_code, FailureCode::BudgetExceeded);
        assert_eq!(detail.step_name.as_deref(), Some("step-1"));
        assert!(detail.replay_safe);
        assert_eq!(detail.error_code, String::from("BudgetExceeded"));
        assert_eq!(detail.run_id, 1);
    }

    #[test]
    fn test_get_failure_detail_step_id_populated() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(5, Some("step-2"), FailureCode::ActionTimeout, "timeout");
        let detail = screen.get_failure_detail(0).unwrap();
        assert!(detail.step_id.is_none());
        assert_eq!(detail.step_name.as_deref(), Some("step-2"));
    }

    #[test]
    fn test_repair_suggestions_for_timeout() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        let suggestions = screen.repair_suggestions(0);
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.action == RepairAction::IncreaseTimeout));
        assert!(suggestions.iter().any(|s| s.kind == RepairKind::IncreaseTimeout));
    }

    #[test]
    fn test_repair_suggestions_for_missing_incident() {
        let screen = IncidentScreen::new();
        assert!(screen.repair_suggestions(999).is_empty());
    }

    #[test]
    fn test_repair_suggestions_for_taint_leak() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        let suggestions = screen.repair_suggestions(0);
        assert!(suggestions.iter().any(|s| s.action == RepairAction::FixSecretLeak));
        assert!(suggestions.iter().any(|s| s.kind == RepairKind::FixSecretLeak));
    }

    #[test]
    fn test_incidents_slice_empty() {
        let screen = IncidentScreen::new();
        assert!(screen.incidents().is_empty());
    }

    #[test]
    fn test_active_count_empty() {
        let screen = IncidentScreen::new();
        assert_eq!(screen.active_count(), 0);
    }

    #[test]
    fn test_incidents_after_multiple_failures() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t1");
        screen.process_run_failure(2, None, FailureCode::TaintLeak, "t2");
        screen.process_run_failure(3, None, FailureCode::BudgetExceeded, "t3");
        assert_eq!(screen.active_count(), 3);
        let ids: Vec<u64> = screen.incidents().iter().map(|i| i.run_id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn test_dismiss_reduces_count() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        screen.dismiss(0);
        assert_eq!(screen.active_count(), 1);
        assert_eq!(screen.incidents().first().unwrap().run_id, 2);
    }

    #[test]
    fn test_dismiss_out_of_bounds_is_noop() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.dismiss(5);
        assert_eq!(screen.active_count(), 1);
    }

    #[test]
    fn test_dismiss_all_incidents() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        screen.dismiss(1);
        screen.dismiss(0);
        assert_eq!(screen.active_count(), 0);
    }

    #[test]
    fn test_select_and_selected() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        assert!(screen.selected().is_none());
        screen.select(1);
        assert_eq!(screen.selected().unwrap().run_id, 2);
        screen.select(99);
        assert_eq!(screen.selected().unwrap().run_id, 2);
    }

    #[test]
    fn test_selected_suggestions() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        assert!(screen.selected_suggestions().is_empty());
        screen.select(0);
        assert!(!screen.selected_suggestions().is_empty());
    }

    #[test]
    fn test_timeline_entry_has_timestamp_micros() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        let detail = screen.get_failure_detail(0).unwrap();
        let _micros = detail.timeline.first().unwrap().timestamp_micros;
    }

    #[test]
    fn test_timeline_replay_divergence_seq_numbers() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(10, "x", "y");
        let detail = screen.get_failure_detail(0).unwrap();
        assert_eq!(detail.timeline.len(), 2);
        assert_eq!(detail.timeline.first().map(|t| t.seq), Some(0));
        assert_eq!(detail.timeline.get(1).map(|t| t.seq), Some(1));
    }

    #[test]
    fn test_format_failure_code_variants() {
        assert_eq!(format_failure_code(&FailureCode::ActionTimeout), "ActionTimeout");
        assert_eq!(format_failure_code(&FailureCode::ActionFailed(String::from("db"))), "ActionFailed(db)");
        assert_eq!(format_failure_code(&FailureCode::BudgetExceeded), "BudgetExceeded");
        assert_eq!(format_failure_code(&FailureCode::StepPanicked), "StepPanicked");
        assert_eq!(format_failure_code(&FailureCode::ValidationError(String::from("bad"))), "ValidationError(bad)");
        assert_eq!(format_failure_code(&FailureCode::TaintLeak), "TaintLeak");
        assert_eq!(format_failure_code(&FailureCode::ReplayDivergence), "ReplayDivergence");
        assert_eq!(format_failure_code(&FailureCode::Unknown(String::from("x"))), "Unknown(x)");
    }

    #[test]
    fn test_incident_type_action_failure_for_timeout() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        assert_eq!(screen.incidents().first().unwrap().incident_type, IncidentType::ActionFailure);
    }

    #[test]
    fn test_incident_type_secret_leak_for_taint() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        assert_eq!(screen.incidents().first().unwrap().incident_type, IncidentType::SecretLeak);
    }

    #[test]
    fn test_incident_type_blocked_reconciliation_for_unknown() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::Unknown("err".into()), "err");
        assert_eq!(screen.incidents().first().unwrap().incident_type, IncidentType::BlockedReconciliation);
    }

    #[test]
    fn test_empty_screen_no_critical() {
        let screen = IncidentScreen::new();
        assert_eq!(screen.critical_count(), 0);
    }

    #[test]
    fn test_empty_screen_get_failure_detail_none() {
        let screen = IncidentScreen::new();
        assert!(screen.get_failure_detail(0).is_none());
        assert!(screen.get_failure_detail(100).is_none());
    }

    #[test]
    fn test_repair_kind_add_retry_backoff_for_action_failed() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionFailed("x".into()), "fail");
        assert!(screen.repair_suggestions(0).iter().any(|s| s.kind == RepairKind::AddRetryBackoff));
    }

    #[test]
    fn test_repair_kind_reduce_payload_for_validation_error() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ValidationError("v".into()), "v");
        assert!(screen.repair_suggestions(0).iter().any(|s| s.kind == RepairKind::ReducePayload));
    }

    #[test]
    fn test_repair_kind_pin_idempotency_for_replay_divergence() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(1, "a", "b");
        assert!(screen.repair_suggestions(0).iter().any(|s| s.kind == RepairKind::PinIdempotency));
    }

    #[test]
    fn test_register_incident_assigns_id() {
        use vb_core::{ActionId, RunId, StepIdx};
        use vb_storage::EventSeq;
        let mut screen = IncidentScreen::new();
        let event = vb_storage::JournalEvent::ActionFailedEvent { run: RunId::new(42), seq: EventSeq::new(1), step: StepIdx::new(0), action: ActionId::new(1) };
        let incident = IncidentScreen::process_failure(&event).unwrap();
        assert_eq!(incident.id, 0);
        let idx = screen.register_incident(incident);
        assert_eq!(idx, 0);
        assert_eq!(screen.active_count(), 1);
        assert_eq!(screen.incidents().first().unwrap().id, 1);
    }

    #[test]
    fn test_register_multiple_increments_ids() {
        use vb_core::{ActionId, RunId, StepIdx};
        use vb_storage::EventSeq;
        let mut screen = IncidentScreen::new();
        let e1 = vb_storage::JournalEvent::ActionFailedEvent { run: RunId::new(1), seq: EventSeq::new(1), step: StepIdx::new(0), action: ActionId::new(1) };
        let e2 = vb_storage::JournalEvent::RunFailedEvent { run: RunId::new(2), seq: EventSeq::new(2) };
        let inc1 = IncidentScreen::process_failure(&e1).unwrap();
        let inc2 = IncidentScreen::process_failure(&e2).unwrap();
        screen.register_incident(inc1);
        screen.register_incident(inc2);
        assert_eq!(screen.active_count(), 2);
        assert_eq!(screen.incidents().first().unwrap().id, 1);
        assert_eq!(screen.incidents().get(1).unwrap().id, 2);
    }

    // ---------------------------------------------------------------------------
    // summary_text tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_summary_text_empty() {
        let screen = IncidentScreen::new();
        assert_eq!(screen.summary_text(), "0 incidents");
    }

    #[test]
    fn test_summary_text_single_critical() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        assert_eq!(screen.summary_text(), "1 incidents: 1 Critical");
    }

    #[test]
    fn test_summary_text_single_error() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        assert_eq!(screen.summary_text(), "1 incidents: 1 Error");
    }

    #[test]
    fn test_summary_text_single_minor() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ValidationError("v".into()), "v");
        assert_eq!(screen.summary_text(), "1 incidents: 1 Minor");
    }

    #[test]
    fn test_summary_text_mixed_severities() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        screen.process_run_failure(2, None, FailureCode::ActionTimeout, "timeout");
        screen.process_run_failure(3, None, FailureCode::ValidationError("v".into()), "v");
        let text = screen.summary_text();
        assert!(text.starts_with("3 incidents:"), "actual: {text}");
        assert!(text.contains("1 Critical"), "actual: {text}");
        assert!(text.contains("1 Error"), "actual: {text}");
        assert!(text.contains("1 Minor"), "actual: {text}");
    }

    // ---------------------------------------------------------------------------
    // has_critical tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_has_critical_empty() {
        let screen = IncidentScreen::new();
        assert!(!screen.has_critical());
    }

    #[test]
    fn test_has_critical_true() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        assert!(screen.has_critical());
    }

    #[test]
    fn test_has_critical_false_when_only_major() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        assert!(!screen.has_critical());
    }

    #[test]
    fn test_has_critical_mixed_with_critical() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        screen.process_run_failure(2, None, FailureCode::TaintLeak, "leak");
        screen.process_run_failure(3, None, FailureCode::BudgetExceeded, "budget");
        assert!(screen.has_critical());
    }

    #[test]
    fn test_has_critical_after_dismiss() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        assert!(screen.has_critical());
        screen.dismiss(0);
        assert!(!screen.has_critical());
    }

    // ---------------------------------------------------------------------------
    // filter_by_severity tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_filter_by_severity_empty() {
        let screen = IncidentScreen::new();
        let result = screen.filter_by_severity(IncidentSeverity::Critical);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_by_severity_single_match() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        let result = screen.filter_by_severity(IncidentSeverity::Critical);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].run_id, 1);
    }

    #[test]
    fn test_filter_by_severity_no_match() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        let result = screen.filter_by_severity(IncidentSeverity::Critical);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_by_severity_multiple_of_same() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak1");
        screen.process_run_failure(2, None, FailureCode::StepPanicked, "panic");
        screen.process_run_failure(3, None, FailureCode::ActionTimeout, "timeout");
        let result = screen.filter_by_severity(IncidentSeverity::Critical);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_filter_by_severity_major() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "budget");
        screen.process_run_failure(3, None, FailureCode::TaintLeak, "leak");
        let result = screen.filter_by_severity(IncidentSeverity::Major);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_filter_by_severity_minor() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ValidationError("v".into()), "v");
        let result = screen.filter_by_severity(IncidentSeverity::Minor);
        assert_eq!(result.len(), 1);
    }

    // ---------------------------------------------------------------------------
    // Additional tests: defaults, empty list, ordering, severity
    // ---------------------------------------------------------------------------

    #[test]
    fn test_new_defaults_empty_and_no_selection() {
        let screen = IncidentScreen::new();
        assert!(screen.incidents().is_empty(), "new screen should have no incidents");
        assert_eq!(screen.active_count(), 0, "active_count should be 0");
        assert_eq!(screen.critical_count(), 0, "critical_count should be 0");
        assert!(screen.selected().is_none(), "no incident should be selected");
        assert!(screen.selected_suggestions().is_empty(), "no suggestions without selection");
    }

    #[test]
    fn test_default_trait_matches_new() {
        let from_new = IncidentScreen::new();
        let from_default = IncidentScreen::default();
        assert_eq!(from_new.active_count(), from_default.active_count());
        assert_eq!(from_new.critical_count(), from_default.critical_count());
        assert!(from_default.incidents().is_empty());
        assert!(from_default.selected().is_none());
    }

    #[test]
    fn test_empty_list_repair_suggestions_returns_empty() {
        let screen = IncidentScreen::new();
        assert!(screen.repair_suggestions(0).is_empty());
        assert!(screen.repair_suggestions(1).is_empty());
    }

    #[test]
    fn test_empty_list_dismiss_is_noop() {
        let mut screen = IncidentScreen::new();
        screen.dismiss(0);
        screen.dismiss(100);
        assert_eq!(screen.active_count(), 0);
    }

    #[test]
    fn test_empty_list_select_does_not_panic() {
        let mut screen = IncidentScreen::new();
        screen.select(0);
        screen.select(999);
        assert!(screen.selected().is_none());
    }

    #[test]
    fn test_severity_ordering_via_color_dominance() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "critical");
        screen.process_run_failure(2, None, FailureCode::ActionTimeout, "major");
        screen.process_run_failure(3, None, FailureCode::ValidationError("v".into()), "minor");

        let incidents = screen.incidents();
        let [crit_r, ..] = incidents[0].severity.severity_color();
        let [major_r, ..] = incidents[1].severity.severity_color();
        let [minor_r, ..] = incidents[2].severity.severity_color();
        assert!(crit_r >= major_r, "Critical red should dominate Major red");
        assert!(major_r > minor_r, "Major red should dominate Minor red");
    }

    #[test]
    fn test_incidents_preserve_insertion_order() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(10, None, FailureCode::ActionTimeout, "a");
        screen.process_run_failure(20, None, FailureCode::TaintLeak, "b");
        screen.process_run_failure(30, None, FailureCode::BudgetExceeded, "c");

        let run_ids: Vec<u64> = screen.incidents().iter().map(|i| i.run_id).collect();
        assert_eq!(run_ids, vec![10, 20, 30], "incidents should maintain insertion order");
    }

    // =========================================================================
    // select_incident, selected_incident, dismiss_selected, detail_sections
    // =========================================================================

    // -- select_incident tests --

    #[test]
    fn test_select_incident_valid_index() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        let result = screen.select_incident(1);
        assert!(result.is_some());
        assert_eq!(result.map(|i| i.run_id), Some(2));
    }

    #[test]
    fn test_select_incident_first_index() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(10, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(20, None, FailureCode::TaintLeak, "l");
        let result = screen.select_incident(0);
        assert!(result.is_some());
        assert_eq!(result.map(|i| i.run_id), Some(10));
    }

    #[test]
    fn test_select_incident_out_of_bounds_returns_none() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        let result = screen.select_incident(5);
        assert!(result.is_none());
    }

    #[test]
    fn test_select_incident_empty_screen_returns_none() {
        let mut screen = IncidentScreen::new();
        let result = screen.select_incident(0);
        assert!(result.is_none());
    }

    #[test]
    fn test_select_incident_changes_selection() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        screen.select_incident(0);
        assert_eq!(screen.selected_incident().map(|i| i.run_id), Some(1));
        screen.select_incident(1);
        assert_eq!(screen.selected_incident().map(|i| i.run_id), Some(2));
    }

    #[test]
    fn test_select_incident_reselect_same_index() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(42, None, FailureCode::ActionTimeout, "t");
        let first_run_id = screen.select_incident(0).map(|i| i.run_id);
        drop(first_run_id);
        let second_run_id = screen.select_incident(0).map(|i| i.run_id);
        assert!(second_run_id.is_some());
    }

    #[test]
    fn test_select_incident_returns_reference_to_selected() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(99, Some("step-x"), FailureCode::ActionTimeout, "timeout");
        let incident = screen.select_incident(0);
        assert!(incident.is_some());
        let inc = incident.map_or(false, |i| {
            i.run_id == 99 && i.step_name.as_deref() == Some("step-x")
        });
        assert!(inc);
    }

    // -- selected_incident tests --

    #[test]
    fn test_selected_incident_none_when_no_selection() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        assert!(screen.selected_incident().is_none());
    }

    #[test]
    fn test_selected_incident_none_on_empty_screen() {
        let screen = IncidentScreen::new();
        assert!(screen.selected_incident().is_none());
    }

    #[test]
    fn test_selected_incident_after_select() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(7, None, FailureCode::TaintLeak, "leak");
        screen.select_incident(0);
        let selected = screen.selected_incident();
        assert!(selected.is_some());
        assert_eq!(selected.map(|i| i.run_id), Some(7));
        assert_eq!(selected.map(|i| i.failure_code.clone()), Some(FailureCode::TaintLeak));
    }

    #[test]
    fn test_selected_incident_after_legacy_select() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(3, None, FailureCode::BudgetExceeded, "b");
        screen.select(0);
        let selected = screen.selected_incident();
        assert!(selected.is_some());
        assert_eq!(selected.map(|i| i.run_id), Some(3));
    }

    #[test]
    fn test_selected_incident_after_dismiss_becomes_none() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        assert!(screen.selected_incident().is_some());
        screen.dismiss(0);
        assert!(screen.selected_incident().is_none());
    }

    // -- dismiss_selected tests --

    #[test]
    fn test_dismiss_selected_with_selection() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        screen.select_incident(0);
        let dismissed = screen.dismiss_selected();
        assert!(dismissed);
        assert_eq!(screen.active_count(), 1);
        assert!(screen.selected_incident().is_none());
    }

    #[test]
    fn test_dismiss_selected_no_selection_returns_false() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        let dismissed = screen.dismiss_selected();
        assert!(!dismissed);
        assert_eq!(screen.active_count(), 1);
    }

    #[test]
    fn test_dismiss_selected_empty_screen_returns_false() {
        let mut screen = IncidentScreen::new();
        let dismissed = screen.dismiss_selected();
        assert!(!dismissed);
    }

    #[test]
    fn test_dismiss_selected_clears_selection() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::TaintLeak, "leak");
        screen.select_incident(1);
        assert_eq!(screen.selected_incident().map(|i| i.run_id), Some(2));
        let dismissed = screen.dismiss_selected();
        assert!(dismissed);
        assert!(screen.selected_incident().is_none());
    }

    #[test]
    fn test_dismiss_selected_reduces_count() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(10, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(20, None, FailureCode::BudgetExceeded, "b");
        screen.process_run_failure(30, None, FailureCode::TaintLeak, "l");
        screen.select_incident(1);
        assert!(screen.dismiss_selected());
        assert_eq!(screen.active_count(), 2);
    }

    #[test]
    fn test_dismiss_selected_twice_second_fails() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        assert!(screen.dismiss_selected());
        assert!(!screen.dismiss_selected(), "second dismiss with no selection should return false");
    }

    #[test]
    fn test_dismiss_selected_then_select_another() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.process_run_failure(2, None, FailureCode::TaintLeak, "l");
        screen.select_incident(0);
        assert!(screen.dismiss_selected());
        assert_eq!(screen.active_count(), 1);
        screen.select_incident(0);
        assert_eq!(screen.selected_incident().map(|i| i.run_id), Some(2));
        assert!(screen.dismiss_selected());
        assert_eq!(screen.active_count(), 0);
    }

    #[test]
    fn test_dismiss_selected_all_incidents_one_by_one() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "a");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        screen.process_run_failure(3, None, FailureCode::TaintLeak, "c");
        screen.select_incident(2);
        assert!(screen.dismiss_selected());
        screen.select_incident(0);
        assert!(screen.dismiss_selected());
        screen.select_incident(0);
        assert!(screen.dismiss_selected());
        assert_eq!(screen.active_count(), 0);
        assert!(screen.selected_incident().is_none());
    }

    // -- detail_sections tests --

    #[test]
    fn test_detail_sections_no_selection_returns_empty() {
        let screen = IncidentScreen::new();
        let sections = screen.detail_sections();
        assert!(sections.cause.is_none());
        assert!(sections.timeline.is_empty());
        assert!(sections.state_diff.is_empty());
        assert!(sections.repair_suggestions.is_empty());
        assert!(!sections.replay_safe);
        assert_eq!(sections.side_effect_certainty, SideEffectCertainty::None);
    }

    #[test]
    fn test_detail_sections_empty_screen_returns_empty() {
        let screen = IncidentScreen::new();
        let sections = screen.detail_sections();
        assert!(sections.cause.is_none());
    }

    #[test]
    fn test_detail_sections_with_selected_incident_has_cause() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(42, Some("step-fetch"), FailureCode::ActionTimeout, "timed out");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        let cause = sections.cause.as_ref();
        assert!(cause.is_some());
        let c = cause.map_or(false, |v| {
            v.run_id == 42
                && v.error_message.contains("timed out")
                && v.severity == IncidentSeverity::Major
                && v.step_name.as_deref() == Some("step-fetch")
                && v.category == "action"
                && v.failure_code == FailureCode::ActionTimeout
        });
        assert!(c);
    }

    #[test]
    fn test_detail_sections_timeline_entries() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert!(!sections.timeline.is_empty());
        assert_eq!(sections.timeline.first().map(|e| e.seq), Some(0));
        assert_eq!(
            sections.timeline.first().map(|e| e.event_kind),
            Some(TimelineEventKind::FailureObserved)
        );
        assert!(sections.timeline.first().map_or(false, |e| !e.description.is_empty()));
    }

    #[test]
    fn test_detail_sections_replay_divergence_timeline() {
        let mut screen = IncidentScreen::new();
        screen.process_replay_divergence(100, "expected-val", "actual-val");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert_eq!(sections.timeline.len(), 2);
        assert_eq!(sections.timeline.first().map(|e| e.event_kind), Some(TimelineEventKind::FailureObserved));
        assert_eq!(sections.timeline.get(1).map(|e| e.event_kind), Some(TimelineEventKind::ReplayDivergence));
    }

    #[test]
    fn test_detail_sections_replay_safe_flag() {
        let mut screen = IncidentScreen::new();
        // ActionTimeout is replay_safe
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert!(sections.replay_safe);

        // TaintLeak is not replay_safe
        screen.dismiss(0);
        screen.process_run_failure(2, None, FailureCode::TaintLeak, "leak");
        screen.select_incident(0);
        let sections2 = screen.detail_sections();
        assert!(!sections2.replay_safe);
    }

    #[test]
    fn test_detail_sections_side_effect_certainty() {
        let mut screen = IncidentScreen::new();
        // ActionTimeout => SideEffectCertainty::None
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert_eq!(sections.side_effect_certainty, SideEffectCertainty::None);

        // ActionFailed => SideEffectCertainty::Unknown
        screen.process_run_failure(2, None, FailureCode::ActionFailed("db".into()), "db error");
        screen.select_incident(1);
        let sections2 = screen.detail_sections();
        assert_eq!(sections2.side_effect_certainty, SideEffectCertainty::Unknown);

        // TaintLeak => SideEffectCertainty::Certain
        screen.process_run_failure(3, None, FailureCode::TaintLeak, "leak");
        screen.select_incident(2);
        let sections3 = screen.detail_sections();
        assert_eq!(sections3.side_effect_certainty, SideEffectCertainty::Certain);
    }

    #[test]
    fn test_detail_sections_repair_suggestions() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert!(!sections.repair_suggestions.is_empty());
        assert!(sections.repair_suggestions.iter().any(|s| s.kind == RepairKind::IncreaseTimeout));
    }

    #[test]
    fn test_detail_sections_repair_suggestions_taint_leak() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert!(sections.repair_suggestions.iter().any(|s| s.kind == RepairKind::FixSecretLeak));
    }

    #[test]
    fn test_detail_sections_state_diff_empty_by_default() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert!(sections.state_diff.is_empty());
    }

    #[test]
    fn test_detail_sections_switching_selection() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        screen.process_run_failure(2, None, FailureCode::TaintLeak, "leak");
        screen.select_incident(0);
        let sections1 = screen.detail_sections();
        assert_eq!(sections1.cause.as_ref().map(|c| c.run_id), Some(1));
        assert!(sections1.replay_safe);

        screen.select_incident(1);
        let sections2 = screen.detail_sections();
        assert_eq!(sections2.cause.as_ref().map(|c| c.run_id), Some(2));
        assert!(!sections2.replay_safe);
    }

    #[test]
    fn test_detail_sections_cause_category_matches_failure_code() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::TaintLeak, "leak");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        let cause = sections.cause.as_ref();
        assert!(cause.is_some());
        assert_eq!(cause.map(|c| c.category.as_str()), Some("security"));
    }

    #[test]
    fn test_detail_sections_cause_severity_matches() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ValidationError("bad".into()), "v");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        let cause = sections.cause.as_ref();
        assert!(cause.is_some());
        assert_eq!(cause.map(|c| c.severity), Some(IncidentSeverity::Minor));
    }

    #[test]
    fn test_detail_sections_after_dismiss_selected_returns_empty() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        let _ = screen.detail_sections();
        assert!(screen.dismiss_selected());
        let sections = screen.detail_sections();
        assert!(sections.cause.is_none());
        assert!(sections.timeline.is_empty());
    }

    #[test]
    fn test_detail_sections_timeline_entry_has_timestamp_micros() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        let first = sections.timeline.first();
        assert!(first.is_some());
        // timestamp_micros should be a valid u64 (we just verify it exists)
        let _micros = first.map(|e| e.timestamp_micros);
    }

    #[test]
    fn test_detail_sections_all_failure_codes_produce_cause() {
        let codes = [
            FailureCode::ActionTimeout,
            FailureCode::ActionFailed("err".into()),
            FailureCode::BudgetExceeded,
            FailureCode::StepPanicked,
            FailureCode::ValidationError("bad".into()),
            FailureCode::TaintLeak,
            FailureCode::ReplayDivergence,
            FailureCode::Unknown("x".into()),
        ];
        for code in &codes {
            let mut screen = IncidentScreen::new();
            screen.process_run_failure(1, Some("step"), code.clone(), "error");
            screen.select_incident(0);
            let sections = screen.detail_sections();
            assert!(
                sections.cause.is_some(),
                "cause should be present for {:?}",
                code
            );
            assert!(
                !sections.repair_suggestions.is_empty(),
                "repair suggestions should exist for {:?}",
                code
            );
        }
    }

    #[test]
    fn test_detail_sections_incident_without_step_name() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "timeout");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        let cause = sections.cause.as_ref();
        assert!(cause.is_some());
        assert!(cause.map_or(false, |c| c.step_name.is_none()));
    }

    #[test]
    fn test_detail_sections_incident_with_step_name() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, Some("deploy-step"), FailureCode::BudgetExceeded, "budget");
        screen.select_incident(0);
        let sections = screen.detail_sections();
        let cause = sections.cause.as_ref();
        assert!(cause.is_some());
        assert_eq!(cause.map(|c| c.step_name.as_deref()), Some(Some("deploy-step")));
    }

    // =========================================================================
    // IncidentCauseView, IncidentTimelineEntry, IncidentSlotDiff construction
    // =========================================================================

    #[test]
    fn test_incident_cause_view_fields() {
        let cause = super::super::types::IncidentCauseView {
            category: String::from("action"),
            failure_code: FailureCode::ActionTimeout,
            error_message: String::from("timed out"),
            severity: IncidentSeverity::Major,
            step_name: Some(String::from("fetch")),
            run_id: 42,
        };
        assert_eq!(cause.category, "action");
        assert_eq!(cause.failure_code, FailureCode::ActionTimeout);
        assert_eq!(cause.error_message, "timed out");
        assert_eq!(cause.severity, IncidentSeverity::Major);
        assert_eq!(cause.step_name.as_deref(), Some("fetch"));
        assert_eq!(cause.run_id, 42);
    }

    #[test]
    fn test_incident_cause_view_clone() {
        let cause = super::super::types::IncidentCauseView {
            category: String::from("security"),
            failure_code: FailureCode::TaintLeak,
            error_message: String::from("leak detected"),
            severity: IncidentSeverity::Critical,
            step_name: None,
            run_id: 1,
        };
        let cloned = cause.clone();
        assert_eq!(cloned.category, cause.category);
        assert_eq!(cloned.failure_code, cause.failure_code);
        assert_eq!(cloned.error_message, cause.error_message);
        assert_eq!(cloned.severity, cause.severity);
        assert_eq!(cloned.step_name, cause.step_name);
        assert_eq!(cloned.run_id, cause.run_id);
    }

    #[test]
    fn test_incident_timeline_entry_fields() {
        let entry = super::super::types::IncidentTimelineEntry {
            seq: 5,
            description: String::from("retry attempted"),
            timestamp_micros: 1_000_000,
            event_kind: TimelineEventKind::RetryAttempted,
        };
        assert_eq!(entry.seq, 5);
        assert_eq!(entry.description, "retry attempted");
        assert_eq!(entry.timestamp_micros, 1_000_000);
        assert_eq!(entry.event_kind, TimelineEventKind::RetryAttempted);
    }

    #[test]
    fn test_incident_timeline_entry_clone() {
        let entry = super::super::types::IncidentTimelineEntry {
            seq: 3,
            description: String::from("failure observed"),
            timestamp_micros: 500_000,
            event_kind: TimelineEventKind::FailureObserved,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.seq, entry.seq);
        assert_eq!(cloned.description, entry.description);
        assert_eq!(cloned.timestamp_micros, entry.timestamp_micros);
        assert_eq!(cloned.event_kind, entry.event_kind);
    }

    #[test]
    fn test_incident_slot_diff_fields() {
        let diff = super::super::types::IncidentSlotDiff {
            slot_index: 7,
            value_before: String::from("old"),
            value_after: String::from("new"),
            change_label: String::from("modified"),
        };
        assert_eq!(diff.slot_index, 7);
        assert_eq!(diff.value_before, "old");
        assert_eq!(diff.value_after, "new");
        assert_eq!(diff.change_label, "modified");
    }

    #[test]
    fn test_incident_slot_diff_unchanged() {
        let diff = super::super::types::IncidentSlotDiff {
            slot_index: 1,
            value_before: String::from("same"),
            value_after: String::from("same"),
            change_label: String::from("unchanged"),
        };
        assert_eq!(diff.value_before, diff.value_after);
        assert_eq!(diff.change_label, "unchanged");
    }

    #[test]
    fn test_incident_slot_diff_clone() {
        let diff = super::super::types::IncidentSlotDiff {
            slot_index: 2,
            value_before: String::from("before"),
            value_after: String::from("after"),
            change_label: String::from("modified"),
        };
        let cloned = diff.clone();
        assert_eq!(cloned.slot_index, diff.slot_index);
        assert_eq!(cloned.value_before, diff.value_before);
        assert_eq!(cloned.value_after, diff.value_after);
        assert_eq!(cloned.change_label, diff.change_label);
    }

    #[test]
    fn test_incident_detail_sections_default_fields() {
        let sections = super::super::types::IncidentDetailSections {
            cause: None,
            timeline: Vec::new(),
            state_diff: Vec::new(),
            repair_suggestions: Vec::new(),
            replay_safe: false,
            side_effect_certainty: SideEffectCertainty::None,
        };
        assert!(sections.cause.is_none());
        assert!(sections.timeline.is_empty());
        assert!(sections.state_diff.is_empty());
        assert!(sections.repair_suggestions.is_empty());
        assert!(!sections.replay_safe);
        assert_eq!(sections.side_effect_certainty, SideEffectCertainty::None);
    }

    #[test]
    fn test_incident_detail_sections_clone() {
        let sections = super::super::types::IncidentDetailSections {
            cause: Some(super::super::types::IncidentCauseView {
                category: String::from("action"),
                failure_code: FailureCode::ActionTimeout,
                error_message: String::from("err"),
                severity: IncidentSeverity::Major,
                step_name: None,
                run_id: 1,
            }),
            timeline: vec![super::super::types::IncidentTimelineEntry {
                seq: 0,
                description: String::from("event"),
                timestamp_micros: 100,
                event_kind: TimelineEventKind::FailureObserved,
            }],
            state_diff: Vec::new(),
            repair_suggestions: Vec::new(),
            replay_safe: true,
            side_effect_certainty: SideEffectCertainty::None,
        };
        let cloned = sections.clone();
        assert!(cloned.cause.is_some());
        assert_eq!(cloned.timeline.len(), 1);
        assert_eq!(cloned.replay_safe, sections.replay_safe);
        assert_eq!(cloned.side_effect_certainty, sections.side_effect_certainty);
    }

    // =========================================================================
    // Interaction tests: select + dismiss + detail round-trips
    // =========================================================================

    #[test]
    fn test_select_then_dismiss_selected_then_select_remaining() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "a");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "b");
        screen.process_run_failure(3, None, FailureCode::TaintLeak, "c");

        // Select middle one
        screen.select_incident(1);
        assert_eq!(screen.selected_incident().map(|i| i.run_id), Some(2));

        // Dismiss it
        assert!(screen.dismiss_selected());
        assert_eq!(screen.active_count(), 2);

        // Select the new first one (was index 0, still index 0)
        screen.select_incident(0);
        assert_eq!(screen.selected_incident().map(|i| i.run_id), Some(1));
    }

    #[test]
    fn test_detail_sections_after_multiple_dismissals() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "a");
        screen.process_run_failure(2, None, FailureCode::TaintLeak, "b");

        // Dismiss first via dismiss_selected
        screen.select_incident(0);
        assert!(screen.dismiss_selected());

        // Now only one left, select it
        screen.select_incident(0);
        let sections = screen.detail_sections();
        assert_eq!(sections.cause.as_ref().map(|c| c.run_id), Some(2));
        assert!(!sections.replay_safe);
    }

    #[test]
    fn test_select_incident_after_all_dismissed() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, None, FailureCode::ActionTimeout, "t");
        screen.select_incident(0);
        assert!(screen.dismiss_selected());
        assert_eq!(screen.active_count(), 0);

        let result = screen.select_incident(0);
        assert!(result.is_none());
        let sections = screen.detail_sections();
        assert!(sections.cause.is_none());
    }

    #[test]
    fn test_detail_sections_with_mixed_incidents() {
        let mut screen = IncidentScreen::new();
        screen.process_run_failure(1, Some("deploy"), FailureCode::ActionFailed("net".into()), "network error");
        screen.process_run_failure(2, None, FailureCode::BudgetExceeded, "budget");
        screen.process_replay_divergence(3, "expected", "actual");

        // Select the replay divergence (2 timeline entries)
        screen.select_incident(2);
        let sections = screen.detail_sections();
        assert_eq!(sections.timeline.len(), 2);
        assert!(!sections.replay_safe);

        // Select the budget exceeded
        screen.select_incident(1);
        let sections2 = screen.detail_sections();
        assert!(sections2.replay_safe);
        assert_eq!(sections2.side_effect_certainty, SideEffectCertainty::None);
    }
}
