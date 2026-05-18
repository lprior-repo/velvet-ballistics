#![forbid(unsafe_code)]
//! Lane models: ShardLane and ActivityLanes aggregation.

use vb_ipc::ShardMetrics;

#[derive(Debug, Clone, PartialEq)]
pub struct ShardLane {
    pub shard_id: u32,
    pub active_runs: u32,
    pub ready_queue_depth: u32,
    pub action_queue_depth: u32,
    pub timer_count: u32,
    pub frame_pool_free: u32,
    pub frame_pool_total: u32,
    pub trace_ring_fill_pct: f32,
    pub steps_per_second: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivityLanes {
    lanes: Vec<ShardLane>,
}

impl ActivityLanes {
    #[must_use]
    pub fn new() -> Self {
        Self { lanes: Vec::new() }
    }

    pub fn update_from_metrics(&mut self, m: &ShardMetrics) {
        let updated = ShardLane {
            shard_id: m.shard_id,
            active_runs: m.active_runs,
            ready_queue_depth: m.ready_queue_depth,
            action_queue_depth: m.action_queue_depth,
            timer_count: m.timer_count,
            frame_pool_free: m.frame_pool_free,
            frame_pool_total: m.frame_pool_total,
            trace_ring_fill_pct: m.trace_ring_fill_pct,
            steps_per_second: 0,
        };

        let existing = self.lanes.iter_mut().find(|l| l.shard_id == m.shard_id);
        match existing {
            Some(lane) => *lane = updated,
            None => self.lanes.push(updated),
        }
    }

    #[must_use]
    pub fn lanes(&self) -> &[ShardLane] {
        &self.lanes
    }

    #[must_use]
    pub fn total_active_runs(&self) -> u32 {
        self.lanes
            .iter()
            .fold(0u32, |acc, l| acc.saturating_add(l.active_runs))
    }

    #[must_use]
    pub fn total_ready_queue(&self) -> u32 {
        self.lanes
            .iter()
            .fold(0u32, |acc, l| acc.saturating_add(l.ready_queue_depth))
    }

    #[must_use]
    pub fn total_action_queue(&self) -> u32 {
        self.lanes
            .iter()
            .fold(0u32, |acc, l| acc.saturating_add(l.action_queue_depth))
    }

    #[must_use]
    pub fn most_loaded_shard(&self) -> Option<usize> {
        self.lanes
            .iter()
            .enumerate()
            .max_by_key(|(_idx, l)| l.ready_queue_depth.saturating_add(l.action_queue_depth))
            .map(|(idx, _l)| idx)
    }

    #[must_use]
    pub fn avg_trace_fill(&self) -> f32 {
        if self.lanes.is_empty() {
            return 0.0;
        }
        let (sum, count) = self.lanes.iter().fold((0.0_f32, 0.0_f32), |(s, c), l| {
            (s + l.trace_ring_fill_pct, c + 1.0)
        });
        sum / count
    }
}

impl Default for ActivityLanes {
    fn default() -> Self {
        Self::new()
    }
}
