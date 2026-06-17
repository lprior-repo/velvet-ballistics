//! vb-h6ix benchmarks: Replay Latest Execution Attempt Only
//!
//! Criterion benchmarks for the latest-attempt filtering replay logic.
//!
//! RED PHASE: These benchmarks will fail to compile until the implementation adds:
//!   1. `attempt: u16` field to JournalEvent variants
//!   2. Latest-attempt filtering logic in replay_events()

#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::cmp_owned,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::derivable_impls,
    clippy::duplicated_attributes,
    clippy::enum_variant_names,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::identity_op,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::if_same_then_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::io_other_error,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_contains,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::manual_unwrap_or_default,
    clippy::map_clone,
    clippy::map_flatten,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::multiple_bound_locations,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::new_without_default,
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
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_sort_by,
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
    clippy::useless_asref,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    missing_docs,
    unused_imports,
    unused_variables,
)]

use criterion::{BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use vb_core::{ActionId, RunId, StepIdx, WorkflowDigest};
use vb_storage::recovery::{ActionReplayTracker, replay_events};
use vb_storage::{EventSeq, JournalEvent};

/// Benchmark metadata format.
fn metadata(_name: &str, extra: &str) -> String {
    format!("profile=bench;tool=criterion;bead=vb-h6ix;{}", extra)
}

/// Helper: create a test digest.
fn test_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

// ============================================================================
// Benchmark: replay_events with single attempt (baseline)
// ============================================================================

fn bench_replay_single_attempt(c: &mut Criterion) {
    let run = RunId::new(1);
    let _workflow = test_digest(0xAB);

    // Build events for a single attempt
    let events: Vec<JournalEvent> = (0..100)
        .flat_map(|i| {
            let action = ActionId::new((i % 10) as u16);
            vec![
                JournalEvent::ActionScheduled {
                    run,
                    seq: EventSeq::new(i as u64 * 2),
                    step: StepIdx::new((i % 5) as u16),
                    action,
                    attempt: 1,
                },
                JournalEvent::ActionCompletedEvent {
                    run,
                    seq: EventSeq::new(i as u64 * 2 + 1),
                    step: StepIdx::new((i % 5) as u16),
                    action,
                    attempt: 1,
                },
            ]
        })
        .collect();

    let mut group = c.benchmark_group("vb_h6ix_replay");
    group.throughput(Throughput::Elements(events.len() as u64));
    group.bench_function(
        BenchmarkId::from_parameter(metadata(
            "replay_single_attempt_100",
            "fixture=single_attempt_100",
        )),
        |b| {
            b.iter(|| {
                let mut tracker = ActionReplayTracker::new();
                black_box(replay_events(black_box(&events), &mut tracker, &[]))
            })
        },
    );
    group.finish();
}

// ============================================================================
// Benchmark: replay_events with mixed attempts (vb_h6ix core scenario)
// ============================================================================

fn bench_replay_mixed_attempts(c: &mut Criterion) {
    let run = RunId::new(1);
    let _workflow = test_digest(0xCD);

    // Build events for two interleaved attempts
    // Attempt 1: actions 1-50 (stale)
    // Attempt 2: actions 51-100 (latest)
    let events: Vec<JournalEvent> = (0..100)
        .flat_map(|i| {
            let attempt = if i < 50 { 1 } else { 2 };
            let action = ActionId::new((i % 20) as u16);
            vec![
                JournalEvent::ActionScheduled {
                    run,
                    seq: EventSeq::new(i as u64 * 2),
                    step: StepIdx::new((i % 5) as u16),
                    action,
                    attempt,
                },
                JournalEvent::ActionCompletedEvent {
                    run,
                    seq: EventSeq::new(i as u64 * 2 + 1),
                    step: StepIdx::new((i % 5) as u16),
                    action,
                    attempt,
                },
            ]
        })
        .collect();

    let mut group = c.benchmark_group("vb_h6ix_replay");
    group.throughput(Throughput::Elements(events.len() as u64));
    group.bench_function(
        BenchmarkId::from_parameter(metadata(
            "replay_mixed_attempts_100",
            "fixture=mixed_attempts_100",
        )),
        |b| {
            b.iter(|| {
                let mut tracker = ActionReplayTracker::new();
                black_box(replay_events(black_box(&events), &mut tracker, &[]))
            })
        },
    );
    group.finish();
}

// ============================================================================
// Benchmark: replay_events with many stale events (worst case filtering)
// ============================================================================

fn bench_replay_many_stale_events(c: &mut Criterion) {
    let run = RunId::new(1);
    let _workflow = test_digest(0xEF);

    // Build events where 90% are stale (attempt 1), only 10% are latest (attempt 2)
    let events: Vec<JournalEvent> = (0..1000)
        .flat_map(|i| {
            let attempt = if i < 900 { 1 } else { 2 };
            let action = ActionId::new((i % 50) as u16);
            vec![
                JournalEvent::ActionScheduled {
                    run,
                    seq: EventSeq::new(i as u64 * 2),
                    step: StepIdx::new((i % 10) as u16),
                    action,
                    attempt,
                },
                JournalEvent::ActionCompletedEvent {
                    run,
                    seq: EventSeq::new(i as u64 * 2 + 1),
                    step: StepIdx::new((i % 10) as u16),
                    action,
                    attempt,
                },
            ]
        })
        .collect();

    let mut group = c.benchmark_group("vb_h6ix_replay");
    group.throughput(Throughput::Elements(events.len() as u64));
    group.bench_function(
        BenchmarkId::from_parameter(metadata(
            "replay_900_stale_100_latest",
            "fixture=many_stale",
        )),
        |b| {
            b.iter(|| {
                let mut tracker = ActionReplayTracker::new();
                black_box(replay_events(black_box(&events), &mut tracker, &[]))
            })
        },
    );
    group.finish();
}

// ============================================================================
// Benchmark: tracker operations (isolated from replay)
// ============================================================================

fn bench_tracker_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("vb_h6ix_tracker");
    group.bench_function(
        BenchmarkId::from_parameter(metadata("tracker_mark_completed", "surface=tracker_mark")),
        |b| {
            b.iter(|| {
                let mut tracker = ActionReplayTracker::new();
                for i in 0..100 {
                    tracker.mark_completed(ActionId::new(i), StepIdx::ZERO);
                }
                black_box(tracker)
            })
        },
    );
    group.bench_function(
        BenchmarkId::from_parameter(metadata("tracker_is_resolved", "surface=tracker_query")),
        |b| {
            let mut tracker = ActionReplayTracker::new();
            for i in 0..100 {
                tracker.mark_completed(ActionId::new(i), StepIdx::ZERO);
            }
            b.iter(|| {
                for i in 0..100 {
                    black_box(tracker.is_resolved(ActionId::new(i), StepIdx::ZERO));
                }
            })
        },
    );
    group.finish();
}

// ============================================================================
// Criterion entry point
// ============================================================================

criterion::criterion_group!(
    vb_h6ix_benches,
    bench_replay_single_attempt,
    bench_replay_mixed_attempts,
    bench_replay_many_stale_events,
    bench_tracker_operations,
);

criterion::criterion_main!(vb_h6ix_benches);
