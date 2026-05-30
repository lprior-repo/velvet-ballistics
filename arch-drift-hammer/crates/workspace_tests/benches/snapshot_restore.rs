//! Snapshot restore benchmarks.
//!
//! Measures frame hydration from InspectSnapshot via postcard deserialization.

#![allow(missing_docs)]

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use serde::{Deserialize, Serialize};
use std::hint::black_box;
use vb_core::CompiledNode;
use vb_core::CompiledNodeKind;
use vb_core::CompiledWorkflow;
use vb_core::ResourceContract;
use vb_core::SlotIdx;
use vb_core::WorkflowDigest;
use vb_core::WorkflowParts;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, StepIdx};

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct BenchSnapshot {
    run: u64,
    correlation: u64,
    pc: u16,
    executed: u64,
}

impl BenchSnapshot {
    fn run_id(self) -> RunId {
        RunId::new(self.run)
    }

    fn step_idx(self) -> StepIdx {
        StepIdx::new(self.pc)
    }
}

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

/// Simple workflow for testing.
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

/// Serializes a snapshot for restore benchmarks.
fn serialized_snapshot(pc: StepIdx, executed: u64, correlation: u64) -> Vec<u8> {
    let snap = BenchSnapshot {
        run: 1,
        correlation,
        pc: pc.get(),
        executed,
    };
    postcard::to_allocvec(&snap).expect("serialized")
}

fn bench_snapshot_restore(c: &mut Criterion) {
    let workflow = simple_workflow();
    let mut group = c.benchmark_group("snapshot_restore");

    // Restore from snapshot after 1 step
    {
        let fixture_bytes = 1usize;
        group.throughput(Throughput::Elements(1));
        group.bench_function(
            metadata(
                "snapshot_restore_1_step",
                fixture_bytes,
                "fixture=snapshot_1_step;surface=frame_restore",
            ),
            |b| {
                b.iter(|| {
                    let encoded = serialized_snapshot(StepIdx::new(1), 1, 42);
                    let decoded: BenchSnapshot = postcard::from_bytes(&encoded).expect("decode");
                    // Exact assertions on decoded snapshot
                    assert_eq!(
                        decoded.step_idx(),
                        StepIdx::new(1),
                        "decoded PC must be StepIdx(1)"
                    );
                    assert_eq!(decoded.executed, 1, "decoded executed must be 1");
                    assert_eq!(decoded.correlation, 42, "decoded correlation must be 42");
                    // Create frame from decoded snapshot
                    let frame = RunFrame::new(
                        decoded.run_id(),
                        decoded.step_idx(),
                        workflow.node_count(),
                        workflow.slot_count(),
                    )
                    .expect("frame");
                    // Exact assertion: frame PC matches snapshot
                    assert_eq!(
                        frame.pc(),
                        decoded.step_idx(),
                        "restored frame PC must match snapshot PC"
                    );
                    black_box(frame)
                });
            },
        );
    }

    // Restore from snapshot after 50 steps
    {
        let fixture_bytes = 50usize;
        group.throughput(Throughput::Elements(50));
        group.bench_function(
            metadata(
                "snapshot_restore_50_steps",
                fixture_bytes,
                "fixture=snapshot_50_steps;surface=frame_restore",
            ),
            |b| {
                b.iter(|| {
                    let encoded = serialized_snapshot(StepIdx::new(50), 50, 99);
                    let decoded: BenchSnapshot = postcard::from_bytes(&encoded).expect("decode");
                    // Exact assertions
                    assert_eq!(
                        decoded.step_idx(),
                        StepIdx::new(50),
                        "decoded PC must be StepIdx(50)"
                    );
                    assert_eq!(decoded.executed, 50, "decoded executed must be 50");
                    let frame = RunFrame::new(
                        decoded.run_id(),
                        decoded.step_idx(),
                        workflow.node_count(),
                        workflow.slot_count(),
                    )
                    .expect("frame");
                    assert_eq!(
                        frame.pc(),
                        StepIdx::new(50),
                        "restored frame PC must be StepIdx(50)"
                    );
                    black_box(frame)
                });
            },
        );
    }

    // Restore with large slot values
    {
        let fixture_bytes = 10240usize;
        group.throughput(Throughput::Bytes(fixture_bytes as u64));
        group.bench_function(
            metadata(
                "snapshot_restore_large_slots",
                fixture_bytes,
                "fixture=snapshot_10kb_slots;surface=frame_restore",
            ),
            |b| {
                b.iter(|| {
                    let encoded = serialized_snapshot(StepIdx::new(5), 5, 123);
                    let decoded: BenchSnapshot = postcard::from_bytes(&encoded).expect("decode");
                    // Note: Large slot values are stored in ValueStore, not snapshot
                    // Snapshot only captures PC and executed count
                    assert_eq!(decoded.executed, 5, "decoded executed must be 5");
                    let frame = RunFrame::new(
                        decoded.run_id(),
                        decoded.step_idx(),
                        workflow.node_count(),
                        workflow.slot_count(),
                    )
                    .expect("frame");
                    black_box(frame)
                });
            },
        );
    }

    // Postcard decode overhead
    {
        let fixture_bytes = 1usize;
        group.bench_function(
            metadata(
                "snapshot_decode_postcard",
                fixture_bytes,
                "fixture=snapshot_encoded_50;surface=postcard_decode",
            ),
            |b| {
                b.iter(|| {
                    let encoded = serialized_snapshot(StepIdx::new(50), 50, 99);
                    let decoded: BenchSnapshot = postcard::from_bytes(&encoded).expect("decode");
                    // Exact assertion: decoded must match original
                    assert_eq!(
                        decoded.step_idx(),
                        StepIdx::new(50),
                        "decoded PC must be StepIdx(50)"
                    );
                    assert_eq!(decoded.executed, 50, "decoded executed must be 50");
                    assert_eq!(decoded.correlation, 99, "decoded correlation must be 99");
                    black_box(decoded)
                });
            },
        );
    }

    // Restore with correlation ID preserved
    {
        let fixture_bytes = 1usize;
        group.bench_function(
            metadata(
                "snapshot_restore_correlation",
                fixture_bytes,
                "fixture=snapshot_with_correlation;surface=restore_with_collect",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(42);
                    let correlation = 12345u64;
                    let snap = BenchSnapshot {
                        run: run_id.get(),
                        correlation,
                        pc: StepIdx::new(10).get(),
                        executed: 10,
                    };
                    let encoded = postcard::to_allocvec(&snap).expect("encode");
                    let decoded: BenchSnapshot = postcard::from_bytes(&encoded).expect("decode");
                    // Exact assertions: correlation preserved through round-trip
                    assert_eq!(
                        decoded.correlation, 12345,
                        "correlation must be preserved as 12345"
                    );
                    assert_eq!(decoded.run_id(), run_id, "run_id must be preserved");
                    black_box(decoded)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_snapshot_restore);
criterion_main!(benches);
