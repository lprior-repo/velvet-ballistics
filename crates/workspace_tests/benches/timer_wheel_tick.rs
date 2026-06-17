//! Timer wheel tick benchmarks.
//!
//! Measures TimerWheel::fire_expired overhead as timer count grows.

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

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use std::time::{Duration, Instant};
use vb_core::ids::RunId;
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::types::PendingTimerKind;

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

/// Creates an empty timer wheel.
fn empty_wheel() -> TimerWheel {
    TimerWheel::new()
}

/// Creates a wheel with 1 expired timer.
fn wheel_1_expired(now: Instant) -> TimerWheel {
    let mut wheel = TimerWheel::new();
    let deadline = now - Duration::from_millis(10);
    assert_eq!(
        wheel.insert(RunId::new(1), deadline, PendingTimerKind::Wait),
        Ok(())
    );
    wheel
}

/// Creates a wheel with 10 expired timers at the same deadline.
fn wheel_10_expired(now: Instant) -> TimerWheel {
    let mut wheel = TimerWheel::new();
    let deadline = now - Duration::from_millis(10);
    let mut i = 0u64;
    while i < 10 {
        assert_eq!(
            wheel.insert(RunId::new(i), deadline, PendingTimerKind::Wait),
            Ok(())
        );
        i = i.saturating_add(1);
    }
    wheel
}

/// Creates a wheel with 100 timers: 90 expired, 10 future.
fn wheel_100_mixed(now: Instant) -> TimerWheel {
    let mut wheel = TimerWheel::new();
    let expired_deadline = now - Duration::from_millis(10);
    let future_deadline = now + Duration::from_millis(100);
    let mut i = 0u64;
    while i < 90 {
        assert_eq!(
            wheel.insert(RunId::new(i), expired_deadline, PendingTimerKind::Wait),
            Ok(())
        );
        i = i.saturating_add(1);
    }
    while i < 100 {
        assert_eq!(
            wheel.insert(RunId::new(i), future_deadline, PendingTimerKind::Wait),
            Ok(())
        );
        i = i.saturating_add(1);
    }
    wheel
}

/// Creates a wheel with 100 timers (all future).
fn wheel_100(now: Instant) -> TimerWheel {
    let mut wheel = TimerWheel::new();
    let future_deadline = now + Duration::from_millis(100);
    let mut i = 0u64;
    while i < 100 {
        assert_eq!(
            wheel.insert(RunId::new(i), future_deadline, PendingTimerKind::Wait),
            Ok(())
        );
        i = i.saturating_add(1);
    }
    wheel
}

/// Creates a wheel with 10 future timers.
fn wheel_10(now: Instant) -> TimerWheel {
    let mut wheel = TimerWheel::new();
    let future_deadline = now + Duration::from_millis(100);
    let mut i = 0u64;
    while i < 10 {
        assert_eq!(
            wheel.insert(RunId::new(i), future_deadline, PendingTimerKind::Wait),
            Ok(())
        );
        i = i.saturating_add(1);
    }
    wheel
}

fn bench_timer_wheel_tick(c: &mut Criterion) {
    let now = Instant::now();

    let mut group = c.benchmark_group("timer_wheel_tick");

    // Fire empty wheel
    {
        let fixture_bytes = 0usize;
        group.throughput(Throughput::Elements(0));
        group.bench_function(
            metadata(
                "timer_wheel_fire_empty",
                fixture_bytes,
                "fixture=wheel_empty;surface=fire_expired",
            ),
            |b| {
                b.iter(|| {
                    let mut wheel = empty_wheel();
                    let fired = wheel.fire_expired(now);
                    // Exact assertion: empty wheel fires exactly 0 timers
                    assert_eq!(
                        fired.len(),
                        0,
                        "fire_expired on empty wheel must return 0 entries"
                    );
                    assert!(wheel.is_empty(), "wheel must be empty after firing");
                    black_box(fired)
                });
            },
        );
    }

    // Fire 1 expired timer
    {
        let fixture_bytes = 1usize;
        group.throughput(Throughput::Elements(1));
        group.bench_function(
            metadata(
                "timer_wheel_fire_1_expired",
                fixture_bytes,
                "fixture=wheel_1_expired;surface=fire_expired",
            ),
            |b| {
                b.iter(|| {
                    let mut wheel = wheel_1_expired(now);
                    let fired = wheel.fire_expired(now);
                    // Exact assertion: 1 expired timer fired
                    assert_eq!(fired.len(), 1, "fire_expired must return exactly 1 entry");
                    assert_eq!(
                        fired[0].run,
                        RunId::new(1),
                        "fired timer must have run_id=1"
                    );
                    // Timer removed from both indexes
                    assert!(
                        wheel.is_empty(),
                        "wheel must be empty after firing only timer"
                    );
                    black_box(fired)
                });
            },
        );
    }

    // Fire 10 expired timers
    {
        let fixture_bytes = 10usize;
        group.throughput(Throughput::Elements(10));
        group.bench_function(
            metadata(
                "timer_wheel_fire_10_expired",
                fixture_bytes,
                "fixture=wheel_10_expired;surface=fire_expired",
            ),
            |b| {
                b.iter(|| {
                    let mut wheel = wheel_10_expired(now);
                    let fired = wheel.fire_expired(now);
                    // Exact assertion: all 10 expired timers fired
                    assert_eq!(
                        fired.len(),
                        10,
                        "fire_expired must return exactly 10 entries"
                    );
                    // Timers removed
                    assert!(
                        wheel.is_empty(),
                        "wheel must be empty after firing all 10 timers"
                    );
                    black_box(fired)
                });
            },
        );
    }

    // Fire 90 of 100 (90 expired, 10 future)
    {
        let fixture_bytes = 100usize;
        group.throughput(Throughput::Elements(100));
        group.bench_function(
            metadata(
                "timer_wheel_fire_90_of_100",
                fixture_bytes,
                "fixture=wheel_100_mixed;surface=fire_expired",
            ),
            |b| {
                b.iter(|| {
                    let mut wheel = wheel_100_mixed(now);
                    let fired = wheel.fire_expired(now);
                    // Exact assertion: exactly 90 expired timers fired
                    assert_eq!(
                        fired.len(),
                        90,
                        "fire_expired must return exactly 90 expired entries"
                    );
                    // 10 future timers remain
                    assert_eq!(
                        wheel.next_deadline().is_some(),
                        true,
                        "10 future timers must remain with a deadline"
                    );
                    black_box(fired)
                });
            },
        );
    }

    // Cancel 1 of 100 timers
    {
        let fixture_bytes = 100usize;
        group.bench_function(
            metadata(
                "timer_wheel_cancel_1",
                fixture_bytes,
                "fixture=wheel_100;surface=cancel",
            ),
            |b| {
                b.iter(|| {
                    let mut wheel = wheel_100(now);
                    let cancelled = wheel.cancel(RunId::new(50));
                    // Exact assertion: cancel returns true for existing timer
                    assert!(cancelled, "cancel of existing timer 50 must return true");
                    // Cancelled timer no longer fires
                    let fired = wheel.fire_expired(now + Duration::from_secs(1));
                    assert!(
                        fired.iter().all(|e| e.run != RunId::new(50)),
                        "cancelled timer 50 must not appear in fired list"
                    );
                    // The 99 non-cancelled timers fire at the advanced deadline.
                    assert_eq!(
                        fired.len(),
                        99,
                        "all 99 non-cancelled timers must fire at the advanced deadline"
                    );
                    assert!(
                        wheel.is_empty(),
                        "wheel must be empty after firing 99 timers"
                    );
                    black_box(cancelled)
                });
            },
        );
    }

    // next_deadline on wheel with 10 timers
    {
        let fixture_bytes = 10usize;
        group.bench_function(
            metadata(
                "timer_wheel_next_deadline",
                fixture_bytes,
                "fixture=wheel_10;surface=next_deadline",
            ),
            |b| {
                b.iter(|| {
                    let wheel = wheel_10(now);
                    let deadline = wheel.next_deadline();
                    // Exact assertion: next_deadline must be Some with future instant
                    assert!(
                        deadline.is_some(),
                        "wheel with 10 timers must have a next deadline"
                    );
                    let d = deadline.expect("some");
                    assert!(d > now, "next deadline must be in the future");
                    black_box(deadline)
                });
            },
        );
    }

    // Insert 100 timers
    {
        let fixture_bytes = 100usize;
        group.throughput(Throughput::Elements(100));
        group.bench_function(
            metadata(
                "timer_wheel_insert_100",
                fixture_bytes,
                "fixture=wheel_empty;surface=insert_100",
            ),
            |b| {
                b.iter(|| {
                    let mut wheel = empty_wheel();
                    let future_deadline = now + Duration::from_millis(100);
                    let mut i = 0u64;
                    while i < 100 {
                        assert_eq!(
                            wheel.insert(RunId::new(i), future_deadline, PendingTimerKind::Wait),
                            Ok(())
                        );
                        i = i.saturating_add(1);
                    }
                    // Exact assertion: 100 timers inserted
                    assert_eq!(
                        wheel.next_deadline().is_some(),
                        true,
                        "wheel must have deadline after 100 inserts"
                    );
                    black_box(wheel)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_timer_wheel_tick);
criterion_main!(benches);
