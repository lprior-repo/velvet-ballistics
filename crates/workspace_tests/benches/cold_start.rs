//! Cold start benchmarks.
//!
//! Measures time to initialize a new run from a compiled workflow.

#![allow(missing_docs)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ResourceContract,
    RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts, new_run_frame,
};

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir-and-generated;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

/// Small workflow YAML.
const SMALL_WORKFLOW_YAML: &str = r#"version: velvet-ballastics/v1
name: bench_minimal
when:
  manual: {}
steps:
  - id: save_value
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;

/// Builds a chain workflow with `count` SetConst nodes plus a Finish.
fn save_chain_workflow(count: u16) -> Option<CompiledWorkflow> {
    let mut nodes = Vec::with_capacity(usize::from(count).saturating_add(1));
    let mut step = 0_u16;
    while step < count {
        nodes.push(CompiledNode {
            id: StepIdx::new(step),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(step.saturating_add(1))),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        });
        step = step.saturating_add(1);
    }
    nodes.push(CompiledNode {
        id: StepIdx::new(count),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    });
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("bench_chain"),
        digest: WorkflowDigest::from_bytes([0x44; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(1)]),
        slot_count: 2,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
        symbols_count: 0,
    })
    .ok()
}

/// Builds a workflow with `count` steps from YAML string.
fn many_step_workflow(count: u16) -> String {
    let mut source = String::from(
        "version: velvet-ballastics/v1\nname: many_steps\nwhen:\n  manual: {}\nsteps:\n",
    );
    let mut step = 0_u16;
    while step < count {
        source.push_str("  - id: step_");
        source.push_str(&step.to_string());
        source.push_str("\n    save:\n      value: ");
        source.push_str(&step.to_string());
        source.push('\n');
        step = step.saturating_add(1);
    }
    source.push_str("  - id: done\n    finish:\n      result: 0\n");
    source
}

fn bench_cold_start(c: &mut Criterion) {
    let small_workflow = vb_compile::compile_workflow(SMALL_WORKFLOW_YAML.as_bytes());
    let chain_100 = save_chain_workflow(100);
    let chain_1000 = save_chain_workflow(1000);
    let yaml_100 = many_step_workflow(100);
    let yaml_100_bytes = yaml_100.len();

    let mut group = c.benchmark_group("cold_start");

    // Small workflow cold start
    if let Ok(ref plan) = small_workflow {
        let wf_bytes = SMALL_WORKFLOW_YAML.len();
        group.throughput(Throughput::Bytes(wf_bytes as u64));
        group.bench_function(
            metadata(
                "cold_start_small",
                wf_bytes,
                "fixture=small_workflow;surface=new_run_frame",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(1);
                    let frame = new_run_frame(run_id, black_box(plan));
                    // Exact assertion: frame must be valid with slot_count=2, entry=StepIdx(0)
                    assert!(
                        frame.is_ok(),
                        "new_run_frame must succeed for small workflow"
                    );
                    let f = frame.expect("ok");
                    assert_eq!(
                        f.slot_count(),
                        2,
                        "small workflow frame must have slot_count=2"
                    );
                    assert_eq!(f.pc(), StepIdx::new(0), "initial PC must be StepIdx(0)");
                    black_box(f)
                });
            },
        );
    }

    // 100-step chain cold start
    if let Some(ref plan) = chain_100 {
        let wf_bytes = 1024usize;
        group.throughput(Throughput::Bytes(wf_bytes as u64));
        group.bench_function(
            metadata(
                "cold_start_100_steps",
                wf_bytes,
                "fixture=save_chain_100;surface=new_run_frame",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(1);
                    let frame = new_run_frame(run_id, black_box(plan));
                    // Exact assertion: 100 SetConst + 1 Finish = 101 nodes
                    assert!(
                        frame.is_ok(),
                        "new_run_frame must succeed for 100-step chain"
                    );
                    let f = frame.expect("ok");
                    assert_eq!(
                        f.step_count(),
                        101,
                        "100-step chain frame must have step_count=101"
                    );
                    black_box(f)
                });
            },
        );
    }

    // 1000-step chain cold start
    if let Some(ref plan) = chain_1000 {
        let wf_bytes = 10240usize;
        group.throughput(Throughput::Bytes(wf_bytes as u64));
        group.bench_function(
            metadata(
                "cold_start_1000_steps",
                wf_bytes,
                "fixture=save_chain_1000;surface=new_run_frame",
            ),
            |b| {
                b.iter(|| {
                    let run_id = RunId::new(1);
                    let frame = new_run_frame(run_id, black_box(plan));
                    // Exact assertion: 1000 SetConst + 1 Finish = 1001 nodes
                    assert!(
                        frame.is_ok(),
                        "new_run_frame must succeed for 1000-step chain"
                    );
                    let f = frame.expect("ok");
                    assert_eq!(
                        f.step_count(),
                        1001,
                        "1000-step chain frame must have step_count=1001"
                    );
                    black_box(f)
                });
            },
        );
    }

    // Full pipeline: YAML parse → compile → new_run_frame
    {
        group.throughput(Throughput::Bytes(yaml_100_bytes as u64));
        group.bench_function(
            metadata(
                "cold_start_full_pipeline",
                yaml_100_bytes,
                "fixture=small_workflow_yaml;surface=parse_compile_frame",
            ),
            |b| {
                b.iter(|| {
                    // Full cold-start pipeline
                    let compiled = vb_compile::compile_workflow(black_box(yaml_100.as_bytes()));
                    if let Ok(ref plan) = compiled.as_ref() {
                        let run_id = RunId::new(1);
                        let frame = new_run_frame(run_id, black_box(plan));
                        assert!(frame.is_ok(), "full pipeline must produce valid frame");
                        black_box(frame);
                    } else {
                        black_box(None::<vb_core::frame::RunFrame>);
                    }
                });
            },
        );
    }

    // 10 concurrent runs (single-threaded sequential simulation)
    if let Ok(ref plan) = small_workflow {
        let wf_bytes = SMALL_WORKFLOW_YAML.len();
        group.bench_function(
            metadata(
                "cold_start_10_sequential",
                wf_bytes,
                "fixture=small_workflow;surface=new_run_frame_sequential",
            ),
            |b| {
                b.iter(|| {
                    // Sequential simulation of concurrent cold starts
                    // Each run creates a fresh frame
                    let mut frames = Vec::with_capacity(10);
                    let mut i = 0u64;
                    while i < 10 {
                        let run_id = RunId::new(i);
                        let frame = new_run_frame(run_id, black_box(plan));
                        assert!(frame.is_ok(), "sequential run {} must succeed", i);
                        frames.push(frame.expect("ok"));
                        i = i.saturating_add(1);
                    }
                    // Exact assertion: 10 frames created
                    assert_eq!(frames.len(), 10, "must create exactly 10 frames");
                    black_box(frames)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_cold_start);
criterion_main!(benches);
