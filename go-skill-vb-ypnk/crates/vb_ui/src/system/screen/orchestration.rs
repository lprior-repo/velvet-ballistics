#![forbid(unsafe_code)]
//! System screen orchestration - SystemScreen.

use vb_ipc::ShardMetrics;

use crate::system::alerts::AlertManager;
use crate::system::map::ShardNode;
use crate::system::metrics::{HealthStatus, ShardDisplay, SystemMetrics};
use crate::system::queue_monitor::{QueueMonitor, QueueStatus};
use crate::system::ticker::EventTicker;
use crate::system::topology::TopologySnapshot;

use super::layout_models::LatencyBreakdown;

#[must_use]
pub fn format_queue_depth(depth: u32, capacity: u32) -> QueueStatus {
    QueueStatus::from_depth_capacity(depth, capacity)
}

#[derive(Debug, Clone)]
pub struct ShardSummaryLine {
    pub shard_id: u32,
    pub health: HealthStatus,
    pub health_label: String,
    pub queue_label: String,
    pub frame_label: String,
    pub trace_label: String,
    pub queue_status: QueueStatus,
}

pub struct SystemScreen {
    topology: TopologySnapshot,
    metrics: SystemMetrics,
    alerts: AlertManager,
    ticker: EventTicker,
    queue_monitors: Vec<QueueMonitor>,
    latency_breakdown: LatencyBreakdown,
}

const MAX_ALERTS: usize = 64;
const MAX_TICKER_EVENTS: usize = 128;

impl SystemScreen {
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
            latency_breakdown: LatencyBreakdown {
                segments: Vec::new(),
            },
        }
    }

    pub fn update_from_metrics(&mut self, m: &ShardMetrics) {
        let display = ShardDisplay::from(m);

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

        let monitor_idx = self
            .metrics
            .shards
            .iter()
            .position(|s| s.shard_id == m.shard_id);
        if let Some(idx) = monitor_idx {
            if idx >= self.queue_monitors.len() {
                self.queue_monitors
                    .resize_with(idx.saturating_add(1), QueueMonitor::new);
            }
            if let Some(monitor) = self.queue_monitors.get_mut(idx) {
                monitor.update_from_metrics(m);
            }
        }

        self.metrics.recompute();
        self.sync_topology();
    }

    #[must_use]
    pub fn active_alert_count(&self) -> usize {
        self.alerts.active().len()
    }

    #[must_use]
    pub fn critical_alert_count(&self) -> usize {
        self.alerts.critical_count()
    }

    #[must_use]
    pub fn shard_summary(&self) -> Vec<ShardSummaryLine> {
        let mut lines = Vec::with_capacity(self.metrics.shards.len());
        for (idx, shard) in self.metrics.shards.iter().enumerate() {
            let health_label = match shard.health {
                HealthStatus::Healthy => "Healthy".to_string(),
                HealthStatus::Degraded => "Degraded".to_string(),
                HealthStatus::Critical => "Critical".to_string(),
            };
            let queue_label = format!("{}/{}", shard.ready_queue_depth, shard.action_queue_depth);
            let frame_label = format!("{}/{}", shard.frame_pool_free, shard.frame_pool_total);
            let trace_label = format!("{:.0}%", shard.trace_ring_fill_pct);

            let queue_status = self
                .queue_monitors
                .get(idx)
                .map_or(QueueStatus::Normal, QueueMonitor::worst_status);

            lines.push(ShardSummaryLine {
                shard_id: shard.shard_id,
                health: shard.health,
                health_label,
                queue_label,
                frame_label,
                trace_label,
                queue_status,
            });
        }
        lines
    }

    #[must_use]
    pub fn alerts(&self) -> &AlertManager {
        &self.alerts
    }

    pub fn alerts_mut(&mut self) -> &mut AlertManager {
        &mut self.alerts
    }

    #[must_use]
    pub fn ticker(&self) -> &EventTicker {
        &self.ticker
    }

    pub fn ticker_mut(&mut self) -> &mut EventTicker {
        &mut self.ticker
    }

    #[must_use]
    pub fn topology(&self) -> &TopologySnapshot {
        &self.topology
    }

    #[must_use]
    pub fn metrics(&self) -> &SystemMetrics {
        &self.metrics
    }

    #[must_use]
    pub fn overall_health(&self) -> HealthStatus {
        self.metrics.overall_health
    }

    #[must_use]
    pub fn queue_monitors(&self) -> &[QueueMonitor] {
        &self.queue_monitors
    }

    #[must_use]
    pub fn latency_breakdown(&self) -> &LatencyBreakdown {
        &self.latency_breakdown
    }

    #[must_use]
    pub fn latency_breakdown_mut(&mut self) -> &mut LatencyBreakdown {
        &mut self.latency_breakdown
    }

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

    fn sync_topology(&mut self) {
        self.topology = TopologySnapshot::from_shards(
            self.metrics
                .shards
                .iter()
                .map(|s| {
                    ShardNode::new(
                        s.shard_id,
                        s.active_runs,
                        0,
                        s.ready_queue_depth,
                        s.action_queue_depth,
                    )
                })
                .collect(),
        );
    }
}

impl Default for SystemScreen {
    fn default() -> Self {
        Self::new()
    }
}
