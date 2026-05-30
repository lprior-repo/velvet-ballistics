//! Action queuing benchmarks.
//!
//! Measures ShardCommandQueue enqueue/dequeue throughput.

#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use vb_core::ids::RunId;
use vb_runtime::shard::types::{ShardCommand, ShardCommandQueue};
use vb_runtime::RuntimeError;

const BENCH_METADATA: &str =
    "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

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
        run_id: RunId::new(run_id),
        workflow: vb_core::WorkflowDigest::from_bytes([0x11; 32]),
        input: vec![1, 2, 3, 4],
    }
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
                    assert!(
                        result.is_ok(),
                        "enqueue on empty queue must succeed"
                    );
                    assert_eq!(
                        queue.len(), 1,
                        "queue len must be 1 after single enqueue"
                    );
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
                    assert_eq!(
                        initial_len, 100,
                        "pre-filled queue must have len=100"
                    );
                    let mut dequeued = 0usize;
                    while let Some(cmd) = queue.pop() {
                        black_box(cmd);
                        dequeued = dequeued.saturating_add(1);
                    }
                    // Exact assertion: all 100 items dequeued in FIFO order
                    assert_eq!(
                        dequeued, 100,
                        "must dequeue exactly 100 items"
                    );
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
                    assert!(
                        result.is_err(),
                        "enqueue on full queue must return Err"
                    );
                    match result.expect_err("err") {
                        RuntimeError::QueueFull => {}
                        other => panic!("expected RuntimeError::QueueFull, got {:?}", other),
                    }
                    // Queue unchanged
                    assert_eq!(
                        queue.len(), 1,
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
                        assert!(
                            result.is_ok(),
                            "enqueue {} must succeed",
                            i
                        );
                        i = i.saturating_add(1);
                    }
                    // Exact assertion: 100 items enqueued
                    assert_eq!(
                        queue.len(), 100,
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
                    assert!(
                        queue.is_full(),
                        "queue with 1024 items must be full"
                    );
                    assert_eq!(
                        queue.len(), 1024,
                        "queue len must be 1024"
                    );
                    assert_eq!(
                        queue.capacity(), 1024,
                        "queue capacity must be 1024"
                    );
                    assert_eq!(
                        queue.remaining_capacity(), 0,
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
