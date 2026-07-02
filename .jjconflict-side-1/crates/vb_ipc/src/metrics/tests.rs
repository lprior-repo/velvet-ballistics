use super::*;

// ── Postcard serialization roundtrip tests ──

#[test]
fn runtime_metrics_postcard_roundtrip() {
    let metrics = RuntimeMetrics {
        shards: vec![ShardMetrics {
            shard_id: 0,
            active_runs: 5,
            ready_queue_depth: 3,
            action_queue_depth: 10,
            timer_count: 2,
            frame_pool_free: 100,
            frame_pool_total: 256,
            trace_ring_fill_pct: 45.5,
            steps_total: 1000,
            actions_total: 500,
        }],
        journal: JournalMetrics {
            writer_queue_depth: 7,
            total_events: 9999,
            total_runs: 42,
        },
        ipc: IpcMetrics {
            connected_clients: 3,
            commands_processed: 100_000,
        },
        totals: AggregateMetrics {
            runs_active: 12,
            runs_waiting: 4,
            runs_failed_total: 7,
            runs_finished_total: 200,
        },
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: RuntimeMetrics = postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert_eq!(decoded, metrics);
}

#[test]
fn shard_metrics_postcard_roundtrip() {
    let metrics = ShardMetrics {
        shard_id: 1,
        active_runs: 0,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: 0.0,
        steps_total: 0,
        actions_total: 0,
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: ShardMetrics = postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert_eq!(decoded, metrics);
}

#[test]
fn shard_metrics_with_max_values_roundtrip() {
    let metrics = ShardMetrics {
        shard_id: u32::MAX,
        active_runs: u32::MAX,
        ready_queue_depth: u32::MAX,
        action_queue_depth: u32::MAX,
        timer_count: u32::MAX,
        frame_pool_free: u32::MAX,
        frame_pool_total: u32::MAX,
        trace_ring_fill_pct: 100.0,
        steps_total: u64::MAX,
        actions_total: u64::MAX,
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: ShardMetrics = postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert_eq!(decoded, metrics);
}

#[test]
fn journal_metrics_postcard_roundtrip() {
    let metrics = JournalMetrics {
        writer_queue_depth: 42,
        total_events: u64::MAX,
        total_runs: 0,
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: JournalMetrics = postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert_eq!(decoded, metrics);
}

#[test]
fn ipc_metrics_postcard_roundtrip() {
    let metrics = IpcMetrics {
        connected_clients: 100,
        commands_processed: 999_999_999,
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: IpcMetrics = postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert_eq!(decoded, metrics);
}

#[test]
fn aggregate_metrics_postcard_roundtrip() {
    let metrics = AggregateMetrics {
        runs_active: 50,
        runs_waiting: 10,
        runs_failed_total: u64::MAX,
        runs_finished_total: u64::MAX,
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: AggregateMetrics =
        postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert_eq!(decoded, metrics);
}

#[test]
fn runtime_metrics_empty_shards_roundtrip() {
    let metrics = RuntimeMetrics {
        shards: Vec::new(),
        journal: JournalMetrics {
            writer_queue_depth: 0,
            total_events: 0,
            total_runs: 0,
        },
        ipc: IpcMetrics {
            connected_clients: 0,
            commands_processed: 0,
        },
        totals: AggregateMetrics {
            runs_active: 0,
            runs_waiting: 0,
            runs_failed_total: 0,
            runs_finished_total: 0,
        },
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: RuntimeMetrics = postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert!(decoded.shards.is_empty());
    assert_eq!(decoded, metrics);
}

#[test]
fn runtime_metrics_multiple_shards_roundtrip() {
    let metrics = RuntimeMetrics {
        shards: vec![
            ShardMetrics {
                shard_id: 0,
                active_runs: 1,
                ready_queue_depth: 2,
                action_queue_depth: 3,
                timer_count: 4,
                frame_pool_free: 5,
                frame_pool_total: 10,
                trace_ring_fill_pct: 50.0,
                steps_total: 100,
                actions_total: 50,
            },
            ShardMetrics {
                shard_id: 1,
                active_runs: 6,
                ready_queue_depth: 7,
                action_queue_depth: 8,
                timer_count: 9,
                frame_pool_free: 10,
                frame_pool_total: 20,
                trace_ring_fill_pct: 75.5,
                steps_total: 200,
                actions_total: 100,
            },
        ],
        journal: JournalMetrics {
            writer_queue_depth: 3,
            total_events: 300,
            total_runs: 15,
        },
        ipc: IpcMetrics {
            connected_clients: 2,
            commands_processed: 500,
        },
        totals: AggregateMetrics {
            runs_active: 7,
            runs_waiting: 3,
            runs_failed_total: 1,
            runs_finished_total: 100,
        },
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: RuntimeMetrics = postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert_eq!(decoded.shards.len(), 2);
    assert_eq!(decoded, metrics);
}

// ── Shard count edge cases ──

#[test]
fn runtime_metrics_one_shard_roundtrip() {
    let metrics = RuntimeMetrics {
        shards: vec![ShardMetrics {
            shard_id: 0,
            active_runs: 1,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 0,
            frame_pool_total: 0,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        }],
        journal: JournalMetrics {
            writer_queue_depth: 0,
            total_events: 0,
            total_runs: 0,
        },
        ipc: IpcMetrics {
            connected_clients: 0,
            commands_processed: 0,
        },
        totals: AggregateMetrics {
            runs_active: 0,
            runs_waiting: 0,
            runs_failed_total: 0,
            runs_finished_total: 0,
        },
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: RuntimeMetrics = postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert_eq!(decoded.shards.len(), 1);
    assert_eq!(decoded, metrics);
}

#[test]
fn runtime_metrics_three_shards_roundtrip() {
    let metrics = RuntimeMetrics {
        shards: vec![
            ShardMetrics {
                shard_id: 0,
                active_runs: 1,
                ready_queue_depth: 0,
                action_queue_depth: 0,
                timer_count: 0,
                frame_pool_free: 0,
                frame_pool_total: 0,
                trace_ring_fill_pct: 0.0,
                steps_total: 0,
                actions_total: 0,
            },
            ShardMetrics {
                shard_id: 1,
                active_runs: 2,
                ready_queue_depth: 0,
                action_queue_depth: 0,
                timer_count: 0,
                frame_pool_free: 0,
                frame_pool_total: 0,
                trace_ring_fill_pct: 0.0,
                steps_total: 0,
                actions_total: 0,
            },
            ShardMetrics {
                shard_id: 2,
                active_runs: 3,
                ready_queue_depth: 0,
                action_queue_depth: 0,
                timer_count: 0,
                frame_pool_free: 0,
                frame_pool_total: 0,
                trace_ring_fill_pct: 0.0,
                steps_total: 0,
                actions_total: 0,
            },
        ],
        journal: JournalMetrics {
            writer_queue_depth: 0,
            total_events: 0,
            total_runs: 0,
        },
        ipc: IpcMetrics {
            connected_clients: 0,
            commands_processed: 0,
        },
        totals: AggregateMetrics {
            runs_active: 0,
            runs_waiting: 0,
            runs_failed_total: 0,
            runs_finished_total: 0,
        },
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: RuntimeMetrics = postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert_eq!(decoded.shards.len(), 3);
    assert_eq!(decoded, metrics);
}

// ── Edge value tests ──

#[test]
fn shard_metrics_with_nan_trace_ring_fill_pct_roundtrip() {
    let metrics = ShardMetrics {
        shard_id: 0,
        active_runs: 0,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: f32::NAN,
        steps_total: 0,
        actions_total: 0,
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: ShardMetrics = postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert!(decoded.trace_ring_fill_pct.is_nan());
}

#[test]
fn shard_metrics_with_negative_trace_ring_fill_pct_roundtrip() {
    let metrics = ShardMetrics {
        shard_id: 0,
        active_runs: 0,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: -1.0,
        steps_total: 0,
        actions_total: 0,
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: ShardMetrics = postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert_eq!(decoded.trace_ring_fill_pct, -1.0);
    assert_eq!(decoded, metrics);
}

#[test]
fn ipc_metrics_with_max_connected_clients_roundtrip() {
    let metrics = IpcMetrics {
        connected_clients: u32::MAX,
        commands_processed: 0,
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: IpcMetrics = postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert_eq!(decoded, metrics);
}

#[test]
fn ipc_metrics_with_max_commands_processed_roundtrip() {
    let metrics = IpcMetrics {
        connected_clients: 0,
        commands_processed: u64::MAX,
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: IpcMetrics = postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert_eq!(decoded, metrics);
}

#[test]
fn aggregate_metrics_with_all_max_values_roundtrip() {
    let metrics = AggregateMetrics {
        runs_active: u32::MAX,
        runs_waiting: u32::MAX,
        runs_failed_total: u64::MAX,
        runs_finished_total: u64::MAX,
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: AggregateMetrics =
        postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert_eq!(decoded, metrics);
}

// ── PartialEq field-by-field tests ──

#[test]
fn shard_metrics_equality() {
    let a = ShardMetrics {
        shard_id: 0,
        active_runs: 1,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: 0.0,
        steps_total: 0,
        actions_total: 0,
    };
    let b = ShardMetrics {
        shard_id: 0,
        active_runs: 1,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: 0.0,
        steps_total: 0,
        actions_total: 0,
    };
    assert_eq!(a, b);
}

#[test]
fn shard_metrics_inequality_different_shard_id() {
    let a = ShardMetrics {
        shard_id: 0,
        active_runs: 1,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: 0.0,
        steps_total: 0,
        actions_total: 0,
    };
    let b = ShardMetrics {
        shard_id: 1,
        active_runs: 1,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: 0.0,
        steps_total: 0,
        actions_total: 0,
    };
    assert_ne!(a, b);
}

#[test]
fn shard_metrics_inequality_different_active_runs() {
    let a = ShardMetrics {
        shard_id: 0,
        active_runs: 1,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: 0.0,
        steps_total: 0,
        actions_total: 0,
    };
    let mut b = a.clone();
    b.active_runs = 2;
    assert_ne!(a, b);
}

#[test]
fn shard_metrics_inequality_different_ready_queue_depth() {
    let a = ShardMetrics {
        shard_id: 0,
        active_runs: 1,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: 0.0,
        steps_total: 0,
        actions_total: 0,
    };
    let mut b = a.clone();
    b.ready_queue_depth = 1;
    assert_ne!(a, b);
}

#[test]
fn shard_metrics_inequality_different_action_queue_depth() {
    let a = ShardMetrics {
        shard_id: 0,
        active_runs: 1,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: 0.0,
        steps_total: 0,
        actions_total: 0,
    };
    let mut b = a.clone();
    b.action_queue_depth = 1;
    assert_ne!(a, b);
}

#[test]
fn shard_metrics_inequality_different_timer_count() {
    let a = ShardMetrics {
        shard_id: 0,
        active_runs: 1,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: 0.0,
        steps_total: 0,
        actions_total: 0,
    };
    let mut b = a.clone();
    b.timer_count = 1;
    assert_ne!(a, b);
}

#[test]
fn shard_metrics_inequality_different_frame_pool_free() {
    let a = ShardMetrics {
        shard_id: 0,
        active_runs: 1,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: 0.0,
        steps_total: 0,
        actions_total: 0,
    };
    let mut b = a.clone();
    b.frame_pool_free = 1;
    assert_ne!(a, b);
}

#[test]
fn shard_metrics_inequality_different_frame_pool_total() {
    let a = ShardMetrics {
        shard_id: 0,
        active_runs: 1,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: 0.0,
        steps_total: 0,
        actions_total: 0,
    };
    let mut b = a.clone();
    b.frame_pool_total = 1;
    assert_ne!(a, b);
}

#[test]
fn shard_metrics_inequality_different_trace_ring_fill_pct() {
    let a = ShardMetrics {
        shard_id: 0,
        active_runs: 1,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: 0.0,
        steps_total: 0,
        actions_total: 0,
    };
    let mut b = a.clone();
    b.trace_ring_fill_pct = 1.0;
    assert_ne!(a, b);
}

#[test]
fn shard_metrics_inequality_different_steps_total() {
    let a = ShardMetrics {
        shard_id: 0,
        active_runs: 1,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: 0.0,
        steps_total: 0,
        actions_total: 0,
    };
    let mut b = a.clone();
    b.steps_total = 1;
    assert_ne!(a, b);
}

#[test]
fn shard_metrics_inequality_different_actions_total() {
    let a = ShardMetrics {
        shard_id: 0,
        active_runs: 1,
        ready_queue_depth: 0,
        action_queue_depth: 0,
        timer_count: 0,
        frame_pool_free: 0,
        frame_pool_total: 0,
        trace_ring_fill_pct: 0.0,
        steps_total: 0,
        actions_total: 0,
    };
    let mut b = a.clone();
    b.actions_total = 1;
    assert_ne!(a, b);
}

#[test]
fn journal_metrics_inequality_different_writer_queue_depth() {
    let a = JournalMetrics {
        writer_queue_depth: 0,
        total_events: 0,
        total_runs: 0,
    };
    let mut b = a.clone();
    b.writer_queue_depth = 1;
    assert_ne!(a, b);
}

#[test]
fn journal_metrics_inequality_different_total_events() {
    let a = JournalMetrics {
        writer_queue_depth: 0,
        total_events: 0,
        total_runs: 0,
    };
    let mut b = a.clone();
    b.total_events = 1;
    assert_ne!(a, b);
}

#[test]
fn journal_metrics_inequality_different_total_runs() {
    let a = JournalMetrics {
        writer_queue_depth: 0,
        total_events: 0,
        total_runs: 0,
    };
    let mut b = a.clone();
    b.total_runs = 1;
    assert_ne!(a, b);
}

#[test]
fn ipc_metrics_inequality_different_connected_clients() {
    let a = IpcMetrics {
        connected_clients: 0,
        commands_processed: 0,
    };
    let mut b = a.clone();
    b.connected_clients = 1;
    assert_ne!(a, b);
}

#[test]
fn ipc_metrics_inequality_different_commands_processed() {
    let a = IpcMetrics {
        connected_clients: 0,
        commands_processed: 0,
    };
    let mut b = a.clone();
    b.commands_processed = 1;
    assert_ne!(a, b);
}

#[test]
fn aggregate_metrics_inequality_different_runs_active() {
    let a = AggregateMetrics {
        runs_active: 0,
        runs_waiting: 0,
        runs_failed_total: 0,
        runs_finished_total: 0,
    };
    let mut b = a.clone();
    b.runs_active = 1;
    assert_ne!(a, b);
}

#[test]
fn aggregate_metrics_inequality_different_runs_waiting() {
    let a = AggregateMetrics {
        runs_active: 0,
        runs_waiting: 0,
        runs_failed_total: 0,
        runs_finished_total: 0,
    };
    let mut b = a.clone();
    b.runs_waiting = 1;
    assert_ne!(a, b);
}

#[test]
fn aggregate_metrics_inequality_different_runs_failed_total() {
    let a = AggregateMetrics {
        runs_active: 0,
        runs_waiting: 0,
        runs_failed_total: 0,
        runs_finished_total: 0,
    };
    let mut b = a.clone();
    b.runs_failed_total = 1;
    assert_ne!(a, b);
}

#[test]
fn aggregate_metrics_inequality_different_runs_finished_total() {
    let a = AggregateMetrics {
        runs_active: 0,
        runs_waiting: 0,
        runs_failed_total: 0,
        runs_finished_total: 0,
    };
    let mut b = a.clone();
    b.runs_finished_total = 1;
    assert_ne!(a, b);
}

// ── RuntimeMetrics equality / inequality ──

#[test]
fn runtime_metrics_equality() {
    let a = RuntimeMetrics {
        shards: vec![ShardMetrics {
            shard_id: 0,
            active_runs: 1,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 0,
            frame_pool_total: 0,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        }],
        journal: JournalMetrics {
            writer_queue_depth: 0,
            total_events: 0,
            total_runs: 0,
        },
        ipc: IpcMetrics {
            connected_clients: 0,
            commands_processed: 0,
        },
        totals: AggregateMetrics {
            runs_active: 0,
            runs_waiting: 0,
            runs_failed_total: 0,
            runs_finished_total: 0,
        },
    };
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn runtime_metrics_inequality_different_shards() {
    let a = RuntimeMetrics {
        shards: vec![ShardMetrics {
            shard_id: 0,
            active_runs: 1,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 0,
            frame_pool_total: 0,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        }],
        journal: JournalMetrics {
            writer_queue_depth: 0,
            total_events: 0,
            total_runs: 0,
        },
        ipc: IpcMetrics {
            connected_clients: 0,
            commands_processed: 0,
        },
        totals: AggregateMetrics {
            runs_active: 0,
            runs_waiting: 0,
            runs_failed_total: 0,
            runs_finished_total: 0,
        },
    };
    let mut b = a.clone();
    b.shards[0].active_runs = 2;
    assert_ne!(a, b);
}

#[test]
fn runtime_metrics_inequality_different_journal() {
    let a = RuntimeMetrics {
        shards: vec![ShardMetrics {
            shard_id: 0,
            active_runs: 1,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 0,
            frame_pool_total: 0,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        }],
        journal: JournalMetrics {
            writer_queue_depth: 0,
            total_events: 0,
            total_runs: 0,
        },
        ipc: IpcMetrics {
            connected_clients: 0,
            commands_processed: 0,
        },
        totals: AggregateMetrics {
            runs_active: 0,
            runs_waiting: 0,
            runs_failed_total: 0,
            runs_finished_total: 0,
        },
    };
    let mut b = a.clone();
    b.journal.total_events = 1;
    assert_ne!(a, b);
}

#[test]
fn runtime_metrics_inequality_different_ipc() {
    let a = RuntimeMetrics {
        shards: vec![ShardMetrics {
            shard_id: 0,
            active_runs: 1,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 0,
            frame_pool_total: 0,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        }],
        journal: JournalMetrics {
            writer_queue_depth: 0,
            total_events: 0,
            total_runs: 0,
        },
        ipc: IpcMetrics {
            connected_clients: 0,
            commands_processed: 0,
        },
        totals: AggregateMetrics {
            runs_active: 0,
            runs_waiting: 0,
            runs_failed_total: 0,
            runs_finished_total: 0,
        },
    };
    let mut b = a.clone();
    b.ipc.commands_processed = 1;
    assert_ne!(a, b);
}

#[test]
fn runtime_metrics_inequality_different_totals() {
    let a = RuntimeMetrics {
        shards: vec![ShardMetrics {
            shard_id: 0,
            active_runs: 1,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 0,
            frame_pool_total: 0,
            trace_ring_fill_pct: 0.0,
            steps_total: 0,
            actions_total: 0,
        }],
        journal: JournalMetrics {
            writer_queue_depth: 0,
            total_events: 0,
            total_runs: 0,
        },
        ipc: IpcMetrics {
            connected_clients: 0,
            commands_processed: 0,
        },
        totals: AggregateMetrics {
            runs_active: 0,
            runs_waiting: 0,
            runs_failed_total: 0,
            runs_finished_total: 0,
        },
    };
    let mut b = a.clone();
    b.totals.runs_active = 1;
    assert_ne!(a, b);
}

#[test]
fn aggregate_metrics_zero_values_roundtrip() {
    let metrics = AggregateMetrics {
        runs_active: 0,
        runs_waiting: 0,
        runs_failed_total: 0,
        runs_finished_total: 0,
    };
    let encoded = postcard::to_allocvec(&metrics).expect("encoding should succeed");
    let decoded: AggregateMetrics =
        postcard::from_bytes(&encoded).expect("decoding should succeed");
    assert_eq!(decoded, metrics);
}
