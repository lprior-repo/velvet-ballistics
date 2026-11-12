//! Paired raw SPSC and public TraceRing operations with identical events and bounds.

use criterion::{BatchSize, Criterion};
use rtrb::{Consumer, Producer, RingBuffer};
use std::hint::black_box;
use vb_core::ids::{RunId, StepIdx};
use vb_runtime::{TraceEvent, TraceRing};

const CAPACITY: usize = 128;
const DRAIN_LIMIT: usize = 16;
const FIXTURE_BATCH: BatchSize = BatchSize::NumIterations(32);

type RawRing = (Producer<TraceEvent>, Consumer<TraceEvent>);
type PushFixture<R> = (R, TraceEvent);
type DrainFixture<R> = (R, TraceEvent, Vec<TraceEvent>);
type FullRawFixture = (RawRing, TraceEvent, Result<(), rtrb::PushError<TraceEvent>>);

fn benchmark_event() -> TraceEvent {
    TraceEvent::StepStarted {
        run: RunId::new(1),
        step: StepIdx::new(1),
    }
}

fn empty_raw_fixture() -> PushFixture<RawRing> {
    (RingBuffer::<TraceEvent>::new(CAPACITY), benchmark_event())
}

fn empty_wrapper_fixture() -> PushFixture<TraceRing> {
    (TraceRing::new(CAPACITY), benchmark_event())
}

fn full_raw_fixture() -> FullRawFixture {
    let (mut producer, consumer) = RingBuffer::<TraceEvent>::new(CAPACITY);
    let event = benchmark_event();
    let expected: Result<(), rtrb::PushError<TraceEvent>> =
        Err(rtrb::PushError::Full(event.clone()));
    for _ in 0..CAPACITY {
        let result = producer.push(event.clone());
        assert_eq!(result, Ok(()), "raw prefill must succeed for every slot");
    }
    ((producer, consumer), event, expected)
}

fn full_wrapper_fixture() -> PushFixture<TraceRing> {
    let mut ring = TraceRing::new(CAPACITY);
    let event = benchmark_event();
    for _ in 0..CAPACITY {
        assert!(
            ring.push(event.clone()),
            "wrapper prefill must succeed for every slot"
        );
    }
    assert_eq!(ring.pending_len(), CAPACITY, "wrapper fixture must be full");
    (ring, event)
}

fn raw_drain_fixture() -> DrainFixture<RawRing> {
    let (mut producer, consumer) = RingBuffer::<TraceEvent>::new(CAPACITY);
    let event = benchmark_event();
    for _ in 0..DRAIN_LIMIT {
        let result = producer.push(event.clone());
        assert_eq!(result, Ok(()), "raw drain fixture prefill must succeed");
    }
    let output = Vec::with_capacity(DRAIN_LIMIT);
    ((producer, consumer), event, output)
}

fn wrapper_drain_fixture() -> DrainFixture<TraceRing> {
    let mut ring = TraceRing::new(CAPACITY);
    let event = benchmark_event();
    for _ in 0..DRAIN_LIMIT {
        assert!(
            ring.push(event.clone()),
            "wrapper drain fixture prefill must succeed"
        );
    }
    assert_eq!(
        ring.pending_len(),
        DRAIN_LIMIT,
        "wrapper fixture must contain 16 events"
    );
    let output = Vec::with_capacity(DRAIN_LIMIT);
    (ring, event, output)
}

fn bench_raw_push(c: &mut Criterion) {
    c.bench_function("trace_ring_public/raw_push_empty", |b| {
        b.iter_batched(
            empty_raw_fixture,
            |((mut producer, consumer), event)| {
                let result = producer.push(event);
                assert_eq!(result, Ok(()), "raw push into empty ring must succeed");
                black_box((result, producer, consumer))
            },
            FIXTURE_BATCH,
        )
    });
}

fn bench_wrapper_push(c: &mut Criterion) {
    c.bench_function("trace_ring_public/wrapper_push_empty", |b| {
        b.iter_batched(
            empty_wrapper_fixture,
            |(mut ring, event)| {
                let result = ring.push(event);
                assert!(result, "TraceRing push into empty ring must succeed");
                black_box((result, ring))
            },
            FIXTURE_BATCH,
        )
    });
}

fn bench_raw_overflow(c: &mut Criterion) {
    c.bench_function(
        "trace_ring_public/raw_push_full_and_verify_returned_event",
        |b| {
            b.iter_batched(
                full_raw_fixture,
                |((mut producer, consumer), event, expected)| {
                    let result = producer.push(event);
                    assert_eq!(
                        result, expected,
                        "full raw push must return the independently expected event"
                    );
                    black_box((result, expected, producer, consumer))
                },
                FIXTURE_BATCH,
            )
        },
    );
}

fn bench_wrapper_overflow(c: &mut Criterion) {
    c.bench_function(
        "trace_ring_public/wrapper_push_full_and_observe_dropped",
        |b| {
            b.iter_batched(
                full_wrapper_fixture,
                |(mut ring, event)| {
                    let pushed = ring.push(event);
                    assert!(!pushed, "TraceRing push into full ring must be rejected");
                    let dropped = ring.dropped();
                    assert_eq!(
                        dropped, 1,
                        "one overflow attempt must increment dropped count"
                    );
                    black_box((pushed, dropped, ring))
                },
                FIXTURE_BATCH,
            )
        },
    );
}

fn bench_raw_drain(c: &mut Criterion) {
    c.bench_function("trace_ring_public/raw_pop_16", |b| {
        b.iter_batched(
            raw_drain_fixture,
            |((producer, mut consumer), event, mut output)| {
                for _ in 0..DRAIN_LIMIT {
                    match consumer.pop() {
                        Ok(item) => output.push(item),
                        Err(_) => break,
                    }
                }
                assert_eq!(
                    output.len(),
                    DRAIN_LIMIT,
                    "raw fixture must drain exactly 16 events"
                );
                black_box((event, output, producer, consumer))
            },
            FIXTURE_BATCH,
        )
    });
}

fn bench_wrapper_drain(c: &mut Criterion) {
    c.bench_function("trace_ring_public/wrapper_drain_16", |b| {
        b.iter_batched(
            wrapper_drain_fixture,
            |(mut ring, event, mut output)| {
                ring.drain_into(DRAIN_LIMIT, &mut output);
                assert_eq!(
                    output.len(),
                    DRAIN_LIMIT,
                    "wrapper fixture must drain exactly 16 events"
                );
                black_box((event, output, ring))
            },
            FIXTURE_BATCH,
        )
    });
}

pub(super) fn bench_trace_ring(c: &mut Criterion) {
    bench_raw_push(c);
    bench_wrapper_push(c);
    bench_raw_overflow(c);
    bench_wrapper_overflow(c);
    bench_raw_drain(c);
    bench_wrapper_drain(c);
}
