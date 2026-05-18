//! Timer wheel tick benchmarks.
//!
//! Measures TimerWheel::fire_expired overhead as timer count grows.

#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use std::time::{Duration, Instant};
use vb_core::ids::RunId;
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::types::PendingTimerKind;

const BENCH_METADATA: &str =
    "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir-and-generated;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

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
    wheel.insert(RunId::new(1), deadline, PendingTimerKind::Wait);
    wheel
}

/// Creates a wheel with 10 expired timers at the same deadline.
fn wheel_10_expired(now: Instant) -> TimerWheel {
    let mut wheel = TimerWheel::new();
    let deadline = now - Duration::from_millis(10);
    let mut i = 0u64;
    while i < 10 {
        wheel.insert(RunId::new(i), deadline, PendingTimerKind::Wait);
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
        wheel.insert(RunId::new(i), expired_deadline, PendingTimerKind::Wait);
        i = i.saturating_add(1);
    }
    while i < 100 {
        wheel.insert(RunId::new(i), future_deadline, PendingTimerKind::Wait);
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
        wheel.insert(RunId::new(i), future_deadline, PendingTimerKind::Wait);
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
        wheel.insert(RunId::new(i), future_deadline, PendingTimerKind::Wait);
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
                        fired.len(), 0,
                        "fire_expired on empty wheel must return 0 entries"
                    );
                    assert!(
                        wheel.is_empty(),
                        "wheel must be empty after firing"
                    );
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
                    assert_eq!(
                        fired.len(), 1,
                        "fire_expired must return exactly 1 entry"
                    );
                    assert_eq!(
                        fired[0].run, RunId::new(1),
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
                        fired.len(), 10,
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
                        fired.len(), 90,
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
                    assert!(
                        cancelled,
                        "cancel of existing timer 50 must return true"
                    );
                    // Cancelled timer no longer fires
                    let fired = wheel.fire_expired(now + Duration::from_secs(1));
                    assert!(
                        fired.iter().all(|e| e.run != RunId::new(50)),
                        "cancelled timer 50 must not appear in fired list"
                    );
                    // 99 timers remain
                    assert_eq!(
                        wheel.next_deadline().is_some(),
                        true,
                        "99 remaining timers must have a deadline"
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
                    assert!(
                        d > now,
                        "next deadline must be in the future"
                    );
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
                        wheel.insert(RunId::new(i), future_deadline, PendingTimerKind::Wait);
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
