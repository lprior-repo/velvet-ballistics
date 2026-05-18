#![forbid(unsafe_code)]
//! Tests for system screen module.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::alerts::{Alert, AlertKind, AlertSeverity};
    use crate::system::ticker::{TickerEvent, TickerEventKind};
    use std::time::Instant;

    fn stub_shard_metrics(
        shard_id: u32,
        ready: u32,
        action: u32,
        pool_free: u32,
        pool_total: u32,
        trace_pct: f32,
    ) -> vb_ipc::ShardMetrics {
        vb_ipc::ShardMetrics {
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

    fn info_alert(msg: &str) -> Alert {
        Alert {
            severity: AlertSeverity::Info,
            kind: AlertKind::QueuePressure,
            message: msg.to_string(),
            run_id: None,
            shard_id: None,
            timestamp: Instant::now(),
        }
    }

    fn critical_alert(msg: &str) -> Alert {
        Alert {
            severity: AlertSeverity::Critical,
            kind: AlertKind::RunFailed,
            message: msg.to_string(),
            run_id: Some(1),
            shard_id: Some(0),
            timestamp: Instant::now(),
        }
    }

    fn ticker_event(kind: &str) -> TickerEvent {
        TickerEvent {
            seq: 0,
            shard: 0,
            run_id: None,
            kind: match kind {
                "RunAccepted" => TickerEventKind::RunAccepted,
                "StepStarted" => TickerEventKind::StepStarted,
                "StepSucceeded" => TickerEventKind::StepSucceeded,
                "ActionScheduled" => TickerEventKind::ActionScheduled,
                "ActionCompleted" => TickerEventKind::ActionCompleted,
                "ActionFailed" => TickerEventKind::ActionFailed,
                "RunFinished" => TickerEventKind::RunFinished,
                "RunFailed" => TickerEventKind::RunFailed,
                _ => TickerEventKind::Other,
            },
            summary: kind.to_string(),
        }
    }

    // -- format_queue_depth tests --

    #[test]
    fn format_queue_depth_normal() {
        assert_eq!(format_queue_depth(10, 100), QueueStatus::Normal);
    }

    #[test]
    fn format_queue_depth_pressured() {
        assert_eq!(format_queue_depth(60, 100), QueueStatus::Pressured);
    }

    #[test]
    fn format_queue_depth_critical() {
        assert_eq!(format_queue_depth(90, 100), QueueStatus::Critical);
    }

    #[test]
    fn format_queue_depth_zero_capacity() {
        assert_eq!(format_queue_depth(0, 0), QueueStatus::Normal);
    }

    // -- SystemScreen construction tests --

    #[test]
    fn system_screen_new_starts_healthy() {
        let screen = SystemScreen::new();
        assert_eq!(screen.overall_health(), HealthStatus::Healthy);
        assert_eq!(screen.active_alert_count(), 0);
        assert_eq!(screen.critical_alert_count(), 0);
        assert!(screen.shard_summary().is_empty());
        assert_eq!(screen.worst_queue_status(), QueueStatus::Normal);
    }

    #[test]
    fn system_screen_default_matches_new() {
        let screen = SystemScreen::default();
        assert_eq!(screen.overall_health(), HealthStatus::Healthy);
    }

    // -- update_from_metrics tests --

    #[test]
    fn update_from_metrics_adds_first_shard() {
        let mut screen = SystemScreen::new();
        let m = stub_shard_metrics(0, 10, 5, 90, 100, 20.0);
        screen.update_from_metrics(&m);
        assert_eq!(screen.metrics().shards.len(), 1);
        assert_eq!(screen.metrics().shards[0].shard_id, 0);
        assert_eq!(screen.overall_health(), HealthStatus::Healthy);
    }

    #[test]
    fn update_from_metrics_replaces_existing_shard() {
        let mut screen = SystemScreen::new();
        let m1 = stub_shard_metrics(2, 10, 5, 90, 100, 20.0);
        screen.update_from_metrics(&m1);
        assert_eq!(screen.metrics().shards[0].ready_queue_depth, 10);

        let m2 = stub_shard_metrics(2, 50, 20, 80, 100, 30.0);
        screen.update_from_metrics(&m2);
        assert_eq!(screen.metrics().shards.len(), 1);
        assert_eq!(screen.metrics().shards[0].ready_queue_depth, 50);
    }

    #[test]
    fn update_from_metrics_multiple_shards_propagates_health() {
        let mut screen = SystemScreen::new();
        // Shard 0: healthy
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 20.0));
        assert_eq!(screen.overall_health(), HealthStatus::Healthy);

        // Shard 1: critical (pool used = 95/100 = 95%)
        screen.update_from_metrics(&stub_shard_metrics(1, 10, 5, 5, 100, 20.0));
        assert_eq!(screen.overall_health(), HealthStatus::Critical);
    }

    #[test]
    fn update_from_metrics_syncs_topology_shards() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(0, 5, 2, 90, 100, 10.0));
        screen.update_from_metrics(&stub_shard_metrics(1, 8, 3, 85, 100, 15.0));
        assert_eq!(screen.topology().topology.shards.len(), 2);
        assert_eq!(screen.topology().topology.shards[0].shard_id, 0);
        assert_eq!(screen.topology().topology.shards[1].shard_id, 1);
    }

    // -- Alert accessor tests --

    #[test]
    fn active_and_critical_alert_counts() {
        let mut screen = SystemScreen::new();
        assert_eq!(screen.active_alert_count(), 0);
        assert_eq!(screen.critical_alert_count(), 0);

        screen.alerts_mut().add(info_alert("info"));
        screen.alerts_mut().add(critical_alert("crit1"));
        screen.alerts_mut().add(critical_alert("crit2"));

        assert_eq!(screen.active_alert_count(), 3);
        assert_eq!(screen.critical_alert_count(), 2);
    }

    #[test]
    fn dismiss_alert_via_mut_accessor() {
        let mut screen = SystemScreen::new();
        screen.alerts_mut().add(info_alert("a"));
        screen.alerts_mut().add(info_alert("b"));
        screen.alerts_mut().dismiss(0);
        assert_eq!(screen.active_alert_count(), 1);
    }

    // -- Ticker accessor tests --

    #[test]
    fn ticker_push_and_recent() {
        let mut screen = SystemScreen::new();
        screen.ticker_mut().push(ticker_event("StepSucceeded"));
        screen.ticker_mut().push(ticker_event("ActionCompleted"));
        let events = screen.ticker().events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, TickerEventKind::StepSucceeded);
        assert_eq!(events[1].kind, TickerEventKind::ActionCompleted);
    }

    // -- shard_summary tests --

    #[test]
    fn shard_summary_empty_when_no_shards() {
        let screen = SystemScreen::new();
        assert!(screen.shard_summary().is_empty());
    }

    #[test]
    fn shard_summary_formats_single_healthy_shard() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 20.0));
        let lines = screen.shard_summary();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].shard_id, 0);
        assert_eq!(lines[0].health_label, "Healthy");
        assert_eq!(lines[0].queue_label, "10/5");
        assert_eq!(lines[0].frame_label, "90/100");
        assert_eq!(lines[0].trace_label, "20%");
        assert_eq!(lines[0].queue_status, QueueStatus::Normal);
    }

    #[test]
    fn shard_summary_formats_critical_shard() {
        let mut screen = SystemScreen::new();
        // pool 5/100 used = 95%, trace 85% → Critical
        screen.update_from_metrics(&stub_shard_metrics(3, 210, 5, 5, 100, 85.0));
        let lines = screen.shard_summary();
        assert_eq!(lines[0].health_label, "Critical");
        // queue_status should be Critical (ready=210/256 ≈ 82%)
        assert_eq!(lines[0].queue_status, QueueStatus::Critical);
    }

    #[test]
    fn shard_summary_formats_degraded_shard() {
        let mut screen = SystemScreen::new();
        // trace 75% → Degraded
        screen.update_from_metrics(&stub_shard_metrics(1, 10, 5, 60, 100, 75.0));
        let lines = screen.shard_summary();
        assert_eq!(lines[0].health_label, "Degraded");
    }

    #[test]
    fn shard_summary_multiple_shards_ordered() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(5, 10, 5, 90, 100, 10.0));
        screen.update_from_metrics(&stub_shard_metrics(2, 20, 10, 80, 100, 20.0));
        let lines = screen.shard_summary();
        assert_eq!(lines.len(), 2);
        // Ordered by insertion: shard 5 first, shard 2 second
        assert_eq!(lines[0].shard_id, 5);
        assert_eq!(lines[1].shard_id, 2);
    }

    // -- worst_queue_status tests --

    #[test]
    fn worst_queue_status_normal_with_no_monitors() {
        let screen = SystemScreen::new();
        assert_eq!(screen.worst_queue_status(), QueueStatus::Normal);
    }

    #[test]
    fn worst_queue_status_reflects_critical_shard() {
        let mut screen = SystemScreen::new();
        // Healthy shard
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 20.0));
        assert_eq!(screen.worst_queue_status(), QueueStatus::Normal);

        // Critical shard (ready=210/256 ≈ 82%)
        screen.update_from_metrics(&stub_shard_metrics(1, 210, 5, 5, 100, 85.0));
        assert_eq!(screen.worst_queue_status(), QueueStatus::Critical);
    }

    #[test]
    fn worst_queue_status_reflects_pressured_shard() {
        let mut screen = SystemScreen::new();
        // ready=130/256 ≈ 50.8% → Pressured
        screen.update_from_metrics(&stub_shard_metrics(0, 130, 5, 90, 100, 20.0));
        assert_eq!(screen.worst_queue_status(), QueueStatus::Pressured);
    }

    // -- Saturating arithmetic edge cases --

    #[test]
    fn update_from_metrics_handles_large_shard_id_without_overflow() {
        let mut screen = SystemScreen::new();
        let m = vb_ipc::ShardMetrics {
            shard_id: u32::MAX,
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
        screen.update_from_metrics(&m);
        assert_eq!(screen.metrics().shards.len(), 1);
        assert_eq!(screen.metrics().shards[0].shard_id, u32::MAX);
    }

    #[test]
    fn shard_summary_trace_label_rounds_zero() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 0.0));
        let lines = screen.shard_summary();
        assert_eq!(lines[0].trace_label, "0%");
    }

    #[test]
    fn shard_summary_trace_label_rounds_fractional() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 33.7));
        let lines = screen.shard_summary();
        assert_eq!(lines[0].trace_label, "34%");
    }

    #[test]
    fn topology_syncs_shards_from_metrics() {
        let mut screen = SystemScreen::new();
        screen.update_from_metrics(&stub_shard_metrics(0, 10, 5, 90, 100, 20.0));
        // After update, topology should reflect the shard from metrics.
        assert_eq!(screen.topology().topology.shards.len(), 1);
        assert_eq!(screen.topology().topology.shards[0].shard_id, 0);
    }

    // =========================================================================
    // SystemOverviewScreen layout model tests
    // =========================================================================

    // -- Construction tests --

    #[test]
    fn overview_new_screen_has_placeholder_health_label() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.health_label, "Active");
    }

    #[test]
    fn overview_new_screen_has_cyan_health_color() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.health_color, SYS_NEON_CYAN);
    }

    #[test]
    fn overview_new_screen_has_four_shards() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.shard_count, 4);
    }

    #[test]
    fn overview_new_screen_has_active_runs() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.total_active_runs, 37);
    }

    #[test]
    fn overview_new_screen_has_pending() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.total_pending, 25);
    }

    #[test]
    fn overview_default_matches_new() {
        let from_new = SystemOverviewScreen::new();
        let from_default = SystemOverviewScreen::default();
        assert_eq!(from_new.health_label, from_default.health_label);
        assert_eq!(from_new.shard_count, from_default.shard_count);
        assert_eq!(from_new.shard_row_count(), from_default.shard_row_count());
        assert_eq!(from_new.lane_count(), from_default.lane_count());
        assert_eq!(from_new.queue_bar_count(), from_default.queue_bar_count());
        assert_eq!(
            from_new.ticker_chip_count(),
            from_default.ticker_chip_count()
        );
        assert_eq!(from_new.alert_count(), from_default.alert_count());
        assert_eq!(
            from_new.latency_segment_count(),
            from_default.latency_segment_count()
        );
    }

    // -- Title / header text tests --

    #[test]
    fn overview_title_text_returns_vb() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.title_text(), "vb");
    }

    #[test]
    fn overview_page_title_returns_system_overview() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.page_title(), "System Overview");
    }

    #[test]
    fn overview_status_summary_formats_correctly() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(
            screen.status_summary(),
            "Active shards=4 active=37 pending=25"
        );
    }

    #[test]
    fn overview_topology_header_text() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.topology_header_text(), "TOPOLOGY");
    }

    #[test]
    fn overview_activity_header_text() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.activity_header_text(), "ACTIVITY LANES");
    }

    #[test]
    fn overview_queue_monitor_header_text() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.queue_monitor_header_text(), "QUEUE MONITOR");
    }

    #[test]
    fn overview_event_ticker_header_text() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.event_ticker_header_text(), "EVENT TICKER");
    }

    #[test]
    fn overview_alert_stack_header_text() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.alert_stack_header_text(), "ALERTS");
    }

    #[test]
    fn overview_latency_header_text() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.latency_header_text(), "LATENCY BREAKDOWN");
    }

    // -- Topology panel tests --

    #[test]
    fn overview_topology_panel_has_four_shard_rows() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.shard_row_count(), 4);
    }

    #[test]
    fn overview_topology_first_shard_is_active() {
        let screen = SystemOverviewScreen::new();
        let row = screen.topology_panel.shard_rows.first().expect("first row");
        assert_eq!(row.shard_id, 0);
        assert_eq!(row.status_label, "Active");
        assert_eq!(row.status_color, SYS_NEON_CYAN);
        assert_eq!(row.active_runs, 12);
    }

    #[test]
    fn overview_topology_second_shard_is_idle() {
        let screen = SystemOverviewScreen::new();
        let row = screen.topology_panel.shard_rows.get(1).expect("second row");
        assert_eq!(row.shard_id, 1);
        assert_eq!(row.status_label, "Idle");
        assert_eq!(row.status_color, SYS_NEON_GREEN);
        assert_eq!(row.active_runs, 0);
    }

    #[test]
    fn overview_topology_third_shard_is_overloaded() {
        let screen = SystemOverviewScreen::new();
        let row = screen.topology_panel.shard_rows.get(2).expect("third row");
        assert_eq!(row.shard_id, 2);
        assert_eq!(row.status_label, "Overloaded");
        assert_eq!(row.status_color, SYS_NEON_RED);
        assert_eq!(row.active_runs, 20);
    }

    #[test]
    fn overview_topology_fourth_shard_is_active() {
        let screen = SystemOverviewScreen::new();
        let row = screen.topology_panel.shard_rows.get(3).expect("fourth row");
        assert_eq!(row.shard_id, 3);
        assert_eq!(row.status_label, "Active");
        assert_eq!(row.active_runs, 5);
    }

    #[test]
    fn overview_topology_journal_is_healthy() {
        let screen = SystemOverviewScreen::new();
        let js = &screen.topology_panel.journal_status;
        assert_eq!(js.label, "healthy");
        assert_eq!(js.label_color, SYS_NEON_GREEN);
        assert_eq!(js.queue_depth, 3);
    }

    #[test]
    fn overview_topology_timer_count() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.topology_panel.timer_count, 42);
    }

    #[test]
    fn overview_topology_ipc_connections() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.topology_panel.ipc_connections, 3);
    }

    #[test]
    fn overview_topology_shard_rows_all_have_nonempty_colors() {
        let screen = SystemOverviewScreen::new();
        for (i, row) in screen.topology_panel.shard_rows.iter().enumerate() {
            assert!(
                !row.status_color.is_empty(),
                "empty status_color at index {i}"
            );
            assert!(!row.bg_color.is_empty(), "empty bg_color at index {i}");
        }
    }

    // -- Activity lanes tests --

    #[test]
    fn overview_activity_lanes_has_four_lanes() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.lane_count(), 4);
    }

    #[test]
    fn overview_activity_lane_first_has_correct_metrics() {
        let screen = SystemOverviewScreen::new();
        let lane = screen.activity_lanes().first().expect("first lane");
        assert_eq!(lane.shard_id, 0);
        assert_eq!(lane.active_runs, 12);
        assert_eq!(lane.ready_queue_depth, 8);
        assert_eq!(lane.action_queue_depth, 4);
        assert!((lane.steps_per_sec - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn overview_activity_lane_second_is_idle() {
        let screen = SystemOverviewScreen::new();
        let lane = screen.activity_lanes().get(1).expect("second lane");
        assert_eq!(lane.active_runs, 0);
        assert!(lane.segments.is_empty());
        assert!((lane.steps_per_sec - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn overview_activity_lane_third_is_overloaded() {
        let screen = SystemOverviewScreen::new();
        let lane = screen.activity_lanes().get(2).expect("third lane");
        assert_eq!(lane.shard_id, 2);
        assert_eq!(lane.active_runs, 20);
        assert_eq!(lane.lane_label_color, SYS_NEON_RED);
    }

    #[test]
    fn overview_activity_lane_segments_widths_sum_to_one() {
        let screen = SystemOverviewScreen::new();
        for (idx, lane) in screen.activity_lanes().iter().enumerate() {
            if lane.segments.is_empty() {
                continue;
            }
            let total: f64 = lane.segments.iter().map(|s| s.width_ratio).sum();
            assert!(
                (total - 1.0).abs() < 0.01,
                "lane {idx}: segment widths sum to {total}, expected ~1.0"
            );
        }
    }

    #[test]
    fn overview_activity_lane_segments_have_non_negative_widths() {
        let screen = SystemOverviewScreen::new();
        for lane in screen.activity_lanes() {
            for seg in &lane.segments {
                assert!(
                    seg.width_ratio >= 0.0,
                    "segment {} has negative width_ratio",
                    seg.label
                );
            }
        }
    }

    #[test]
    fn overview_activity_lane_segments_have_labels() {
        let screen = SystemOverviewScreen::new();
        for lane in screen.activity_lanes() {
            for seg in &lane.segments {
                assert!(
                    seg.label.starts_with('R'),
                    "segment label should start with 'R', got '{}'",
                    seg.label
                );
            }
        }
    }

    // -- Queue monitor tests --

    #[test]
    fn overview_queue_monitor_has_five_bars() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.queue_bar_count(), 5);
    }

    #[test]
    fn overview_queue_monitor_bar_labels() {
        let screen = SystemOverviewScreen::new();
        let labels: Vec<&str> = screen
            .queue_monitor
            .bars
            .iter()
            .map(|b| b.label.as_str())
            .collect();
        assert_eq!(labels, vec!["Ready", "Action", "Journal", "Trace", "Frame"]);
    }

    #[test]
    fn overview_queue_monitor_all_bars_normal() {
        let screen = SystemOverviewScreen::new();
        for (i, bar) in screen.queue_monitor.bars.iter().enumerate() {
            assert_eq!(bar.status, QueueStatus::Normal, "bar {i} should be Normal");
        }
    }

    #[test]
    fn overview_queue_monitor_fill_ratios_in_range() {
        let screen = SystemOverviewScreen::new();
        for (i, bar) in screen.queue_monitor.bars.iter().enumerate() {
            assert!(
                bar.fill_ratio >= 0.0 && bar.fill_ratio <= 1.0,
                "bar {i} fill_ratio {} out of [0, 1]",
                bar.fill_ratio
            );
        }
    }

    #[test]
    fn overview_queue_monitor_bars_have_nonempty_colors() {
        let screen = SystemOverviewScreen::new();
        for (i, bar) in screen.queue_monitor.bars.iter().enumerate() {
            assert!(!bar.fill_color.is_empty(), "bar {i} has empty fill_color");
        }
    }

    #[test]
    fn overview_queue_monitor_depth_texts() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.queue_monitor.bars[0].depth_text, "26/256");
        assert_eq!(screen.queue_monitor.bars[2].depth_text, "3/64");
        assert_eq!(screen.queue_monitor.bars[3].depth_text, "35%");
    }

    // -- Event ticker tests --

    #[test]
    fn overview_event_ticker_has_five_chips() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.ticker_chip_count(), 5);
    }

    #[test]
    fn overview_event_ticker_first_chip_is_run_accepted() {
        let screen = SystemOverviewScreen::new();
        let chip = screen.event_ticker.chips.first().expect("first chip");
        assert_eq!(chip.kind_label, "RunAccepted");
        assert_eq!(chip.seq, 1);
        assert_eq!(chip.text_color, SYS_NEON_CYAN);
    }

    #[test]
    fn overview_event_ticker_last_chip_is_run_finished() {
        let screen = SystemOverviewScreen::new();
        let chip = screen.event_ticker.chips.last().expect("last chip");
        assert_eq!(chip.kind_label, "RunFinished");
        assert_eq!(chip.seq, 5);
    }

    #[test]
    fn overview_event_ticker_chips_have_monotonic_seq() {
        let screen = SystemOverviewScreen::new();
        for i in 1..screen.event_ticker.chips.len() {
            let prev = screen.event_ticker.chips.get(i - 1).expect("prev chip");
            let curr = screen.event_ticker.chips.get(i).expect("curr chip");
            assert!(
                curr.seq > prev.seq,
                "chip {} seq {} should be > chip {} seq {}",
                i,
                curr.seq,
                i - 1,
                prev.seq
            );
        }
    }

    #[test]
    fn overview_event_ticker_chips_have_nonempty_colors() {
        let screen = SystemOverviewScreen::new();
        for (i, chip) in screen.event_ticker.chips.iter().enumerate() {
            assert!(!chip.bg_color.is_empty(), "chip {i} has empty bg_color");
            assert!(!chip.text_color.is_empty(), "chip {i} has empty text_color");
        }
    }

    #[test]
    fn overview_event_ticker_chips_have_summaries() {
        let screen = SystemOverviewScreen::new();
        for (i, chip) in screen.event_ticker.chips.iter().enumerate() {
            assert!(!chip.summary.is_empty(), "chip {i} has empty summary");
        }
    }

    // -- Alert stack tests --

    #[test]
    fn overview_alert_stack_has_three_alerts() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.alert_count(), 3);
    }

    #[test]
    fn overview_alert_first_is_critical() {
        let screen = SystemOverviewScreen::new();
        let alert = screen.alert_stack.alerts.first().expect("first alert");
        assert_eq!(alert.severity_label, "Critical");
        assert_eq!(alert.severity_color, SYS_NEON_RED);
        assert!(!alert.acknowledged);
    }

    #[test]
    fn overview_alert_second_is_warning() {
        let screen = SystemOverviewScreen::new();
        let alert = screen.alert_stack.alerts.get(1).expect("second alert");
        assert_eq!(alert.severity_label, "Warning");
        assert_eq!(alert.severity_color, SYS_NEON_YELLOW);
        assert!(!alert.acknowledged);
    }

    #[test]
    fn overview_alert_third_is_info_and_acknowledged() {
        let screen = SystemOverviewScreen::new();
        let alert = screen.alert_stack.alerts.get(2).expect("third alert");
        assert_eq!(alert.severity_label, "Info");
        assert_eq!(alert.severity_color, SYS_NEON_CYAN);
        assert!(alert.acknowledged);
    }

    #[test]
    fn overview_unacknowledged_alert_count_is_two() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.unacknowledged_alert_count(), 2);
    }

    #[test]
    fn overview_alert_cards_have_nonempty_colors() {
        let screen = SystemOverviewScreen::new();
        for (i, alert) in screen.alert_stack.alerts.iter().enumerate() {
            assert!(
                !alert.severity_color.is_empty(),
                "alert {i} has empty severity_color"
            );
            assert!(!alert.bg_color.is_empty(), "alert {i} has empty bg_color");
        }
    }

    // -- Latency breakdown tests --

    #[test]
    fn overview_latency_breakdown_has_five_segments() {
        let screen = SystemOverviewScreen::new();
        assert_eq!(screen.latency_segment_count(), 5);
    }

    #[test]
    fn overview_latency_first_segment_is_submit_to_admit() {
        let screen = SystemOverviewScreen::new();
        let seg = screen
            .latency_breakdown
            .segments
            .first()
            .expect("first seg");
        assert_eq!(seg.label, "submit -> admit");
        assert_eq!(seg.avg_us, 325);
        assert_eq!(seg.display, "325us");
        assert_eq!(seg.fill_color, SYS_NEON_CYAN);
    }

    #[test]
    fn overview_latency_fourth_segment_is_action_to_completed() {
        let screen = SystemOverviewScreen::new();
        let seg = screen
            .latency_breakdown
            .segments
            .get(3)
            .expect("fourth seg");
        assert_eq!(seg.label, "action -> completed");
        assert_eq!(seg.avg_us, 3_250_000);
        assert_eq!(seg.display, "3.25s");
        assert_eq!(seg.fill_color, SYS_NEON_RED);
        assert!((seg.width_ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn overview_latency_last_segment_is_completed_to_finish() {
        let screen = SystemOverviewScreen::new();
        let seg = screen.latency_breakdown.segments.last().expect("last seg");
        assert_eq!(seg.label, "completed -> finish");
        assert_eq!(seg.avg_us, 225);
        assert_eq!(seg.display, "225us");
    }

    #[test]
    fn overview_latency_segments_width_ratios_non_negative() {
        let screen = SystemOverviewScreen::new();
        for seg in &screen.latency_breakdown.segments {
            assert!(
                seg.width_ratio >= 0.0,
                "segment '{}' has negative width_ratio",
                seg.label
            );
        }
    }

    #[test]
    fn overview_latency_segments_have_pipeline_order() {
        let screen = SystemOverviewScreen::new();
        let labels: Vec<&str> = screen
            .latency_breakdown
            .segments
            .iter()
            .map(|s| s.label.as_str())
            .collect();
        assert_eq!(
            labels,
            vec![
                "submit -> admit",
                "admit -> step",
                "step -> action",
                "action -> completed",
                "completed -> finish",
            ]
        );
    }

    // -- Accessor tests --

    #[test]
    fn overview_topology_panel_accessor() {
        let screen = SystemOverviewScreen::new();
        let panel = screen.topology_panel();
        assert_eq!(panel.shard_rows.len(), 4);
        assert_eq!(panel.timer_count, 42);
        assert_eq!(panel.ipc_connections, 3);
    }

    #[test]
    fn overview_activity_lanes_accessor() {
        let screen = SystemOverviewScreen::new();
        let lanes = screen.activity_lanes();
        assert_eq!(lanes.len(), 4);
    }

    #[test]
    fn overview_queue_monitor_panel_accessor() {
        let screen = SystemOverviewScreen::new();
        let monitor = screen.queue_monitor_panel();
        assert_eq!(monitor.bars.len(), 5);
    }

    #[test]
    fn overview_event_ticker_panel_accessor() {
        let screen = SystemOverviewScreen::new();
        let ticker = screen.event_ticker_panel();
        assert_eq!(ticker.chips.len(), 5);
    }

    #[test]
    fn overview_alert_stack_panel_accessor() {
        let screen = SystemOverviewScreen::new();
        let stack = screen.alert_stack_panel();
        assert_eq!(stack.alerts.len(), 3);
    }

    #[test]
    fn overview_latency_breakdown_panel_accessor() {
        let screen = SystemOverviewScreen::new();
        let breakdown = screen.latency_breakdown_panel();
        assert_eq!(breakdown.segments.len(), 5);
    }

    // -- Color constants tests --

    #[test]
    fn overview_color_constants_match_spec() {
        assert_eq!(SYS_PANEL_BG, "#12121f");
        assert_eq!(SYS_CARD_BG, "#16162a");
        assert_eq!(SYS_BORDER, "#2a2a4a");
        assert_eq!(SYS_TEXT_PRIMARY, "#e8e8ff");
        assert_eq!(SYS_TEXT_SECONDARY, "#8888aa");
        assert_eq!(SYS_NEON_CYAN, "#00f5ff");
        assert_eq!(SYS_NEON_GREEN, "#39ff14");
        assert_eq!(SYS_NEON_RED, "#ff073a");
        assert_eq!(SYS_NEON_ORANGE, "#ff6b00");
        assert_eq!(SYS_NEON_YELLOW, "#ffe600");
        assert_eq!(SYS_NEON_PURPLE, "#b14dff");
        assert_eq!(SYS_TEXT_DIM, "#555577");
        assert_eq!(SYS_CANVAS_BG, "#0a0a12");
    }

    // -- Clone roundtrip tests --

    #[test]
    fn overview_topology_shard_row_clone_roundtrip() {
        let row = TopologyShardRow {
            shard_id: 7,
            status_label: String::from("Active"),
            status_color: String::from(SYS_NEON_CYAN),
            active_runs: 5,
            bg_color: String::from("#0d1a2a"),
        };
        let cloned = row.clone();
        assert_eq!(cloned.shard_id, row.shard_id);
        assert_eq!(cloned.status_label, row.status_label);
        assert_eq!(cloned.active_runs, row.active_runs);
    }

    #[test]
    fn overview_journal_status_row_clone_roundtrip() {
        let row = JournalStatusRow {
            label: String::from("degraded"),
            label_color: String::from(SYS_NEON_YELLOW),
            queue_depth: 50,
        };
        let cloned = row.clone();
        assert_eq!(cloned.label, row.label);
        assert_eq!(cloned.queue_depth, row.queue_depth);
    }

    #[test]
    fn overview_topology_panel_clone_roundtrip() {
        let screen = SystemOverviewScreen::new();
        let cloned = screen.topology_panel.clone();
        assert_eq!(
            cloned.shard_rows.len(),
            screen.topology_panel.shard_rows.len()
        );
        assert_eq!(cloned.timer_count, screen.topology_panel.timer_count);
    }

    #[test]
    fn overview_activity_segment_clone_roundtrip() {
        let seg = ActivitySegment {
            run_id: 8172,
            width_ratio: 0.5,
            color: String::from(SYS_NEON_CYAN),
            label: String::from("R8172"),
        };
        let cloned = seg.clone();
        assert_eq!(cloned.run_id, seg.run_id);
        assert_eq!(cloned.width_ratio, seg.width_ratio);
        assert_eq!(cloned.color, seg.color);
    }

    #[test]
    fn overview_activity_lane_clone_roundtrip() {
        let screen = SystemOverviewScreen::new();
        let lane = screen.activity_lanes.first().expect("first lane");
        let cloned = lane.clone();
        assert_eq!(cloned.shard_id, lane.shard_id);
        assert_eq!(cloned.active_runs, lane.active_runs);
        assert_eq!(cloned.segments.len(), lane.segments.len());
    }

    #[test]
    fn overview_queue_monitor_bar_clone_roundtrip() {
        let screen = SystemOverviewScreen::new();
        let bar = screen.queue_monitor.bars.first().expect("first bar");
        let cloned = bar.clone();
        assert_eq!(cloned.label, bar.label);
        assert_eq!(cloned.fill_ratio, bar.fill_ratio);
        assert_eq!(cloned.depth_text, bar.depth_text);
    }

    #[test]
    fn overview_queue_monitor_panel_clone_roundtrip() {
        let screen = SystemOverviewScreen::new();
        let cloned = screen.queue_monitor.clone();
        assert_eq!(cloned.bars.len(), screen.queue_monitor.bars.len());
    }

    #[test]
    fn overview_ticker_chip_clone_roundtrip() {
        let screen = SystemOverviewScreen::new();
        let chip = screen.event_ticker.chips.first().expect("first chip");
        let cloned = chip.clone();
        assert_eq!(cloned.kind_label, chip.kind_label);
        assert_eq!(cloned.seq, chip.seq);
        assert_eq!(cloned.summary, chip.summary);
    }

    #[test]
    fn overview_event_ticker_panel_clone_roundtrip() {
        let screen = SystemOverviewScreen::new();
        let cloned = screen.event_ticker.clone();
        assert_eq!(cloned.chips.len(), screen.event_ticker.chips.len());
    }

    #[test]
    fn overview_alert_card_clone_roundtrip() {
        let screen = SystemOverviewScreen::new();
        let alert = screen.alert_stack.alerts.first().expect("first alert");
        let cloned = alert.clone();
        assert_eq!(cloned.severity_label, alert.severity_label);
        assert_eq!(cloned.message, alert.message);
        assert_eq!(cloned.acknowledged, alert.acknowledged);
    }

    #[test]
    fn overview_alert_stack_clone_roundtrip() {
        let screen = SystemOverviewScreen::new();
        let cloned = screen.alert_stack.clone();
        assert_eq!(cloned.alerts.len(), screen.alert_stack.alerts.len());
    }

    #[test]
    fn overview_latency_segment_clone_roundtrip() {
        let screen = SystemOverviewScreen::new();
        let seg = screen
            .latency_breakdown
            .segments
            .first()
            .expect("first seg");
        let cloned = seg.clone();
        assert_eq!(cloned.label, seg.label);
        assert_eq!(cloned.avg_us, seg.avg_us);
        assert_eq!(cloned.display, seg.display);
    }

    #[test]
    fn overview_latency_breakdown_clone_roundtrip() {
        let screen = SystemOverviewScreen::new();
        let cloned = screen.latency_breakdown.clone();
        assert_eq!(
            cloned.segments.len(),
            screen.latency_breakdown.segments.len()
        );
    }

    // -- build_segments helper tests --

    #[test]
    fn build_segments_zero_count_returns_empty() {
        let segs = super::overview::build_segments(0, 0);
        assert!(segs.is_empty());
    }

    #[test]
    fn build_segments_single_segment_gets_full_width() {
        let segs = super::overview::build_segments(0, 1);
        assert_eq!(segs.len(), 1);
        assert!((segs[0].width_ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn build_segments_multiple_sum_to_one() {
        let segs = super::overview::build_segments(0, 5);
        assert_eq!(segs.len(), 5);
        let total: f64 = segs.iter().map(|s| s.width_ratio).sum();
        assert!(
            (total - 1.0).abs() < 0.01,
            "segment widths should sum to ~1.0, got {total}"
        );
    }

    #[test]
    fn build_segments_run_ids_include_shard_offset() {
        let segs = super::overview::build_segments(3, 2);
        let base_id = 80_000u64 + u64::from(3u32) * 10_000;
        assert_eq!(segs[0].run_id, base_id);
        assert_eq!(segs[1].run_id, base_id + 1);
    }

    // -- Debug format smoke tests --

    #[test]
    fn overview_topology_shard_row_debug_format() {
        let row = TopologyShardRow {
            shard_id: 0,
            status_label: String::from("Active"),
            status_color: String::from("#00f5ff"),
            active_runs: 5,
            bg_color: String::from("#0d1a2a"),
        };
        let debug = format!("{row:?}");
        assert!(debug.contains("shard_id"));
        assert!(debug.contains("Active"));
    }

    #[test]
    fn overview_activity_segment_debug_format() {
        let seg = ActivitySegment {
            run_id: 1,
            width_ratio: 0.5,
            color: String::from("#00f5ff"),
            label: String::from("R1"),
        };
        let debug = format!("{seg:?}");
        assert!(debug.contains("run_id"));
    }

    #[test]
    fn overview_queue_monitor_bar_debug_format() {
        let bar = QueueMonitorBar {
            label: String::from("Ready"),
            fill_color: String::from("#00f5ff"),
            fill_ratio: 0.5,
            depth_text: String::from("10/100"),
            status: QueueStatus::Normal,
        };
        let debug = format!("{bar:?}");
        assert!(debug.contains("Ready"));
    }

    #[test]
    fn overview_ticker_chip_debug_format() {
        let chip = TickerChip {
            kind_label: String::from("RunAccepted"),
            bg_color: String::from("#0d2a3a"),
            text_color: String::from("#00f5ff"),
            summary: String::from("Run #1 accepted"),
            seq: 1,
        };
        let debug = format!("{chip:?}");
        assert!(debug.contains("RunAccepted"));
    }

    #[test]
    fn overview_alert_card_debug_format() {
        let card = AlertCard {
            severity_label: String::from("Critical"),
            severity_color: String::from("#ff073a"),
            message: String::from("shard overloaded"),
            source: String::from("topology"),
            bg_color: String::from("#1a0d0d"),
            acknowledged: false,
        };
        let debug = format!("{card:?}");
        assert!(debug.contains("Critical"));
    }

    #[test]
    fn overview_latency_segment_debug_format() {
        let seg = LatencySegment {
            label: String::from("submit -> admit"),
            avg_us: 325,
            display: String::from("325us"),
            fill_color: String::from("#00f5ff"),
            width_ratio: 0.001,
            p50_us: 300,
            p95_us: 400,
            p99_us: 500,
        };
        let debug = format!("{seg:?}");
        assert!(debug.contains("submit -> admit"));
    }
}
