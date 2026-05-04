use std::collections::HashSet;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Existing types (Alert, AlertKind, AlertManager) — used by SystemScreen
// ---------------------------------------------------------------------------

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
    /// Display priority: Info=0, Warning=1, Critical=2.
    #[must_use]
    pub const fn priority(self) -> u8 {
        match self {
            Self::Info => 0,
            Self::Warning => 1,
            Self::Critical => 2,
        }
    }

    /// Color for rendering severity badges and indicators.
    ///
    /// - Info: neon cyan `#00f5ff`
    /// - Warning: neon yellow `#ffe600`
    /// - Critical: neon red `#ff073a`
    #[must_use]
    pub const fn color(self) -> [f32; 4] {
        match self {
            Self::Info => [0.0, 0.961, 1.0, 1.0],
            Self::Warning => [1.0, 0.902, 0.0, 1.0],
            Self::Critical => [1.0, 0.027, 0.227, 1.0],
        }
    }

    /// Derive the highest-priority route for this severity level.
    ///
    /// - Critical routes to all channels (Dashboard + Notification + Pager)
    /// - Warning routes to Dashboard + Notification
    /// - Info routes to Dashboard only
    #[must_use]
    pub const fn default_route(self) -> AlertRoute {
        match self {
            Self::Critical => AlertRoute::Pager,
            Self::Warning => AlertRoute::Notification,
            Self::Info => AlertRoute::Dashboard,
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

// ---------------------------------------------------------------------------
// AlertRoute — severity-based routing destination
// ---------------------------------------------------------------------------

/// Routing destination for a system alert.
///
/// Critical alerts route to all three channels, Warning to Dashboard +
/// Notification, and Info to Dashboard only.  The route is stored on
/// `SystemAlert` so the rendering layer can filter by destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertRoute {
    /// Dashboard panel only.
    Dashboard,
    /// Dashboard panel + notification toast.
    Notification,
    /// Dashboard + notification + pager / on-call escalation.
    Pager,
}

impl AlertRoute {
    /// Whether this route includes the dashboard channel.
    #[must_use]
    pub const fn includes_dashboard(self) -> bool {
        true // all routes include dashboard
    }

    /// Whether this route includes the notification channel.
    #[must_use]
    pub const fn includes_notification(self) -> bool {
        matches!(self, Self::Notification | Self::Pager)
    }

    /// Whether this route includes the pager / escalation channel.
    #[must_use]
    pub const fn includes_pager(self) -> bool {
        matches!(self, Self::Pager)
    }
}

// ---------------------------------------------------------------------------
// AlertDedupKey — content-addressable deduplication
// ---------------------------------------------------------------------------

/// Deduplication key derived from the alert's source and fingerprint.
///
/// Two alerts with the same `(source, fingerprint)` pair are considered
/// duplicates and only the first is retained by `AlertRouter`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AlertDedupKey {
    pub source: String,
    pub fingerprint: u64,
}

// ---------------------------------------------------------------------------
// SystemAlert — routed, timestamped, acknowledgeable alert
// ---------------------------------------------------------------------------

/// A routed system alert with deduplication support.
///
/// Each `SystemAlert` carries a monotonically-increasing ID, the severity-
/// derived route, and an acknowledgement flag.
#[derive(Debug, Clone)]
pub struct SystemAlert {
    /// Monotonically-increasing alert ID assigned by `AlertRouter`.
    pub id: u64,
    /// Alert severity (Info / Warning / Critical).
    pub severity: AlertSeverity,
    /// Human-readable alert message.
    pub message: String,
    /// Originating subsystem or source tag.
    pub source: String,
    /// Content hash for deduplication (caller-supplied).
    pub fingerprint: u64,
    /// Routing destination derived from severity at insertion time.
    pub route: AlertRoute,
    /// Microsecond-precision timestamp (caller-supplied).
    pub timestamp_us: u64,
    /// Whether an operator has acknowledged this alert.
    pub acknowledged: bool,
}

// ---------------------------------------------------------------------------
// AlertRouter — severity-based routing with dedup and trim
// ---------------------------------------------------------------------------

/// Severity-based alert router with deduplication and capacity management.
///
/// - `route_alert` inserts a new alert if its `(source, fingerprint)` pair
///   has not been seen before, returning `Some(id)` on success and `None`
///   for duplicates.
/// - `acknowledge` marks an alert as acknowledged by ID.
/// - `trim` evicts the oldest acknowledged alerts when the buffer exceeds
///   `max_alerts`.
pub struct AlertRouter {
    alerts: Vec<SystemAlert>,
    next_id: u64,
    dedup_keys: HashSet<AlertDedupKey>,
    max_alerts: usize,
}

impl AlertRouter {
    /// Create a new router with the given capacity.
    ///
    /// A `max_alerts` of zero means the router will not store any alerts
    /// (all calls to `route_alert` return `None`).
    #[must_use]
    pub fn new(max_alerts: usize) -> Self {
        Self {
            alerts: Vec::new(),
            next_id: 1,
            dedup_keys: HashSet::new(),
            max_alerts,
        }
    }

    /// Attempt to insert a new alert.
    ///
    /// Returns `Some(id)` if the alert was new (not a duplicate), or `None`
    /// if a `(source, fingerprint)` pair was already seen or the router has
    /// zero capacity.
    ///
    /// The route is automatically derived from the severity:
    /// - Critical -> Pager (all channels)
    /// - Warning  -> Notification (dashboard + toast)
    /// - Info     -> Dashboard only
    pub fn route_alert(
        &mut self,
        severity: AlertSeverity,
        message: String,
        source: String,
        fingerprint: u64,
        timestamp_us: u64,
    ) -> Option<u64> {
        if self.max_alerts == 0 {
            return None;
        }

        // Guard: if next_id has saturated to u64::MAX, we cannot assign a
        // unique ID, so reject the alert to avoid duplicate IDs that would
        // break acknowledge().
        if self.next_id == u64::MAX {
            return None;
        }

        let key = AlertDedupKey {
            source: source.clone(),
            fingerprint,
        };
        if self.dedup_keys.contains(&key) {
            return None;
        }

        let route = severity.default_route();
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);

        self.dedup_keys.insert(key);
        self.alerts.push(SystemAlert {
            id,
            severity,
            message,
            source,
            fingerprint,
            route,
            timestamp_us,
            acknowledged: false,
        });

        Some(id)
    }

    /// Mark an alert as acknowledged by ID.
    ///
    /// Returns `true` if an alert with the given ID was found and updated
    /// (including if it was already acknowledged), or `false` if no such
    /// alert exists.
    pub fn acknowledge(&mut self, id: u64) -> bool {
        for alert in &mut self.alerts {
            if alert.id == id {
                alert.acknowledged = true;
                return true;
            }
        }
        false
    }

    /// Return references to all alerts matching the given severity.
    #[must_use]
    pub fn alerts_by_severity(&self, severity: AlertSeverity) -> Vec<&SystemAlert> {
        self.alerts
            .iter()
            .filter(|a| a.severity == severity)
            .collect()
    }

    /// Return references to all unacknowledged Critical alerts.
    #[must_use]
    pub fn unacknowledged_criticals(&self) -> Vec<&SystemAlert> {
        self.alerts
            .iter()
            .filter(|a| a.severity == AlertSeverity::Critical && !a.acknowledged)
            .collect()
    }

    /// Evict oldest acknowledged alerts when the buffer exceeds `max_alerts`.
    ///
    /// Unacknowledged alerts are never trimmed.  Dedup keys for evicted
    /// alerts are removed so that a future `route_alert` with the same
    /// `(source, fingerprint)` will be accepted again.
    pub fn trim(&mut self) {
        if self.alerts.len() <= self.max_alerts {
            return;
        }

        // Safe: we just checked alerts.len() > max_alerts.
        #[allow(clippy::arithmetic_side_effects)]
        let excess = self.alerts.len() - self.max_alerts;

        // Collect indices of acknowledged alerts, limited to the excess count,
        // preferring the oldest (lowest index) first.
        let mut acked_indices: Vec<usize> = self
            .alerts
            .iter()
            .enumerate()
            .filter(|(_, a)| a.acknowledged)
            .map(|(i, _)| i)
            .take(excess)
            .collect();

        // Nothing to trim -- no acknowledged alerts.
        if acked_indices.is_empty() {
            return;
        }

        // Remove dedup keys for evicted alerts.
        for &idx in &acked_indices {
            // Safe: idx came from a valid enumerate() over self.alerts.
            #[allow(clippy::indexing_slicing)]
            let alert = &self.alerts[idx];
            self.dedup_keys.remove(&AlertDedupKey {
                source: alert.source.clone(),
                fingerprint: alert.fingerprint,
            });
        }

        // Remove in reverse index order to keep earlier indices valid.
        acked_indices.reverse();
        for idx in acked_indices {
            self.alerts.remove(idx);
        }
    }

    /// Read-only access to the full alert list.
    #[must_use]
    pub fn alerts(&self) -> &[SystemAlert] {
        &self.alerts
    }

    /// Number of alerts currently stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.alerts.len()
    }

    /// Whether the router is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.alerts.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- AlertSeverity::priority tests --

    #[test]
    fn severity_info_priority_is_zero() {
        assert_eq!(AlertSeverity::Info.priority(), 0);
    }

    #[test]
    fn severity_warning_priority_is_one() {
        assert_eq!(AlertSeverity::Warning.priority(), 1);
    }

    #[test]
    fn severity_critical_priority_is_two() {
        assert_eq!(AlertSeverity::Critical.priority(), 2);
    }

    // -- AlertSeverity::color tests (existing) --

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

    // -- AlertSeverity::default_route tests --

    #[test]
    fn info_routes_to_dashboard() {
        assert_eq!(AlertSeverity::Info.default_route(), AlertRoute::Dashboard);
    }

    #[test]
    fn warning_routes_to_notification() {
        assert_eq!(
            AlertSeverity::Warning.default_route(),
            AlertRoute::Notification
        );
    }

    #[test]
    fn critical_routes_to_pager() {
        assert_eq!(
            AlertSeverity::Critical.default_route(),
            AlertRoute::Pager
        );
    }

    // -- AlertRoute channel membership tests --

    #[test]
    fn dashboard_route_includes_only_dashboard() {
        let route = AlertRoute::Dashboard;
        assert!(route.includes_dashboard());
        assert!(!route.includes_notification());
        assert!(!route.includes_pager());
    }

    #[test]
    fn notification_route_includes_dashboard_and_notification() {
        let route = AlertRoute::Notification;
        assert!(route.includes_dashboard());
        assert!(route.includes_notification());
        assert!(!route.includes_pager());
    }

    #[test]
    fn pager_route_includes_all_channels() {
        let route = AlertRoute::Pager;
        assert!(route.includes_dashboard());
        assert!(route.includes_notification());
        assert!(route.includes_pager());
    }

    // -- Existing AlertManager tests --

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
    fn alert_manager_zero_capacity_evicts_immediately() {
        let mut mgr = AlertManager::new(0);
        mgr.add(info_alert("gone"));
        assert!(mgr.active().is_empty());
    }

    // -- AlertRouter tests --

    #[test]
    fn router_new_is_empty() {
        let router = AlertRouter::new(10);
        assert!(router.is_empty());
        assert_eq!(router.len(), 0);
        assert!(router.alerts().is_empty());
    }

    #[test]
    fn router_route_alert_returns_incrementing_ids() {
        let mut router = AlertRouter::new(10);
        let id1 = router.route_alert(
            AlertSeverity::Info,
            "msg1".to_string(),
            "src".to_string(),
            1,
            100,
        );
        let id2 = router.route_alert(
            AlertSeverity::Warning,
            "msg2".to_string(),
            "src".to_string(),
            2,
            200,
        );
        assert_eq!(id1, Some(1));
        assert_eq!(id2, Some(2));
        assert_eq!(router.len(), 2);
    }

    #[test]
    fn router_route_alert_assigns_correct_route_from_severity() {
        let mut router = AlertRouter::new(10);

        router.route_alert(
            AlertSeverity::Info,
            "info".to_string(),
            "a".to_string(),
            1,
            100,
        );
        router.route_alert(
            AlertSeverity::Warning,
            "warn".to_string(),
            "b".to_string(),
            2,
            200,
        );
        router.route_alert(
            AlertSeverity::Critical,
            "crit".to_string(),
            "c".to_string(),
            3,
            300,
        );

        let alerts = router.alerts();
        assert_eq!(alerts[0].route, AlertRoute::Dashboard);
        assert_eq!(alerts[1].route, AlertRoute::Notification);
        assert_eq!(alerts[2].route, AlertRoute::Pager);
    }

    #[test]
    fn router_route_alert_stores_all_fields() {
        let mut router = AlertRouter::new(10);
        router.route_alert(
            AlertSeverity::Critical,
            "shard overloaded".to_string(),
            "shard-0".to_string(),
            0xABCD,
            9_000_000,
        );

        let alert = &router.alerts()[0];
        assert_eq!(alert.id, 1);
        assert_eq!(alert.severity, AlertSeverity::Critical);
        assert_eq!(alert.message, "shard overloaded");
        assert_eq!(alert.source, "shard-0");
        assert_eq!(alert.fingerprint, 0xABCD);
        assert_eq!(alert.route, AlertRoute::Pager);
        assert_eq!(alert.timestamp_us, 9_000_000);
        assert!(!alert.acknowledged);
    }

    #[test]
    fn router_deduplicates_same_source_and_fingerprint() {
        let mut router = AlertRouter::new(10);
        let id1 = router.route_alert(
            AlertSeverity::Warning,
            "queue pressure".to_string(),
            "shard-0".to_string(),
            42,
            100,
        );
        let id2 = router.route_alert(
            AlertSeverity::Warning,
            "queue pressure".to_string(),
            "shard-0".to_string(),
            42,
            200,
        );
        assert_eq!(id1, Some(1));
        assert_eq!(id2, None);
        assert_eq!(router.len(), 1);
    }

    #[test]
    fn router_allows_same_fingerprint_from_different_source() {
        let mut router = AlertRouter::new(10);
        let id1 = router.route_alert(
            AlertSeverity::Info,
            "msg".to_string(),
            "source-a".to_string(),
            99,
            100,
        );
        let id2 = router.route_alert(
            AlertSeverity::Info,
            "msg".to_string(),
            "source-b".to_string(),
            99,
            200,
        );
        assert_eq!(id1, Some(1));
        assert_eq!(id2, Some(2));
        assert_eq!(router.len(), 2);
    }

    #[test]
    fn router_zero_capacity_returns_none() {
        let mut router = AlertRouter::new(0);
        let id = router.route_alert(
            AlertSeverity::Critical,
            "msg".to_string(),
            "src".to_string(),
            1,
            100,
        );
        assert_eq!(id, None);
        assert!(router.is_empty());
    }

    #[test]
    fn router_acknowledge_existing_alert_returns_true() {
        let mut router = AlertRouter::new(10);
        router.route_alert(
            AlertSeverity::Critical,
            "msg".to_string(),
            "src".to_string(),
            1,
            100,
        );
        assert!(router.acknowledge(1));
        assert!(router.alerts()[0].acknowledged);
    }

    #[test]
    fn router_acknowledge_nonexistent_returns_false() {
        let mut router = AlertRouter::new(10);
        assert!(!router.acknowledge(999));
    }

    #[test]
    fn router_acknowledge_idempotent() {
        let mut router = AlertRouter::new(10);
        router.route_alert(
            AlertSeverity::Warning,
            "msg".to_string(),
            "src".to_string(),
            1,
            100,
        );
        assert!(router.acknowledge(1));
        assert!(router.acknowledge(1));
        assert!(router.alerts()[0].acknowledged);
    }

    #[test]
    fn router_alerts_by_severity_filters_correctly() {
        let mut router = AlertRouter::new(10);
        router.route_alert(
            AlertSeverity::Info,
            "i1".to_string(),
            "a".to_string(),
            1,
            100,
        );
        router.route_alert(
            AlertSeverity::Critical,
            "c1".to_string(),
            "b".to_string(),
            2,
            200,
        );
        router.route_alert(
            AlertSeverity::Info,
            "i2".to_string(),
            "c".to_string(),
            3,
            300,
        );
        router.route_alert(
            AlertSeverity::Warning,
            "w1".to_string(),
            "d".to_string(),
            4,
            400,
        );

        let infos = router.alerts_by_severity(AlertSeverity::Info);
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].message, "i1");
        assert_eq!(infos[1].message, "i2");

        let warnings = router.alerts_by_severity(AlertSeverity::Warning);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].message, "w1");

        let criticals = router.alerts_by_severity(AlertSeverity::Critical);
        assert_eq!(criticals.len(), 1);
        assert_eq!(criticals[0].message, "c1");
    }

    #[test]
    fn router_alerts_by_severity_returns_empty_for_no_match() {
        let router = AlertRouter::new(10);
        let result = router.alerts_by_severity(AlertSeverity::Critical);
        assert!(result.is_empty());
    }

    #[test]
    fn router_unacknowledged_criticals_returns_only_unacked_criticals() {
        let mut router = AlertRouter::new(10);
        router.route_alert(
            AlertSeverity::Critical,
            "c1".to_string(),
            "a".to_string(),
            1,
            100,
        );
        router.route_alert(
            AlertSeverity::Critical,
            "c2".to_string(),
            "b".to_string(),
            2,
            200,
        );
        router.route_alert(
            AlertSeverity::Info,
            "i1".to_string(),
            "c".to_string(),
            3,
            300,
        );

        // Acknowledge the first critical.
        router.acknowledge(1);

        let unacked = router.unacknowledged_criticals();
        assert_eq!(unacked.len(), 1);
        assert_eq!(unacked[0].message, "c2");
    }

    #[test]
    fn router_unacknowledged_criticals_empty_when_all_acked() {
        let mut router = AlertRouter::new(10);
        router.route_alert(
            AlertSeverity::Critical,
            "c1".to_string(),
            "a".to_string(),
            1,
            100,
        );
        router.acknowledge(1);
        assert!(router.unacknowledged_criticals().is_empty());
    }

    #[test]
    fn router_unacknowledged_criticals_empty_when_no_criticals() {
        let mut router = AlertRouter::new(10);
        router.route_alert(
            AlertSeverity::Info,
            "i1".to_string(),
            "a".to_string(),
            1,
            100,
        );
        assert!(router.unacknowledged_criticals().is_empty());
    }

    #[test]
    fn router_trim_removes_oldest_acknowledged_when_over_capacity() {
        let mut router = AlertRouter::new(3);
        router.route_alert(
            AlertSeverity::Info,
            "a".to_string(),
            "s".to_string(),
            1,
            100,
        );
        router.route_alert(
            AlertSeverity::Warning,
            "b".to_string(),
            "s".to_string(),
            2,
            200,
        );
        router.route_alert(
            AlertSeverity::Critical,
            "c".to_string(),
            "s".to_string(),
            3,
            300,
        );

        // Acknowledge the first alert so it becomes trimmable.
        router.acknowledge(1);

        // Add one more to exceed capacity.
        router.route_alert(
            AlertSeverity::Info,
            "d".to_string(),
            "s".to_string(),
            4,
            400,
        );

        router.trim();
        assert_eq!(router.len(), 3);
        // Alert "a" should have been trimmed.
        let messages: Vec<&str> = router.alerts().iter().map(|a| a.message.as_str()).collect();
        assert_eq!(messages, vec!["b", "c", "d"]);
    }

    #[test]
    fn router_trim_does_not_remove_unacknowledged_alerts() {
        let mut router = AlertRouter::new(2);
        router.route_alert(
            AlertSeverity::Critical,
            "c1".to_string(),
            "a".to_string(),
            1,
            100,
        );
        router.route_alert(
            AlertSeverity::Critical,
            "c2".to_string(),
            "b".to_string(),
            2,
            200,
        );
        // Add a third to exceed capacity.
        router.route_alert(
            AlertSeverity::Critical,
            "c3".to_string(),
            "c".to_string(),
            3,
            300,
        );

        // None acknowledged — trim should not remove anything.
        router.trim();
        assert_eq!(router.len(), 3);
    }

    #[test]
    fn router_trim_removes_dedup_key_so_alert_can_be_rerouted() {
        let mut router = AlertRouter::new(2);
        router.route_alert(
            AlertSeverity::Info,
            "msg".to_string(),
            "src".to_string(),
            42,
            100,
        );
        router.acknowledge(1);
        router.route_alert(
            AlertSeverity::Warning,
            "w".to_string(),
            "src2".to_string(),
            99,
            200,
        );
        // Exceed capacity to trigger trim.
        router.route_alert(
            AlertSeverity::Critical,
            "c".to_string(),
            "src3".to_string(),
            100,
            300,
        );

        router.trim();
        // The acknowledged alert should be gone; re-routing same key should succeed.
        let id = router.route_alert(
            AlertSeverity::Info,
            "msg again".to_string(),
            "src".to_string(),
            42,
            400,
        );
        assert!(id.is_some());
    }

    #[test]
    fn router_trim_is_noop_when_under_capacity() {
        let mut router = AlertRouter::new(10);
        router.route_alert(
            AlertSeverity::Info,
            "a".to_string(),
            "s".to_string(),
            1,
            100,
        );
        router.acknowledge(1);
        router.trim();
        assert_eq!(router.len(), 1);
    }

    #[test]
    fn router_next_id_saturates_returns_none() {
        let mut router = AlertRouter::new(10);
        // Manually push next_id to max to test saturation guard.
        router.next_id = u64::MAX;
        let id = router.route_alert(
            AlertSeverity::Info,
            "msg".to_string(),
            "src".to_string(),
            1,
            100,
        );
        // At u64::MAX the guard triggers: no unique ID can be assigned.
        assert_eq!(id, None);
        assert_eq!(router.len(), 0);

        // Second call also returns None — still saturated.
        let id2 = router.route_alert(
            AlertSeverity::Info,
            "msg2".to_string(),
            "src2".to_string(),
            2,
            200,
        );
        assert_eq!(id2, None);
        assert_eq!(router.len(), 0);
    }
}
