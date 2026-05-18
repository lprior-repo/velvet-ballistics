#![forbid(unsafe_code)]
use std::time::Duration;

use vb_ipc::{AggregateMetrics, ShardMetrics};

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub shards: Vec<ShardDisplay>,
    pub total_active_runs: u32,
    pub total_ready_queue_depth: u32,
    pub total_action_queue_depth: u32,
    pub overall_health: HealthStatus,
}

#[derive(Debug, Clone)]
pub struct ShardDisplay {
    pub shard_id: u32,
    pub active_runs: u32,
    pub ready_queue_depth: u32,
    pub action_queue_depth: u32,
    pub timer_count: u32,
    pub frame_pool_free: u32,
    pub frame_pool_total: u32,
    pub trace_ring_fill_pct: f32,
    pub steps_per_sec: f64,
    pub tick_duration_p95: Duration,
    pub health: HealthStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
}

impl HealthStatus {
    #[must_use]
    pub const fn color(self) -> [f32; 4] {
        match self {
            Self::Healthy => [0.0, 0.961, 1.0, 1.0],
            Self::Degraded => [1.0, 0.902, 0.0, 1.0],
            Self::Critical => [1.0, 0.027, 0.227, 1.0],
        }
    }
}

#[must_use]
pub fn queue_health(depth: u32, max: u32) -> HealthStatus {
    if max == 0 {
        return HealthStatus::Healthy;
    }
    let ratio = f64::from(depth) / f64::from(max);
    if ratio >= 0.8 {
        HealthStatus::Critical
    } else if ratio >= 0.5 {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    }
}

#[must_use]
pub fn queue_pressure_color(depth: u32, max: u32) -> [f32; 4] {
    queue_health(depth, max).color()
}

impl From<&ShardMetrics> for ShardDisplay {
    fn from(m: &ShardMetrics) -> Self {
        let pool_used_ratio = if m.frame_pool_total > 0 {
            f64::from(m.frame_pool_total.saturating_sub(m.frame_pool_free))
                / f64::from(m.frame_pool_total)
        } else {
            0.0
        };

        let health = if pool_used_ratio >= 0.8 || m.trace_ring_fill_pct >= 90.0 {
            HealthStatus::Critical
        } else if pool_used_ratio >= 0.5 || m.trace_ring_fill_pct >= 70.0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        Self {
            shard_id: m.shard_id,
            active_runs: m.active_runs,
            ready_queue_depth: m.ready_queue_depth,
            action_queue_depth: m.action_queue_depth,
            timer_count: m.timer_count,
            frame_pool_free: m.frame_pool_free,
            frame_pool_total: m.frame_pool_total,
            trace_ring_fill_pct: m.trace_ring_fill_pct,
            steps_per_sec: 0.0,
            tick_duration_p95: Duration::ZERO,
            health,
        }
    }
}

impl From<&AggregateMetrics> for SystemMetrics {
    fn from(_agg: &AggregateMetrics) -> Self {
        Self {
            shards: Vec::new(),
            total_active_runs: 0,
            total_ready_queue_depth: 0,
            total_action_queue_depth: 0,
            overall_health: HealthStatus::Healthy,
        }
    }
}

impl SystemMetrics {
    pub fn recompute(&mut self) {
        self.total_active_runs = self.shards.iter().map(|s| s.active_runs).sum();
        self.total_ready_queue_depth = self.shards.iter().map(|s| s.ready_queue_depth).sum();
        self.total_action_queue_depth = self.shards.iter().map(|s| s.action_queue_depth).sum();

        let any_critical = self
            .shards
            .iter()
            .any(|s| s.health == HealthStatus::Critical);
        let any_degraded = self
            .shards
            .iter()
            .any(|s| s.health == HealthStatus::Degraded);

        self.overall_health = if any_critical {
            HealthStatus::Critical
        } else if any_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_status_healthy_color_is_neon_cyan() {
        let [r, g, b, a] = HealthStatus::Healthy.color();
        assert_eq!(r, 0.0);
        assert!((g - 0.961).abs() < 0.002);
        assert_eq!(b, 1.0);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn health_status_degraded_color_is_neon_yellow() {
        let [r, g, b, a] = HealthStatus::Degraded.color();
        assert_eq!(r, 1.0);
        assert!((g - 0.902).abs() < 0.002);
        assert_eq!(b, 0.0);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn health_status_critical_color_is_neon_red() {
        let [r, g, b, a] = HealthStatus::Critical.color();
        assert_eq!(r, 1.0);
        assert!((g - 0.027).abs() < 0.002);
        assert!((b - 0.227).abs() < 0.002);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn queue_health_zero_max_returns_healthy() {
        assert_eq!(queue_health(0, 0), HealthStatus::Healthy);
    }

    #[test]
    fn queue_health_below_50_pct_is_healthy() {
        assert_eq!(queue_health(49, 100), HealthStatus::Healthy);
    }

    #[test]
    fn queue_health_50_to_80_pct_is_degraded() {
        assert_eq!(queue_health(50, 100), HealthStatus::Degraded);
        assert_eq!(queue_health(79, 100), HealthStatus::Degraded);
    }

    #[test]
    fn queue_health_80_pct_and_above_is_critical() {
        assert_eq!(queue_health(80, 100), HealthStatus::Critical);
        assert_eq!(queue_health(100, 100), HealthStatus::Critical);
    }

    #[test]
    fn queue_pressure_color_delegates_to_queue_health() {
        assert_eq!(queue_pressure_color(10, 100), HealthStatus::Healthy.color());
        assert_eq!(
            queue_pressure_color(60, 100),
            HealthStatus::Degraded.color()
        );
        assert_eq!(
            queue_pressure_color(90, 100),
            HealthStatus::Critical.color()
        );
    }

    #[test]
    fn shard_display_from_ipc_metrics_healthy() {
        let ipc_shard = ShardMetrics {
            shard_id: 3,
            active_runs: 5,
            ready_queue_depth: 2,
            action_queue_depth: 10,
            timer_count: 1,
            frame_pool_free: 80,
            frame_pool_total: 100,
            trace_ring_fill_pct: 30.0,
            steps_total: 1000,
            actions_total: 500,
        };
        let display = ShardDisplay::from(&ipc_shard);
        assert_eq!(display.shard_id, 3);
        assert_eq!(display.active_runs, 5);
        assert_eq!(display.health, HealthStatus::Healthy);
    }

    #[test]
    fn shard_display_critical_when_pool_near_empty() {
        let ipc_shard = ShardMetrics {
            shard_id: 0,
            active_runs: 50,
            ready_queue_depth: 100,
            action_queue_depth: 0,
            timer_count: 10,
            frame_pool_free: 5,
            frame_pool_total: 100,
            trace_ring_fill_pct: 50.0,
            steps_total: 0,
            actions_total: 0,
        };
        let display = ShardDisplay::from(&ipc_shard);
        assert_eq!(display.health, HealthStatus::Critical);
    }

    #[test]
    fn shard_display_degraded_when_trace_ring_high() {
        let ipc_shard = ShardMetrics {
            shard_id: 1,
            active_runs: 5,
            ready_queue_depth: 10,
            action_queue_depth: 5,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 75.0,
            steps_total: 0,
            actions_total: 0,
        };
        let display = ShardDisplay::from(&ipc_shard);
        assert_eq!(display.health, HealthStatus::Degraded);
    }

    #[test]
    fn system_metrics_from_aggregate_defaults_healthy() {
        let agg = AggregateMetrics {
            runs_active: 10,
            runs_waiting: 3,
            runs_failed_total: 1,
            runs_finished_total: 42,
        };
        let metrics = SystemMetrics::from(&agg);
        assert!(metrics.shards.is_empty());
        assert_eq!(metrics.overall_health, HealthStatus::Healthy);
    }

    #[test]
    fn system_metrics_recompute_sums_shards_and_propagates_health() {
        let mut metrics = SystemMetrics {
            shards: vec![
                ShardDisplay {
                    shard_id: 0,
                    active_runs: 3,
                    ready_queue_depth: 5,
                    action_queue_depth: 2,
                    timer_count: 0,
                    frame_pool_free: 100,
                    frame_pool_total: 100,
                    trace_ring_fill_pct: 10.0,
                    steps_per_sec: 0.0,
                    tick_duration_p95: Duration::ZERO,
                    health: HealthStatus::Healthy,
                },
                ShardDisplay {
                    shard_id: 1,
                    active_runs: 7,
                    ready_queue_depth: 15,
                    action_queue_depth: 8,
                    timer_count: 2,
                    frame_pool_free: 20,
                    frame_pool_total: 100,
                    trace_ring_fill_pct: 85.0,
                    steps_per_sec: 0.0,
                    tick_duration_p95: Duration::ZERO,
                    health: HealthStatus::Critical,
                },
            ],
            total_active_runs: 0,
            total_ready_queue_depth: 0,
            total_action_queue_depth: 0,
            overall_health: HealthStatus::Healthy,
        };
        metrics.recompute();
        assert_eq!(metrics.total_active_runs, 10);
        assert_eq!(metrics.total_ready_queue_depth, 20);
        assert_eq!(metrics.total_action_queue_depth, 10);
        assert_eq!(metrics.overall_health, HealthStatus::Critical);
    }

    #[test]
    fn system_metrics_recompute_degraded_when_no_critical() {
        let mut metrics = SystemMetrics {
            shards: vec![ShardDisplay {
                shard_id: 0,
                active_runs: 1,
                ready_queue_depth: 0,
                action_queue_depth: 0,
                timer_count: 0,
                frame_pool_free: 60,
                frame_pool_total: 100,
                trace_ring_fill_pct: 75.0,
                steps_per_sec: 0.0,
                tick_duration_p95: Duration::ZERO,
                health: HealthStatus::Degraded,
            }],
            total_active_runs: 0,
            total_ready_queue_depth: 0,
            total_action_queue_depth: 0,
            overall_health: HealthStatus::Healthy,
        };
        metrics.recompute();
        assert_eq!(metrics.overall_health, HealthStatus::Degraded);
    }

    #[test]
    fn system_metrics_recompute_empty_shards_is_healthy() {
        let mut metrics = SystemMetrics {
            shards: Vec::new(),
            total_active_runs: 99,
            total_ready_queue_depth: 99,
            total_action_queue_depth: 99,
            overall_health: HealthStatus::Critical,
        };
        metrics.recompute();
        assert_eq!(metrics.total_active_runs, 0);
        assert_eq!(metrics.total_ready_queue_depth, 0);
        assert_eq!(metrics.total_action_queue_depth, 0);
        assert_eq!(metrics.overall_health, HealthStatus::Healthy);
    }

    // =========================================================================
    // BLACKHAT security review tests
    // =========================================================================

    /// SEVERITY: HIGH
    /// DESCRIPTION: SystemMetrics::recompute() uses Iterator::sum() to
    /// accumulate u32 shard totals (lines 114-116). The default Sum<u32>
    /// implementation panics on overflow in debug mode and wraps in release.
    /// With two shards each having active_runs = u32::MAX / 2 + 1, the sum
    /// panics in debug (crashes the process) or wraps to 0 in release
    /// (silently wrong). Both outcomes are severe bugs.
    #[test]
    #[should_panic(expected = "attempt to add with overflow")]
    fn blackhat_recompute_u32_sum_panics_on_overflow() {
        let half = u32::MAX / 2 + 1; // 2_147_483_648
        let mut metrics = SystemMetrics {
            shards: vec![
                ShardDisplay {
                    shard_id: 0,
                    active_runs: half,
                    ready_queue_depth: half,
                    action_queue_depth: half,
                    timer_count: 0,
                    frame_pool_free: 100,
                    frame_pool_total: 100,
                    trace_ring_fill_pct: 0.0,
                    steps_per_sec: 0.0,
                    tick_duration_p95: Duration::ZERO,
                    health: HealthStatus::Healthy,
                },
                ShardDisplay {
                    shard_id: 1,
                    active_runs: half,
                    ready_queue_depth: half,
                    action_queue_depth: half,
                    timer_count: 0,
                    frame_pool_free: 100,
                    frame_pool_total: 100,
                    trace_ring_fill_pct: 0.0,
                    steps_per_sec: 0.0,
                    tick_duration_p95: Duration::ZERO,
                    health: HealthStatus::Healthy,
                },
            ],
            total_active_runs: 0,
            total_ready_queue_depth: 0,
            total_action_queue_depth: 0,
            overall_health: HealthStatus::Healthy,
        };
        // BUG: sum() panics in debug, wraps in release.
        // The fix should use saturating_add via fold instead of sum().
        metrics.recompute();
    }

    /// SEVERITY: Medium
    /// DESCRIPTION: ShardDisplay::from(&ShardMetrics) computes pool_used_ratio
    /// as (total - free) / total using f64. If frame_pool_free > frame_pool_total
    /// (inconsistent data from IPC), the subtraction saturates to 0, hiding the
    /// anomaly. The health classification silently returns Healthy even though
    /// the data is corrupt.
    #[test]
    fn blackhat_shard_display_free_exceeds_total_silently_healthy() {
        let ipc_shard = ShardMetrics {
            shard_id: 0,
            active_runs: 50,
            ready_queue_depth: 100,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 200, // MORE than total!
            frame_pool_total: 100,
            trace_ring_fill_pct: 50.0,
            steps_total: 0,
            actions_total: 0,
        };
        let display = ShardDisplay::from(&ipc_shard);
        // saturating_sub: 100 - 200 = 0, ratio = 0.0 -> Healthy
        // But this is clearly an invalid state (free > total).
        assert_eq!(
            display.health,
            HealthStatus::Healthy,
            "corrupt free > total data is silently classified as Healthy"
        );
    }

    /// SEVERITY: Low
    /// DESCRIPTION: queue_health uses f64::from(depth) / f64::from(max) for
    /// ratio computation. While f64 can exactly represent all u32 values, the
    /// threshold comparison `ratio >= 0.8` uses floating-point. For most values
    /// this is fine, but the boundary cases should be exact. Testing shows that
    /// 4/5 = 0.8 is exact in f64, so the boundary is correct for small values.
    #[test]
    fn blackhat_queue_health_exact_boundary_check() {
        // depth=4, max=5 -> ratio = 0.8 exactly -> Critical
        assert_eq!(queue_health(4, 5), HealthStatus::Critical);
        // depth=3, max=5 -> ratio = 0.6 -> Degraded
        assert_eq!(queue_health(3, 5), HealthStatus::Degraded);
        // depth=2, max=5 -> ratio = 0.4 -> Healthy
        assert_eq!(queue_health(2, 5), HealthStatus::Healthy);
    }

    /// SEVERITY: Low
    /// DESCRIPTION: SystemMetrics::recompute health check uses iter().any()
    /// which is correct but doesn't account for the interaction between the
    /// shard-level health (from ShardDisplay::from) and the recompute-level
    /// health. If a shard's health is stale (set before new queue data arrived),
    /// recompute propagates the stale health to overall_health without
    /// recalculating per-shard health from current queue depths.
    #[test]
    fn blackhat_recompute_propagates_stale_shard_health() {
        let mut metrics = SystemMetrics {
            shards: vec![ShardDisplay {
                shard_id: 0,
                active_runs: 0,
                ready_queue_depth: 0,
                action_queue_depth: 0,
                timer_count: 0,
                frame_pool_free: 100,
                frame_pool_total: 100,
                trace_ring_fill_pct: 0.0,
                steps_per_sec: 0.0,
                tick_duration_p95: Duration::ZERO,
                health: HealthStatus::Critical, // Stale: set externally
            }],
            total_active_runs: 0,
            total_ready_queue_depth: 0,
            total_action_queue_depth: 0,
            overall_health: HealthStatus::Healthy,
        };
        metrics.recompute();
        // recompute doesn't recalculate per-shard health, just propagates it.
        assert_eq!(
            metrics.overall_health,
            HealthStatus::Critical,
            "stale Critical health propagates to overall without recalculation"
        );
    }
}
