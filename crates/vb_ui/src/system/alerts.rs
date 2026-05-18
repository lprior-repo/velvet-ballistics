#![forbid(unsafe_code)]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
        assert_eq!(AlertSeverity::Critical.default_route(), AlertRoute::Pager);
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

    // =========================================================================
    // BLACKHAT security review tests
    // =========================================================================

    /// SEVERITY: Medium
    /// DESCRIPTION: AlertRouter::trim only removes acknowledged alerts, but the
    /// dedup_keys HashSet grows without bound for unacknowledged alerts. An
    /// attacker who sends unique (source, fingerprint) pairs with Critical
    /// severity (which are never acknowledged) can exhaust memory because trim
    /// never evicts their dedup keys. Even after alerts are trimmed, the dedup
    /// keys remain, preventing re-routing of those alerts and leaking memory.
    /// The dedup set is unbounded and only grows.
    #[test]
    fn blackhat_dedup_keys_grow_unbounded_for_unacked_alerts() {
        let mut router = AlertRouter::new(3);
        // Fill with 3 critical (unacked) alerts with unique keys.
        for i in 0..3u64 {
            router.route_alert(
                AlertSeverity::Critical,
                format!("crit-{i}"),
                format!("src-{i}"),
                i,
                100 + i,
            );
        }
        assert_eq!(router.len(), 3);

        // Try to add a 4th -- buffer is full and none are acked, so trim
        // cannot evict anything, but the alert IS accepted because it has
        // a new dedup key. The buffer grows beyond max_alerts.
        let id4 = router.route_alert(
            AlertSeverity::Critical,
            "crit-overflow".to_string(),
            "src-overflow".to_string(),
            999,
            999,
        );
        // The alert is accepted (new dedup key) even though we're at capacity.
        // This means the router has NO hard cap on alert count for unacked alerts.
        assert!(id4.is_some(), "alert accepted despite exceeding max_alerts");
        assert_eq!(router.len(), 4, "buffer exceeds max_alerts of 3");

        // trim cannot remove anything because nothing is acknowledged.
        router.trim();
        assert_eq!(router.len(), 4, "trim is powerless against unacked alerts");
    }

    /// SEVERITY: Low
    /// DESCRIPTION: AlertManager::add uses Vec::remove(0) for eviction which is
    /// O(n). Combined with rapid alert insertion at max_alerts capacity, this
    /// creates O(n^2) total time for n insertions. Not a correctness bug but a
    /// denial-of-service vector if max_alerts is set very large.
    #[test]
    fn blackhat_alert_manager_add_eviction_is_on_linear_time() {
        // Demonstrate that add with eviction works correctly even under pressure.
        let mut mgr = AlertManager::new(5);
        // Fill to capacity then overflow to trigger eviction path.
        for i in 0..100 {
            mgr.add(Alert {
                severity: AlertSeverity::Info,
                kind: AlertKind::QueuePressure,
                message: format!("alert-{i}"),
                run_id: None,
                shard_id: None,
                timestamp: Instant::now(),
            });
        }
        // Should still be capped at max_alerts.
        assert_eq!(mgr.active().len(), 5);
        // Most recent alerts should be retained.
        assert_eq!(mgr.active()[4].message, "alert-99");
    }

    /// SEVERITY: Medium
    /// DESCRIPTION: When AlertRouter::route_alert is called with next_id at
    /// u64::MAX - 1, it assigns id = MAX-1, then next_id becomes MAX via
    /// saturating_add. The NEXT call is rejected by the guard. However, the
    /// alert with id = MAX-1 can still be acknowledged and trimmed normally.
    /// The issue is that after saturation, no more alerts can EVER be routed,
    /// even if all existing alerts are trimmed -- the router becomes permanently
    /// unusable. This is a latent state-machine deadlock.
    #[test]
    fn blackhat_router_permanently_dead_after_id_saturation() {
        let mut router = AlertRouter::new(10);
        // Simulate reaching the saturation point.
        router.next_id = u64::MAX;
        // Router rejects all new alerts.
        let result = router.route_alert(
            AlertSeverity::Info,
            "msg".to_string(),
            "src".to_string(),
            1,
            100,
        );
        assert_eq!(result, None);

        // Even if we acknowledge and trim everything, router stays dead.
        // (There's nothing to trim since nothing was added, but conceptually
        // even if existing alerts were trimmed, next_id stays at MAX forever.)
        assert_eq!(router.next_id, u64::MAX, "next_id is permanently saturated");
    }

    /// SEVERITY: Low
    /// DESCRIPTION: AlertRouter::trim removes dedup keys for evicted alerts,
    /// which means the same (source, fingerprint) pair can be re-routed after
    /// trim. However, if the original alert was NOT acknowledged and was NOT
    /// evicted (because trim only evicts acknowledged alerts), the dedup key
    /// stays. This is correct behavior, but it means that an alert storm with
    /// unique fingerprints can bypass the dedup entirely since each has a
    /// unique key.
    #[test]
    fn blackhat_unique_fingerprints_bypass_dedup_entirely() {
        let mut router = AlertRouter::new(100);
        // Insert 50 alerts with unique fingerprints from the same source.
        for i in 0..50u64 {
            let id = router.route_alert(
                AlertSeverity::Info,
                format!("msg-{i}"),
                "same-source".to_string(),
                i, // unique fingerprint per alert
                i * 100,
            );
            assert!(id.is_some(), "each unique fingerprint should be accepted");
        }
        assert_eq!(router.len(), 50);
        // All 50 dedup keys are in the set -- no actual dedup happened
        // because fingerprints are unique.
        assert_eq!(router.dedup_keys.len(), 50);
    }

    /// SEVERITY: Low
    /// DESCRIPTION: Alert dismiss at index 0 is O(n) due to Vec::remove(0),
    /// which shifts all remaining elements. If max_alerts is large and dismiss
    /// is called repeatedly on index 0, total cost is O(n^2).
    #[test]
    fn blackhat_dismiss_index_zero_shifts_all_remaining() {
        let mut mgr = AlertManager::new(10);
        for i in 0..10 {
            mgr.add(Alert {
                severity: AlertSeverity::Info,
                kind: AlertKind::QueuePressure,
                message: format!("alert-{i}"),
                run_id: None,
                shard_id: None,
                timestamp: Instant::now(),
            });
        }
        // Dismiss from front repeatedly -- each shifts remaining elements.
        mgr.dismiss(0);
        assert_eq!(mgr.active().len(), 9);
        assert_eq!(mgr.active()[0].message, "alert-1");
        mgr.dismiss(0);
        assert_eq!(mgr.active()[0].message, "alert-2");
    }

    // =========================================================================
    // Additional comprehensive coverage tests
    // =========================================================================

    #[test]
    fn alert_struct_fields_preserved() {
        let ts = Instant::now();
        let alert = Alert {
            severity: AlertSeverity::Warning,
            kind: AlertKind::SecretLeak,
            message: "secret exposed".to_string(),
            run_id: Some(123),
            shard_id: Some(5),
            timestamp: ts,
        };
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert_eq!(alert.kind, AlertKind::SecretLeak);
        assert_eq!(alert.message, "secret exposed");
        assert_eq!(alert.run_id, Some(123));
        assert_eq!(alert.shard_id, Some(5));
        assert_eq!(alert.timestamp, ts);
    }

    #[test]
    fn alert_clone_preserves_data() {
        let alert = Alert {
            severity: AlertSeverity::Critical,
            kind: AlertKind::ReplayDivergence,
            message: "divergence detected".to_string(),
            run_id: None,
            shard_id: None,
            timestamp: Instant::now(),
        };
        let cloned = alert.clone();
        assert_eq!(cloned.severity, alert.severity);
        assert_eq!(cloned.kind, alert.kind);
        assert_eq!(cloned.message, alert.message);
    }

    #[test]
    fn alert_debug_format_contains_fields() {
        let alert = Alert {
            severity: AlertSeverity::Info,
            kind: AlertKind::JournalLag,
            message: "lagging".to_string(),
            run_id: Some(1),
            shard_id: Some(2),
            timestamp: Instant::now(),
        };
        let debug = format!("{alert:?}");
        assert!(debug.contains("severity"));
        assert!(debug.contains("message"));
    }

    #[test]
    fn alert_severity_ordering() {
        assert!(AlertSeverity::Critical.priority() > AlertSeverity::Warning.priority());
        assert!(AlertSeverity::Warning.priority() > AlertSeverity::Info.priority());
    }

    #[test]
    fn alert_severity_copy_and_equality() {
        let sev = AlertSeverity::Warning;
        let copied = sev;
        assert_eq!(sev, copied);
        assert_ne!(sev, AlertSeverity::Critical);
    }

    #[test]
    fn alert_severity_debug_format() {
        assert!(format!("{:?}", AlertSeverity::Info).contains("Info"));
        assert!(format!("{:?}", AlertSeverity::Warning).contains("Warning"));
        assert!(format!("{:?}", AlertSeverity::Critical).contains("Critical"));
    }

    #[test]
    fn alert_kind_variants_exist() {
        let kinds = [
            AlertKind::QueuePressure,
            AlertKind::RunFailed,
            AlertKind::ReplayDivergence,
            AlertKind::JournalLag,
            AlertKind::SecretLeak,
            AlertKind::ShardOverloaded,
        ];
        // Verify Copy, Clone, PartialEq, Eq, Debug all work.
        for kind in kinds {
            let copied = kind;
            assert_eq!(kind, copied);
            let _ = format!("{kind:?}");
        }
        // Verify all are distinct.
        for i in 0..kinds.len() {
            for j in (i + 1)..kinds.len() {
                assert_ne!(kinds[i], kinds[j]);
            }
        }
    }

    #[test]
    fn alert_manager_dismiss_last_element() {
        let mut mgr = AlertManager::new(10);
        mgr.add(info_alert("first"));
        mgr.add(info_alert("second"));
        mgr.add(info_alert("third"));
        mgr.dismiss(2);
        assert_eq!(mgr.active().len(), 2);
        assert_eq!(mgr.active()[0].message, "first");
        assert_eq!(mgr.active()[1].message, "second");
    }

    #[test]
    fn alert_manager_dismiss_only_element() {
        let mut mgr = AlertManager::new(10);
        mgr.add(info_alert("only"));
        mgr.dismiss(0);
        assert!(mgr.active().is_empty());
    }

    #[test]
    fn alert_manager_add_multiple_severities() {
        let mut mgr = AlertManager::new(10);
        mgr.add(Alert {
            severity: AlertSeverity::Info,
            kind: AlertKind::QueuePressure,
            message: "info".to_string(),
            run_id: None,
            shard_id: None,
            timestamp: Instant::now(),
        });
        mgr.add(Alert {
            severity: AlertSeverity::Warning,
            kind: AlertKind::JournalLag,
            message: "warning".to_string(),
            run_id: None,
            shard_id: None,
            timestamp: Instant::now(),
        });
        mgr.add(Alert {
            severity: AlertSeverity::Critical,
            kind: AlertKind::RunFailed,
            message: "critical".to_string(),
            run_id: Some(1),
            shard_id: Some(0),
            timestamp: Instant::now(),
        });
        assert_eq!(mgr.active().len(), 3);
        assert_eq!(mgr.critical_count(), 1);
    }

    #[test]
    fn alert_dedup_key_equality_and_hash() {
        let key1 = AlertDedupKey {
            source: "shard-0".to_string(),
            fingerprint: 42,
        };
        let key2 = AlertDedupKey {
            source: "shard-0".to_string(),
            fingerprint: 42,
        };
        assert_eq!(key1, key2);

        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(key1.clone());
        assert!(set.contains(&key2));
    }

    #[test]
    fn alert_dedup_key_inequality() {
        let key1 = AlertDedupKey {
            source: "shard-0".to_string(),
            fingerprint: 42,
        };
        let key2 = AlertDedupKey {
            source: "shard-1".to_string(),
            fingerprint: 42,
        };
        let key3 = AlertDedupKey {
            source: "shard-0".to_string(),
            fingerprint: 99,
        };
        assert_ne!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn alert_dedup_key_debug_format() {
        let key = AlertDedupKey {
            source: "test".to_string(),
            fingerprint: 123,
        };
        let debug = format!("{key:?}");
        assert!(debug.contains("source"));
        assert!(debug.contains("fingerprint"));
    }

    #[test]
    fn system_alert_fields_preserved() {
        let alert = SystemAlert {
            id: 42,
            severity: AlertSeverity::Warning,
            message: "something is off".to_string(),
            source: "monitor".to_string(),
            fingerprint: 0xDEAD_BEEF,
            route: AlertRoute::Notification,
            timestamp_us: 1_700_000_000,
            acknowledged: false,
        };
        assert_eq!(alert.id, 42);
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert_eq!(alert.message, "something is off");
        assert_eq!(alert.source, "monitor");
        assert_eq!(alert.fingerprint, 0xDEAD_BEEF);
        assert_eq!(alert.route, AlertRoute::Notification);
        assert_eq!(alert.timestamp_us, 1_700_000_000);
        assert!(!alert.acknowledged);
    }

    #[test]
    fn system_alert_debug_format() {
        let alert = SystemAlert {
            id: 1,
            severity: AlertSeverity::Critical,
            message: "test".to_string(),
            source: "src".to_string(),
            fingerprint: 0,
            route: AlertRoute::Pager,
            timestamp_us: 0,
            acknowledged: true,
        };
        let debug = format!("{alert:?}");
        assert!(debug.contains("id"));
        assert!(debug.contains("acknowledged"));
    }

    #[test]
    fn alert_route_debug_variants() {
        assert!(format!("{:?}", AlertRoute::Dashboard).contains("Dashboard"));
        assert!(format!("{:?}", AlertRoute::Notification).contains("Notification"));
        assert!(format!("{:?}", AlertRoute::Pager).contains("Pager"));
    }

    #[test]
    fn alert_route_copy_and_equality() {
        let route = AlertRoute::Notification;
        let copied = route;
        assert_eq!(route, copied);
        assert_ne!(route, AlertRoute::Dashboard);
        assert_ne!(route, AlertRoute::Pager);
    }

    #[test]
    fn router_alerts_by_severity_on_empty_router() {
        let router = AlertRouter::new(10);
        assert!(router.alerts_by_severity(AlertSeverity::Info).is_empty());
        assert!(router.alerts_by_severity(AlertSeverity::Warning).is_empty());
        assert!(
            router
                .alerts_by_severity(AlertSeverity::Critical)
                .is_empty()
        );
    }

    #[test]
    fn router_route_alert_increments_id_starting_at_one() {
        let mut router = AlertRouter::new(10);
        let ids: Vec<Option<u64>> = (0..5)
            .map(|i| {
                router.route_alert(
                    AlertSeverity::Info,
                    format!("msg-{i}"),
                    format!("src-{i}"),
                    u64::from(i),
                    i * 100,
                )
            })
            .collect();
        assert_eq!(ids, vec![Some(1), Some(2), Some(3), Some(4), Some(5)]);
    }

    #[test]
    fn router_acknowledge_marks_correct_alert_only() {
        let mut router = AlertRouter::new(10);
        router.route_alert(
            AlertSeverity::Info,
            "a".to_string(),
            "s".to_string(),
            1,
            100,
        );
        router.route_alert(
            AlertSeverity::Info,
            "b".to_string(),
            "s".to_string(),
            2,
            200,
        );
        router.route_alert(
            AlertSeverity::Info,
            "c".to_string(),
            "s".to_string(),
            3,
            300,
        );

        // Acknowledge only id=2.
        assert!(router.acknowledge(2));
        assert!(!router.alerts()[0].acknowledged);
        assert!(router.alerts()[1].acknowledged);
        assert!(!router.alerts()[2].acknowledged);
    }

    #[test]
    fn router_trim_with_multiple_acknowledged() {
        let mut router = AlertRouter::new(3);
        // Add 4 alerts to exceed capacity.
        for i in 0..4u64 {
            router.route_alert(
                AlertSeverity::Info,
                format!("msg-{i}"),
                format!("src-{i}"),
                i,
                i * 100,
            );
        }
        // Acknowledge first two (oldest).
        router.acknowledge(1);
        router.acknowledge(2);

        router.trim();
        // Should have removed 1 acknowledged alert (excess = 4 - 3 = 1).
        assert_eq!(router.len(), 3);
        // Alert id=1 should be gone, id=2 may remain.
        let ids: Vec<u64> = router.alerts().iter().map(|a| a.id).collect();
        assert!(!ids.contains(&1), "id=1 should have been trimmed");
    }

    #[test]
    fn router_trim_noop_at_exact_capacity() {
        let mut router = AlertRouter::new(3);
        for i in 0..3u64 {
            router.route_alert(
                AlertSeverity::Info,
                format!("msg-{i}"),
                format!("src-{i}"),
                i,
                i * 100,
            );
        }
        router.acknowledge(1);
        router.trim();
        assert_eq!(router.len(), 3);
    }

    #[test]
    fn router_unacknowledged_criticals_all_are_criticals() {
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
        let unacked = router.unacknowledged_criticals();
        assert_eq!(unacked.len(), 2);
        for alert in unacked {
            assert_eq!(alert.severity, AlertSeverity::Critical);
            assert!(!alert.acknowledged);
        }
    }

    #[test]
    fn router_alerts_preserves_insertion_order() {
        let mut router = AlertRouter::new(10);
        let severities = [
            AlertSeverity::Info,
            AlertSeverity::Critical,
            AlertSeverity::Warning,
            AlertSeverity::Info,
        ];
        for (i, &sev) in severities.iter().enumerate() {
            router.route_alert(
                sev,
                format!("msg-{i}"),
                format!("src-{i}"),
                u64::try_from(i).unwrap_or(u64::MAX),
                100,
            );
        }
        let alerts = router.alerts();
        assert_eq!(alerts.len(), 4);
        assert_eq!(alerts[0].severity, AlertSeverity::Info);
        assert_eq!(alerts[1].severity, AlertSeverity::Critical);
        assert_eq!(alerts[2].severity, AlertSeverity::Warning);
        assert_eq!(alerts[3].severity, AlertSeverity::Info);
    }

    #[test]
    fn router_same_source_different_fingerprint_both_accepted() {
        let mut router = AlertRouter::new(10);
        let id1 = router.route_alert(
            AlertSeverity::Info,
            "msg1".to_string(),
            "same-source".to_string(),
            1,
            100,
        );
        let id2 = router.route_alert(
            AlertSeverity::Warning,
            "msg2".to_string(),
            "same-source".to_string(),
            2,
            200,
        );
        assert_eq!(id1, Some(1));
        assert_eq!(id2, Some(2));
        assert_eq!(router.len(), 2);
    }

    #[test]
    fn router_dedup_after_trim_allows_reroute() {
        let mut router = AlertRouter::new(2);
        router.route_alert(
            AlertSeverity::Info,
            "msg".to_string(),
            "src".to_string(),
            42,
            100,
        );
        router.acknowledge(1);

        // Exceed capacity so trim will evict.
        router.route_alert(
            AlertSeverity::Warning,
            "w1".to_string(),
            "s2".to_string(),
            10,
            200,
        );
        router.route_alert(
            AlertSeverity::Critical,
            "c1".to_string(),
            "s3".to_string(),
            11,
            300,
        );

        router.trim();
        // The acknowledged alert should be trimmed, removing its dedup key.
        // Re-routing same (source, fingerprint) should succeed.
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
    fn router_len_and_is_empty_consistent() {
        let mut router = AlertRouter::new(5);
        assert!(router.is_empty());
        assert_eq!(router.len(), 0);

        router.route_alert(
            AlertSeverity::Info,
            "msg".to_string(),
            "s".to_string(),
            1,
            100,
        );
        assert!(!router.is_empty());
        assert_eq!(router.len(), 1);
    }

    #[test]
    fn alert_manager_eviction_at_boundary() {
        let mut mgr = AlertManager::new(3);
        mgr.add(info_alert("a"));
        mgr.add(info_alert("b"));
        mgr.add(info_alert("c"));
        // At capacity -- no eviction yet.
        assert_eq!(mgr.active().len(), 3);
        // Add one more -> evicts oldest.
        mgr.add(info_alert("d"));
        assert_eq!(mgr.active().len(), 3);
        assert_eq!(mgr.active()[0].message, "b");
        assert_eq!(mgr.active()[2].message, "d");
    }

    #[test]
    fn alert_manager_critical_count_zero_when_no_criticals() {
        let mut mgr = AlertManager::new(10);
        mgr.add(info_alert("info only"));
        assert_eq!(mgr.critical_count(), 0);
    }

    #[test]
    fn alert_manager_all_severity_types_counted() {
        let mut mgr = AlertManager::new(10);
        mgr.add(Alert {
            severity: AlertSeverity::Info,
            kind: AlertKind::QueuePressure,
            message: "info".to_string(),
            run_id: None,
            shard_id: None,
            timestamp: Instant::now(),
        });
        mgr.add(Alert {
            severity: AlertSeverity::Critical,
            kind: AlertKind::RunFailed,
            message: "crit1".to_string(),
            run_id: None,
            shard_id: None,
            timestamp: Instant::now(),
        });
        mgr.add(Alert {
            severity: AlertSeverity::Warning,
            kind: AlertKind::JournalLag,
            message: "warn".to_string(),
            run_id: None,
            shard_id: None,
            timestamp: Instant::now(),
        });
        mgr.add(Alert {
            severity: AlertSeverity::Critical,
            kind: AlertKind::ShardOverloaded,
            message: "crit2".to_string(),
            run_id: Some(5),
            shard_id: Some(1),
            timestamp: Instant::now(),
        });
        assert_eq!(mgr.active().len(), 4);
        assert_eq!(mgr.critical_count(), 2);
    }

    #[test]
    fn router_alert_timestamp_preserved() {
        let mut router = AlertRouter::new(10);
        router.route_alert(
            AlertSeverity::Info,
            "msg".to_string(),
            "src".to_string(),
            1,
            1_234_567_890,
        );
        assert_eq!(router.alerts()[0].timestamp_us, 1_234_567_890);
    }

    #[test]
    fn router_route_alert_with_all_severity_routes() {
        let mut router = AlertRouter::new(10);

        let id_info = router.route_alert(
            AlertSeverity::Info,
            "info msg".to_string(),
            "src".to_string(),
            1,
            100,
        );
        assert_eq!(id_info, Some(1));
        assert_eq!(router.alerts()[0].route, AlertRoute::Dashboard);

        let id_warn = router.route_alert(
            AlertSeverity::Warning,
            "warn msg".to_string(),
            "src2".to_string(),
            2,
            200,
        );
        assert_eq!(id_warn, Some(2));
        assert_eq!(router.alerts()[1].route, AlertRoute::Notification);

        let id_crit = router.route_alert(
            AlertSeverity::Critical,
            "crit msg".to_string(),
            "src3".to_string(),
            3,
            300,
        );
        assert_eq!(id_crit, Some(3));
        assert_eq!(router.alerts()[2].route, AlertRoute::Pager);
    }

    #[test]
    fn router_dedup_same_source_same_fingerprint_different_severity() {
        let mut router = AlertRouter::new(10);
        let id1 = router.route_alert(
            AlertSeverity::Info,
            "msg1".to_string(),
            "src".to_string(),
            42,
            100,
        );
        // Same (source, fingerprint) -> deduped even with different severity.
        let id2 = router.route_alert(
            AlertSeverity::Critical,
            "msg2".to_string(),
            "src".to_string(),
            42,
            200,
        );
        assert_eq!(id1, Some(1));
        assert_eq!(id2, None);
        assert_eq!(router.len(), 1);
    }
}
