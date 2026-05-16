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
        assert!((avg - 50.0).abs() < 0.01, "expected ~50.0, got {}", avg);
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

    #[test]
    fn total_active_runs_saturates_at_u32_max() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: u32::MAX,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 1,
            active_runs: u32::MAX,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        assert_eq!(lanes.total_active_runs(), u32::MAX);
    }

    #[test]
    fn total_ready_queue_saturates_at_u32_max() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: u32::MAX,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 1,
            active_runs: 0,
            ready_queue_depth: u32::MAX,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        assert_eq!(lanes.total_ready_queue(), u32::MAX);
    }

    #[test]
    fn total_action_queue_saturates_at_u32_max() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: u32::MAX,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 1,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: u32::MAX,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        assert_eq!(lanes.total_action_queue(), u32::MAX);
    }

    #[test]
    fn lanes_maintain_insertion_order() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 5,
            active_runs: 50,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 2,
            active_runs: 20,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 8,
            active_runs: 80,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        let all_lanes = lanes.lanes();
        assert_eq!(all_lanes.len(), 3);
        // Order must be 5, 2, 8 (insertion order), not 2, 5, 8 (sorted).
        let Some(first) = all_lanes.get(0) else {
            return;
        };
        let Some(second) = all_lanes.get(1) else {
            return;
        };
        let Some(third) = all_lanes.get(2) else {
            return;
        };
        assert_eq!(first.shard_id, 5);
        assert_eq!(second.shard_id, 2);
        assert_eq!(third.shard_id, 8);
    }

    #[test]
    fn update_existing_shard_preserves_position() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 10,
            active_runs: 1,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 20,
            active_runs: 2,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 30,
            active_runs: 3,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        // Update shard 20 in place; it must stay at index 1.
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 20,
            active_runs: 99,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        let all_lanes = lanes.lanes();
        assert_eq!(all_lanes.len(), 3);
        let Some(lane_0) = all_lanes.get(0) else {
            return;
        };
        let Some(lane_1) = all_lanes.get(1) else {
            return;
        };
        let Some(lane_2) = all_lanes.get(2) else {
            return;
        };
        assert_eq!(lane_0.shard_id, 10);
        assert_eq!(lane_1.shard_id, 20);
        assert_eq!(lane_2.shard_id, 30);
        // Confirm the update took effect without moving the lane.
        assert_eq!(lane_1.active_runs, 99);
    }

    #[test]
    fn timer_count_carried_from_shard_metrics() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 42,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        let Some(lane) = lanes.lanes().get(0) else {
            return;
        };
        assert_eq!(lane.timer_count, 42);
    }

    // --- Additional coverage tests ---

    #[test]
    fn update_from_metrics_with_four_shards_creates_four_lanes() {
        let mut lanes = ActivityLanes::new();
        let fill_pcts: [f32; 4] = [0.0, 10.0, 20.0, 30.0];
        let specs: [(u32, u32, u32, u32, u32, u32); 4] = [
            (0, 0, 0, 0, 0, 100),
            (1, 1, 10, 5, 1, 99),
            (2, 2, 20, 10, 2, 98),
            (3, 3, 30, 15, 3, 97),
        ];
        for (idx, &(shard_id, active, ready, action, timer, free)) in specs.iter().enumerate() {
            lanes.update_from_metrics(&ShardMetrics {
                shard_id,
                active_runs: active,
                ready_queue_depth: ready,
                action_queue_depth: action,
                timer_count: timer,
                frame_pool_free: free,
                frame_pool_total: 100,
                trace_ring_fill_pct: fill_pcts[idx],
                steps_total: 0,
                actions_total: 0,
            });
        }
        assert_eq!(lanes.lanes().len(), 4);
        assert_eq!(lanes.total_active_runs(), 0 + 1 + 2 + 3);
        assert_eq!(lanes.total_ready_queue(), 0 + 10 + 20 + 30);
        assert_eq!(lanes.total_action_queue(), 0 + 5 + 10 + 15);
        // Shard 3 has the highest combined depth (30 + 15 = 45).
        assert_eq!(lanes.most_loaded_shard(), Some(3));
    }

    #[test]
    fn zero_activity_lane_has_correct_defaults() {
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
            steps_total: 0,
            actions_total: 0,
        });
        let Some(lane) = lanes.lanes().get(0) else {
            return;
        };
        assert_eq!(lane.shard_id, 0);
        assert_eq!(lane.active_runs, 0);
        assert_eq!(lane.ready_queue_depth, 0);
        assert_eq!(lane.action_queue_depth, 0);
        assert_eq!(lane.timer_count, 0);
        assert_eq!(lane.frame_pool_free, 100);
        assert_eq!(lane.frame_pool_total, 100);
        assert_eq!(lane.trace_ring_fill_pct, 0.0);
        assert_eq!(lane.steps_per_second, 0);
    }

    #[test]
    fn max_value_across_lanes_computed_by_most_loaded() {
        let mut lanes = ActivityLanes::new();
        // Shard 0: ready=1, action=1  -> combined=2
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: 1,
            action_queue_depth: 1,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        // Shard 1: ready=500, action=0 -> combined=500
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 1,
            active_runs: 0,
            ready_queue_depth: 500,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        // Shard 2: ready=0, action=499 -> combined=499
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 2,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 499,
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
    fn bucket_boundaries_trace_fill_zero_and_hundred() {
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
            steps_total: 0,
            actions_total: 0,
        });
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 1,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 0,
            frame_pool_total: 100,
            trace_ring_fill_pct: 100.0,
            steps_total: 0,
            actions_total: 0,
        });
        let avg = lanes.avg_trace_fill();
        assert!((avg - 50.0).abs() < 0.01, "expected ~50.0, got {}", avg);

        let Some(lane_0) = lanes.lanes().get(0) else {
            return;
        };
        let Some(lane_1) = lanes.lanes().get(1) else {
            return;
        };
        assert_eq!(lane_0.trace_ring_fill_pct, 0.0);
        assert_eq!(lane_1.trace_ring_fill_pct, 100.0);
        // Frame pool at boundary: fully free vs fully consumed.
        assert_eq!(lane_0.frame_pool_free, 100);
        assert_eq!(lane_0.frame_pool_total, 100);
        assert_eq!(lane_1.frame_pool_free, 0);
        assert_eq!(lane_1.frame_pool_total, 100);
    }

    #[test]
    fn mixed_activity_levels_aggregate_correctly() {
        let mut lanes = ActivityLanes::new();
        // Idle shard
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 64,
            frame_pool_total: 64,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        // Moderate shard
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 1,
            active_runs: 3,
            ready_queue_depth: 7,
            action_queue_depth: 12,
            timer_count: 2,
            frame_pool_free: 50,
            frame_pool_total: 64,
            trace_ring_fill_pct: 33.3,
            steps_total: 0,
            actions_total: 0,
        });
        // Heavy shard
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 2,
            active_runs: 10,
            ready_queue_depth: 40,
            action_queue_depth: 55,
            timer_count: 15,
            frame_pool_free: 1,
            frame_pool_total: 64,
            trace_ring_fill_pct: 98.7,
            steps_total: 0,
            actions_total: 0,
        });

        assert_eq!(lanes.total_active_runs(), 13);
        assert_eq!(lanes.total_ready_queue(), 47);
        assert_eq!(lanes.total_action_queue(), 67);
        // Heavy shard (index 2) has combined depth 40+55=95.
        assert_eq!(lanes.most_loaded_shard(), Some(2));

        let avg = lanes.avg_trace_fill();
        let expected = (0.0 + 33.3 + 98.7) / 3.0;
        assert!(
            (avg - expected).abs() < 0.05,
            "expected ~{}, got {}",
            expected,
            avg
        );
    }

    #[test]
    fn empty_metrics_no_shards_produces_empty_lanes() {
        let lanes = ActivityLanes::new();
        assert!(lanes.lanes().is_empty());
        assert_eq!(lanes.total_active_runs(), 0);
        assert_eq!(lanes.total_ready_queue(), 0);
        assert_eq!(lanes.total_action_queue(), 0);
        assert!(lanes.most_loaded_shard().is_none());
        assert_eq!(lanes.avg_trace_fill(), 0.0);
    }

    #[test]
    fn repeated_update_from_metrics_accumulates_then_refreshes() {
        let mut lanes = ActivityLanes::new();

        // First wave: two shards with initial values.
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 4,
            ready_queue_depth: 8,
            action_queue_depth: 12,
            timer_count: 1,
            frame_pool_free: 80,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 100,
            actions_total: 50,
        });
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 1,
            active_runs: 6,
            ready_queue_depth: 14,
            action_queue_depth: 18,
            timer_count: 3,
            frame_pool_free: 60,
            frame_pool_total: 100,
            trace_ring_fill_pct: 40.0,
            steps_total: 200,
            actions_total: 100,
        });
        assert_eq!(lanes.lanes().len(), 2);
        assert_eq!(lanes.total_active_runs(), 10);
        assert_eq!(lanes.total_ready_queue(), 22);
        assert_eq!(lanes.total_action_queue(), 30);

        // Second wave: update shard 0 with new values (should replace, not add).
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 2,
            ready_queue_depth: 3,
            action_queue_depth: 5,
            timer_count: 0,
            frame_pool_free: 95,
            frame_pool_total: 100,
            trace_ring_fill_pct: 10.0,
            steps_total: 300,
            actions_total: 150,
        });
        // Shard 1 was not updated, so its values persist.
        // Only shard 0 should reflect new totals.
        assert_eq!(lanes.lanes().len(), 2);
        assert_eq!(lanes.total_active_runs(), 2 + 6);
        assert_eq!(lanes.total_ready_queue(), 3 + 14);
        assert_eq!(lanes.total_action_queue(), 5 + 18);

        // Third wave: add a new shard, updating totals further.
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 2,
            active_runs: 1,
            ready_queue_depth: 2,
            action_queue_depth: 3,
            timer_count: 0,
            frame_pool_free: 99,
            frame_pool_total: 100,
            trace_ring_fill_pct: 5.0,
            steps_total: 0,
            actions_total: 0,
        });
        assert_eq!(lanes.lanes().len(), 3);
        assert_eq!(lanes.total_active_runs(), 2 + 6 + 1);
        assert_eq!(lanes.total_ready_queue(), 3 + 14 + 2);
        assert_eq!(lanes.total_action_queue(), 5 + 18 + 3);
    }

    // --- LaneSegment / LaneSegmentBuilder tests ---

    #[test]
    fn segment_builder_empty_shard_produces_no_segments() {
        let m = ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        assert!(segments.is_empty());
    }

    #[test]
    fn segment_builder_single_run_gets_full_width() {
        let m = ShardMetrics {
            shard_id: 0,
            active_runs: 1,
            ready_queue_depth: 10,
            action_queue_depth: 5,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        assert_eq!(segments.len(), 1);
        assert!((segments[0].width_ratio - 1.0).abs() < 0.001);
        assert_eq!(segments[0].label, format!("R{}", segments[0].run_id));
    }

    #[test]
    fn segment_builder_multiple_runs_proportional_widths_sum_to_one() {
        let m = ShardMetrics {
            shard_id: 0,
            active_runs: 4,
            ready_queue_depth: 10,
            action_queue_depth: 10,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        assert_eq!(segments.len(), 4);

        let total: f32 = segments.iter().map(|s| s.width_ratio).sum();
        assert!(
            (total - 1.0).abs() < 0.001,
            "width ratios must sum to ~1.0, got {}",
            total
        );

        // Each non-last segment should have equal width.
        for seg in &segments[..3] {
            assert!(
                (seg.width_ratio - 0.25).abs() < 0.001,
                "expected 0.25, got {}",
                seg.width_ratio
            );
        }
    }

    #[test]
    fn segment_builder_color_mapping_matches_theme_state_colors() {
        // Running state: action_queue <= ready_queue, pool healthy, trace low.
        let running_m = ShardMetrics {
            shard_id: 0,
            active_runs: 2,
            ready_queue_depth: 20,
            action_queue_depth: 10,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 0,
            actions_total: 0,
        };
        let running_segments = LaneSegmentBuilder::build(&running_m);
        let running_color = RunState::Running.color();
        assert_eq!(running_segments.len(), 2);
        assert_eq!(running_segments[0].state_color, running_color);

        // Waiting state: action_queue > ready_queue.
        let waiting_m = ShardMetrics {
            shard_id: 1,
            active_runs: 3,
            ready_queue_depth: 5,
            action_queue_depth: 50,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 0,
            actions_total: 0,
        };
        let waiting_segments = LaneSegmentBuilder::build(&waiting_m);
        let waiting_color = RunState::Waiting.color();
        assert_eq!(waiting_segments.len(), 3);
        assert_eq!(waiting_segments[0].state_color, waiting_color);

        // Critical state: pool exhausted AND queue ratio >= 0.3.
        let critical_m = ShardMetrics {
            shard_id: 2,
            active_runs: 1,
            ready_queue_depth: 50,
            action_queue_depth: 50,
            timer_count: 0,
            frame_pool_free: 2,
            frame_pool_total: 100,
            trace_ring_fill_pct: 50.0,
            steps_total: 0,
            actions_total: 0,
        };
        let critical_segments = LaneSegmentBuilder::build(&critical_m);
        let critical_color = RunState::Critical.color();
        assert_eq!(critical_segments.len(), 1);
        assert_eq!(critical_segments[0].state_color, critical_color);
    }

    #[test]
    fn segment_builder_zero_queue_depth_runs_get_equal_width() {
        let m = ShardMetrics {
            shard_id: 0,
            active_runs: 3,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 10.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        assert_eq!(segments.len(), 3);

        let total: f32 = segments.iter().map(|s| s.width_ratio).sum();
        assert!(
            (total - 1.0).abs() < 0.001,
            "width ratios must sum to ~1.0, got {}",
            total
        );

        // All segments should be Running since no queue pressure.
        let running_color = RunState::Running.color();
        for seg in &segments {
            assert_eq!(seg.state_color, running_color);
        }
    }

    #[test]
    fn segment_builder_labels_use_run_id() {
        let m = ShardMetrics {
            shard_id: 2,
            active_runs: 2,
            ready_queue_depth: 10,
            action_queue_depth: 5,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        assert_eq!(segments.len(), 2);
        for seg in &segments {
            assert!(
                seg.label.starts_with('R'),
                "label should start with 'R', got '{}'",
                seg.label
            );
        }
    }

    #[test]
    fn segment_builder_run_ids_are_unique_per_shard() {
        let m = ShardMetrics {
            shard_id: 0,
            active_runs: 5,
            ready_queue_depth: 10,
            action_queue_depth: 5,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        let ids: Vec<u64> = segments.iter().map(|s| s.run_id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "run IDs must be unique");
    }

    #[test]
    fn segment_builder_degraded_state_from_trace_ring() {
        // trace >= 70.0 and queue_ratio >= 0.2 should yield Degraded.
        // With 3 runs, each gets queue_ratio = 1/3 ~ 0.33 >= 0.2.
        let m = ShardMetrics {
            shard_id: 0,
            active_runs: 3,
            ready_queue_depth: 10,
            action_queue_depth: 5,
            timer_count: 0,
            frame_pool_free: 70,
            frame_pool_total: 100,
            trace_ring_fill_pct: 75.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        let degraded_color = RunState::Degraded.color();
        assert_eq!(segments[0].state_color, degraded_color);
    }

    #[test]
    fn segment_builder_width_ratios_non_negative() {
        let m = ShardMetrics {
            shard_id: 0,
            active_runs: 7,
            ready_queue_depth: 10,
            action_queue_depth: 5,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        for seg in &segments {
            assert!(
                seg.width_ratio >= 0.0,
                "width_ratio must be non-negative, got {}",
                seg.width_ratio
            );
        }
    }

    #[test]
    fn run_state_color_running_matches_neon_cyan() {
        assert_eq!(RunState::Running.color(), [0.0, 0.961, 1.0, 1.0]);
    }

    #[test]
    fn run_state_color_waiting_matches_neon_blue() {
        assert_eq!(RunState::Waiting.color(), [0.176, 0.420, 1.0, 1.0]);
    }

    #[test]
    fn run_state_color_degraded_matches_neon_yellow() {
        assert_eq!(RunState::Degraded.color(), [1.0, 0.902, 0.0, 1.0]);
    }

    #[test]
    fn run_state_color_critical_matches_neon_red() {
        assert_eq!(RunState::Critical.color(), [1.0, 0.027, 0.227, 1.0]);
    }

    #[test]
    fn segment_builder_many_runs_all_get_segments() {
        let m = ShardMetrics {
            shard_id: 0,
            active_runs: 50,
            ready_queue_depth: 100,
            action_queue_depth: 100,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        assert_eq!(segments.len(), 50);

        let total: f32 = segments.iter().map(|s| s.width_ratio).sum();
        assert!(
            (total - 1.0).abs() < 0.01,
            "width ratios must sum to ~1.0 for 50 runs, got {}",
            total
        );
    }

    // =========================================================================
    // Additional comprehensive coverage tests
    // =========================================================================

    #[test]
    fn segment_builder_last_segment_absorbs_rounding_residual() {
        // With 3 runs, each gets 1/3. The last segment absorbs residual so
        // the total is exactly 1.0 in f64 before the f64->f32 conversion.
        let m = ShardMetrics {
            shard_id: 0,
            active_runs: 3,
            ready_queue_depth: 10,
            action_queue_depth: 5,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        assert_eq!(segments.len(), 3);
        let total: f32 = segments.iter().map(|s| s.width_ratio).sum();
        assert!(
            (total - 1.0).abs() < 0.01,
            "total width must be ~1.0, got {}",
            total
        );
        // The last segment's width may differ slightly from the others.
        assert!(segments[2].width_ratio > 0.0);
    }

    #[test]
    fn segment_builder_run_id_includes_shard_offset() {
        // Run IDs should be SYNTHETIC_RUN_ID_OFFSET + shard_id * 10_000 + i.
        let m = ShardMetrics {
            shard_id: 2,
            active_runs: 3,
            ready_queue_depth: 10,
            action_queue_depth: 5,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        assert_eq!(segments.len(), 3);
        let expected_base =
            SYNTHETIC_RUN_ID_OFFSET.saturating_add(u64::from(2u32).saturating_mul(10_000));
        assert_eq!(segments[0].run_id, expected_base);
        assert_eq!(segments[1].run_id, expected_base.saturating_add(1));
        assert_eq!(segments[2].run_id, expected_base.saturating_add(2));
    }

    #[test]
    fn shard_lane_equality_works() {
        let lane_a = ShardLane {
            shard_id: 0,
            active_runs: 5,
            ready_queue_depth: 10,
            action_queue_depth: 20,
            timer_count: 3,
            frame_pool_free: 40,
            frame_pool_total: 100,
            trace_ring_fill_pct: 25.0,
            steps_per_second: 0,
        };
        let lane_b = ShardLane {
            shard_id: 0,
            active_runs: 5,
            ready_queue_depth: 10,
            action_queue_depth: 20,
            timer_count: 3,
            frame_pool_free: 40,
            frame_pool_total: 100,
            trace_ring_fill_pct: 25.0,
            steps_per_second: 0,
        };
        assert_eq!(lane_a, lane_b);
    }

    #[test]
    fn shard_lane_inequality_by_shard_id() {
        let lane_a = ShardLane {
            shard_id: 0,
            active_runs: 5,
            ready_queue_depth: 10,
            action_queue_depth: 20,
            timer_count: 3,
            frame_pool_free: 40,
            frame_pool_total: 100,
            trace_ring_fill_pct: 25.0,
            steps_per_second: 0,
        };
        let lane_b = ShardLane {
            shard_id: 1,
            active_runs: 5,
            ready_queue_depth: 10,
            action_queue_depth: 20,
            timer_count: 3,
            frame_pool_free: 40,
            frame_pool_total: 100,
            trace_ring_fill_pct: 25.0,
            steps_per_second: 0,
        };
        assert_ne!(lane_a, lane_b);
    }

    #[test]
    fn activity_lanes_equality_works() {
        let a = ActivityLanes::new();
        let b = ActivityLanes::new();
        assert_eq!(a, b);
    }

    #[test]
    fn activity_lanes_inequality_after_update() {
        let mut a = ActivityLanes::new();
        let b = ActivityLanes::new();
        a.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 1,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        assert_ne!(a, b);
    }

    #[test]
    fn run_state_debug_format() {
        assert!(format!("{:?}", RunState::Running).contains("Running"));
        assert!(format!("{:?}", RunState::Waiting).contains("Waiting"));
        assert!(format!("{:?}", RunState::Degraded).contains("Degraded"));
        assert!(format!("{:?}", RunState::Critical).contains("Critical"));
    }

    #[test]
    fn run_state_copy_and_equality() {
        let state = RunState::Running;
        let copied = state;
        assert_eq!(state, copied);
        assert_ne!(state, RunState::Critical);
    }

    #[test]
    fn lane_segment_equality_works() {
        let a = LaneSegment {
            run_id: 1,
            width_ratio: 0.5,
            state_color: [0.0, 0.0, 0.0, 1.0],
            label: "R1".to_string(),
        };
        let b = LaneSegment {
            run_id: 1,
            width_ratio: 0.5,
            state_color: [0.0, 0.0, 0.0, 1.0],
            label: "R1".to_string(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn segment_builder_critical_from_pool_exhaustion() {
        // Pool ratio >= 0.8 AND queue_ratio >= 0.3 -> Critical.
        // With 2 runs, each gets queue_ratio = 0.5 >= 0.3.
        // frame_pool_free = 10 out of 100 -> pool_ratio = 0.9 >= 0.8.
        let m = ShardMetrics {
            shard_id: 0,
            active_runs: 2,
            ready_queue_depth: 10,
            action_queue_depth: 5,
            timer_count: 0,
            frame_pool_free: 10,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        let critical_color = RunState::Critical.color();
        assert_eq!(segments[0].state_color, critical_color);
    }

    #[test]
    fn segment_builder_critical_from_trace_ring_high() {
        // trace_ring_fill_pct >= 90.0 AND queue_ratio >= 0.3 -> Critical.
        // With 2 runs, each gets queue_ratio = 0.5 >= 0.3.
        let m = ShardMetrics {
            shard_id: 0,
            active_runs: 2,
            ready_queue_depth: 10,
            action_queue_depth: 5,
            timer_count: 0,
            frame_pool_free: 50,
            frame_pool_total: 100,
            trace_ring_fill_pct: 95.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        let critical_color = RunState::Critical.color();
        assert_eq!(segments[0].state_color, critical_color);
    }

    #[test]
    fn segment_builder_waiting_from_action_queue_greater_than_ready() {
        // action_queue_depth > ready_queue_depth -> Waiting.
        // Pool and trace are healthy so no Critical/Degraded.
        let m = ShardMetrics {
            shard_id: 0,
            active_runs: 2,
            ready_queue_depth: 5,
            action_queue_depth: 50,
            timer_count: 0,
            frame_pool_free: 95,
            frame_pool_total: 100,
            trace_ring_fill_pct: 10.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        let waiting_color = RunState::Waiting.color();
        assert_eq!(segments[0].state_color, waiting_color);
    }

    #[test]
    fn segment_builder_running_when_all_conditions_healthy() {
        // No pool pressure, low trace, action <= ready -> Running.
        let m = ShardMetrics {
            shard_id: 0,
            active_runs: 2,
            ready_queue_depth: 20,
            action_queue_depth: 10,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 10.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        let running_color = RunState::Running.color();
        assert_eq!(segments[0].state_color, running_color);
    }

    #[test]
    fn segment_builder_with_zero_frame_pool_total() {
        // frame_pool_total == 0 -> pool_ratio = 0.0, so no pool pressure.
        // If action <= ready and trace < 70, state should be Running.
        let m = ShardMetrics {
            shard_id: 0,
            active_runs: 2,
            ready_queue_depth: 10,
            action_queue_depth: 5,
            timer_count: 0,
            frame_pool_free: 0,
            frame_pool_total: 0,
            trace_ring_fill_pct: 10.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        assert_eq!(segments.len(), 2);
        let running_color = RunState::Running.color();
        assert_eq!(segments[0].state_color, running_color);
    }

    #[test]
    fn segment_builder_with_high_shard_id() {
        let m = ShardMetrics {
            shard_id: 255,
            active_runs: 2,
            ready_queue_depth: 10,
            action_queue_depth: 5,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 0,
            actions_total: 0,
        };
        let segments = LaneSegmentBuilder::build(&m);
        assert_eq!(segments.len(), 2);
        let expected_base =
            SYNTHETIC_RUN_ID_OFFSET.saturating_add(u64::from(255u32).saturating_mul(10_000));
        assert_eq!(segments[0].run_id, expected_base);
    }

    #[test]
    fn avg_trace_fill_single_lane_matches_exactly() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 50,
            frame_pool_total: 100,
            trace_ring_fill_pct: 100.0,
            steps_total: 0,
            actions_total: 0,
        });
        let avg = lanes.avg_trace_fill();
        assert!((avg - 100.0).abs() < 0.01);
    }

    #[test]
    fn most_loaded_shard_with_single_shard() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 5,
            ready_queue_depth: 10,
            action_queue_depth: 20,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        });
        assert_eq!(lanes.most_loaded_shard(), Some(0));
    }

    #[test]
    fn trace_ring_fill_pct_carried_from_metrics() {
        let mut lanes = ActivityLanes::new();
        lanes.update_from_metrics(&ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 100,
            trace_ring_fill_pct: 67.8,
            steps_total: 0,
            actions_total: 0,
        });
        let Some(lane) = lanes.lanes().get(0) else {
            return;
        };
        assert!((lane.trace_ring_fill_pct - 67.8).abs() < 0.01);
    }

    #[test]
    fn shard_lane_debug_contains_fields() {
        let lane = ShardLane {
            shard_id: 5,
            active_runs: 3,
            ready_queue_depth: 10,
            action_queue_depth: 20,
            timer_count: 1,
            frame_pool_free: 50,
            frame_pool_total: 100,
            trace_ring_fill_pct: 42.0,
            steps_per_second: 99,
        };
        let debug = format!("{lane:?}");
        assert!(debug.contains("shard_id"));
        assert!(debug.contains("active_runs"));
    }

    #[test]
    fn activity_lanes_debug_format() {
        let lanes = ActivityLanes::new();
        let debug = format!("{lanes:?}");
        assert!(debug.contains("ActivityLanes") || debug.contains("lanes"));
    }

    #[test]
    fn lane_segment_debug_format() {
        let seg = LaneSegment {
            run_id: 42,
            width_ratio: 0.25,
            state_color: RunState::Running.color(),
            label: "R42".to_string(),
        };
        let debug = format!("{seg:?}");
        assert!(debug.contains("run_id") || debug.contains("42"));
    }

    // =========================================================================
    // ActivityHeatmap tests
    // =========================================================================

    #[test]
    fn heatmap_new_returns_none_for_zero_bucket_count() {
        assert!(ActivityHeatmap::new(0, 100).is_none());
    }

    #[test]
    fn heatmap_new_returns_none_for_zero_bucket_duration() {
        assert!(ActivityHeatmap::new(10, 0).is_none());
    }

    #[test]
    fn heatmap_new_returns_none_for_both_zero() {
        assert!(ActivityHeatmap::new(0, 0).is_none());
    }

    #[test]
    fn heatmap_new_succeeds_with_valid_args() {
        let hm = ActivityHeatmap::new(5, 1000);
        assert!(hm.is_some());
        let hm = hm.expect("valid heatmap");
        assert_eq!(hm.bucket_count, 5);
        assert_eq!(hm.bucket_duration_ms, 1000);
        assert_eq!(hm.max_bucket, 0);
    }

    #[test]
    fn heatmap_record_event_increments_correct_bucket() {
        let mut hm = ActivityHeatmap::new(4, 100).expect("valid");
        hm.record_event(50);
        assert_eq!(hm.max_bucket, 1);
        assert!((hm.intensity(0) - 1.0).abs() < 0.001);
        assert!(hm.intensity(1).abs() < 0.001);
        assert!(hm.intensity(2).abs() < 0.001);
        assert!(hm.intensity(3).abs() < 0.001);
    }

    #[test]
    fn heatmap_record_event_multiple_in_same_bucket() {
        let mut hm = ActivityHeatmap::new(3, 100).expect("valid");
        hm.record_event(10);
        hm.record_event(20);
        hm.record_event(30);
        assert_eq!(hm.max_bucket, 3);
        assert!((hm.intensity(0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn heatmap_record_event_across_multiple_buckets() {
        let mut hm = ActivityHeatmap::new(4, 100).expect("valid");
        hm.record_event(50);
        hm.record_event(150);
        hm.record_event(250);
        hm.record_event(350);
        assert_eq!(hm.max_bucket, 1);
        for i in 0..4u32 {
            assert!(
                (hm.intensity(i) - 1.0).abs() < 0.001,
                "bucket {} should have intensity 1.0",
                i
            );
        }
    }

    #[test]
    fn heatmap_intensity_zero_when_no_events() {
        let hm = ActivityHeatmap::new(5, 100).expect("valid");
        assert!(hm.intensity(0).abs() < 0.001);
        assert!(hm.intensity(4).abs() < 0.001);
    }

    #[test]
    fn heatmap_intensity_zero_for_out_of_range_bucket() {
        let mut hm = ActivityHeatmap::new(3, 100).expect("valid");
        hm.record_event(50);
        assert!(hm.intensity(100).abs() < 0.001);
        assert!(hm.intensity(u32::MAX).abs() < 0.001);
    }

    #[test]
    fn heatmap_normalisation_with_uneven_distribution() {
        let mut hm = ActivityHeatmap::new(3, 100).expect("valid");
        for _ in 0..5 {
            hm.record_event(10);
        }
        hm.record_event(150);
        assert_eq!(hm.max_bucket, 5);
        assert!((hm.intensity(0) - 1.0).abs() < 0.001);
        assert!((hm.intensity(1) - 0.2).abs() < 0.001);
        assert!(hm.intensity(2).abs() < 0.001);
    }

    #[test]
    fn heatmap_record_event_clamps_to_last_bucket() {
        let mut hm = ActivityHeatmap::new(3, 100).expect("valid");
        hm.record_event(500);
        assert_eq!(hm.max_bucket, 1);
        assert!((hm.intensity(2) - 1.0).abs() < 0.001);
    }

    #[test]
    fn heatmap_record_event_at_exact_boundary() {
        let mut hm = ActivityHeatmap::new(4, 100).expect("valid");
        hm.record_event(100);
        assert_eq!(hm.max_bucket, 1);
        assert!(hm.intensity(0).abs() < 0.001);
        assert!((hm.intensity(1) - 1.0).abs() < 0.001);
    }

    #[test]
    fn heatmap_record_event_saturating_add() {
        let mut hm = ActivityHeatmap::new(1, 100).expect("valid");
        hm.buckets = vec![u32::MAX].into_boxed_slice();
        hm.max_bucket = u32::MAX;
        hm.record_event(50);
        assert_eq!(hm.max_bucket, u32::MAX);
    }

    #[test]
    fn heatmap_debug_format() {
        let hm = ActivityHeatmap::new(3, 100).expect("valid");
        let debug = format!("{hm:?}");
        assert!(debug.contains("ActivityHeatmap") || debug.contains("buckets"));
    }

    #[test]
    fn heatmap_equality() {
        let a = ActivityHeatmap::new(3, 100);
        let b = ActivityHeatmap::new(3, 100);
        assert_eq!(a, b);
    }

    #[test]
    fn heatmap_inequality_different_bucket_count() {
        let a = ActivityHeatmap::new(3, 100);
        let b = ActivityHeatmap::new(5, 100);
        assert_ne!(a, b);
    }

    // =========================================================================
    // LaneHealth tests
    // =========================================================================

    #[test]
    fn lane_health_copy_and_equality() {
        let green = LaneHealth::Green;
        let copied = green;
        assert_eq!(green, copied);
        assert_ne!(green, LaneHealth::Red);
    }

    #[test]
    fn lane_health_debug_format() {
        assert!(format!("{:?}", LaneHealth::Green).contains("Green"));
        assert!(format!("{:?}", LaneHealth::Yellow).contains("Yellow"));
        assert!(format!("{:?}", LaneHealth::Red).contains("Red"));
    }

    // =========================================================================
    // ShardLaneSummary tests
    // =========================================================================

    #[test]
    fn summary_empty_has_all_zeros() {
        let s = ShardLaneSummary::empty();
        assert_eq!(s.active_runs, 0);
        assert_eq!(s.waiting_runs, 0);
        assert_eq!(s.failed_runs, 0);
        assert!(s.throughput_per_sec.abs() < 0.001);
        assert_eq!(s.avg_latency_ms, 0);
    }

    #[test]
    fn summary_health_green_when_healthy() {
        let s = ShardLaneSummary {
            active_runs: 10,
            waiting_runs: 0,
            failed_runs: 0,
            throughput_per_sec: 50.0,
            avg_latency_ms: 100,
        };
        assert_eq!(s.health(), LaneHealth::Green);
    }

    #[test]
    fn summary_health_red_with_failures_and_low_throughput() {
        let s = ShardLaneSummary {
            active_runs: 5,
            waiting_runs: 2,
            failed_runs: 3,
            throughput_per_sec: 0.5,
            avg_latency_ms: 100,
        };
        assert_eq!(s.health(), LaneHealth::Red);
    }

    #[test]
    fn summary_health_red_with_failures_and_high_latency() {
        let s = ShardLaneSummary {
            active_runs: 5,
            waiting_runs: 0,
            failed_runs: 1,
            throughput_per_sec: 50.0,
            avg_latency_ms: 3000,
        };
        assert_eq!(s.health(), LaneHealth::Red);
    }

    #[test]
    fn summary_health_not_red_if_failures_but_throughput_ok_and_latency_ok() {
        let s = ShardLaneSummary {
            active_runs: 5,
            waiting_runs: 0,
            failed_runs: 1,
            throughput_per_sec: 50.0,
            avg_latency_ms: 100,
        };
        assert_eq!(s.health(), LaneHealth::Green);
    }

    #[test]
    fn summary_health_yellow_with_low_throughput_and_active_runs() {
        let s = ShardLaneSummary {
            active_runs: 5,
            waiting_runs: 0,
            failed_runs: 0,
            throughput_per_sec: 5.0,
            avg_latency_ms: 100,
        };
        assert_eq!(s.health(), LaneHealth::Yellow);
    }

    #[test]
    fn summary_health_yellow_with_elevated_latency() {
        let s = ShardLaneSummary {
            active_runs: 0,
            waiting_runs: 0,
            failed_runs: 0,
            throughput_per_sec: 100.0,
            avg_latency_ms: 600,
        };
        assert_eq!(s.health(), LaneHealth::Yellow);
    }

    #[test]
    fn summary_health_not_yellow_with_zero_active_runs_and_normal_latency() {
        let s = ShardLaneSummary {
            active_runs: 0,
            waiting_runs: 0,
            failed_runs: 0,
            throughput_per_sec: 5.0,
            avg_latency_ms: 100,
        };
        assert_eq!(s.health(), LaneHealth::Green);
    }

    #[test]
    fn summary_health_red_takes_precedence_over_yellow() {
        let s = ShardLaneSummary {
            active_runs: 5,
            waiting_runs: 0,
            failed_runs: 1,
            throughput_per_sec: 0.5,
            avg_latency_ms: 3000,
        };
        assert_eq!(s.health(), LaneHealth::Red);
    }

    #[test]
    fn summary_debug_format() {
        let s = ShardLaneSummary::empty();
        let debug = format!("{s:?}");
        assert!(debug.contains("ShardLaneSummary") || debug.contains("active_runs"));
    }

    #[test]
    fn summary_equality() {
        let a = ShardLaneSummary::empty();
        let b = ShardLaneSummary::empty();
        assert_eq!(a, b);
    }

    #[test]
    fn summary_inequality() {
        let a = ShardLaneSummary::empty();
        let b = ShardLaneSummary {
            active_runs: 1,
            ..ShardLaneSummary::empty()
        };
        assert_ne!(a, b);
    }

    #[test]
    fn summary_health_red_at_exact_throughput_boundary() {
        let s = ShardLaneSummary {
            active_runs: 1,
            waiting_runs: 0,
            failed_runs: 1,
            throughput_per_sec: 1.0,
            avg_latency_ms: 100,
        };
        assert_eq!(s.health(), LaneHealth::Red);
    }

    #[test]
    fn summary_health_yellow_at_exact_latency_boundary() {
        let s = ShardLaneSummary {
            active_runs: 5,
            waiting_runs: 0,
            failed_runs: 0,
            throughput_per_sec: 5.0,
            avg_latency_ms: 500,
        };
        assert_eq!(s.health(), LaneHealth::Yellow);
    }

    #[test]
    fn summary_health_green_at_throughput_just_above_ten() {
        let s = ShardLaneSummary {
            active_runs: 3,
            waiting_runs: 0,
            failed_runs: 0,
            throughput_per_sec: 10.5,
            avg_latency_ms: 100,
        };
        assert_eq!(s.health(), LaneHealth::Green);
    }

    #[test]
    fn summary_health_yellow_with_failures_and_moderate_throughput() {
        let s = ShardLaneSummary {
            active_runs: 2,
            waiting_runs: 0,
            failed_runs: 1,
            throughput_per_sec: 2.0,
            avg_latency_ms: 200,
        };
        assert_eq!(s.health(), LaneHealth::Yellow);
    }
}
