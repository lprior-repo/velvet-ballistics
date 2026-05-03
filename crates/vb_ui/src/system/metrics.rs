use std::time::Duration;

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
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
}

pub fn queue_pressure_color(depth: u32, max: u32) -> [f32; 4] {
    if max == 0 {
        return [0.0, 0.96, 1.0, 1.0]; // neon cyan
    }
    let ratio = depth as f32 / max as f32;
    if ratio < 0.5 {
        [0.0, 0.96, 1.0, 1.0] // neon cyan
    } else if ratio < 0.8 {
        [1.0, 0.9, 0.0, 1.0] // neon yellow
    } else {
        [1.0, 0.03, 0.23, 1.0] // neon red
    }
}

impl SystemMetrics {
    pub fn compute_health(&mut self) {
        let any_critical = self.shards.iter().any(|s| s.health == HealthStatus::Critical);
        let any_degraded = self.shards.iter().any(|s| s.health == HealthStatus::Degraded);
        self.overall_health = if any_critical {
            HealthStatus::Critical
        } else if any_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
    }
}
