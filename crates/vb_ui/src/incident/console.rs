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
