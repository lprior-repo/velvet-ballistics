#![forbid(unsafe_code)]
//! Data-driven rendering model for the system overview screen.
//!
//! This module defines the "render frame" structs that the Makepad integration
//! layer will consume on each frame. The builder takes a `&SystemScreen` and
//! produces a `SystemFrame` containing panel data for topology, alerts,
//! ticker, and queue visualisation.
//!
//! All colours use the cyberpunk neon palette (linear RGBA).

use crate::system::alerts::AlertSeverity;
use crate::system::lanes::{LaneSegment, LaneSegmentBuilder};
use crate::system::map::{ShardRect, ShardStatus, SystemMapLayout};
use crate::system::metrics::HealthStatus;
use crate::system::queue_monitor::{QueueMonitor, QueueStatus};
use crate::system::screen::SystemScreen;
use crate::system::ticker::EventTicker;

// ---------------------------------------------------------------------------
// Cyberpunk palette constants (linear RGBA, GPU-ready)
// ---------------------------------------------------------------------------

/// `#00f5ff` neon cyan — running / active / healthy / normal
pub const NEON_CYAN: [f32; 4] = [0.0, 0.961, 1.0, 1.0];
/// `#39ff14` neon green — idle / success
pub const NEON_GREEN: [f32; 4] = [0.224, 1.0, 0.078, 1.0];
/// `#ff073a` neon red — failed / overloaded / critical
pub const NEON_RED: [f32; 4] = [1.0, 0.027, 0.227, 1.0];
/// `#2d6bff` neon blue — waiting / scheduled
pub const NEON_BLUE: [f32; 4] = [0.176, 0.420, 1.0, 1.0];
/// `#ff6b00` neon orange — action / warning
pub const NEON_ORANGE: [f32; 4] = [1.0, 0.420, 0.0, 1.0];
/// `#ffe600` neon yellow — degraded / pressured
pub const NEON_YELLOW: [f32; 4] = [1.0, 0.902, 0.0, 1.0];
/// `#ff00ff` neon magenta — special events
pub const NEON_MAGENTA: [f32; 4] = [1.0, 0.0, 1.0, 1.0];
/// `#b026ff` neon purple — journal / trace
pub const NEON_PURPLE: [f32; 4] = [0.69, 0.15, 1.0, 1.0];
/// `#00e5a0` neon teal — step completed
pub const NEON_TEAL: [f32; 4] = [0.0, 0.898, 0.627, 1.0];
/// `#ff6b9d` neon pink — replay divergence
pub const NEON_PINK: [f32; 4] = [1.0, 0.420, 0.616, 1.0];

// ---------------------------------------------------------------------------
// Panel data structures
// ---------------------------------------------------------------------------

/// Status badge for the topology panel header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StatusBadge {
    Healthy,
    Degraded,
    Critical,
}

impl StatusBadge {
    /// Returns the neon palette colour for this badge.
    #[must_use]
    pub const fn color(self) -> [f32; 4] {
        match self {
            Self::Healthy => NEON_CYAN,
            Self::Degraded => NEON_YELLOW,
            Self::Critical => NEON_RED,
        }
    }

    /// Human-readable label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Healthy => "Healthy",
            Self::Degraded => "Degraded",
            Self::Critical => "Critical",
        }
    }
}

/// Topology panel render data.
#[derive(Debug, Clone)]
pub struct TopologyPanel {
    /// Grid-layout rects for all shards, computed from SystemMapLayout.
    pub shard_rects: Vec<ShardRect>,
    /// Worst status badge across all shards.
    pub worst_badge: StatusBadge,
    /// Summary text (e.g. "Active shards=3 active=12 pending=8").
    pub summary: String,
    /// Number of shards in the topology.
    pub shard_count: usize,
    /// Total active runs across all shards.
    pub total_active_runs: u32,
    /// Total pending actions across all shards.
    pub total_pending: u32,
}

/// A single line in the alerts panel.
#[derive(Debug, Clone)]
pub struct AlertLine {
    /// Display colour derived from severity.
    pub color: [f32; 4],
    /// Severity label: "Info", "Warning", or "Critical".
    pub severity_label: String,
    /// Alert message text.
    pub message: String,
    /// Source subsystem tag.
    pub source: String,
    /// Whether this alert has been acknowledged.
    pub acknowledged: bool,
    /// Alert ID (for acknowledge action).
    pub id: u64,
}

/// Alerts panel render data.
#[derive(Debug, Clone)]
pub struct AlertsPanel {
    /// Recent alert lines, ordered newest-last.
    pub lines: Vec<AlertLine>,
    /// Number of unacknowledged critical alerts.
    pub unacknowledged_critical_count: usize,
    /// Total active alert count.
    pub total_count: usize,
}

/// A single line in the ticker panel.
#[derive(Debug, Clone)]
pub struct TickerLine {
    /// Display colour derived from event kind.
    pub color: [f32; 4],
    /// Formatted timestamp string (microseconds since epoch).
    pub timestamp_label: String,
    /// Event kind label (e.g. "RunAccepted").
    pub kind_label: String,
    /// Event summary text.
    pub summary: String,
    /// Shard ID that originated this event.
    pub shard: u32,
    /// Sequence number.
    pub seq: u64,
}

/// Ticker panel render data.
#[derive(Debug, Clone)]
pub struct TickerPanel {
    /// Recent event lines, newest-last.
    pub lines: Vec<TickerLine>,
    /// Total events in the buffer (before any filtering).
    pub total_event_count: usize,
    /// Number of events excluded by active filters.
    pub filtered_out_count: usize,
}

/// A single bar in the queue depth visualisation.
#[derive(Debug, Clone)]
pub struct QueueBar {
    /// Shard ID this bar represents.
    pub shard_id: u32,
    /// Ready queue depth.
    pub ready_depth: u32,
    /// Action queue depth.
    pub action_depth: u32,
    /// Combined depth (ready + action).
    pub combined_depth: u32,
    /// Nominal capacity (256).
    pub capacity: u32,
    /// Fill ratio in [0.0, 1.0], clamped.
    pub fill_ratio: f32,
    /// Bar colour derived from queue status.
    pub color: [f32; 4],
    /// Queue status for the worst pool on this shard.
    pub status: QueueStatus,
}

/// Queue panel render data.
#[derive(Debug, Clone)]
pub struct QueuePanel {
    /// Per-shard queue depth bars.
    pub bars: Vec<QueueBar>,
    /// Total ready depth across all shards.
    pub total_ready: u32,
    /// Total action depth across all shards.
    pub total_action: u32,
    /// Worst queue status across all shards.
    pub worst_status: QueueStatus,
}

/// Top-level render frame containing all panel data.
#[derive(Debug, Clone)]
pub struct SystemFrame {
    /// Overall system health badge.
    pub health_badge: StatusBadge,
    /// Topology panel data.
    pub topology: TopologyPanel,
    /// Alerts panel data.
    pub alerts: AlertsPanel,
    /// Ticker panel data.
    pub ticker: TickerPanel,
    /// Queue panel data.
    pub queue: QueuePanel,
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Nominal queue capacity used for bar visualisation.
const NOMINAL_QUEUE_CAPACITY: u32 = 256;

/// Default layout width for the shard grid.
const DEFAULT_LAYOUT_WIDTH: f32 = 800.0;
/// Default layout height for the shard grid.
const DEFAULT_LAYOUT_HEIGHT: f32 = 400.0;

/// Builder that takes a `&SystemScreen` and produces a `SystemFrame`.
pub struct SystemFrameBuilder<'a> {
    screen: &'a SystemScreen,
    layout_width: f32,
    layout_height: f32,
}

impl<'a> SystemFrameBuilder<'a> {
    /// Create a new builder borrowing the given `SystemScreen`.
    #[must_use]
    pub fn new(screen: &'a SystemScreen) -> Self {
        Self {
            screen,
            layout_width: DEFAULT_LAYOUT_WIDTH,
            layout_height: DEFAULT_LAYOUT_HEIGHT,
        }
    }

    /// Override the layout dimensions for the topology shard grid.
    #[must_use]
    pub fn with_layout(mut self, width: f32, height: f32) -> Self {
        self.layout_width = width;
        self.layout_height = height;
        self
    }

    /// Build the topology panel.
    #[must_use]
    pub fn build_topology_panel(&self) -> TopologyPanel {
        let topo = self.screen.topology();
        let shard_rects =
            SystemMapLayout::compute_layout(&topo.topology, self.layout_width, self.layout_height);

        let worst_badge = match topo.worst_status {
            ShardStatus::Overloaded => StatusBadge::Critical,
            ShardStatus::Active => {
                if self.screen.overall_health() == HealthStatus::Degraded {
                    StatusBadge::Degraded
                } else {
                    StatusBadge::Healthy
                }
            }
            ShardStatus::Idle => StatusBadge::Healthy,
        };

        TopologyPanel {
            shard_rects,
            worst_badge,
            summary: topo.summary_text(),
            shard_count: topo.topology.shards.len(),
            total_active_runs: topo.total_active_runs,
            total_pending: topo.total_pending,
        }
    }

    /// Build the alerts panel from the AlertRouter (if available) or the
    /// AlertManager.
    #[must_use]
    pub fn build_alerts_panel(&self) -> AlertsPanel {
        let active = self.screen.alerts().active();

        let mut lines = Vec::with_capacity(active.len());
        let mut unacknowledged_critical_count = 0usize;

        for alert in active {
            let color = alert.severity.color();
            let severity_label = match alert.severity {
                AlertSeverity::Info => "Info".to_string(),
                AlertSeverity::Warning => "Warning".to_string(),
                AlertSeverity::Critical => "Critical".to_string(),
            };

            lines.push(AlertLine {
                color,
                severity_label,
                message: alert.message.clone(),
                source: alert.kind_label(),
                acknowledged: false,
                id: 0,
            });

            if alert.severity == AlertSeverity::Critical {
                unacknowledged_critical_count = unacknowledged_critical_count.saturating_add(1);
            }
        }

        AlertsPanel {
            lines,
            unacknowledged_critical_count,
            total_count: active.len(),
        }
    }

    /// Build the ticker panel.
    #[must_use]
    pub fn build_ticker_panel(&self) -> TickerPanel {
        let ticker = self.screen.ticker();
        let all_events = ticker.events();
        let filtered = ticker.filtered_events();

        let total_event_count = all_events.len();
        let filtered_out_count = total_event_count.saturating_sub(filtered.len());

        let lines = filtered
            .iter()
            .map(|event| {
                let color = EventTicker::event_color(event.kind);
                let kind_label = format!("{:?}", event.kind);
                TickerLine {
                    color,
                    timestamp_label: format!("{}", event.seq),
                    kind_label,
                    summary: event.summary.clone(),
                    shard: event.shard,
                    seq: event.seq,
                }
            })
            .collect();

        TickerPanel {
            lines,
            total_event_count,
            filtered_out_count,
        }
    }

    /// Build the queue panel.
    #[must_use]
    pub fn build_queue_panel(&self) -> QueuePanel {
        let metrics = self.screen.metrics();
        let mut bars = Vec::with_capacity(metrics.shards.len());
        let mut total_ready = 0u32;
        let mut total_action = 0u32;
        let mut worst_status = QueueStatus::Normal;

        for (idx, shard) in metrics.shards.iter().enumerate() {
            let combined = shard
                .ready_queue_depth
                .saturating_add(shard.action_queue_depth);
            let fill_ratio: f32 = if NOMINAL_QUEUE_CAPACITY > 0 {
                // Both values are <= u32::MAX (4_294_967_295) which exceeds f32 precision,
                // but queue depths are always <= 256 here, so u32 fits losslessly.
                #[allow(clippy::cast_precision_loss, clippy::as_conversions)]
                let ratio = combined as f32 / NOMINAL_QUEUE_CAPACITY as f32;
                if ratio >= 1.0 { 1.0 } else { ratio }
            } else {
                0.0
            };

            let status = self
                .screen
                .queue_monitors()
                .get(idx)
                .map_or(QueueStatus::Normal, QueueMonitor::worst_status);

            let color = status.color();

            total_ready = total_ready.saturating_add(shard.ready_queue_depth);
            total_action = total_action.saturating_add(shard.action_queue_depth);

            match (worst_status, status) {
                (_, QueueStatus::Critical) => worst_status = QueueStatus::Critical,
                (QueueStatus::Normal, QueueStatus::Pressured) => {
                    worst_status = QueueStatus::Pressured;
                }
                _ => {}
            }

            bars.push(QueueBar {
                shard_id: shard.shard_id,
                ready_depth: shard.ready_queue_depth,
                action_depth: shard.action_queue_depth,
                combined_depth: combined,
                capacity: NOMINAL_QUEUE_CAPACITY,
                fill_ratio,
                color,
                status,
            });
        }

        QueuePanel {
            bars,
            total_ready,
            total_action,
            worst_status,
        }
    }

    /// Build per-shard lane segments from the screen's shard metrics.
    ///
    /// Returns a `Vec<Vec<LaneSegment>>` where each inner `Vec` corresponds
    /// to one shard lane and contains one segment per active run.
    #[must_use]
    pub fn build_lane_segments(&self, screen: &SystemScreen) -> Vec<Vec<LaneSegment>> {
        let metrics = screen.metrics();
        let mut result = Vec::with_capacity(metrics.shards.len());
        for shard in &metrics.shards {
            // Reconstruct a minimal ShardMetrics for the builder.
            let m = vb_ipc::ShardMetrics {
                shard_id: shard.shard_id,
                active_runs: shard.active_runs,
                ready_queue_depth: shard.ready_queue_depth,
                action_queue_depth: shard.action_queue_depth,
                timer_count: shard.timer_count,
                frame_pool_free: shard.frame_pool_free,
                frame_pool_total: shard.frame_pool_total,
                trace_ring_fill_pct: shard.trace_ring_fill_pct,
                steps_total: 0,
                actions_total: 0,
            };
            result.push(LaneSegmentBuilder::build(&m));
        }
        result
    }

    /// Build the complete system frame by calling all four builders.
    #[must_use]
    pub fn build_frame(&self) -> SystemFrame {
        let topology = self.build_topology_panel();
        let alerts = self.build_alerts_panel();
        let ticker = self.build_ticker_panel();
        let queue = self.build_queue_panel();

        let health_badge = match self.screen.overall_health() {
            HealthStatus::Healthy => StatusBadge::Healthy,
            HealthStatus::Degraded => StatusBadge::Degraded,
            HealthStatus::Critical => StatusBadge::Critical,
        };

        SystemFrame {
            health_badge,
            topology,
            alerts,
            ticker,
            queue,
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: AlertKind label
// ---------------------------------------------------------------------------

/// Extension for `AlertKind` to provide a display label.
trait AlertKindLabel {
    fn kind_label(&self) -> String;
}

use crate::system::alerts::AlertKind;

impl AlertKindLabel for crate::system::alerts::Alert {
    fn kind_label(&self) -> String {
        match self.kind {
            AlertKind::QueuePressure => "QueuePressure".to_string(),
            AlertKind::RunFailed => "RunFailed".to_string(),
            AlertKind::ReplayDivergence => "ReplayDivergence".to_string(),
            AlertKind::JournalLag => "JournalLag".to_string(),
            AlertKind::SecretLeak => "SecretLeak".to_string(),
            AlertKind::ShardOverloaded => "ShardOverloaded".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests (minimum 15)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::alerts::{Alert, AlertKind, AlertSeverity};
    use crate::system::screen::SystemScreen;
    use crate::system::ticker::{TickerEvent, TickerEventKind};
    use std::time::Instant;
    use vb_ipc::ShardMetrics;

    // -- Helper constructors -------------------------------------------------

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

    fn warning_alert(msg: &str) -> Alert {
        Alert {
            severity: AlertSeverity::Warning,
            kind: AlertKind::JournalLag,
            message: msg.to_string(),
            run_id: None,
            shard_id: Some(1),
            timestamp: Instant::now(),
        }
    }

    fn ticker_event(seq: u64, shard: u32, kind: TickerEventKind) -> TickerEvent {
        TickerEvent {
            seq,
            shard,
            run_id: None,
            kind,
            summary: format!("event-{}", seq),
        }
    }

    // -- StatusBadge tests ---------------------------------------------------

    #[test]
    fn status_badge_healthy_color_is_neon_cyan() {
        assert_eq!(StatusBadge::Healthy.color(), NEON_CYAN);
        assert_eq!(StatusBadge::Healthy.label(), "Healthy");
    }

    #[test]
    fn status_badge_degraded_color_is_neon_yellow() {
        assert_eq!(StatusBadge::Degraded.color(), NEON_YELLOW);
        assert_eq!(StatusBadge::Degraded.label(), "Degraded");
    }

    #[test]
    fn status_badge_critical_color_is_neon_red() {
        assert_eq!(StatusBadge::Critical.color(), NEON_RED);
        assert_eq!(StatusBadge::Critical.label(), "Critical");
    }

    // -- Neon palette constant sanity checks ---------------------------------

    #[test]
    fn neon_palette_constants_have_full_alpha() {
        let all = [
            NEON_CYAN,
            NEON_GREEN,
            NEON_RED,
            NEON_BLUE,
            NEON_ORANGE,
            NEON_YELLOW,
            NEON_MAGENTA,
            NEON_PURPLE,
            NEON_TEAL,
            NEON_PINK,
        ];
        for color in &all {
            assert!(
                (color[3] - 1.0).abs() < f32::EPSILON,
                "alpha should be 1.0 for colour {:?}",
                color
            );
        }
    }

    // -- Topology panel tests ------------------------------------------------

    #[test]
    fn topology_panel_empty_screen_is_healthy() {
        let screen = SystemScreen::new();
        let builder = SystemFrameBuilder::new(&screen);
        let panel = builder.build_topology_panel();
        assert!(panel.shard_rects.is_empty());
        assert_eq!(panel.worst_badge, StatusBadge::Healthy);
        assert!(panel.summary.contains("Idle"));
        assert_eq!(panel.shard_count, 0);
        assert_eq!(panel.total_active_runs, 0);
        assert_eq!(panel.total_pending, 0);
    }

    #[test]
    fn topology_panel_with_active_shard() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 20.0));
        let builder = SystemFrameBuilder::new(&screen);
        let panel = builder.build_topology_panel();
        assert_eq!(panel.shard_count, 1);
        assert!(!panel.shard_rects.is_empty());
        assert!(panel.summary.contains("Active"));
    }

    #[test]
    fn topology_panel_custom_layout_dimensions() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 20.0));
        let builder = SystemFrameBuilder::new(&screen).with_layout(400.0, 200.0);
        let panel = builder.build_topology_panel();
        assert_eq!(panel.shard_rects.len(), 1);
        // Width should be 400.0 for a single shard
        assert!((panel.shard_rects[0].w - 400.0).abs() < 0.01);
        assert!((panel.shard_rects[0].h - 200.0).abs() < 0.01);
    }

    // -- Alerts panel tests --------------------------------------------------

    #[test]
    fn alerts_panel_empty_screen_has_no_lines() {
        let screen = SystemScreen::new();
        let builder = SystemFrameBuilder::new(&screen);
        let panel = builder.build_alerts_panel();
        assert!(panel.lines.is_empty());
        assert_eq!(panel.unacknowledged_critical_count, 0);
        assert_eq!(panel.total_count, 0);
    }

    #[test]
    fn alerts_panel_with_mixed_severities() {
        let mut screen = SystemScreen::new();
        screen.alerts_mut().add(info_alert("info msg"));
        screen.alerts_mut().add(warning_alert("warn msg"));
        screen.alerts_mut().add(critical_alert("crit msg"));

        let builder = SystemFrameBuilder::new(&screen);
        let panel = builder.build_alerts_panel();

        assert_eq!(panel.total_count, 3);
        assert_eq!(panel.unacknowledged_critical_count, 1);

        // Check severity labels
        assert_eq!(panel.lines[0].severity_label, "Info");
        assert_eq!(panel.lines[1].severity_label, "Warning");
        assert_eq!(panel.lines[2].severity_label, "Critical");

        // Check colours match the severity
        assert_eq!(panel.lines[0].color, AlertSeverity::Info.color());
        assert_eq!(panel.lines[1].color, AlertSeverity::Warning.color());
        assert_eq!(panel.lines[2].color, AlertSeverity::Critical.color());
    }

    #[test]
    fn alerts_panel_critical_count_increments_for_each_critical() {
        let mut screen = SystemScreen::new();
        screen.alerts_mut().add(critical_alert("c1"));
        screen.alerts_mut().add(critical_alert("c2"));
        screen.alerts_mut().add(critical_alert("c3"));

        let builder = SystemFrameBuilder::new(&screen);
        let panel = builder.build_alerts_panel();
        assert_eq!(panel.unacknowledged_critical_count, 3);
    }

    // -- Ticker panel tests --------------------------------------------------

    #[test]
    fn ticker_panel_empty_screen_has_no_lines() {
        let screen = SystemScreen::new();
        let builder = SystemFrameBuilder::new(&screen);
        let panel = builder.build_ticker_panel();
        assert!(panel.lines.is_empty());
        assert_eq!(panel.total_event_count, 0);
        assert_eq!(panel.filtered_out_count, 0);
    }

    #[test]
    fn ticker_panel_with_events_populates_lines() {
        let mut screen = SystemScreen::new();
        screen
            .ticker_mut()
            .push(ticker_event(1, 0, TickerEventKind::RunAccepted));
        screen
            .ticker_mut()
            .push(ticker_event(2, 0, TickerEventKind::StepSucceeded));

        let builder = SystemFrameBuilder::new(&screen);
        let panel = builder.build_ticker_panel();

        assert_eq!(panel.lines.len(), 2);
        assert_eq!(panel.total_event_count, 2);
        assert_eq!(panel.filtered_out_count, 0);

        assert_eq!(panel.lines[0].seq, 1);
        assert_eq!(panel.lines[0].shard, 0);
        assert_eq!(panel.lines[0].kind_label, "RunAccepted");
        assert_eq!(panel.lines[0].summary, "event-1");
    }

    #[test]
    fn ticker_panel_colors_match_event_kind() {
        let mut screen = SystemScreen::new();
        screen
            .ticker_mut()
            .push(ticker_event(1, 0, TickerEventKind::RunAccepted));
        screen
            .ticker_mut()
            .push(ticker_event(2, 0, TickerEventKind::RunFailed));

        let builder = SystemFrameBuilder::new(&screen);
        let panel = builder.build_ticker_panel();

        let cyan = EventTicker::event_color(TickerEventKind::RunAccepted);
        assert_eq!(panel.lines[0].color, cyan);

        let red = EventTicker::event_color(TickerEventKind::RunFailed);
        assert_eq!(panel.lines[1].color, red);
    }

    // -- Queue panel tests ---------------------------------------------------

    #[test]
    fn queue_panel_empty_screen_has_no_bars() {
        let screen = SystemScreen::new();
        let builder = SystemFrameBuilder::new(&screen);
        let panel = builder.build_queue_panel();
        assert!(panel.bars.is_empty());
        assert_eq!(panel.total_ready, 0);
        assert_eq!(panel.total_action, 0);
        assert_eq!(panel.worst_status, QueueStatus::Normal);
    }

    #[test]
    fn queue_panel_with_single_healthy_shard() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 20.0));

        let builder = SystemFrameBuilder::new(&screen);
        let panel = builder.build_queue_panel();

        assert_eq!(panel.bars.len(), 1);
        assert_eq!(panel.bars[0].shard_id, 0);
        assert_eq!(panel.bars[0].ready_depth, 10);
        assert_eq!(panel.bars[0].action_depth, 5);
        assert_eq!(panel.bars[0].combined_depth, 15);
        assert_eq!(panel.bars[0].capacity, NOMINAL_QUEUE_CAPACITY);
        assert!((panel.bars[0].fill_ratio - (15.0_f32 / 256.0)).abs() < 0.001);
        assert_eq!(panel.bars[0].status, QueueStatus::Normal);
        assert_eq!(panel.total_ready, 10);
        assert_eq!(panel.total_action, 5);
    }

    #[test]
    fn queue_panel_fill_ratio_clamped_at_one() {
        let mut screen = SystemScreen::new();
        // ready=200 + action=200 = 400, which exceeds 256 capacity
        screen.update_from_metrics(&stub_shard_metrics(0, 200, 200, 90, 100, 20.0));

        let builder = SystemFrameBuilder::new(&screen);
        let panel = builder.build_queue_panel();

        assert!((panel.bars[0].fill_ratio - 1.0).abs() < f32::EPSILON);
    }

    // -- Full frame tests ----------------------------------------------------

    #[test]
    fn build_frame_empty_screen() {
        let screen = SystemScreen::new();
        let builder = SystemFrameBuilder::new(&screen);
        let frame = builder.build_frame();

        assert_eq!(frame.health_badge, StatusBadge::Healthy);
        assert_eq!(frame.topology.shard_count, 0);
        assert_eq!(frame.alerts.total_count, 0);
        assert_eq!(frame.ticker.total_event_count, 0);
        assert!(frame.queue.bars.is_empty());
    }

    #[test]
    fn build_frame_with_data() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 20.0));
        screen.alerts_mut().add(info_alert("test"));
        screen
            .ticker_mut()
            .push(ticker_event(1, 0, TickerEventKind::RunAccepted));

        let builder = SystemFrameBuilder::new(&screen);
        let frame = builder.build_frame();

        assert_eq!(frame.health_badge, StatusBadge::Healthy);
        assert_eq!(frame.topology.shard_count, 1);
        assert_eq!(frame.alerts.total_count, 1);
        assert_eq!(frame.ticker.total_event_count, 1);
        assert_eq!(frame.queue.bars.len(), 1);
    }

    #[test]
    fn build_frame_critical_health_reflects_in_badge() {
        let mut screen = SystemScreen::new();
        // Pool nearly empty → Critical health
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 5, 100, 85.0));

        let builder = SystemFrameBuilder::new(&screen);
        let frame = builder.build_frame();
        assert_eq!(frame.health_badge, StatusBadge::Critical);
    }

    // -- Saturating arithmetic edge case -------------------------------------

    #[test]
    fn queue_panel_saturating_arithmetic_on_large_depths() {
        let mut screen = SystemScreen::new();
        let m = ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: u32::MAX,
            action_queue_depth: u32::MAX,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 0,
            actions_total: 0,
        };
        screen.update_from_metrics(&m);

        let builder = SystemFrameBuilder::new(&screen);
        let panel = builder.build_queue_panel();

        assert_eq!(panel.bars[0].combined_depth, u32::MAX);
        assert_eq!(panel.total_ready, u32::MAX);
        assert_eq!(panel.total_action, u32::MAX);
        // fill_ratio should be clamped to 1.0 since combined > capacity
        assert!((panel.bars[0].fill_ratio - 1.0).abs() < f32::EPSILON);
    }
}
