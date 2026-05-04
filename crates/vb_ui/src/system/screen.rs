/// System overview screen orchestration.
///
/// `SystemScreen` owns the four subsystems (topology, metrics, alerts,
/// ticker) and exposes the query methods the Makepad rendering layer will
/// call on each frame.
use vb_ipc::ShardMetrics;

use crate::system::alerts::AlertManager;
use crate::system::map::ShardNode;
use crate::system::metrics::{HealthStatus, ShardDisplay, SystemMetrics};
use crate::system::queue_monitor::{QueueMonitor, QueueStatus};
use crate::system::ticker::EventTicker;
use crate::system::topology::TopologySnapshot;

// ---------------------------------------------------------------------------
// QueueStatus display helper (re-exported for convenience)
// ---------------------------------------------------------------------------

/// Format a queue depth/capacity pair into a `QueueStatus` suitable for
/// rendering.  This is the canonical entry-point the UI layer calls.
#[must_use]
pub fn format_queue_depth(depth: u32, capacity: u32) -> QueueStatus {
    QueueStatus::from_depth_capacity(depth, capacity)
}

// ---------------------------------------------------------------------------
// ShardSummary — lightweight formatted line for the topology panel
// ---------------------------------------------------------------------------

/// A single line in the shard summary table.
#[derive(Debug, Clone)]
pub struct ShardSummaryLine {
    /// Shard index.
    pub shard_id: u32,
    /// Formatted health label: `"Healthy"`, `"Degraded"`, or `"Critical"`.
    pub health_label: String,
    /// Formatted queue string: `"{ready}/{action}"`.
    pub queue_label: String,
    /// Formatted frame pool string: `"{free}/{total}"`.
    pub frame_label: String,
    /// Trace ring fill percentage string: `"{pct}%"`.
    pub trace_label: String,
    /// Queue status for this shard (worst pool).
    pub queue_status: QueueStatus,
}

// ---------------------------------------------------------------------------
// SystemScreen
// ---------------------------------------------------------------------------

/// Top-level orchestrator for the system overview screen.
pub struct SystemScreen {
    /// Current topology snapshot.
    topology: TopologySnapshot,
    /// Aggregated system metrics.
    metrics: SystemMetrics,
    /// Alert manager.
    alerts: AlertManager,
    /// Event ticker.
    ticker: EventTicker,
    /// Per-shard queue monitors, indexed by shard position.
    queue_monitors: Vec<QueueMonitor>,
}

/// Maximum number of alerts retained in the ring buffer.
const MAX_ALERTS: usize = 64;
/// Maximum number of ticker events retained.
const MAX_TICKER_EVENTS: usize = 128;

impl SystemScreen {
    /// Create an empty system screen with sensible defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            topology: TopologySnapshot::from_shards(Vec::new()),
            metrics: SystemMetrics {
                shards: Vec::new(),
                total_active_runs: 0,
                total_ready_queue_depth: 0,
                total_action_queue_depth: 0,
                overall_health: HealthStatus::Healthy,
            },
            alerts: AlertManager::new(MAX_ALERTS),
            ticker: EventTicker::new(MAX_TICKER_EVENTS),
            queue_monitors: Vec::new(),
        }
    }

    // -- Refresh from IPC metrics ----------------------------------------

    /// Refresh internal state from a single `ShardMetrics` snapshot.
    ///
    /// This finds (or appends) the matching `ShardDisplay` in the metrics
    /// struct, recompute totals, and updates the corresponding queue monitor.
    pub fn update_from_metrics(&mut self, m: &ShardMetrics) {
        let display = ShardDisplay::from(m);

        // Update or append the shard in metrics.
        let found = self
            .metrics
            .shards
            .iter_mut()
            .find(|s| s.shard_id == m.shard_id);
        match found {
            Some(existing) => {
                *existing = display;
            }
            None => {
                self.metrics.shards.push(display);
            }
        }

        // Ensure queue monitor exists for this shard.
        let monitor_idx = self
            .metrics
            .shards
            .iter()
            .position(|s| s.shard_id == m.shard_id);
        if let Some(idx) = monitor_idx {
            if idx >= self.queue_monitors.len() {
                self.queue_monitors.resize_with(idx.saturating_add(1), QueueMonitor::new);
            }
            if let Some(monitor) = self.queue_monitors.get_mut(idx) {
                monitor.update_from_metrics(m);
            }
        }

        self.metrics.recompute();
        self.sync_topology();
    }

    // -- Accessors -------------------------------------------------------

    /// Number of currently active (non-dismissed) alerts.
    #[must_use]
    pub fn active_alert_count(&self) -> usize {
        self.alerts.active().len()
    }

    /// Number of active alerts with `Critical` severity.
    #[must_use]
    pub fn critical_alert_count(&self) -> usize {
        self.alerts.critical_count()
    }

    /// Return a formatted summary line for every shard in the topology.
    #[must_use]
    pub fn shard_summary(&self) -> Vec<ShardSummaryLine> {
        let mut lines = Vec::with_capacity(self.metrics.shards.len());
        for (idx, shard) in self.metrics.shards.iter().enumerate() {
            let health_label = match shard.health {
                HealthStatus::Healthy => "Healthy".to_string(),
                HealthStatus::Degraded => "Degraded".to_string(),
                HealthStatus::Critical => "Critical".to_string(),
            };
            let queue_label = format!(
                "{}/{}",
                shard.ready_queue_depth, shard.action_queue_depth
            );
            let frame_label = format!(
                "{}/{}",
                shard.frame_pool_free, shard.frame_pool_total
            );
            let trace_label = format!("{:.0}%", shard.trace_ring_fill_pct);

            let queue_status = self
                .queue_monitors
                .get(idx)
                .map_or(QueueStatus::Normal, QueueMonitor::worst_status);

            lines.push(ShardSummaryLine {
                shard_id: shard.shard_id,
                health_label,
                queue_label,
                frame_label,
                trace_label,
                queue_status,
            });
        }
        lines
    }

    /// Read-only reference to the underlying alert manager.
    #[must_use]
    pub fn alerts(&self) -> &AlertManager {
        &self.alerts
    }

    /// Mutable reference to the alert manager (for dismissing alerts).
    pub fn alerts_mut(&mut self) -> &mut AlertManager {
        &mut self.alerts
    }

    /// Read-only reference to the event ticker.
    #[must_use]
    pub fn ticker(&self) -> &EventTicker {
        &self.ticker
    }

    /// Mutable reference to the event ticker (for pushing events).
    pub fn ticker_mut(&mut self) -> &mut EventTicker {
        &mut self.ticker
    }

    /// Read-only reference to the topology snapshot.
    #[must_use]
    pub fn topology(&self) -> &TopologySnapshot {
        &self.topology
    }

    /// Read-only reference to the aggregated metrics.
    #[must_use]
    pub fn metrics(&self) -> &SystemMetrics {
        &self.metrics
    }

    /// Overall system health derived from aggregated metrics.
    #[must_use]
    pub fn overall_health(&self) -> HealthStatus {
        self.metrics.overall_health
    }

    /// Worst queue status across all shards.
    #[must_use]
    pub fn worst_queue_status(&self) -> QueueStatus {
        let mut worst = QueueStatus::Normal;
        for monitor in &self.queue_monitors {
            let status = monitor.worst_status();
            match (worst, status) {
                (_, QueueStatus::Critical) => worst = QueueStatus::Critical,
                (QueueStatus::Normal, QueueStatus::Pressured) => worst = QueueStatus::Pressured,
                _ => {}
            }
        }
        worst
    }

    // -- Internal helpers ------------------------------------------------

    /// Re-derive the topology snapshot from current metrics.
    fn sync_topology(&mut self) {
        self.topology = TopologySnapshot::from_shards(
            self.metrics
                .shards
                .iter()
                .map(|s| ShardNode::new(
                    s.shard_id,
                    s.active_runs,
                    0,
                    s.ready_queue_depth,
                    s.action_queue_depth,
                ))
                .collect(),
        );
    }
}

impl Default for SystemScreen {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::alerts::{Alert, AlertKind, AlertSeverity};
    use crate::system::ticker::{TickerEvent, TickerEventKind};
    use std::time::Instant;

    fn stub_shard_metrics(
        shard_id: u32,
        ready: u32,
        action: u32,
        pool_free: u32,
        pool_total: u32,
        trace_pct: f32,
    ) -> ShardMetrics {
        ShardMetrics {
            shard_id,
            active_runs: 1,
            ready_queue_depth: ready,
            action_queue_depth: action,
            timer_count: 0,
            frame_pool_free: pool_free,
            frame_pool_total: pool_total,
            trace_ring_fill_pct: trace_pct,
            steps_total: 0,
            actions_total: 0,
        }
    }

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
            run_id: Some(1),
            shard_id: Some(0),
            timestamp: Instant::now(),
        }
    }

    fn ticker_event(kind: &str) -> TickerEvent {
        TickerEvent {
            seq: 0,
            shard: 0,
            run_id: None,
            kind: match kind {
                "RunAccepted" => TickerEventKind::RunAccepted,
                "StepStarted" => TickerEventKind::StepStarted,
                "StepSucceeded" => TickerEventKind::StepSucceeded,
                "ActionScheduled" => TickerEventKind::ActionScheduled,
                "ActionCompleted" => TickerEventKind::ActionCompleted,
                "ActionFailed" => TickerEventKind::ActionFailed,
                "RunFinished" => TickerEventKind::RunFinished,
                "RunFailed" => TickerEventKind::RunFailed,
                _ => TickerEventKind::Other,
            },
            summary: kind.to_string(),
        }
    }

    // -- format_queue_depth tests --

    #[test]
    fn format_queue_depth_normal() {
        assert_eq!(format_queue_depth(10, 100), QueueStatus::Normal);
    }

    #[test]
    fn format_queue_depth_pressured() {
        assert_eq!(format_queue_depth(60, 100), QueueStatus::Pressured);
    }

    #[test]
    fn format_queue_depth_critical() {
        assert_eq!(format_queue_depth(90, 100), QueueStatus::Critical);
    }

    #[test]
    fn format_queue_depth_zero_capacity() {
        assert_eq!(format_queue_depth(0, 0), QueueStatus::Normal);
    }

    // -- SystemScreen construction tests --

    #[test]
    fn system_screen_new_starts_healthy() {
        let screen = SystemScreen::new();
        assert_eq!(screen.overall_health(), HealthStatus::Healthy);
        assert_eq!(screen.active_alert_count(), 0);
        assert_eq!(screen.critical_alert_count(), 0);
        assert!(screen.shard_summary().is_empty());
        assert_eq!(screen.worst_queue_status(), QueueStatus::Normal);
    }

    #[test]
    fn system_screen_default_matches_new() {
        let screen = SystemScreen::default();
        assert_eq!(screen.overall_health(), HealthStatus::Healthy);
    }

    // -- update_from_metrics tests --

    #[test]
    fn update_from_metrics_adds_first_shard() {
        let mut screen = SystemScreen::new();
        let m = stub_shard_metrics(0, 10, 5, 90, 100, 20.0);
        screen.update_from_metrics(&m);
        assert_eq!(screen.metrics().shards.len(), 1);
        assert_eq!(screen.metrics().shards[0].shard_id, 0);
        assert_eq!(screen.overall_health(), HealthStatus::Healthy);
    }

    #[test]
    fn update_from_metrics_replaces_existing_shard() {
        let mut screen = SystemScreen::new();
        let m1 = stub_shard_metrics(2, 10, 5, 90, 100, 20.0);
        screen.update_from_metrics(&m1);
        assert_eq!(screen.metrics().shards[0].ready_queue_depth, 10);

        let m2 = stub_shard_metrics(2, 50, 20, 80, 100, 30.0);
        screen.update_from_metrics(&m2);
        assert_eq!(screen.metrics().shards.len(), 1);
        assert_eq!(screen.metrics().shards[0].ready_queue_depth, 50);
    }

    #[test]
    fn update_from_metrics_multiple_shards_propagates_health() {
        let mut screen = SystemScreen::new();
        // Shard 0: healthy
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 20.0));
        assert_eq!(screen.overall_health(), HealthStatus::Healthy);

        // Shard 1: critical (pool used = 95/100 = 95%)
        screen.update_from_metrics(&stub_shard_metrics(1, 10, 5, 5, 100, 20.0));
        assert_eq!(screen.overall_health(), HealthStatus::Critical);
    }

    #[test]
    fn update_from_metrics_syncs_topology_shards() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(0, 5, 2, 90, 100, 10.0));
        screen.update_from_metrics(&stub_shard_metrics(1, 8, 3, 85, 100, 15.0));
        assert_eq!(screen.topology().topology.shards.len(), 2);
        assert_eq!(screen.topology().topology.shards[0].shard_id, 0);
        assert_eq!(screen.topology().topology.shards[1].shard_id, 1);
    }

    // -- Alert accessor tests --

    #[test]
    fn active_and_critical_alert_counts() {
        let mut screen = SystemScreen::new();
        assert_eq!(screen.active_alert_count(), 0);
        assert_eq!(screen.critical_alert_count(), 0);

        screen.alerts_mut().add(info_alert("info"));
        screen.alerts_mut().add(critical_alert("crit1"));
        screen.alerts_mut().add(critical_alert("crit2"));

        assert_eq!(screen.active_alert_count(), 3);
        assert_eq!(screen.critical_alert_count(), 2);
    }

    #[test]
    fn dismiss_alert_via_mut_accessor() {
        let mut screen = SystemScreen::new();
        screen.alerts_mut().add(info_alert("a"));
        screen.alerts_mut().add(info_alert("b"));
        screen.alerts_mut().dismiss(0);
        assert_eq!(screen.active_alert_count(), 1);
    }

    // -- Ticker accessor tests --

    #[test]
    fn ticker_push_and_recent() {
        let mut screen = SystemScreen::new();
        screen.ticker_mut().push(ticker_event("StepSucceeded"));
        screen.ticker_mut().push(ticker_event("ActionCompleted"));
        let events = screen.ticker().events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, TickerEventKind::StepSucceeded);
        assert_eq!(events[1].kind, TickerEventKind::ActionCompleted);
    }

    // -- shard_summary tests --

    #[test]
    fn shard_summary_empty_when_no_shards() {
        let screen = SystemScreen::new();
        assert!(screen.shard_summary().is_empty());
    }

    #[test]
    fn shard_summary_formats_single_healthy_shard() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 20.0));
        let lines = screen.shard_summary();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].shard_id, 0);
        assert_eq!(lines[0].health_label, "Healthy");
        assert_eq!(lines[0].queue_label, "10/5");
        assert_eq!(lines[0].frame_label, "90/100");
        assert_eq!(lines[0].trace_label, "20%");
        assert_eq!(lines[0].queue_status, QueueStatus::Normal);
    }

    #[test]
    fn shard_summary_formats_critical_shard() {
        let mut screen = SystemScreen::new();
        // pool 5/100 used = 95%, trace 85% → Critical
        screen.update_from_metrics(&stub_shard_metrics(3, 210, 5, 5, 100, 85.0));
        let lines = screen.shard_summary();
        assert_eq!(lines[0].health_label, "Critical");
        // queue_status should be Critical (ready=210/256 ≈ 82%)
        assert_eq!(lines[0].queue_status, QueueStatus::Critical);
    }

    #[test]
    fn shard_summary_formats_degraded_shard() {
        let mut screen = SystemScreen::new();
        // trace 75% → Degraded
        screen.update_from_metrics(&stub_shard_metrics(1, 10, 5, 60, 100, 75.0));
        let lines = screen.shard_summary();
        assert_eq!(lines[0].health_label, "Degraded");
    }

    #[test]
    fn shard_summary_multiple_shards_ordered() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(5, 10, 5, 90, 100, 10.0));
        screen.update_from_metrics(&stub_shard_metrics(2, 20, 10, 80, 100, 20.0));
        let lines = screen.shard_summary();
        assert_eq!(lines.len(), 2);
        // Ordered by insertion: shard 5 first, shard 2 second
        assert_eq!(lines[0].shard_id, 5);
        assert_eq!(lines[1].shard_id, 2);
    }

    // -- worst_queue_status tests --

    #[test]
    fn worst_queue_status_normal_with_no_monitors() {
        let screen = SystemScreen::new();
        assert_eq!(screen.worst_queue_status(), QueueStatus::Normal);
    }

    #[test]
    fn worst_queue_status_reflects_critical_shard() {
        let mut screen = SystemScreen::new();
        // Healthy shard
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 20.0));
        assert_eq!(screen.worst_queue_status(), QueueStatus::Normal);

        // Critical shard (ready=210/256 ≈ 82%)
        screen.update_from_metrics(&stub_shard_metrics(1, 210, 5, 5, 100, 85.0));
        assert_eq!(screen.worst_queue_status(), QueueStatus::Critical);
    }

    #[test]
    fn worst_queue_status_reflects_pressured_shard() {
        let mut screen = SystemScreen::new();
        // ready=130/256 ≈ 50.8% → Pressured
        screen.update_from_metrics(&stub_shard_metrics(0, 130, 5, 90, 100, 20.0));
        assert_eq!(screen.worst_queue_status(), QueueStatus::Pressured);
    }

    // -- Saturating arithmetic edge cases --

    #[test]
    fn update_from_metrics_handles_large_shard_id_without_overflow() {
        let mut screen = SystemScreen::new();
        let m = ShardMetrics {
            shard_id: u32::MAX,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        };
        screen.update_from_metrics(&m);
        assert_eq!(screen.metrics().shards.len(), 1);
        assert_eq!(screen.metrics().shards[0].shard_id, u32::MAX);
    }

    #[test]
    fn shard_summary_trace_label_rounds_zero() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 0.0));
        let lines = screen.shard_summary();
        assert_eq!(lines[0].trace_label, "0%");
    }

    #[test]
    fn shard_summary_trace_label_rounds_fractional() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 33.7));
        let lines = screen.shard_summary();
        assert_eq!(lines[0].trace_label, "34%");
    }

    #[test]
    fn topology_syncs_shards_from_metrics() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 20.0));
        // After update, topology should reflect the shard from metrics.
        assert_eq!(screen.topology().topology.shards.len(), 1);
        assert_eq!(screen.topology().topology.shards[0].shard_id, 0);
    }
}
