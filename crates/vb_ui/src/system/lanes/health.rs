#![forbid(unsafe_code)]
//! Lane health classification and summary.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LaneHealth {
    Green,
    Yellow,
    Red,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShardLaneSummary {
    pub active_runs: u32,
    pub waiting_runs: u32,
    pub failed_runs: u32,
    pub throughput_per_sec: f32,
    pub avg_latency_ms: u64,
}

impl ShardLaneSummary {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            active_runs: 0,
            waiting_runs: 0,
            failed_runs: 0,
            throughput_per_sec: 0.0,
            avg_latency_ms: 0,
        }
    }

    #[must_use]
    pub fn health(&self) -> LaneHealth {
        if self.failed_runs > 0 && (self.throughput_per_sec <= 1.0 || self.avg_latency_ms > 2000) {
            return LaneHealth::Red;
        }
        if (self.throughput_per_sec <= 10.0 && self.active_runs > 0) || self.avg_latency_ms > 500 {
            return LaneHealth::Yellow;
        }
        LaneHealth::Green
    }
}
