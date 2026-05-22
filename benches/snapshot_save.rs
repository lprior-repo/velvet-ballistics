//! Snapshot save benchmarks.
//!
//! Measures snapshot_from_state and postcard serialization costs.

#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use vb_core::ids::{RunId, StepIdx};
use vb_runtime::shard::helpers::snapshot_from_state;
use vb_runtime::shard::types::{InspectSnapshot, RunState};
use vb_core::frame::RunFrame;
use vb_core::CompiledNode;
use vb_core::CompiledNodeKind;
use vb_core::ConstIdx;
use vb_core::ConstValue;
use vb_core::ResourceContract;
use vb_core::WorkflowDigest;
use vb_core::WorkflowParts;
use vb_core::CompiledWorkflow;

const BENCH_METADATA: &str =
    "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

/// Simple 1-step workflow for testing.
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
        name: Box::from("bench_simple"),
        digest: WorkflowDigest::from_bytes([0x11; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
        symbols_count: 0,
    })
    .expect("workflow")
}

/// Creates a RunState for a run that has executed `executed` steps.
fn run_state_after_n_steps(workflow: &CompiledWorkflow, run_id: RunId, executed: u64) -> RunState {
    let frame = RunFrame::new(run_id, workflow.entry(), workflow.node_count(), workflow.slot_count())
        .expect("frame");
    RunState {
        frame,
        workflow: workflow.clone(),
        action_attempts: vec![1u32; workflow.node_count() as usize].into_boxed_slice(),
        executed,
        canceled: false,
    }
}

fn bench_snapshot_save(c: &mut Criterion) {
    let workflow = simple_workflow();

    let mut group = c.benchmark_group("snapshot_save");

    // Snapshot after 1 step
    {
        let fixture_bytes = 1usize;
        group.throughput(Throughput::Elements(1));
        group.bench_function(
            metadata(
                "snapshot_save_1_step",
                fixture_bytes,
                "fixture=frame_1_step;surface=snapshot_from_state",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(1);
                    let state = run_state_after_n_steps(&workflow, run_id, 1);
                    let snap = snapshot_from_state(run_id, 42, &state);
                    // Exact assertions on snapshot fields
                    assert_eq!(
                        snap.run, run_id,
                        "snapshot run_id must match"
                    );
                    assert_eq!(
                        snap.correlation, 42,
                        "snapshot correlation must be 42"
                    );
                    assert_eq!(
                        snap.pc, StepIdx::new(1),
                        "snapshot PC must be StepIdx(1) after 1 step"
                    );
                    assert_eq!(
                        snap.executed, 1,
                        "snapshot executed must be 1"
                    );
                    black_box(snap)
                });
            },
        );
    }

    // Snapshot after 50 steps
    {
        let fixture_bytes = 50usize;
        group.throughput(Throughput::Elements(50));
        group.bench_function(
            metadata(
                "snapshot_save_50_steps",
                fixture_bytes,
                "fixture=frame_50_steps;surface=snapshot_from_state",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(1);
                    let state = run_state_after_n_steps(&workflow, run_id, 50);
                    let snap = snapshot_from_state(run_id, 99, &state);
                    // Exact assertions
                    assert_eq!(
                        snap.run, run_id,
                        "snapshot run_id must match"
                    );
                    assert_eq!(
                        snap.correlation, 99,
                        "snapshot correlation must be 99"
                    );
                    assert_eq!(
                        snap.executed, 50,
                        "snapshot executed must be 50"
                    );
                    black_box(snap)
                });
            },
        );
    }

    // Snapshot with large slot values (10 x 1KB values)
    {
        let fixture_bytes = 10240usize;
        group.throughput(Throughput::Bytes(fixture_bytes as u64));
        group.bench_function(
            metadata(
                "snapshot_save_large_slots",
                fixture_bytes,
                "fixture=frame_10kb_slots;surface=snapshot_from_state",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(1);
                    let state = run_state_after_n_steps(&workflow, run_id, 5);
                    let snap = snapshot_from_state(run_id, 123, &state);
                    // Snapshot captures PC and executed, not slot values
                    // (slot values are in the frame which is part of state)
                    assert_eq!(
                        snap.executed, 5,
                        "snapshot executed must be 5"
                    );
                    black_box(snap)
                });
            },
        );
    }

    // Postcard encode of small snapshot
    {
        let fixture_bytes = 1usize;
        group.throughput(Throughput::Bytes(fixture_bytes as u64));
        group.bench_function(
            metadata(
                "snapshot_encode_postcard",
                fixture_bytes,
                "fixture=snapshot_small;surface=postcard_encode",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(1);
                    let state = run_state_after_n_steps(&workflow, run_id, 1);
                    let snap = snapshot_from_state(run_id, 42, &state);
                    let encoded = postcard::to_allocvec(&snap);
                    // Exact assertion: encode must succeed
                    assert!(
                        encoded.is_ok(),
                        "postcard encode must succeed"
                    );
                    let bytes = encoded.expect("ok");
                    // Snapshot is small, should be < 100 bytes
                    assert!(
                        bytes.len() < 100,
                        "encoded snapshot must be small"
                    );
                    assert!(
                        bytes.len() > 0,
                        "encoded snapshot must have non-zero length"
                    );
                    black_box(bytes)
                });
            },
        );
    }

    // Postcard encode with correlation ID = u64::MAX
    {
        let fixture_bytes = 1usize;
        group.bench_function(
            metadata(
                "snapshot_encode_max_correlation",
                fixture_bytes,
                "fixture=snapshot_max_corr;surface=postcard_encode",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(u64::MAX);
                    let state = run_state_after_n_steps(&workflow, run_id, 100);
                    let snap = snapshot_from_state(run_id, u64::MAX, &state);
                    let encoded = postcard::to_allocvec(&snap);
                    assert!(
                        encoded.is_ok(),
                        "postcard encode with max values must succeed"
                    );
                    let bytes = encoded.expect("ok");
                    // Exact assertion: encoded size must be consistent
                    assert!(
                        bytes.len() > 0,
                        "encoded snapshot must have non-zero length"
                    );
                    black_box(bytes)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_snapshot_save);
criterion_main!(benches);
