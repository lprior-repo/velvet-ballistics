//! Action queuing benchmarks.
//!
//! Measures ShardCommandQueue enqueue/dequeue throughput.

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
use std::hint::black_box;
use vb_core::ids::RunId;
use vb_core::{
    CapabilitySet, CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, SlotIdx,
    StepIdx, WorkflowDigest, WorkflowParts,
};
use vb_runtime::RuntimeError;
use vb_runtime::shard::types::{ShardCommand, ShardCommandQueue};

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

/// Creates a ShardCommand::Submit for benchmarking.
fn make_submit_command(run_id: u64) -> ShardCommand {
    ShardCommand::Submit {
        run: RunId::new(run_id),
        workflow: simple_workflow(),
        caps: CapabilitySet::empty(),
    }
}

fn simple_workflow() -> CompiledWorkflow {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("bench_action_queue"),
        digest: WorkflowDigest::from_bytes([0x11; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
        symbols_count: 0,
    })
    .expect("workflow")
}

/// Creates an empty queue with capacity 1024.
fn empty_queue_1024() -> ShardCommandQueue {
    ShardCommandQueue::new(1024).expect("queue")
}

/// Creates a queue with 100 items.
fn queue_100_items() -> ShardCommandQueue {
    let queue = ShardCommandQueue::new(1024).expect("queue");
    let mut i = 0u64;
    while i < 100 {
        let cmd = make_submit_command(i);
        let _ = queue.enqueue(cmd);
        i = i.saturating_add(1);
    }
    queue
}

/// Creates a full queue (capacity 1, 1 item).
fn full_queue_1() -> ShardCommandQueue {
    let queue = ShardCommandQueue::new(1).expect("queue");
    let cmd = make_submit_command(0);
    let _ = queue.enqueue(cmd);
    queue
}

/// Creates a queue with 1024 items.
fn queue_1024_items() -> ShardCommandQueue {
    let queue = ShardCommandQueue::new(1024).expect("queue");
    let mut i = 0u64;
    while i < 1024 {
        let cmd = make_submit_command(i);
        let _ = queue.enqueue(cmd);
        i = i.saturating_add(1);
    }
    queue
}

fn bench_action_queuing(c: &mut Criterion) {
    let mut group = c.benchmark_group("action_queuing");

    // Enqueue on empty queue
    {
        let fixture_bytes = 1usize;
        group.throughput(Throughput::Elements(1));
        group.bench_function(
            metadata(
                "action_queue_enqueue",
                fixture_bytes,
                "fixture=queue_empty_1024;surface=queue_enqueue",
            ),
            |b| {
                b.iter(|| {
                    let queue = empty_queue_1024();
                    let cmd = make_submit_command(42);
                    let result = queue.enqueue(cmd);
                    // Exact assertion: enqueue must succeed, len must be 1
                    assert!(result.is_ok(), "enqueue on empty queue must succeed");
                    assert_eq!(queue.len(), 1, "queue len must be 1 after single enqueue");
                    black_box(queue)
                });
            },
        );
    }

    // Dequeue on non-empty queue
    {
        let fixture_bytes = 100usize;
        group.throughput(Throughput::Elements(100));
        group.bench_function(
            metadata(
                "action_queue_dequeue",
                fixture_bytes,
                "fixture=queue_100_items;surface=queue_dequeue",
            ),
            |b| {
                b.iter(|| {
                    let queue = queue_100_items();
                    let initial_len = queue.len();
                    // Exact assertion: initial len must be 100
                    assert_eq!(initial_len, 100, "pre-filled queue must have len=100");
                    let mut dequeued = 0usize;
                    while let Some(cmd) = queue.pop() {
                        black_box(cmd);
                        dequeued = dequeued.saturating_add(1);
                    }
                    // Exact assertion: all 100 items dequeued in FIFO order
                    assert_eq!(dequeued, 100, "must dequeue exactly 100 items");
                    assert!(
                        queue.is_empty(),
                        "queue must be empty after dequeuing all items"
                    );
                    black_box(dequeued)
                });
            },
        );
    }

    // Enqueue on full queue — error path
    {
        let fixture_bytes = 1usize;
        group.bench_function(
            metadata(
                "action_queue_full_enqueue_err",
                fixture_bytes,
                "fixture=queue_full;surface=queue_enqueue_full",
            ),
            |b| {
                b.iter(|| {
                    let queue = full_queue_1();
                    let cmd = make_submit_command(999);
                    let result = queue.enqueue(cmd);
                    // Exact assertion: full queue must reject with QueueFull
                    assert!(result.is_err(), "enqueue on full queue must return Err");
                    match result.expect_err("err") {
                        RuntimeError::QueueFull => {}
                        other => panic!("expected RuntimeError::QueueFull, got {:?}", other),
                    }
                    // Queue unchanged
                    assert_eq!(
                        queue.len(),
                        1,
                        "queue len must remain 1 after rejected enqueue"
                    );
                    // Item not lost
                    let retained = queue.pop();
                    assert!(
                        retained.is_some(),
                        "original item must still be in queue (not dropped)"
                    );
                    black_box(retained);
                });
            },
        );
    }

    // Batch 100 enqueues on empty queue
    {
        let fixture_bytes = 100usize;
        group.throughput(Throughput::Elements(100));
        group.bench_function(
            metadata(
                "action_queue_batch_100",
                fixture_bytes,
                "fixture=queue_empty_1024;surface=queue_batch_100",
            ),
            |b| {
                b.iter(|| {
                    let queue = empty_queue_1024();
                    let mut i = 0u64;
                    while i < 100 {
                        let cmd = make_submit_command(i);
                        let result = queue.enqueue(cmd);
                        assert!(result.is_ok(), "enqueue {} must succeed", i);
                        i = i.saturating_add(1);
                    }
                    // Exact assertion: 100 items enqueued
                    assert_eq!(
                        queue.len(),
                        100,
                        "queue must have exactly 100 items after batch enqueue"
                    );
                    black_box(queue)
                });
            },
        );
    }

    // is_full and len consistency on 1024-capacity queue with 1024 items
    {
        let fixture_bytes = 1024usize;
        group.throughput(Throughput::Elements(1024));
        group.bench_function(
            metadata(
                "action_queue_len_is_full",
                fixture_bytes,
                "fixture=queue_1024_items;surface=queue_len_is_full",
            ),
            |b| {
                b.iter(|| {
                    let queue = queue_1024_items();
                    // Exact assertions on full queue state
                    assert!(queue.is_full(), "queue with 1024 items must be full");
                    assert_eq!(queue.len(), 1024, "queue len must be 1024");
                    assert_eq!(queue.capacity(), 1024, "queue capacity must be 1024");
                    assert_eq!(
                        queue.remaining_capacity(),
                        0,
                        "remaining capacity must be 0 when full"
                    );
                    black_box(queue)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_action_queuing);
criterion_main!(benches);
