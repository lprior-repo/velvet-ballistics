//! vb-h6ix benchmarks: Replay Latest Execution Attempt Only
//!
//! Criterion benchmarks for the latest-attempt filtering replay logic.
//!
//! RED PHASE: These benchmarks will fail to compile until the implementation adds:
//!   1. `attempt: u16` field to JournalEvent variants
//!   2. Latest-attempt filtering logic in replay_events()

#![allow(missing_docs)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::recovery::{ActionReplayTracker, replay_events};
use vb_storage::{EventSeq, JournalEvent};

/// Benchmark metadata format.
fn metadata(name: &str, extra: &str) -> String {
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
    let workflow = test_digest(0xAB);

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
                black_box(replay_events(black_box(&events), &mut tracker))
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
    let workflow = test_digest(0xCD);

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
                black_box(replay_events(black_box(&events), &mut tracker))
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
    let workflow = test_digest(0xEF);

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
                black_box(replay_events(black_box(&events), &mut tracker))
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
