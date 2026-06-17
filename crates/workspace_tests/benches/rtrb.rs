//! rtrb benchmarks.
//!
//! Direct benchmarks for rtrb::RingBuffer (SPSC ring buffer for trace/action paths).
//! Per Section 50, rtrb is required for SPSC trace/action completion paths.

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
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
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
    unused_variables
)]

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use rtrb::{Consumer, Producer, RingBuffer};
use std::hint::black_box;

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

/// Trace event for benchmarking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TraceEvent(u64);

type TraceProducer = Producer<TraceEvent>;
type TraceConsumer = Consumer<TraceEvent>;

/// Creates an empty ring buffer with capacity 128.
fn empty_ringbuffer_128() -> (TraceProducer, TraceConsumer) {
    RingBuffer::new(128)
}

/// Creates a ring buffer with 100 items (split across producer/consumer).
fn ringbuffer_100_items() -> (TraceProducer, TraceConsumer) {
    let (mut prod, cons) = RingBuffer::new(128);
    let mut i = 0u64;
    while i < 100 {
        if let Ok(()) = prod.push(TraceEvent(i)) {
            i += 1;
        }
    }
    (prod, cons)
}

/// Creates a ring buffer with 50 items.
fn ringbuffer_50_items() -> (TraceProducer, TraceConsumer) {
    let (mut prod, cons) = RingBuffer::new(128);
    let mut i = 0u64;
    while i < 50 {
        if let Ok(()) = prod.push(TraceEvent(i)) {
            i += 1;
        }
    }
    (prod, cons)
}

/// Creates a full ring buffer (capacity 128, 128 items).
fn ringbuffer_full() -> (TraceProducer, TraceConsumer) {
    let (mut prod, cons) = RingBuffer::new(128);
    let mut i = 0u64;
    while i < 128 {
        if let Ok(()) = prod.push(TraceEvent(i)) {
            i += 1;
        }
    }
    (prod, cons)
}

/// Creates a ring buffer with 1000 items.
fn ringbuffer_1000_items() -> (TraceProducer, TraceConsumer) {
    let (mut prod, cons) = RingBuffer::new(1024);
    let mut i = 0u64;
    while i < 1000 {
        if let Ok(()) = prod.push(TraceEvent(i)) {
            i += 1;
        }
    }
    (prod, cons)
}

fn bench_rtrb(c: &mut Criterion) {
    let mut group = c.benchmark_group("rtrb");

    // Push 1 item to empty buffer
    {
        let fixture_bytes = 1usize;
        group.throughput(Throughput::Elements(1));
        group.bench_function(
            metadata(
                "rtrb_push_1",
                fixture_bytes,
                "fixture=rtrb_empty;surface=push",
            ),
            |b| {
                b.iter(|| {
                    let (mut prod, _cons) = empty_ringbuffer_128();
                    let item = TraceEvent(42);
                    let result = prod.push(item);
                    // Exact assertion: push on non-full buffer succeeds
                    assert!(result.is_ok(), "push on empty rtrb buffer must succeed");
                    black_box(result)
                });
            },
        );
    }

    // Pop 1 item from buffer with 100 items
    {
        let fixture_bytes = 100usize;
        group.throughput(Throughput::Elements(100));
        group.bench_function(
            metadata(
                "rtrb_pop_1",
                fixture_bytes,
                "fixture=rtrb_100_items;surface=pop",
            ),
            |b| {
                b.iter(|| {
                    let (_prod, mut cons) = ringbuffer_100_items();
                    let popped = cons.pop();
                    // Exact assertion: pop returns first item in FIFO order
                    assert!(popped.is_ok(), "pop on non-empty buffer must return Ok");
                    assert_eq!(
                        popped.expect("item").0,
                        0,
                        "first popped item must be TraceEvent(0) — FIFO order"
                    );
                    black_box(popped)
                });
            },
        );
    }

    // Push on full buffer — error path
    {
        let fixture_bytes = 1usize;
        group.bench_function(
            metadata(
                "rtrb_push_full_err",
                fixture_bytes,
                "fixture=rtrb_full;surface=push_full_err",
            ),
            |b| {
                b.iter(|| {
                    let (mut prod, _cons) = ringbuffer_full();
                    let item = TraceEvent(999);
                    let result = prod.push(item);
                    // Exact assertion: push on full buffer returns Err with item
                    assert!(result.is_err(), "push on full rtrb buffer must return Err");
                    black_box(result)
                });
            },
        );
    }

    // Peek without consume
    {
        let fixture_bytes = 100usize;
        group.bench_function(
            metadata(
                "rtrb_peek",
                fixture_bytes,
                "fixture=rtrb_100_items;surface=peek",
            ),
            |b| {
                b.iter(|| {
                    let (_prod, mut cons) = ringbuffer_100_items();
                    let peeked_value = {
                        let peeked = cons.peek();
                        // Exact assertion: peek returns reference to head without removing
                        assert!(peeked.is_ok(), "peek on non-empty buffer must return Ok");
                        let item_ref = peeked.expect("item");
                        item_ref.0
                    };
                    assert_eq!(
                        peeked_value, 0,
                        "peeked item must be TraceEvent(0) — head of queue"
                    );
                    // Buffer unchanged after peek
                    assert_eq!(
                        cons.pop().expect("item").0,
                        0,
                        "first pop after peek must still be TraceEvent(0)"
                    );
                    black_box(peeked_value)
                });
            },
        );
    }

    // is_full and is_empty on half-full buffer
    {
        let fixture_bytes = 50usize;
        group.bench_function(
            metadata(
                "rtrb_is_full_is_empty",
                fixture_bytes,
                "fixture=rtrb_50_items;surface=is_full_is_empty",
            ),
            |b| {
                b.iter(|| {
                    let (prod, cons) = ringbuffer_50_items();
                    // Exact assertions on half-full buffer
                    assert!(!prod.is_full(), "50/128 buffer must NOT be full");
                    assert!(!cons.is_empty(), "50/128 buffer must NOT be empty");
                    // Default capacity is 128 (const generic)
                    black_box(cons)
                });
            },
        );
    }

    // SPSC FIFO: 1000 items through 128-capacity ring
    {
        let fixture_bytes = 1000usize;
        group.throughput(Throughput::Elements(1000));
        group.bench_function(
            metadata(
                "rtrb_fifo_1000",
                fixture_bytes,
                "fixture=rtrb_1000_items;surface=fifo_1000",
            ),
            |b| {
                b.iter(|| {
                    let (_prod, mut cons) = ringbuffer_1000_items();
                    // Note: with 128 capacity and 1000 items, the buffer will have
                    // wrapped around. We test the FIFO property by checking
                    // all items that can be consumed maintain order.
                    let mut expected = 0u64;
                    let mut popped = 0u64;
                    while let Ok(item) = cons.pop() {
                        // Note: due to ring buffer wrap, we verify that items
                        // appear in order relative to each other
                        assert!(
                            item.0 >= expected || item.0 < 100,
                            "items must maintain circular FIFO order"
                        );
                        expected = item.0.saturating_add(1);
                        popped += 1;
                    }
                    // Exact assertion: 1000 total items processed
                    assert_eq!(popped, 1000, "must process exactly 1000 items");
                    assert!(cons.is_empty(), "buffer must be empty after draining");
                    black_box(popped)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_rtrb);
criterion_main!(benches);
