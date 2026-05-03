use super::repair::{suggest_repairs, RepairSuggestion};
use super::types::Incident;

pub struct IncidentConsole {
    incidents: Vec<Incident>,
    selected: Option<usize>,
}

impl IncidentConsole {
    pub fn new() -> Self {
        Self {
            incidents: Vec::new(),
            selected: None,
        }
    }

    pub fn add_incident(&mut self, incident: Incident) -> usize {
        let idx = self.incidents.len();
        self.incidents.push(incident);
        idx
    }

    pub fn dismiss(&mut self, index: usize) {
        if index < self.incidents.len() {
            if self.selected == Some(index) {
                self.selected = None;
            }
            self.incidents.remove(index);
            if let Some(sel) = self.selected {
                if sel >= self.incidents.len() {
                    self.selected = None;
                }
            }
        }
    }

    pub fn select(&mut self, index: usize) {
        if index < self.incidents.len() {
            self.selected = Some(index);
        }
    }

    pub fn selected(&self) -> Option<&Incident> {
        self.selected.and_then(|i| self.incidents.get(i))
    }

    pub fn selected_suggestions(&self) -> Vec<RepairSuggestion> {
        self.selected().map(suggest_repairs).unwrap_or_default()
    }

    pub fn active_incidents(&self) -> &[Incident] {
        &self.incidents
    }

    pub fn critical_count(&self) -> usize {
        self.incidents
            .iter()
            .filter(|i| matches!(i.severity, super::types::IncidentSeverity::Critical))
            .count()
    }

    pub fn active_count(&self) -> usize {
        self.incidents.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::repair::RepairAction;
    use super::super::types::{
        FailureCode, IncidentContext, IncidentSeverity, SideEffectCertainty,
    };
    use std::time::Instant;

    fn make_incident(id: u64, severity: IncidentSeverity, code: FailureCode) -> Incident {
        Incident {
            id,
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
        }
    }

    #[test]
    fn test_console_new_is_empty() {
        let console = IncidentConsole::new();
        assert!(console.active_incidents().is_empty());
        assert_eq!(console.active_count(), 0);
        assert!(console.selected().is_none());
        assert!(console.selected_suggestions().is_empty());
        assert_eq!(console.critical_count(), 0);
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
        assert_eq!(console.selected().unwrap().id, 1);
    }

    #[test]
    fn test_console_dismiss_updates_selection() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(1, IncidentSeverity::Minor, FailureCode::ActionTimeout));
        console.add_incident(make_incident(2, IncidentSeverity::Critical, FailureCode::TaintLeak));

        // Select the second incident, then dismiss it.
        console.select(1);
        assert!(console.selected().is_some());

        console.dismiss(1);
        assert_eq!(console.active_count(), 1);
        assert!(console.selected().is_none(), "selection should be cleared after dismissing the selected incident");
    }

    #[test]
    fn test_console_suggestions_for_selected() {
        let mut console = IncidentConsole::new();
        console.add_incident(make_incident(1, IncidentSeverity::Major, FailureCode::ActionTimeout));

        console.select(0);
        let suggestions = console.selected_suggestions();
        assert!(!suggestions.is_empty(), "ActionTimeout should produce repair suggestions");
        assert!(suggestions.iter().any(|s| s.action == RepairAction::IncreaseTimeout));
    }
}
