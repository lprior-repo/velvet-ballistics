//! Activity lanes component for visualizing per-shard load across the runtime.
//!
//! Each [`ShardLane`] captures a point-in-time snapshot of one shard's queue
//! depths, frame pool usage, and trace ring fill. [`ActivityLanes`] aggregates
//! lanes and provides cross-shard totals and load rankings.

use vb_ipc::ShardMetrics;

/// Per-shard activity snapshot used by the UI lane visualisation.
#[derive(Debug, Clone, PartialEq)]
pub struct ShardLane {
    /// Shard index.
    pub shard_id: u32,
    /// Number of active runs on this shard.
    pub active_runs: u32,
    /// Commands waiting in the ready queue.
    pub ready_queue_depth: u32,
    /// Commands in the action queue.
    pub action_queue_depth: u32,
    /// Pending timers on this shard.
    pub timer_count: u32,
    /// Free frames remaining in the frame pool.
    pub frame_pool_free: u32,
    /// Total capacity of the frame pool.
    pub frame_pool_total: u32,
    /// Trace ring fill percentage (0.0 -- 100.0).
    pub trace_ring_fill_pct: f32,
    /// Steps executed per second (requires external rate computation).
    pub steps_per_second: u32,
}

/// Aggregated activity lanes across all shards.
#[derive(Debug, Clone, PartialEq)]
pub struct ActivityLanes {
    lanes: Vec<ShardLane>,
}

impl ActivityLanes {
    /// Create an empty set of activity lanes.
    #[must_use]
    pub fn new() -> Self {
        Self {
            lanes: Vec::new(),
        }
    }

    /// Update an existing lane for the given shard, or append a new one.
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

    /// Read-only slice of all lanes, in insertion order.
    #[must_use]
    pub fn lanes(&self) -> &[ShardLane] {
        &self.lanes
    }

    /// Sum of active runs across all shards.
    #[must_use]
    pub fn total_active_runs(&self) -> u32 {
        self.lanes
            .iter()
            .fold(0u32, |acc, l| acc.saturating_add(l.active_runs))
    }

    /// Sum of ready queue depths across all shards.
    #[must_use]
    pub fn total_ready_queue(&self) -> u32 {
        self.lanes
            .iter()
            .fold(0u32, |acc, l| acc.saturating_add(l.ready_queue_depth))
    }

    /// Sum of action queue depths across all shards.
    #[must_use]
    pub fn total_action_queue(&self) -> u32 {
        self.lanes
            .iter()
            .fold(0u32, |acc, l| acc.saturating_add(l.action_queue_depth))
    }

    /// Index of the shard with the highest combined queue depth
    /// (ready + action). Returns `None` when there are no lanes.
    #[must_use]
    pub fn most_loaded_shard(&self) -> Option<usize> {
        self.lanes
            .iter()
            .enumerate()
            .max_by_key(|(_idx, l)| l.ready_queue_depth.saturating_add(l.action_queue_depth))
            .map(|(idx, _l)| idx)
    }

    /// Average trace ring fill percentage across all shards.
    /// Returns 0.0 when there are no lanes.
    #[must_use]
    pub fn avg_trace_fill(&self) -> f32 {
        if self.lanes.is_empty() {
            return 0.0;
        }
        let (sum, count) = self
            .lanes
            .iter()
            .fold((0.0_f32, 0.0_f32), |(s, c), l| {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_metrics(id: u32) -> ShardMetrics {
        ShardMetrics {
            shard_id: id,
            active_runs: 5,
            ready_queue_depth: 10,
            action_queue_depth: 20,
            timer_count: 3,
            frame_pool_free: 40,
            frame_pool_total: 100,
            trace_ring_fill_pct: 25.0,
            steps_total: 5000,
            actions_total: 2000,
        }
    }

    #[test]
    fn new_activity_lanes_is_empty() {
        let lanes = ActivityLanes::new();
        assert!(lanes.lanes().is_empty());
        assert_eq!(lanes.total_active_runs(), 0);
        assert_eq!(lanes.total_ready_queue(), 0);
        assert_eq!(lanes.total_action_queue(), 0);
        assert!(lanes.most_loaded_shard().is_none());
        assert_eq!(lanes.avg_trace_fill(), 0.0);
    }

    #[test]
    fn default_matches_new() {
        assert_eq!(ActivityLanes::default(), ActivityLanes::new());
    }

    #[test]
    fn single_update_creates_one_lane() {
        let mut lanes = ActivityLanes::new();
        let m = sample_metrics(0);
        lanes.update_from_metrics(&m);
        assert_eq!(lanes.lanes().len(), 1);
        let lane = lanes.lanes().get(0).expect("index 0 must exist after push");
        assert_eq!(lane.shard_id, 0);
        assert_eq!(lane.active_runs, 5);
        assert_eq!(lane.ready_queue_depth, 10);
        assert_eq!(lane.action_queue_depth, 20);
        assert_eq!(lane.timer_count, 3);
        assert_eq!(lane.frame_pool_free, 40);
        assert_eq!(lane.frame_pool_total, 100);
    }

    #[test]
    fn single_update_totals_match_lane() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&sample_metrics(2));
        assert_eq!(lanes.total_active_runs(), 5);
        assert_eq!(lanes.total_ready_queue(), 10);
        assert_eq!(lanes.total_action_queue(), 20);
    }

    #[test]
    fn multiple_shards_create_separate_lanes() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&sample_metrics(0));
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 1,
            active_runs: 8,
            ready_queue_depth: 15,
            action_queue_depth: 25,
            timer_count: 1,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 50.0,
            steps_total: 1000,
            actions_total: 500,
        });
        assert_eq!(lanes.lanes().len(), 2);
    }

    #[test]
    fn multiple_shards_totals_sum_correctly() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 3,
            ready_queue_depth: 4,
            action_queue_depth: 6,
            timer_count: 0,
            frame_pool_free: 50,
            frame_pool_total: 100,
            trace_ring_fill_pct: 10.0,
            steps_total: 0,
            actions_total: 0,
        });
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 1,
            active_runs: 7,
            ready_queue_depth: 11,
            action_queue_depth: 14,
            timer_count: 0,
            frame_pool_free: 80,
            frame_pool_total: 100,
            trace_ring_fill_pct: 30.0,
            steps_total: 0,
            actions_total: 0,
        });
        assert_eq!(lanes.total_active_runs(), 10);
        assert_eq!(lanes.total_ready_queue(), 15);
        assert_eq!(lanes.total_action_queue(), 20);
    }

    #[test]
    fn most_loaded_shard_returns_highest_combined_depth() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 1,
            ready_queue_depth: 5,
            action_queue_depth: 5,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 1,
            active_runs: 1,
            ready_queue_depth: 50,
            action_queue_depth: 50,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        assert_eq!(lanes.most_loaded_shard(), Some(1));
    }

    #[test]
    fn most_loaded_shard_none_when_empty() {
        let lanes = ActivityLanes::new();
        assert_eq!(lanes.most_loaded_shard(), None);
    }

    #[test]
    fn most_loaded_shard_returns_valid_index_on_tie() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 10,
            active_runs: 0,
            ready_queue_depth: 10,
            action_queue_depth: 10,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 20,
            active_runs: 0,
            ready_queue_depth: 10,
            action_queue_depth: 10,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        // Both have combined depth 20; either index is valid.
        let result = lanes.most_loaded_shard();
        assert!(result == Some(0) || result == Some(1));
    }

    #[test]
    fn avg_trace_fill_computes_average() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 40.0,
            steps_total: 0,
            actions_total: 0,
        });
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 1,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 60.0,
            steps_total: 0,
            actions_total: 0,
        });
        let avg = lanes.avg_trace_fill();
        assert!(
            (avg - 50.0).abs() < 0.01,
            "expected ~50.0, got {}",
            avg
        );
    }

    #[test]
    fn avg_trace_fill_zero_when_empty() {
        let lanes = ActivityLanes::new();
        assert_eq!(lanes.avg_trace_fill(), 0.0);
    }

    #[test]
    fn avg_trace_fill_single_shard() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 73.5,
            steps_total: 0,
            actions_total: 0,
        });
        let avg = lanes.avg_trace_fill();
        assert!((avg - 73.5).abs() < 0.01, "expected ~73.5, got {}", avg);
    }

    #[test]
    fn update_existing_shard_replaces_values() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 5,
            active_runs: 1,
            ready_queue_depth: 2,
            action_queue_depth: 3,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 10.0,
            steps_total: 0,
            actions_total: 0,
        });
        assert_eq!(lanes.lanes().len(), 1);

        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 5,
            active_runs: 99,
            ready_queue_depth: 88,
            action_queue_depth: 77,
            timer_count: 10,
            frame_pool_free: 1,
            frame_pool_total: 100,
            trace_ring_fill_pct: 95.0,
            steps_total: 0,
            actions_total: 0,
        });
        assert_eq!(lanes.lanes().len(), 1);
        let lane = lanes.lanes().get(0).expect("lane must exist");
        assert_eq!(lane.active_runs, 99);
        assert_eq!(lane.ready_queue_depth, 88);
        assert_eq!(lane.action_queue_depth, 77);
        assert_eq!(lane.frame_pool_free, 1);
    }

    #[test]
    fn frame_pool_fields_carried_from_metrics() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 33,
            frame_pool_total: 128,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        let lane = lanes.lanes().get(0).expect("lane must exist");
        assert_eq!(lane.frame_pool_free, 33);
        assert_eq!(lane.frame_pool_total, 128);
    }

    #[test]
    fn steps_per_second_defaults_to_zero() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 1_000_000,
            actions_total: 0,
        });
        let lane = lanes.lanes().get(0).expect("lane must exist");
        assert_eq!(lane.steps_per_second, 0);
    }

    #[test]
    fn three_shards_totals_and_most_loaded() {
        let mut lanes = ActivityLanes::new();
        // Shard 0: light load
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 2,
            ready_queue_depth: 1,
            action_queue_depth: 1,
            timer_count: 0,
            frame_pool_free: 95,
            frame_pool_total: 100,
            trace_ring_fill_pct: 5.0,
            steps_total: 0,
            actions_total: 0,
        });
        // Shard 1: medium load
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 1,
            active_runs: 5,
            ready_queue_depth: 10,
            action_queue_depth: 15,
            timer_count: 3,
            frame_pool_free: 70,
            frame_pool_total: 100,
            trace_ring_fill_pct: 40.0,
            steps_total: 0,
            actions_total: 0,
        });
        // Shard 2: heavy load
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 2,
            active_runs: 10,
            ready_queue_depth: 20,
            action_queue_depth: 30,
            timer_count: 8,
            frame_pool_free: 10,
            frame_pool_total: 100,
            trace_ring_fill_pct: 90.0,
            steps_total: 0,
            actions_total: 0,
        });

        assert_eq!(lanes.lanes().len(), 3);
        assert_eq!(lanes.total_active_runs(), 17);
        assert_eq!(lanes.total_ready_queue(), 31);
        assert_eq!(lanes.total_action_queue(), 46);
        assert_eq!(lanes.most_loaded_shard(), Some(2));

        let avg = lanes.avg_trace_fill();
        let expected = (5.0 + 40.0 + 90.0) / 3.0;
        assert!(
            (avg - expected).abs() < 0.01,
            "expected ~{}, got {}",
            expected,
            avg
        );
    }

    #[test]
    fn update_does_not_duplicate_shard() {
        let mut lanes = ActivityLanes::new();
        let m = ShardMetrics {
            shard_id: 7,
            active_runs: 1,
            ready_queue_depth: 1,
            action_queue_depth: 1,
            timer_count: 0,
            frame_pool_free: 50,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 0,
            actions_total: 0,
        };
        lanes.update_from_metrics(&m);
        lanes.update_from_metrics(&m);
        lanes.update_from_metrics(&m);
        assert_eq!(lanes.lanes().len(), 1);
    }
}
