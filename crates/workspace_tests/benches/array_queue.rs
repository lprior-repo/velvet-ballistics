//! ArrayQueue benchmarks.
//!
//! Direct benchmarks for crossbeam_queue::ArrayQueue operations.
//! Per Section 50, ArrayQueue is the mandated backend for ShardCommandQueue.

#![allow(missing_docs)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use crossbeam_queue::ArrayQueue;
use std::hint::black_box;

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir-and-generated;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

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
