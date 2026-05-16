#![forbid(unsafe_code)]
//! IncidentScreen orchestration and UI methods.

use std::time::Instant;

use super::console::IncidentConsole;
use super::repair::{RepairSuggestion, suggest_repairs};
use super::types::{
    FailureCode, FailureDetail, Incident, IncidentCauseView, IncidentContext,
    IncidentDetailSections, IncidentSeverity, IncidentSlotDiff, IncidentTimelineEntry,
    IncidentType, SideEffectCertainty, SuggestionItem, TimelineEntry, TimelineEventKind,
    suggestion_items_for_failure_code,
};

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
            FailureCode::TaintLeak | FailureCode::StepPanicked | FailureCode::ReplayDivergence => {
                IncidentSeverity::Critical
            }
            FailureCode::ActionTimeout
            | FailureCode::ActionFailed(_)
            | FailureCode::BudgetExceeded => IncidentSeverity::Major,
            FailureCode::ValidationError(_) | FailureCode::Unknown(_) => IncidentSeverity::Minor,
        }
    }

    fn incident_type_for_code(code: &FailureCode) -> IncidentType {
        match code {
            FailureCode::ActionTimeout
            | FailureCode::ActionFailed(_)
            | FailureCode::BudgetExceeded
            | FailureCode::StepPanicked
            | FailureCode::ValidationError(_) => IncidentType::ActionFailure,
            FailureCode::ReplayDivergence => IncidentType::ReplayDivergence,
            FailureCode::TaintLeak => IncidentType::SecretLeak,
            FailureCode::Unknown(_) => IncidentType::BlockedReconciliation,
        }
    }

    fn certainty_for_code(code: &FailureCode) -> SideEffectCertainty {
        match code {
            FailureCode::ActionFailed(_) | FailureCode::StepPanicked => {
                SideEffectCertainty::Unknown
            }
            FailureCode::TaintLeak => SideEffectCertainty::Certain,
            _ => SideEffectCertainty::None,
        }
    }

    fn replay_safe_for_code(code: &FailureCode) -> bool {
        matches!(
            code,
            FailureCode::ActionTimeout
                | FailureCode::ValidationError(_)
                | FailureCode::BudgetExceeded
        )
    }

    fn initial_timeline(
        code: &FailureCode,
        error_context: &str,
        timestamp: Instant,
    ) -> Vec<TimelineEntry> {
        vec![TimelineEntry {
            seq: 0,
            description: format!(
                "Failure observed: {} - {}",
                format_failure_code(code),
                error_context
            ),
            timestamp_micros: Self::instant_to_micros(timestamp),
            event_kind: TimelineEventKind::FailureObserved,
            timestamp,
        }]
    }

    pub fn process_failure(event: &vb_storage::JournalEvent) -> Option<Incident> {
        match event {
            vb_storage::JournalEvent::ActionFailedEvent {
                run,
                seq,
                step,
                action,
                ..
            } => {
                let now = Instant::now();
                let failure_code = FailureCode::ActionFailed(format!(
                    "action {} failed in step {}",
                    action.get(),
                    step.get()
                ));
                let timeline = vec![TimelineEntry {
                    seq: 0,
                    description: format!(
                        "ActionFailed: action {} in step {} at seq {}",
                        action.get(),
                        step.get(),
                        seq.get()
                    ),
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
                    error_message: format!(
                        "Action {} failed in step {} for run {}",
                        action.get(),
                        step.get(),
                        run.get()
                    ),
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
                })
            }
            vb_storage::JournalEvent::RunFailedEvent { run, seq, .. } => {
                let now = Instant::now();
                let failure_code =
                    FailureCode::Unknown(format!("Run {} failed at seq {}", run.get(), seq.get()));
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
                    step_id: None,
                    step_name: None,
                    error_message: format!("Run {} failed", run.get()),
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
                })
            }
            _ => None,
        }
    }

    pub fn register_incident(&mut self, mut incident: Incident) -> usize {
        incident.id = self.allocate_id();
        self.console.add_incident(incident)
    }

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
            incident_type: Self::incident_type_for_code(&error_code),
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

    pub fn process_replay_divergence(
        &mut self,
        run_id: u64,
        expected: &str,
        actual: &str,
    ) -> usize {
        let id = self.allocate_id();
        let now = Instant::now();
        let description = format!(
            "Replay divergence: expected {}, actual {}",
            expected, actual
        );
        let timeline = vec![
            TimelineEntry {
                seq: 0,
                description: description.clone(),
                timestamp_micros: Self::instant_to_micros(now),
                event_kind: TimelineEventKind::FailureObserved,
                timestamp: now,
            },
            TimelineEntry {
                seq: 1,
                description: format!(
                    "Divergence detail - expected: {}, actual: {}",
                    expected, actual
                ),
                timestamp_micros: Self::instant_to_micros(now),
                event_kind: TimelineEventKind::ReplayDivergence,
                timestamp: now,
            },
        ];
        let incident = Incident {
            id,
            incident_type: IncidentType::ReplayDivergence,
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

    pub fn get_failure_detail(&self, incident_index: usize) -> Option<FailureDetail> {
        let incidents = self.console.legacy_incidents();
        let incident = incidents.get(incident_index)?;
        Some(FailureDetail {
            error_code: format_failure_code(&incident.failure_code),
            step_id: incident.step_id,
            run_id: incident.run_id,
            workflow_name: incident.workflow_name.clone(),
            replay_safe: incident.replay_safe,
            timeline: incident.timeline.clone(),
            failure_code: incident.failure_code.clone(),
            step_name: incident.step_name.clone(),
            side_effect_certainty: incident.side_effect_certainty,
            error_context: incident.context.clone(),
        })
    }

    pub fn repair_suggestions(&self, incident_index: usize) -> Vec<RepairSuggestion> {
        let incidents = self.console.legacy_incidents();
        match incidents.get(incident_index) {
            Some(incident) => suggest_repairs(incident),
            None => Vec::new(),
        }
    }

    pub fn incidents(&self) -> &[Incident] {
        self.console.legacy_incidents()
    }

    pub fn select(&mut self, index: usize) {
        self.console.select(index);
    }

    pub fn dismiss(&mut self, index: usize) {
        self.console.dismiss(index);
    }

    pub fn active_count(&self) -> usize {
        self.console.active_count()
    }

    pub fn critical_count(&self) -> usize {
        self.console.legacy_critical_count()
    }

    pub fn selected(&self) -> Option<&Incident> {
        self.console.selected()
    }

    pub fn selected_suggestions(&self) -> Vec<RepairSuggestion> {
        self.console.selected_suggestions()
    }

    pub fn selected_suggestion_items(&self) -> Vec<SuggestionItem> {
        match self.console.selected() {
            Some(incident) => suggestion_items_for_failure_code(&incident.failure_code),
            None => Vec::new(),
        }
    }

    pub fn suggestion_items(&self, incident_index: usize) -> Vec<SuggestionItem> {
        match self.console.legacy_incidents().get(incident_index) {
            Some(incident) => suggestion_items_for_failure_code(&incident.failure_code),
            None => Vec::new(),
        }
    }

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

    pub fn has_critical(&self) -> bool {
        self.console
            .legacy_incidents()
            .iter()
            .any(|inc| inc.severity == IncidentSeverity::Critical)
    }

    pub fn filter_by_severity(&self, severity: IncidentSeverity) -> Vec<&Incident> {
        self.console
            .legacy_incidents()
            .iter()
            .filter(|inc| inc.severity == severity)
            .collect()
    }

    pub fn select_incident(&mut self, index: usize) -> Option<&Incident> {
        let incidents = self.console.legacy_incidents();
        if incidents.get(index).is_some() {
            self.console.select(index);
            return self.console.selected();
        }
        None
    }

    pub fn selected_incident(&self) -> Option<&Incident> {
        self.console.selected()
    }

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
                let value_after = matching_taint.map(|(_, v)| v.clone()).unwrap_or_default();
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
