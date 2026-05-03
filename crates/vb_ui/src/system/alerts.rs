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

impl AlertSeverity {
    #[must_use]
    pub const fn color(self) -> [f32; 4] {
        match self {
            Self::Info => [0.0, 0.961, 1.0, 1.0],
            Self::Warning => [1.0, 0.902, 0.0, 1.0],
            Self::Critical => [1.0, 0.027, 0.227, 1.0],
        }
    }
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
    #[must_use]
    pub fn new(max_alerts: usize) -> Self {
        Self {
            alerts: Vec::new(),
            max_alerts,
        }
    }

    pub fn add(&mut self, alert: Alert) {
        if self.max_alerts == 0 {
            return;
        }
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

    #[must_use]
    pub fn active(&self) -> &[Alert] {
        &self.alerts
    }

    #[must_use]
    pub fn critical_count(&self) -> usize {
        self.alerts
            .iter()
            .filter(|a| a.severity == AlertSeverity::Critical)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info_alert(msg: &str) -> Alert {
        Alert {
            severity: AlertSeverity::Info,
            kind: AlertKind::QueuePressure,
            message: msg.to_string(),
            run_id: None,
            shard_id: None,
            timestamp: Instant::now(),
        }
    }

    fn critical_alert(msg: &str) -> Alert {
        Alert {
            severity: AlertSeverity::Critical,
            kind: AlertKind::RunFailed,
            message: msg.to_string(),
            run_id: Some(42),
            shard_id: Some(0),
            timestamp: Instant::now(),
        }
    }

    #[test]
    fn alert_manager_new_is_empty() {
        let mgr = AlertManager::new(10);
        assert!(mgr.active().is_empty());
        assert_eq!(mgr.critical_count(), 0);
    }

    #[test]
    fn alert_manager_add_and_active() {
        let mut mgr = AlertManager::new(10);
        mgr.add(info_alert("queue high"));
        mgr.add(critical_alert("run died"));
        assert_eq!(mgr.active().len(), 2);
        assert_eq!(mgr.active()[0].message, "queue high");
        assert_eq!(mgr.active()[1].message, "run died");
    }

    #[test]
    fn alert_manager_critical_count_filters_by_severity() {
        let mut mgr = AlertManager::new(10);
        mgr.add(info_alert("info"));
        mgr.add(critical_alert("crit"));
        mgr.add(critical_alert("crit2"));
        assert_eq!(mgr.critical_count(), 2);
    }

    #[test]
    fn alert_manager_dismiss_removes_alert() {
        let mut mgr = AlertManager::new(10);
        mgr.add(info_alert("a"));
        mgr.add(info_alert("b"));
        mgr.add(info_alert("c"));
        mgr.dismiss(1);
        assert_eq!(mgr.active().len(), 2);
        assert_eq!(mgr.active()[0].message, "a");
        assert_eq!(mgr.active()[1].message, "c");
    }

    #[test]
    fn alert_manager_dismiss_out_of_bounds_is_noop() {
        let mut mgr = AlertManager::new(10);
        mgr.add(info_alert("a"));
        mgr.dismiss(5);
        assert_eq!(mgr.active().len(), 1);
    }

    #[test]
    fn alert_manager_evicts_oldest_when_full() {
        let mut mgr = AlertManager::new(2);
        mgr.add(info_alert("first"));
        mgr.add(info_alert("second"));
        mgr.add(info_alert("third"));
        assert_eq!(mgr.active().len(), 2);
        assert_eq!(mgr.active()[0].message, "second");
        assert_eq!(mgr.active()[1].message, "third");
    }

    #[test]
    fn alert_severity_info_color_is_cyan() {
        let [r, g, b, a] = AlertSeverity::Info.color();
        assert_eq!(r, 0.0);
        assert!((g - 0.961).abs() < 0.002);
        assert_eq!(b, 1.0);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn alert_severity_warning_color_is_yellow() {
        let [r, g, b, a] = AlertSeverity::Warning.color();
        assert_eq!(r, 1.0);
        assert!((g - 0.902).abs() < 0.002);
        assert_eq!(b, 0.0);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn alert_severity_critical_color_is_red() {
        let [r, g, b, a] = AlertSeverity::Critical.color();
        assert_eq!(r, 1.0);
        assert!((g - 0.027).abs() < 0.002);
        assert!((b - 0.227).abs() < 0.002);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn alert_manager_zero_capacity_evicts_immediately() {
        let mut mgr = AlertManager::new(0);
        mgr.add(info_alert("gone"));
        assert!(mgr.active().is_empty());
    }
}
