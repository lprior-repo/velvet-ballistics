#![forbid(unsafe_code)]
/// Queue monitoring model for the system overview screen.
///
/// Tracks pressure status across the five internal queue pools
/// (ready, action, journal, trace, frame) and maps depth/capacity
/// ratios to color-banded status levels.
use vb_ipc::ShardMetrics;

// ---------------------------------------------------------------------------
// QueueStatus
// ---------------------------------------------------------------------------

/// Pressure status for a single queue pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum QueueStatus {
    /// Depth below 50% capacity. Display color: `#00f5ff` (cyan).
    Normal,
    /// Depth at 50-79% capacity. Display color: `#ffe600` (yellow).
    Pressured,
    /// Depth at 80%+ capacity. Display color: `#ff073a` (red).
    Critical,
}

impl QueueStatus {
    /// Returns the RGBA display color for this status.
    ///
    /// * Normal    → `#00f5ff` → `[0.0, 0.961, 1.0, 1.0]`
    /// * Pressured → `#ffe600` → `[1.0, 0.902, 0.0, 1.0]`
    /// * Critical  → `#ff073a` → `[1.0, 0.027, 0.227, 1.0]`
    #[must_use]
    pub const fn color(self) -> [f32; 4] {
        match self {
            Self::Normal => [0.0, 0.961, 1.0, 1.0],
            Self::Pressured => [1.0, 0.902, 0.0, 1.0],
            Self::Critical => [1.0, 0.027, 0.227, 1.0],
        }
    }

    /// Classify a depth/capacity pair into a status band.
    ///
    /// Returns `Normal` when `capacity == 0` (no meaningful ratio).
    /// Uses integer cross-multiplication to avoid floating point.
    #[must_use]
    pub fn from_depth_capacity(depth: u32, capacity: u32) -> Self {
        if capacity == 0 {
            return Self::Normal;
        }
        // depth / capacity >= threshold  <=>  depth >= capacity * threshold
        // Using scaled thresholds (x10): 0.8 -> 8, 0.5 -> 5
        // depth * 10 >= capacity * 8  <=>  Critical
        // depth * 10 >= capacity * 5  <=>  Pressured
        let depth_x10 = depth.saturating_mul(10);
        let cap_x8 = capacity.saturating_mul(8);
        if depth_x10 >= cap_x8 {
            Self::Critical
        } else {
            let cap_x5 = capacity.saturating_mul(5);
            if depth_x10 >= cap_x5 {
                Self::Pressured
            } else {
                Self::Normal
            }
        }
    }
}

// ---------------------------------------------------------------------------
// QueueMonitor
// ---------------------------------------------------------------------------

/// Tracks pressure across the five internal queue pools.
#[derive(Debug, Clone)]
pub struct QueueMonitor {
    /// Ready-queue (commands waiting to be scheduled).
    pub ready: QueueStatus,
    /// Action-queue (outstanding action completions).
    pub action: QueueStatus,
    /// Journal writer queue.
    pub journal: QueueStatus,
    /// Trace ring buffer.
    pub trace: QueueStatus,
    /// Frame pool (allocated vs total).
    pub frame: QueueStatus,
}

impl QueueMonitor {
    /// Create a monitor with all pools at `Normal`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ready: QueueStatus::Normal,
            action: QueueStatus::Normal,
            journal: QueueStatus::Normal,
            trace: QueueStatus::Normal,
            frame: QueueStatus::Normal,
        }
    }

    /// Refresh every pool status from an IPC `ShardMetrics` snapshot.
    pub fn update_from_metrics(&mut self, m: &ShardMetrics) {
        // Ready queue: depth vs (depth + some assumed headroom).
        // The IPC field gives us current depth; we treat depth itself
        // as the utilisation signal relative to a 256-slot nominal capacity.
        let ready_capacity: u32 = 256;
        self.ready = QueueStatus::from_depth_capacity(m.ready_queue_depth, ready_capacity);

        // Action queue: same treatment, 256-slot nominal capacity.
        let action_capacity: u32 = 256;
        self.action = QueueStatus::from_depth_capacity(m.action_queue_depth, action_capacity);

        // Journal: use action queue depth as a proxy for journal pressure,
        // with a smaller 64-slot nominal capacity.
        let journal_capacity: u32 = 64;
        self.journal = QueueStatus::from_depth_capacity(m.action_queue_depth, journal_capacity);

        // Trace ring: fill percentage is already 0.0-100.0.
        self.trace = if m.trace_ring_fill_pct >= 80.0 {
            QueueStatus::Critical
        } else if m.trace_ring_fill_pct >= 50.0 {
            QueueStatus::Pressured
        } else {
            QueueStatus::Normal
        };

        // Frame pool: used / total ratio.
        let frame_used = m.frame_pool_total.saturating_sub(m.frame_pool_free);
        let frame_total = m.frame_pool_total;
        self.frame = QueueStatus::from_depth_capacity(frame_used, frame_total);
    }

    /// Returns the most severe status across all pools.
    ///
    /// Precedence: `Critical > Pressured > Normal`.
    #[must_use]
    pub fn worst_status(&self) -> QueueStatus {
        let pools = [
            self.ready,
            self.action,
            self.journal,
            self.trace,
            self.frame,
        ];
        let mut worst = QueueStatus::Normal;
        for status in pools {
            match (worst, status) {
                (_, QueueStatus::Critical) => worst = QueueStatus::Critical,
                (QueueStatus::Normal, QueueStatus::Pressured) => worst = QueueStatus::Pressured,
                _ => {}
            }
        }
        worst
    }
}

impl Default for QueueMonitor {
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

    // -- QueueStatus color tests --

    #[test]
    fn queue_status_normal_color_is_cyan() {
        let [r, g, b, a] = QueueStatus::Normal.color();
        assert_eq!(r, 0.0);
        assert!((g - 0.961).abs() < 0.002, "g={g}");
        assert_eq!(b, 1.0);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn queue_status_pressured_color_is_yellow() {
        let [r, g, b, a] = QueueStatus::Pressured.color();
        assert_eq!(r, 1.0);
        assert!((g - 0.902).abs() < 0.002, "g={g}");
        assert_eq!(b, 0.0);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn queue_status_critical_color_is_red() {
        let [r, g, b, a] = QueueStatus::Critical.color();
        assert_eq!(r, 1.0);
        assert!((g - 0.027).abs() < 0.002, "g={g}");
        assert!((b - 0.227).abs() < 0.002, "b={b}");
        assert_eq!(a, 1.0);
    }

    // -- QueueStatus::from_depth_capacity tests --

    #[test]
    fn from_depth_capacity_zero_capacity_is_normal() {
        assert_eq!(QueueStatus::from_depth_capacity(0, 0), QueueStatus::Normal);
        assert_eq!(
            QueueStatus::from_depth_capacity(999, 0),
            QueueStatus::Normal
        );
    }

    #[test]
    fn from_depth_capacity_below_50_pct_is_normal() {
        assert_eq!(
            QueueStatus::from_depth_capacity(49, 100),
            QueueStatus::Normal
        );
    }

    #[test]
    fn from_depth_capacity_50_to_79_pct_is_pressured() {
        assert_eq!(
            QueueStatus::from_depth_capacity(50, 100),
            QueueStatus::Pressured
        );
        assert_eq!(
            QueueStatus::from_depth_capacity(79, 100),
            QueueStatus::Pressured
        );
    }

    #[test]
    fn from_depth_capacity_80_pct_and_above_is_critical() {
        assert_eq!(
            QueueStatus::from_depth_capacity(80, 100),
            QueueStatus::Critical
        );
        assert_eq!(
            QueueStatus::from_depth_capacity(100, 100),
            QueueStatus::Critical
        );
        assert_eq!(
            QueueStatus::from_depth_capacity(200, 100),
            QueueStatus::Critical
        );
    }

    // -- QueueMonitor tests --

    #[test]
    fn queue_monitor_new_all_normal() {
        let mon = QueueMonitor::new();
        assert_eq!(mon.ready, QueueStatus::Normal);
        assert_eq!(mon.action, QueueStatus::Normal);
        assert_eq!(mon.journal, QueueStatus::Normal);
        assert_eq!(mon.trace, QueueStatus::Normal);
        assert_eq!(mon.frame, QueueStatus::Normal);
        assert_eq!(mon.worst_status(), QueueStatus::Normal);
    }

    #[test]
    fn queue_monitor_default_matches_new() {
        let mon = QueueMonitor::default();
        assert_eq!(mon.worst_status(), QueueStatus::Normal);
    }

    fn stub_shard_metrics(
        ready_depth: u32,
        action_depth: u32,
        pool_free: u32,
        pool_total: u32,
        trace_fill_pct: f32,
    ) -> ShardMetrics {
        ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: ready_depth,
            action_queue_depth: action_depth,
            timer_count: 0,
            frame_pool_free: pool_free,
            frame_pool_total: pool_total,
            trace_ring_fill_pct: trace_fill_pct,
            steps_total: 0,
            actions_total: 0,
        }
    }

    #[test]
    fn queue_monitor_update_healthy_shard() {
        let mut mon = QueueMonitor::new();
        let m = stub_shard_metrics(10, 5, 90, 100, 20.0);
        mon.update_from_metrics(&m);
        // ready 10/256 < 0.5 → Normal
        assert_eq!(mon.ready, QueueStatus::Normal);
        // action 5/256 < 0.5 → Normal
        assert_eq!(mon.action, QueueStatus::Normal);
        // trace 20% < 50% → Normal
        assert_eq!(mon.trace, QueueStatus::Normal);
        // frame used=10, total=100 < 0.5 → Normal
        assert_eq!(mon.frame, QueueStatus::Normal);
        assert_eq!(mon.worst_status(), QueueStatus::Normal);
    }

    #[test]
    fn queue_monitor_update_pressured_shard() {
        let mut mon = QueueMonitor::new();
        // ready=130/256 = ~50.8% → Pressured
        // action=5/256 → Normal
        // trace=55% → Pressured
        // frame used=60, total=100 = 60% → Pressured
        let m = stub_shard_metrics(130, 5, 40, 100, 55.0);
        mon.update_from_metrics(&m);
        assert_eq!(mon.ready, QueueStatus::Pressured);
        assert_eq!(mon.action, QueueStatus::Normal);
        assert_eq!(mon.trace, QueueStatus::Pressured);
        assert_eq!(mon.frame, QueueStatus::Pressured);
        assert_eq!(mon.worst_status(), QueueStatus::Pressured);
    }

    #[test]
    fn queue_monitor_update_critical_shard() {
        let mut mon = QueueMonitor::new();
        // ready=210/256 = ~82% → Critical
        // action=5/256 → Normal
        // trace=85% → Critical
        // frame used=95, total=100 = 95% → Critical
        let m = stub_shard_metrics(210, 5, 5, 100, 85.0);
        mon.update_from_metrics(&m);
        assert_eq!(mon.ready, QueueStatus::Critical);
        assert_eq!(mon.trace, QueueStatus::Critical);
        assert_eq!(mon.frame, QueueStatus::Critical);
        assert_eq!(mon.worst_status(), QueueStatus::Critical);
    }

    #[test]
    fn queue_monitor_worst_status_prefers_critical_over_pressured() {
        let mut mon = QueueMonitor::new();
        mon.ready = QueueStatus::Pressured;
        mon.action = QueueStatus::Critical;
        mon.journal = QueueStatus::Normal;
        mon.trace = QueueStatus::Normal;
        mon.frame = QueueStatus::Normal;
        assert_eq!(mon.worst_status(), QueueStatus::Critical);
    }

    #[test]
    fn queue_monitor_worst_status_prefers_pressured_over_normal() {
        let mut mon = QueueMonitor::new();
        mon.ready = QueueStatus::Normal;
        mon.action = QueueStatus::Normal;
        mon.journal = QueueStatus::Pressured;
        mon.trace = QueueStatus::Normal;
        mon.frame = QueueStatus::Normal;
        assert_eq!(mon.worst_status(), QueueStatus::Pressured);
    }

    #[test]
    fn queue_monitor_update_with_zero_frame_pool() {
        let mut mon = QueueMonitor::new();
        let m = stub_shard_metrics(10, 5, 0, 0, 30.0);
        mon.update_from_metrics(&m);
        // frame: capacity 0 → Normal
        assert_eq!(mon.frame, QueueStatus::Normal);
    }

    #[test]
    fn queue_monitor_journal_pressured_via_action_depth() {
        let mut mon = QueueMonitor::new();
        // action_depth=40/64 = 62.5% → Pressured for journal proxy
        let m = stub_shard_metrics(10, 40, 90, 100, 20.0);
        mon.update_from_metrics(&m);
        assert_eq!(mon.journal, QueueStatus::Pressured);
    }

    #[test]
    fn queue_monitor_journal_critical_via_action_depth() {
        let mut mon = QueueMonitor::new();
        // action_depth=55/64 = ~86% → Critical for journal proxy
        let m = stub_shard_metrics(10, 55, 90, 100, 20.0);
        mon.update_from_metrics(&m);
        assert_eq!(mon.journal, QueueStatus::Critical);
    }

    #[test]
    fn queue_monitor_trace_boundary_50_pct_is_pressured() {
        let mut mon = QueueMonitor::new();
        let m = stub_shard_metrics(10, 5, 90, 100, 50.0);
        mon.update_from_metrics(&m);
        assert_eq!(mon.trace, QueueStatus::Pressured);
    }

    #[test]
    fn queue_monitor_trace_boundary_80_pct_is_critical() {
        let mut mon = QueueMonitor::new();
        let m = stub_shard_metrics(10, 5, 90, 100, 80.0);
        mon.update_from_metrics(&m);
        assert_eq!(mon.trace, QueueStatus::Critical);
    }
}
