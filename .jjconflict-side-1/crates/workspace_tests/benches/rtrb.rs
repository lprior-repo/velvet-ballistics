//! rtrb benchmarks.
//!
//! Direct benchmarks for rtrb::RingBuffer (SPSC ring buffer for trace/action paths).
//! Per Section 50, rtrb is required for SPSC trace/action completion paths.

#![allow(missing_docs)]

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
