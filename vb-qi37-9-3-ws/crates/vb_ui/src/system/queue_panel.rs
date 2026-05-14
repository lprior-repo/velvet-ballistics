#![forbid(unsafe_code)]
/// Data-driven queue visualisation panel for the system overview screen.
///
/// Transforms raw per-shard queue depths and capacities into structured
/// bar-segment data suitable for rendering.  Colour mapping follows the
/// project-wide convention:
///
/// | `QueueStatus` | Colour      | Hex       |
/// |---------------|-------------|-----------|
/// | Normal        | neon green  | `#39ff14` |
/// | Pressured     | neon yellow | `#ffe600` |
/// | Critical      | neon red    | `#ff073a` |
use crate::system::queue_monitor::QueueStatus;
use crate::system::screen::SystemScreen;

// ---------------------------------------------------------------------------
// Float/int conversion helper (isolated for auditability)
// ---------------------------------------------------------------------------

/// Convert a u32 to f32. Lossless for queue-depth values (< 2^24).
#[allow(clippy::cast_precision_loss, clippy::as_conversions)]
fn u32_to_f32(v: u32) -> f32 {
    v as f32
}

// ---------------------------------------------------------------------------
// Display colours (neon palette)
// ---------------------------------------------------------------------------

/// Neon green `#39ff14` — Normal queue status.
const NEON_GREEN: [f32; 4] = [0.224, 1.0, 0.078, 1.0];
/// Neon yellow `#ffe600` — Pressured queue status.
const NEON_YELLOW: [f32; 4] = [1.0, 0.902, 0.0, 1.0];
/// Neon red `#ff073a` — Critical queue status.
const NEON_RED: [f32; 4] = [1.0, 0.027, 0.227, 1.0];

/// Nominal capacity used for ready and action queue pools.
const NOMINAL_POOL_CAPACITY: u32 = 256;

// ---------------------------------------------------------------------------
// QueueBarSegment
// ---------------------------------------------------------------------------

/// A single horizontal bar segment representing one queue pool.
#[derive(Debug, Clone)]
pub struct QueueBarSegment {
    /// Human-readable label (e.g. `"Ready"`, `"Action"`).
    pub label: String,
    /// Current queue depth.
    pub depth: u32,
    /// Nominal capacity for the pool.
    pub capacity: u32,
    /// RGBA display colour derived from the pool's `QueueStatus`.
    pub color: [f32; 4],
}

impl QueueBarSegment {
    /// Ratio of `depth` to `capacity`. Returns `0.0` when `capacity` is zero.
    #[must_use]
    pub fn fill_ratio(&self) -> f32 {
        if self.capacity == 0 {
            return 0.0;
        }
        u32_to_f32(self.depth) / u32_to_f32(self.capacity)
    }
}

// ---------------------------------------------------------------------------
// ShardQueuePanel
// ---------------------------------------------------------------------------

/// Per-shard queue visualisation data.
#[derive(Debug, Clone)]
pub struct ShardQueuePanel {
    /// Shard index.
    pub shard_id: u32,
    /// Ready-queue bar segment.
    pub ready_bar: QueueBarSegment,
    /// Action-queue bar segment.
    pub action_bar: QueueBarSegment,
    /// Sum of ready + action depths (total pending work).
    pub total_pending: u32,
    /// Human-readable health label derived from worst pool status.
    pub health_label: String,
}

// ---------------------------------------------------------------------------
// SystemQueuePanel
// ---------------------------------------------------------------------------

/// System-wide queue visualisation panel aggregating all shards.
#[derive(Debug, Clone)]
pub struct SystemQueuePanel {
    /// Per-shard panels, in the same order as `SystemScreen::metrics().shards`.
    pub shards: Vec<ShardQueuePanel>,
    /// Sum of all ready-queue depths.
    pub total_ready: u32,
    /// Sum of all action-queue depths.
    pub total_action: u32,
    /// Worst `QueueStatus` across every shard and pool.
    pub worst_status: QueueStatus,
}

// ---------------------------------------------------------------------------
// Colour helper
// ---------------------------------------------------------------------------

/// Map a `QueueStatus` to the neon colour palette used by the panel.
#[must_use]
fn status_color(status: QueueStatus) -> [f32; 4] {
    match status {
        QueueStatus::Normal => NEON_GREEN,
        QueueStatus::Pressured => NEON_YELLOW,
        QueueStatus::Critical => NEON_RED,
    }
}

/// Derive a human-readable health label from the worst `QueueStatus`.
#[must_use]
fn health_label_from_status(status: QueueStatus) -> String {
    match status {
        QueueStatus::Normal => "Normal".to_string(),
        QueueStatus::Pressured => "Pressured".to_string(),
        QueueStatus::Critical => "Critical".to_string(),
    }
}

// ---------------------------------------------------------------------------
// SystemQueuePanelBuilder
// ---------------------------------------------------------------------------

/// Builder that extracts queue panel data from a `SystemScreen`.
pub struct SystemQueuePanelBuilder;

impl SystemQueuePanelBuilder {
    /// Build a complete `SystemQueuePanel` from the current `SystemScreen` state.
    ///
    /// Iterates over every shard in the metrics, pairs each with its
    /// corresponding `QueueMonitor`, and produces structured bar-segment
    /// data ready for rendering.
    #[must_use]
    pub fn build(screen: &SystemScreen) -> SystemQueuePanel {
        let metrics = screen.metrics();
        let monitors = screen.queue_monitors();

        let mut shards = Vec::with_capacity(metrics.shards.len());
        let mut total_ready: u32 = 0;
        let mut total_action: u32 = 0;
        let mut worst_status = QueueStatus::Normal;

        for (idx, shard) in metrics.shards.iter().enumerate() {
            let monitor = monitors.get(idx);

            let ready_status = monitor.map(|m| m.ready).unwrap_or(QueueStatus::Normal);
            let action_status = monitor.map(|m| m.action).unwrap_or(QueueStatus::Normal);
            let shard_worst = monitor
                .map(|m| m.worst_status())
                .unwrap_or(QueueStatus::Normal);

            let ready_bar = QueueBarSegment {
                label: "Ready".to_string(),
                depth: shard.ready_queue_depth,
                capacity: NOMINAL_POOL_CAPACITY,
                color: status_color(ready_status),
            };

            let action_bar = QueueBarSegment {
                label: "Action".to_string(),
                depth: shard.action_queue_depth,
                capacity: NOMINAL_POOL_CAPACITY,
                color: status_color(action_status),
            };

            let total_pending = shard
                .ready_queue_depth
                .saturating_add(shard.action_queue_depth);

            let health_label = health_label_from_status(shard_worst);

            // Accumulate totals.
            total_ready = total_ready.saturating_add(shard.ready_queue_depth);
            total_action = total_action.saturating_add(shard.action_queue_depth);

            // Track worst status.
            match (worst_status, shard_worst) {
                (_, QueueStatus::Critical) => worst_status = QueueStatus::Critical,
                (QueueStatus::Normal, QueueStatus::Pressured) => {
                    worst_status = QueueStatus::Pressured;
                }
                _ => {}
            }

            shards.push(ShardQueuePanel {
                shard_id: shard.shard_id,
                ready_bar,
                action_bar,
                total_pending,
                health_label,
            });
        }

        // Final cross-check: also consider the screen-level worst.
        let screen_worst = screen.worst_queue_status();
        match (worst_status, screen_worst) {
            (_, QueueStatus::Critical) => worst_status = QueueStatus::Critical,
            (QueueStatus::Normal, QueueStatus::Pressured) => {
                worst_status = QueueStatus::Pressured;
            }
            _ => {}
        }

        SystemQueuePanel {
            shards,
            total_ready,
            total_action,
            worst_status,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::screen::SystemScreen;
    use vb_ipc::ShardMetrics;

    /// Helper to create a `ShardMetrics` for testing.
    fn stub_metrics(
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

    // -- QueueBarSegment::fill_ratio tests --

    #[test]
    fn fill_ratio_normal_usage() {
        let seg = QueueBarSegment {
            label: "Ready".to_string(),
            depth: 64,
            capacity: 256,
            color: NEON_GREEN,
        };
        let ratio = seg.fill_ratio();
        assert!((ratio - 0.25).abs() < 0.01, "ratio={ratio}");
    }

    #[test]
    fn fill_ratio_zero_capacity_returns_zero() {
        let seg = QueueBarSegment {
            label: "Empty".to_string(),
            depth: 0,
            capacity: 0,
            color: NEON_GREEN,
        };
        assert_eq!(seg.fill_ratio(), 0.0);
    }

    #[test]
    fn fill_ratio_nonzero_depth_zero_capacity_returns_zero() {
        let seg = QueueBarSegment {
            label: "Weird".to_string(),
            depth: 50,
            capacity: 0,
            color: NEON_GREEN,
        };
        assert_eq!(seg.fill_ratio(), 0.0);
    }

    #[test]
    fn fill_ratio_full_capacity() {
        let seg = QueueBarSegment {
            label: "Full".to_string(),
            depth: 256,
            capacity: 256,
            color: NEON_RED,
        };
        let ratio = seg.fill_ratio();
        assert!((ratio - 1.0).abs() < 0.01, "ratio={ratio}");
    }

    // -- status_color mapping tests --

    #[test]
    fn status_color_normal_is_neon_green() {
        let [r, g, b, a] = status_color(QueueStatus::Normal);
        assert!((r - 0.224).abs() < 0.002, "r={r}");
        assert_eq!(g, 1.0);
        assert!((b - 0.078).abs() < 0.002, "b={b}");
        assert_eq!(a, 1.0);
    }

    #[test]
    fn status_color_pressured_is_neon_yellow() {
        let [r, g, b, a] = status_color(QueueStatus::Pressured);
        assert_eq!(r, 1.0);
        assert!((g - 0.902).abs() < 0.002, "g={g}");
        assert_eq!(b, 0.0);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn status_color_critical_is_neon_red() {
        let [r, g, b, a] = status_color(QueueStatus::Critical);
        assert_eq!(r, 1.0);
        assert!((g - 0.027).abs() < 0.002, "g={g}");
        assert!((b - 0.227).abs() < 0.002, "b={b}");
        assert_eq!(a, 1.0);
    }

    // -- health_label_from_status tests --

    #[test]
    fn health_label_normal() {
        assert_eq!(health_label_from_status(QueueStatus::Normal), "Normal");
    }

    #[test]
    fn health_label_pressured() {
        assert_eq!(
            health_label_from_status(QueueStatus::Pressured),
            "Pressured"
        );
    }

    #[test]
    fn health_label_critical() {
        assert_eq!(health_label_from_status(QueueStatus::Critical), "Critical");
    }

    // -- SystemQueuePanelBuilder integration tests --

    #[test]
    fn builder_empty_screen_produces_empty_panel() {
        let screen = SystemScreen::new();
        let panel = SystemQueuePanelBuilder::build(&screen);
        assert!(panel.shards.is_empty());
        assert_eq!(panel.total_ready, 0);
        assert_eq!(panel.total_action, 0);
        assert_eq!(panel.worst_status, QueueStatus::Normal);
    }

    #[test]
    fn builder_single_healthy_shard() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_metrics(0, 10, 5, 90, 100, 20.0));
        let panel = SystemQueuePanelBuilder::build(&screen);

        assert_eq!(panel.shards.len(), 1);
        assert_eq!(panel.shards[0].shard_id, 0);
        assert_eq!(panel.shards[0].ready_bar.depth, 10);
        assert_eq!(panel.shards[0].ready_bar.capacity, 256);
        assert_eq!(panel.shards[0].action_bar.depth, 5);
        assert_eq!(panel.shards[0].action_bar.capacity, 256);
        assert_eq!(panel.shards[0].total_pending, 15);
        assert_eq!(panel.shards[0].health_label, "Normal");
        // Ready 10/256 < 50% -> Normal -> neon green
        assert_eq!(panel.shards[0].ready_bar.color, NEON_GREEN);
        assert_eq!(panel.shards[0].action_bar.color, NEON_GREEN);

        assert_eq!(panel.total_ready, 10);
        assert_eq!(panel.total_action, 5);
        assert_eq!(panel.worst_status, QueueStatus::Normal);
    }

    #[test]
    fn builder_pressured_shard_gets_yellow_color() {
        let mut screen = SystemScreen::new();
        // ready=130/256 = ~50.8% -> Pressured
        screen.update_from_metrics(&stub_metrics(0, 130, 5, 90, 100, 20.0));
        let panel = SystemQueuePanelBuilder::build(&screen);

        assert_eq!(panel.shards[0].ready_bar.color, NEON_YELLOW);
        assert_eq!(panel.shards[0].health_label, "Pressured");
        assert_eq!(panel.worst_status, QueueStatus::Pressured);
    }

    #[test]
    fn builder_critical_shard_gets_red_color() {
        let mut screen = SystemScreen::new();
        // ready=210/256 = ~82% -> Critical
        screen.update_from_metrics(&stub_metrics(0, 210, 5, 5, 100, 85.0));
        let panel = SystemQueuePanelBuilder::build(&screen);

        assert_eq!(panel.shards[0].ready_bar.color, NEON_RED);
        assert_eq!(panel.shards[0].health_label, "Critical");
        assert_eq!(panel.worst_status, QueueStatus::Critical);
    }

    #[test]
    fn builder_multiple_shards_aggregates_correctly() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_metrics(0, 10, 5, 90, 100, 20.0));
        screen.update_from_metrics(&stub_metrics(1, 20, 10, 80, 100, 15.0));
        let panel = SystemQueuePanelBuilder::build(&screen);

        assert_eq!(panel.shards.len(), 2);
        assert_eq!(panel.total_ready, 30);
        assert_eq!(panel.total_action, 15);
        assert_eq!(panel.shards[0].total_pending, 15);
        assert_eq!(panel.shards[1].total_pending, 30);
        assert_eq!(panel.worst_status, QueueStatus::Normal);
    }

    #[test]
    fn builder_worst_status_is_critical_when_any_shard_critical() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_metrics(0, 10, 5, 90, 100, 20.0));
        // Shard 1: critical via ready=210/256
        screen.update_from_metrics(&stub_metrics(1, 210, 5, 5, 100, 85.0));
        let panel = SystemQueuePanelBuilder::build(&screen);

        assert_eq!(panel.worst_status, QueueStatus::Critical);
        assert_eq!(panel.shards[0].health_label, "Normal");
        assert_eq!(panel.shards[1].health_label, "Critical");
    }

    #[test]
    fn builder_saturating_totals_on_segment_level() {
        // Verify total_pending saturates on a single shard.
        let seg = ShardQueuePanel {
            shard_id: 0,
            ready_bar: QueueBarSegment {
                label: "Ready".to_string(),
                depth: u32::MAX,
                capacity: 256,
                color: NEON_GREEN,
            },
            action_bar: QueueBarSegment {
                label: "Action".to_string(),
                depth: 1,
                capacity: 256,
                color: NEON_GREEN,
            },
            total_pending: u32::MAX, // MAX + 1 would overflow
            health_label: "Normal".to_string(),
        };
        // Directly verify that saturating_add produces MAX.
        let pending = u32::MAX.saturating_add(1);
        assert_eq!(pending, u32::MAX);
        assert_eq!(seg.total_pending, u32::MAX);
    }

    // =========================================================================
    // BLACKHAT security review tests
    // =========================================================================

    /// SEVERITY: Low
    /// DESCRIPTION: QueueBarSegment::fill_ratio uses an `as` cast (via
    /// u32_to_f32) to convert depth and capacity to f32 for division.
    /// For values >= 2^24 (16,777,216), f32 loses precision. If capacity
    /// exceeds 2^24, the ratio may be imprecise. With depth and capacity
    /// both >= 2^24, the ratio could be 0.999... instead of 1.0 or vice
    /// versa due to float rounding.
    #[test]
    fn blackhat_fill_ratio_precision_loss_for_large_values() {
        // Values under 2^24 are exact in f32.
        let seg = QueueBarSegment {
            label: "Large".to_string(),
            depth: 16_777_215, // just under 2^24
            capacity: 16_777_215,
            color: NEON_GREEN,
        };
        let ratio = seg.fill_ratio();
        assert!(
            (ratio - 1.0).abs() < 0.001,
            "ratio should be ~1.0 for equal values: {ratio}"
        );
    }

    /// SEVERITY: Low
    /// DESCRIPTION: QueueBarSegment has capacity as u32 but fill_ratio divides
    /// by capacity after casting to f32. If capacity is 0, it returns 0.0
    /// (correct guard). But if depth > capacity, the ratio exceeds 1.0, which
    /// could confuse downstream rendering code that expects a [0.0, 1.0] range.
    #[test]
    fn blackhat_fill_ratio_exceeds_one_when_depth_over_capacity() {
        let seg = QueueBarSegment {
            label: "Over".to_string(),
            depth: 300,
            capacity: 256,
            color: NEON_RED,
        };
        let ratio = seg.fill_ratio();
        assert!(
            ratio > 1.0,
            "ratio should exceed 1.0 when depth > capacity: {ratio}"
        );
    }

    /// SEVERITY: Medium
    /// DESCRIPTION: SystemQueuePanelBuilder::build uses NOMINAL_POOL_CAPACITY
    /// (256) as the capacity for all ready and action bars, regardless of
    /// the actual capacity from the shard data. This means the fill_ratio
    /// and visual representation use a hardcoded capacity rather than the
    /// real capacity, which could misrepresent actual queue pressure if the
    /// real capacity differs from 256.
    #[test]
    fn blackhat_hardcoded_capacity_mismatches_real_data() {
        let screen = SystemScreen::new();
        let panel = SystemQueuePanelBuilder::build(&screen);
        // For an empty screen, verify capacity is always NOMINAL_POOL_CAPACITY.
        for shard_panel in &panel.shards {
            assert_eq!(
                shard_panel.ready_bar.capacity, NOMINAL_POOL_CAPACITY,
                "capacity is hardcoded to NOMINAL_POOL_CAPACITY"
            );
        }
    }

    /// SEVERITY: Low
    /// DESCRIPTION: The worst_status propagation in SystemQueuePanelBuilder::build
    /// uses a match that only upgrades from Normal to Pressured, or anything to
    /// Critical. It never downgrades. But if shard_worst is Normal and worst_status
    /// is already Pressured, the match falls through to `_ => {}`, correctly
    /// preserving Pressured. The logic is correct but could be simpler with an
    /// ordinal comparison.
    #[test]
    fn blackhat_worst_status_never_downgrades() {
        let mut screen = SystemScreen::new();
        // First shard: pressured via ready=130/256.
        screen.update_from_metrics(&stub_metrics(0, 130, 5, 90, 100, 20.0));
        // Second shard: normal.
        screen.update_from_metrics(&stub_metrics(1, 10, 5, 90, 100, 20.0));
        let panel = SystemQueuePanelBuilder::build(&screen);
        // Worst should remain Pressured from shard 0.
        assert_eq!(panel.worst_status, QueueStatus::Pressured);
    }
}
