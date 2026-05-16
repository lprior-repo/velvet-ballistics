#![forbid(unsafe_code)]
//! System overview screen with placeholder data.

use crate::system::queue_monitor::QueueStatus;

use super::layout_models::{
    ActivityLane, ActivitySegment, AlertCard, AlertStack, EventTickerPanel, JournalStatusRow,
    LatencyBreakdown, LatencySegment, QueueMonitorBar, QueueMonitorPanel, TickerChip,
    TopologyPanel, TopologyShardRow, SYS_NEON_CYAN, SYS_NEON_GREEN, SYS_NEON_ORANGE,
    SYS_NEON_PURPLE, SYS_NEON_RED, SYS_NEON_YELLOW,
};

pub struct SystemOverviewScreen {
    pub health_label: String,
    pub health_color: String,
    pub shard_count: u32,
    pub total_active_runs: u32,
    pub total_pending: u32,
    pub topology_panel: TopologyPanel,
    pub activity_lanes: Vec<ActivityLane>,
    pub queue_monitor: QueueMonitorPanel,
    pub event_ticker: EventTickerPanel,
    pub alert_stack: AlertStack,
    pub latency_breakdown: LatencyBreakdown,
}

pub(crate) fn build_segments(shard_id: u32, count: u32) -> Vec<ActivitySegment> {
    if count == 0 {
        return Vec::new();
    }
    let count_f = f64::from(count);
    let base_id = 80_000_u64.saturating_add(u64::from(shard_id).saturating_mul(10_000));

    let capacity = match usize::try_from(count) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut segments = Vec::with_capacity(capacity);
    let mut accumulated = 0.0_f64;

    for i in 0..count {
        let run_id = base_id.saturating_add(u64::from(i));
        let is_last = i.saturating_add(1) == count;
        let width = if is_last {
            1.0 - accumulated
        } else {
            1.0 / count_f
        };

        let color = if i % 2 == 0 {
            String::from(SYS_NEON_CYAN)
        } else {
            String::from(SYS_NEON_ORANGE)
        };

        accumulated += width;

        segments.push(ActivitySegment {
            run_id,
            width_ratio: if width < 0.0 { 0.0 } else { width },
            color,
            label: format!("R{}", run_id),
        });
    }

    segments
}

impl SystemOverviewScreen {
    #[must_use]
    pub fn new() -> Self {
        let health_label = String::from("Active");
        let health_color = String::from(SYS_NEON_CYAN);
        let shard_count: u32 = 4;
        let total_active_runs: u32 = 37;
        let total_pending: u32 = 25;

        let shard_rows = vec![
            TopologyShardRow {
                shard_id: 0,
                status_label: String::from("Active"),
                status_color: String::from(SYS_NEON_CYAN),
                active_runs: 12,
                bg_color: String::from("#0d1a2a"),
            },
            TopologyShardRow {
                shard_id: 1,
                status_label: String::from("Idle"),
                status_color: String::from(SYS_NEON_GREEN),
                active_runs: 0,
                bg_color: String::from("#0d1a0d"),
            },
            TopologyShardRow {
                shard_id: 2,
                status_label: String::from("Overloaded"),
                status_color: String::from(SYS_NEON_RED),
                active_runs: 20,
                bg_color: String::from("#1a0d0d"),
            },
            TopologyShardRow {
                shard_id: 3,
                status_label: String::from("Active"),
                status_color: String::from(SYS_NEON_CYAN),
                active_runs: 5,
                bg_color: String::from("#0d1a2a"),
            },
        ];

        let journal_status = JournalStatusRow {
            label: String::from("healthy"),
            label_color: String::from(SYS_NEON_GREEN),
            queue_depth: 3,
        };

        let topology_panel = TopologyPanel {
            shard_rows,
            journal_status,
            timer_count: 42,
            ipc_connections: 3,
        };

        let activity_lanes = vec![
            ActivityLane {
                shard_id: 0,
                active_runs: 12,
                ready_queue_depth: 8,
                action_queue_depth: 4,
                steps_per_sec: 150.0,
                segments: build_segments(0, 12),
                lane_label_color: String::from(SYS_NEON_CYAN),
            },
            ActivityLane {
                shard_id: 1,
                active_runs: 0,
                ready_queue_depth: 0,
                action_queue_depth: 0,
                steps_per_sec: 0.0,
                segments: Vec::new(),
                lane_label_color: String::from(SYS_NEON_GREEN),
            },
            ActivityLane {
                shard_id: 2,
                active_runs: 20,
                ready_queue_depth: 15,
                action_queue_depth: 10,
                steps_per_sec: 45.0,
                segments: build_segments(2, 20),
                lane_label_color: String::from(SYS_NEON_RED),
            },
            ActivityLane {
                shard_id: 3,
                active_runs: 5,
                ready_queue_depth: 3,
                action_queue_depth: 2,
                steps_per_sec: 120.0,
                segments: build_segments(3, 5),
                lane_label_color: String::from(SYS_NEON_CYAN),
            },
        ];

        let bars = vec![
            QueueMonitorBar {
                label: String::from("Ready"),
                fill_color: String::from(SYS_NEON_CYAN),
                fill_ratio: 0.3,
                depth_text: String::from("26/256"),
                status: QueueStatus::Normal,
            },
            QueueMonitorBar {
                label: String::from("Action"),
                fill_color: String::from(SYS_NEON_ORANGE),
                fill_ratio: 0.15,
                depth_text: String::from("16/256"),
                status: QueueStatus::Normal,
            },
            QueueMonitorBar {
                label: String::from("Journal"),
                fill_color: String::from(SYS_NEON_PURPLE),
                fill_ratio: 0.05,
                depth_text: String::from("3/64"),
                status: QueueStatus::Normal,
            },
            QueueMonitorBar {
                label: String::from("Trace"),
                fill_color: String::from(SYS_NEON_YELLOW),
                fill_ratio: 0.35,
                depth_text: String::from("35%"),
                status: QueueStatus::Normal,
            },
            QueueMonitorBar {
                label: String::from("Frame"),
                fill_color: String::from(SYS_NEON_GREEN),
                fill_ratio: 0.25,
                depth_text: String::from("192/256"),
                status: QueueStatus::Normal,
            },
        ];
        let queue_monitor = QueueMonitorPanel { bars };

        let chips = vec![
            TickerChip {
                kind_label: String::from("RunAccepted"),
                bg_color: String::from("#0d2a3a"),
                text_color: String::from(SYS_NEON_CYAN),
                summary: String::from("Run #8172 accepted"),
                seq: 1,
            },
            TickerChip {
                kind_label: String::from("StepStarted"),
                bg_color: String::from("#0d2a0d"),
                text_color: String::from(SYS_NEON_GREEN),
                summary: String::from("Step 0: Do github.issue.create"),
                seq: 2,
            },
            TickerChip {
                kind_label: String::from("ActionScheduled"),
                bg_color: String::from("#1a1a3a"),
                text_color: String::from(SYS_NEON_PURPLE),
                summary: String::from("Action #42 scheduled on shard-0"),
                seq: 3,
            },
            TickerChip {
                kind_label: String::from("ActionCompleted"),
                bg_color: String::from("#0d2a2a"),
                text_color: String::from(SYS_NEON_CYAN),
                summary: String::from("Action #42 completed"),
                seq: 4,
            },
            TickerChip {
                kind_label: String::from("RunFinished"),
                bg_color: String::from("#0d2a0d"),
                text_color: String::from(SYS_NEON_GREEN),
                summary: String::from("Run #8172 finished"),
                seq: 5,
            },
        ];
        let event_ticker = EventTickerPanel { chips };

        let alerts = vec![
            AlertCard {
                severity_label: String::from("Critical"),
                severity_color: String::from(SYS_NEON_RED),
                message: String::from("shard-2 overloaded: 20/20 runs"),
                source: String::from("topology"),
                bg_color: String::from("#1a0d0d"),
                acknowledged: false,
            },
            AlertCard {
                severity_label: String::from("Warning"),
                severity_color: String::from(SYS_NEON_YELLOW),
                message: String::from("queue pressure on shard-2: ready 15/256"),
                source: String::from("queue-monitor"),
                bg_color: String::from("#1a1a0d"),
                acknowledged: false,
            },
            AlertCard {
                severity_label: String::from("Info"),
                severity_color: String::from(SYS_NEON_CYAN),
                message: String::from("IPC reconnection to shard-1 established"),
                source: String::from("ipc-bridge"),
                bg_color: String::from("#0d1a2a"),
                acknowledged: true,
            },
        ];
        let alert_stack = AlertStack { alerts };

        let segments = vec![
            LatencySegment {
                label: String::from("submit -> admit"),
                avg_us: 325,
                display: String::from("325us"),
                fill_color: String::from(SYS_NEON_CYAN),
                width_ratio: 0.000_1,
                p50_us: 300,
                p95_us: 380,
                p99_us: 450,
            },
            LatencySegment {
                label: String::from("admit -> step"),
                avg_us: 110,
                display: String::from("110us"),
                fill_color: String::from(SYS_NEON_GREEN),
                width_ratio: 0.000_034,
                p50_us: 100,
                p95_us: 130,
                p99_us: 160,
            },
            LatencySegment {
                label: String::from("step -> action"),
                avg_us: 12_500,
                display: String::from("12.5ms"),
                fill_color: String::from(SYS_NEON_ORANGE),
                width_ratio: 0.003_8,
                p50_us: 12_000,
                p95_us: 14_000,
                p99_us: 18_000,
            },
            LatencySegment {
                label: String::from("action -> completed"),
                avg_us: 3_250_000,
                display: String::from("3.25s"),
                fill_color: String::from(SYS_NEON_RED),
                width_ratio: 1.0,
                p50_us: 3_200_000,
                p95_us: 3_500_000,
                p99_us: 4_000_000,
            },
            LatencySegment {
                label: String::from("completed -> finish"),
                avg_us: 225,
                display: String::from("225us"),
                fill_color: String::from(SYS_NEON_GREEN),
                width_ratio: 0.000_069,
                p50_us: 200,
                p95_us: 280,
                p99_us: 350,
            },
        ];
        let latency_breakdown = LatencyBreakdown { segments };

        Self {
            health_label,
            health_color,
            shard_count,
            total_active_runs,
            total_pending,
            topology_panel,
            activity_lanes,
            queue_monitor,
            event_ticker,
            alert_stack,
            latency_breakdown,
        }
    }

    #[must_use]
    pub fn title_text(&self) -> String {
        String::from("vb")
    }

    #[must_use]
    pub fn page_title(&self) -> String {
        String::from("System Overview")
    }

    #[must_use]
    pub fn status_summary(&self) -> String {
        format!(
            "{} shards={} active={} pending={}",
            self.health_label, self.shard_count, self.total_active_runs, self.total_pending
        )
    }

    #[must_use]
    pub fn topology_header_text(&self) -> String {
        String::from("TOPOLOGY")
    }

    #[must_use]
    pub fn activity_header_text(&self) -> String {
        String::from("ACTIVITY LANES")
    }

    #[must_use]
    pub fn queue_monitor_header_text(&self) -> String {
        String::from("QUEUE MONITOR")
    }

    #[must_use]
    pub fn event_ticker_header_text(&self) -> String {
        String::from("EVENT TICKER")
    }

    #[must_use]
    pub fn alert_stack_header_text(&self) -> String {
        String::from("ALERTS")
    }

    #[must_use]
    pub fn latency_header_text(&self) -> String {
        String::from("LATENCY BREAKDOWN")
    }

    #[must_use]
    pub fn shard_row_count(&self) -> usize {
        self.topology_panel.shard_rows.len()
    }

    #[must_use]
    pub fn lane_count(&self) -> usize {
        self.activity_lanes.len()
    }

    #[must_use]
    pub fn queue_bar_count(&self) -> usize {
        self.queue_monitor.bars.len()
    }

    #[must_use]
    pub fn ticker_chip_count(&self) -> usize {
        self.event_ticker.chips.len()
    }

    #[must_use]
    pub fn alert_count(&self) -> usize {
        self.alert_stack.alerts.len()
    }

    #[must_use]
    pub fn latency_segment_count(&self) -> usize {
        self.latency_breakdown.segments.len()
    }

    #[must_use]
    pub fn unacknowledged_alert_count(&self) -> usize {
        self.alert_stack
            .alerts
            .iter()
            .filter(|a| !a.acknowledged)
            .count()
    }

    #[must_use]
    pub fn topology_panel(&self) -> &TopologyPanel {
        &self.topology_panel
    }

    #[must_use]
    pub fn activity_lanes(&self) -> &[ActivityLane] {
        &self.activity_lanes
    }

    #[must_use]
    pub fn queue_monitor_panel(&self) -> &QueueMonitorPanel {
        &self.queue_monitor
    }

    #[must_use]
    pub fn event_ticker_panel(&self) -> &EventTickerPanel {
        &self.event_ticker
    }

    #[must_use]
    pub fn alert_stack_panel(&self) -> &AlertStack {
        &self.alert_stack
    }

    #[must_use]
    pub fn latency_breakdown_panel(&self) -> &LatencyBreakdown {
        &self.latency_breakdown
    }
}

impl Default for SystemOverviewScreen {
    fn default() -> Self {
        Self::new()
    }
}
