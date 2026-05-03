use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Alert {
    pub severity: AlertSeverity,
    pub kind: AlertKind,
    pub message: String,
    pub run_id: Option<u64>,
    pub shard_id: Option<u32>,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertKind {
    QueuePressure,
    RunFailed,
    ReplayDivergence,
    JournalLag,
    SecretLeak,
    ShardOverloaded,
}

pub struct AlertManager {
    alerts: Vec<Alert>,
    max_alerts: usize,
}

impl AlertManager {
    pub fn new() -> Self {
        Self {
            alerts: Vec::new(),
            max_alerts: 100,
        }
    }

    pub fn add(&mut self, alert: Alert) {
        if self.alerts.len() >= self.max_alerts {
            self.alerts.remove(0);
        }
        self.alerts.push(alert);
    }

    pub fn dismiss(&mut self, index: usize) {
        if index < self.alerts.len() {
            self.alerts.remove(index);
        }
    }

    pub fn active(&self) -> &[Alert] {
        &self.alerts
    }

    pub fn critical_count(&self) -> usize {
        self.alerts
            .iter()
            .filter(|a| matches!(a.severity, AlertSeverity::Critical))
            .count()
    }
}
