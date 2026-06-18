#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]

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
