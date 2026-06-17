//! ArrayQueue benchmarks.
//!
//! Direct benchmarks for crossbeam_queue::ArrayQueue operations.
//! Per Section 50, ArrayQueue is the mandated backend for ShardCommandQueue.

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
use crossbeam_queue::ArrayQueue;
use std::hint::black_box;

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

/// Test item for queue operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QueueItem(u64);

fn bench_array_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group("array_queue");

    // Push 1 item to empty queue
    {
        let fixture_bytes = 1usize;
        group.throughput(Throughput::Elements(1));
        group.bench_function(
            metadata(
                "array_queue_push_1",
                fixture_bytes,
                "fixture=aq_empty_1024;surface=push",
            ),
            |b| {
                b.iter(|| {
                    let queue = ArrayQueue::new(1024);
                    let item = QueueItem(42);
                    let result = queue.push(item);
                    // Exact assertion: push on non-full queue must succeed
                    assert!(result.is_ok(), "push on empty ArrayQueue must succeed");
                    assert_eq!(queue.len(), 1, "queue len must be 1 after push");
                    black_box(queue)
                });
            },
        );
    }

    // Pop 1 item from queue with 100 items
    {
        let fixture_bytes = 100usize;
        group.throughput(Throughput::Elements(100));
        group.bench_function(
            metadata(
                "array_queue_pop_1",
                fixture_bytes,
                "fixture=aq_100_items;surface=pop",
            ),
            |b| {
                b.iter(|| {
                    let queue = ArrayQueue::new(1024);
                    // Pre-fill
                    let mut i = 0u64;
                    while i < 100 {
                        let _ = queue.push(QueueItem(i));
                        i = i.saturating_add(1);
                    }
                    // Pop one
                    let popped = queue.pop();
                    // Exact assertion: popped item must be first (FIFO)
                    assert!(popped.is_some(), "pop on non-empty queue must return Some");
                    assert_eq!(
                        popped.expect("item").0,
                        0,
                        "first popped item must be QueueItem(0) — FIFO order"
                    );
                    assert_eq!(queue.len(), 99, "queue len must be 99 after one pop");
                    black_box(popped)
                });
            },
        );
    }

    // Push on full queue — error path
    {
        let fixture_bytes = 1usize;
        group.bench_function(
            metadata(
                "array_queue_push_full_err",
                fixture_bytes,
                "fixture=aq_full_1;surface=push_full_err",
            ),
            |b| {
                b.iter(|| {
                    let queue = ArrayQueue::new(1);
                    let _ = queue.push(QueueItem(0));
                    // Exact assertion: queue is full
                    assert!(
                        queue.is_full(),
                        "queue with 1 item and capacity 1 must be full"
                    );
                    // Push second item — must return Err with item
                    let item = QueueItem(999);
                    let result = queue.push(item);
                    assert!(result.is_err(), "push on full ArrayQueue must return Err");
                    // Item NOT lost — returned in Err
                    let returned_item = result.expect_err("err");
                    assert_eq!(
                        returned_item.0, 999,
                        "Err variant must contain the rejected item"
                    );
                    // Original item still in queue
                    assert_eq!(
                        queue.len(),
                        1,
                        "queue must still contain 1 item after rejected push"
                    );
                    let original = queue.pop();
                    assert_eq!(
                        original.expect("item").0,
                        0,
                        "original item QueueItem(0) must still be in queue"
                    );
                    black_box(result)
                });
            },
        );
    }

    // Capacity boundary: push exactly 1024 items
    {
        let fixture_bytes = 1024usize;
        group.throughput(Throughput::Elements(1024));
        group.bench_function(
            metadata(
                "array_queue_capacity_1024",
                fixture_bytes,
                "fixture=aq_empty_1024;surface=push_capacity_boundary",
            ),
            |b| {
                b.iter(|| {
                    let queue = ArrayQueue::new(1024);
                    let mut i = 0u64;
                    while i < 1024 {
                        let result = queue.push(QueueItem(i));
                        assert!(result.is_ok(), "push {} must succeed (within capacity)", i);
                        i = i.saturating_add(1);
                    }
                    // Exact assertion: exactly full
                    assert_eq!(queue.len(), 1024, "queue must have exactly 1024 items");
                    assert!(queue.is_full(), "queue must be full after 1024 pushes");
                    // 1025th push fails
                    let overflow = queue.push(QueueItem(9999));
                    assert!(overflow.is_err(), "1025th push must fail");
                    black_box(queue)
                });
            },
        );
    }

    // is_full and len consistency
    {
        let fixture_bytes = 512usize;
        group.bench_function(
            metadata(
                "array_queue_is_full_len",
                fixture_bytes,
                "fixture=aq_512_items;surface=is_full_len",
            ),
            |b| {
                b.iter(|| {
                    let queue = ArrayQueue::new(1024);
                    let mut i = 0u64;
                    while i < 512 {
                        let _ = queue.push(QueueItem(i));
                        i = i.saturating_add(1);
                    }
                    // Exact assertions: half-full state
                    assert!(!queue.is_full(), "512/1024 queue must NOT be full");
                    assert_eq!(queue.len(), 512, "queue len must be 512");
                    assert_eq!(queue.capacity(), 1024, "queue capacity must be 1024");
                    assert_eq!(
                        queue.len(),
                        queue.capacity() / 2,
                        "len must be exactly half of capacity"
                    );
                    black_box(queue)
                });
            },
        );
    }

    // SPSC FIFO correctness: 1000 items
    {
        let fixture_bytes = 1000usize;
        group.throughput(Throughput::Elements(1000));
        group.bench_function(
            metadata(
                "array_queue_fifo_1000",
                fixture_bytes,
                "fixture=aq_1000_items;surface=fifo_1000",
            ),
            |b| {
                b.iter(|| {
                    let queue = ArrayQueue::new(1024);
                    // Fill with sequential values
                    let mut i = 0u64;
                    while i < 1000 {
                        let _ = queue.push(QueueItem(i));
                        i = i.saturating_add(1);
                    }
                    // Drain and verify FIFO order
                    let mut expected = 0u64;
                    let mut popped = 0u64;
                    while let Some(item) = queue.pop() {
                        assert_eq!(
                            item.0, expected,
                            "popped item {} must equal expected {} — FIFO violation",
                            item.0, expected
                        );
                        expected = expected.saturating_add(1);
                        popped = popped.saturating_add(1);
                    }
                    // Exact assertion: exactly 1000 items popped
                    assert_eq!(popped, 1000, "must pop exactly 1000 items");
                    assert!(queue.is_empty(), "queue must be empty after draining");
                    black_box(popped)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_array_queue);
criterion_main!(benches);
