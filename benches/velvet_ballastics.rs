//! Fixture-backed benchmark suite with explicit metadata in benchmark IDs.

#![allow(missing_docs)]

use bytes::Bytes;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ExprIdx, ResourceContract,
    RunId, SlotBranch, SlotIdx, StepBudget, StepIdx, WorkflowDigest, WorkflowParts,
};
use vb_storage::{EventSeq, JournalEvent};

struct GeneratedBinary {
    path: PathBuf,
    _temp_dir: PathBuf,
}

impl GeneratedBinary {
    fn compile(workflow: &CompiledWorkflow, name: &str) -> Option<Self> {
        let generated = vb_codegen::emit_rust_workflow(workflow).ok()?;
        let temp_dir = std::env::temp_dir().join(format!(
            "vb_bench_gen_{}_{}",
            std::process::id(),
            name
        ));
        std::fs::create_dir_all(&temp_dir).ok()?;
        let source_path = temp_dir.join("generated.rs");
        let binary_path = temp_dir.join("generated_bin");
        let harness = format!(
            "{}\nfn main() {{\n    let mut slots = [None; WORKFLOW_SLOT_COUNT];\n    match drive(slots) {{\n        Ok(value) => println!(\"ok:{{value:#?}}\"),\n        Err(e) => println!(\"err:{{e:#?}}\"),\n    }}\n}}\n",
            generated
        );
        std::fs::write(&source_path, harness).ok()?;
        let output = Command::new("rustc")
            .arg("--edition")
            .arg("2024")
            .arg("-Copt-level=3")
            .arg("-o")
            .arg(&binary_path)
            .arg(&source_path)
            .output()
            .ok()?;
        if !output.status.success() {
            eprintln!("rustc failed: {}", String::from_utf8_lossy(&output.stderr));
            return None;
        }
        Some(Self {
            path: binary_path,
            _temp_dir: temp_dir,
        })
    }

    fn run(&self) -> std::process::Output {
        match Command::new(&self.path).output() {
            Ok(output) => output,
            Err(e) => {
                eprintln!("generated binary failed: {e}");
                std::process::Output {
                    status: std::process::ExitStatus::default(),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                }
            }
        }
    }
}

const SMALL_WORKFLOW: &[u8] = b"version: velvet-ballastics/v1\nname: bench_minimal\nwhen:\n  manual: {}\nsteps:\n  - id: save_value\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n";
const CHOOSE_WORKFLOW: &[u8] = b"version: velvet-ballastics/v1\nname: bench_choose\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose:\n      condition: true\n      on_true: 1\n      on_false: 1\n  - id: done\n    finish:\n      result: true\n";
const EXPR_EQ_SYMBOL: &str = "$input.value == 7";
const EXPR_NUMBER_COMPARE: &str = "7 > 3";
const EXPR_BOOLEAN_CHAIN: &str = "true && false || true";
const EXPR_ARITHMETIC: &str = "1 + 2 * 3";
const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir-and-generated;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";
const JOURNAL_REPLAY_EVENTS: u64 = 1000;

fn bytes_len(bytes: &[u8]) -> u64 {
    u64::try_from(bytes.len()).unwrap_or(u64::MAX)
}

fn parse_yaml_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("yaml_parse");
    let small_meta = metadata("parse_yaml_small", SMALL_WORKFLOW, "fixture=small_workflow");
    group.throughput(Throughput::Bytes(bytes_len(SMALL_WORKFLOW)));
    group.bench_with_input(
        BenchmarkId::from_parameter(small_meta),
        SMALL_WORKFLOW,
        |b, input| {
            b.iter(|| match std::str::from_utf8(input) {
                Ok(text) => vb_yaml::parse_yaml_events(black_box(text)),
                Err(error) => Err(vb_yaml::YamlError::ParseError {
                    line: 0,
                    reason: error.to_string().into_boxed_str(),
                }),
            })
        },
    );

    let one_mb = one_mb_workflow();
    let large_meta = metadata(
        "parse_yaml_1mb",
        one_mb.as_bytes(),
        "fixture=generated_1mb_yaml",
    );
    group.throughput(Throughput::Bytes(bytes_len(one_mb.as_bytes())));
    group.bench_with_input(
        BenchmarkId::from_parameter(large_meta),
        &one_mb,
        |b, input| b.iter(|| vb_yaml::parse_yaml_events(black_box(input.as_str()))),
    );
    group.finish();
}

fn compile_and_validate_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("compile_validate");
    group.throughput(Throughput::Bytes(bytes_len(SMALL_WORKFLOW)));
    group.bench_function(
        metadata(
            "validate_minimal",
            SMALL_WORKFLOW,
            "fixture=small_workflow;surface=validator",
        ),
        |b| {
            b.iter(|| {
                let compiled = vb_compile::compile_workflow(black_box(SMALL_WORKFLOW));
                if let Ok(workflow) = compiled.as_ref() {
                    let parts = workflow.to_parts();
                    let _validated = vb_core::validate_compiled_workflow(&parts);
                }
                compiled
            })
        },
    );
    group.bench_function(
        metadata(
            "compile_ir_minimal",
            SMALL_WORKFLOW,
            "fixture=small_workflow;surface=compiler",
        ),
        |b| b.iter(|| vb_compile::compile_workflow(black_box(SMALL_WORKFLOW))),
    );

    let many_steps = many_step_workflow(1000);
    group.throughput(Throughput::Bytes(bytes_len(many_steps.as_bytes())));
    group.bench_function(
        metadata(
            "compile_ir_1000_steps",
            many_steps.as_bytes(),
            "fixture=generated_1000_steps;surface=compiler",
        ),
        |b| b.iter(|| vb_compile::compile_workflow(black_box(many_steps.as_bytes()))),
    );
    group.bench_function(
        metadata(
            "validate_1000_steps",
            many_steps.as_bytes(),
            "fixture=generated_1000_steps;surface=validator",
        ),
        |b| {
            b.iter(|| {
                let compiled = vb_compile::compile_workflow(black_box(many_steps.as_bytes()));
                if let Ok(workflow) = compiled.as_ref() {
                    let parts = workflow.to_parts();
                    let _validated = vb_core::validate_compiled_workflow(&parts);
                }
                compiled
            })
        },
    );
    group.finish();
}

fn expression_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("expression");
    bench_expr(&mut group, "expr_eq_symbol", EXPR_EQ_SYMBOL);
    bench_expr(&mut group, "expr_number_compare", EXPR_NUMBER_COMPARE);
    bench_expr(&mut group, "expr_boolean_chain", EXPR_BOOLEAN_CHAIN);
    bench_expr(&mut group, "expr_arithmetic", EXPR_ARITHMETIC);
    group.finish();
}

fn slot_and_transition_benches(c: &mut Criterion) {
    let workflow = vb_compile::compile_workflow(SMALL_WORKFLOW);
    let save_chain_10 = save_chain_workflow(10);
    let save_chain_1000 = save_chain_workflow(1000);
    let choose_true = choose_slot_workflow(true);
    let choose_false = choose_slot_workflow(false);
    let finish_only = finish_workflow();
    let mut group = c.benchmark_group("runtime_core");
    group.bench_function(
        metadata(
            "bench_engine_numeric_slots_read_write_i64",
            SMALL_WORKFLOW,
            "fixture=run_frame_slot;surface=slot_i64_rw",
        ),
        |b| {
            b.iter(|| {
                let mut frame = vb_core::RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
                if let Ok(run) = frame.as_mut() {
                    let _written = run.write_slot(SlotIdx::new(0), vb_core::SlotValue::I64(7));
                    let _read = run.read_slot(black_box(SlotIdx::new(0)));
                }
                frame
            })
        },
    );
    group.bench_function(
        metadata("slot_read", SMALL_WORKFLOW, "fixture=run_frame_slot"),
        |b| {
            b.iter(|| {
                let mut frame = vb_core::RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
                if let Ok(run) = frame.as_mut() {
                    let _written = run.write_slot(SlotIdx::new(0), vb_core::SlotValue::I64(7));
                    let _read = run.read_slot(black_box(SlotIdx::new(0)));
                }
                frame
            })
        },
    );
    group.bench_function(
        metadata(
            "bench_engine_step_once_save_const_single_transition",
            SMALL_WORKFLOW,
            "fixture=small_workflow;surface=engine_step",
        ),
        |b| {
            b.iter(|| {
                if let Ok(plan) = workflow.as_ref() {
                    let mut frame = vb_core::new_run_frame(RunId::new(2), plan);
                    let mut store = vb_core::ValueStore::new();
                    if let Ok(run) = frame.as_mut() {
                        let _signal = vb_core::step_once(black_box(plan), run, &mut store);
                    }
                    Some(frame)
                } else {
                    None
                }
            })
        },
    );
    group.bench_function(
        metadata(
            "engine_run_until_blocked_budget_10_small_workflow",
            SMALL_WORKFLOW,
            "fixture=small_workflow;surface=engine_run",
        ),
        |b| {
            b.iter(|| {
                if let Ok(plan) = workflow.as_ref() {
                    let mut frame = vb_core::new_run_frame(RunId::new(3), plan);
                    let mut store = vb_core::ValueStore::new();
                    if let Ok(run) = frame.as_mut() {
                        let _signal = vb_core::run_until_blocked(
                            black_box(plan),
                            run,
                            StepBudget::new(10),
                            &mut store,
                        );
                    }
                    Some(frame)
                } else {
                    None
                }
            })
        },
    );
    bench_run_workflow(
        &mut group,
        "bench_engine_run_save_chain_10_steps",
        &save_chain_10,
        11,
        "fixture=ir_save_chain_10;surface=engine_run",
    );
    bench_run_workflow(
        &mut group,
        "bench_engine_run_save_chain_1000_steps",
        &save_chain_1000,
        1001,
        "fixture=ir_save_chain_1000;surface=engine_run",
    );
    bench_run_workflow(
        &mut group,
        "bench_engine_choose_true_branch",
        &choose_true,
        5,
        "fixture=ir_choose_slot_true;surface=engine_choose",
    );
    bench_run_workflow(
        &mut group,
        "bench_engine_choose_false_branch",
        &choose_false,
        5,
        "fixture=ir_choose_slot_false;surface=engine_choose",
    );
    bench_run_workflow(
        &mut group,
        "bench_engine_finish_no_observability",
        &finish_only,
        1,
        "fixture=ir_finish_only;surface=engine_finish",
    );
    group.finish();
}

fn storage_and_ipc_benches(c: &mut Criterion) {
    let event = bench_event(4, 0);
    let encoded_event = vb_storage::encode_record(
        vb_storage::MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    let payload = vb_ipc::IpcPayload::SubmitRun(vb_ipc::SubmitRunPayload {
        run_id: RunId::new(5),
        workflow: WorkflowDigest::from_bytes([0x22; 32]),
        input: vec![1, 2, 3, 4],
    });
    let max_payload = vb_ipc::MaxPayloadBytes::DEFAULT;
    let encoded_payload = vb_ipc::encode_payload(&payload, max_payload);
    let journal_dir = tempfile::tempdir();
    let journal = match journal_dir.as_ref() {
        Ok(dir) => match vb_storage::FjallJournal::open(dir.path(), None) {
            Ok(journal) => Some(journal),
            Err(error) => {
                eprintln!("journal bench disabled: {error}");
                None
            }
        },
        Err(error) => {
            eprintln!("journal bench tempdir unavailable: {error}");
            None
        }
    };
    let replay_dir = tempfile::tempdir();
    let replay_journal = match replay_dir.as_ref() {
        Ok(dir) => match vb_storage::FjallJournal::open(dir.path(), None) {
            Ok(journal) => Some(journal),
            Err(error) => {
                eprintln!("journal replay bench disabled: {error}");
                None
            }
        },
        Err(error) => {
            eprintln!("journal replay bench tempdir unavailable: {error}");
            None
        }
    };
    if let Some(journal) = replay_journal.as_ref() {
        let _seeded = seed_journal(journal, RunId::new(43), JOURNAL_REPLAY_EVENTS);
    }
    let frame_bytes = match encoded_payload.as_ref() {
        Ok(bytes) => {
            vb_ipc::frame::encode_frame(vb_ipc::IpcCommand::SubmitRun, 0, 7, bytes.bytes()).ok()
        }
        Err(_) => None,
    };
    let ingress_frame = sample_ingress_frame();

    let mut group = c.benchmark_group("storage_ipc");
    group.bench_function(
        metadata(
            "bench_memory_ingress_try_submit_capacity_1024",
            SMALL_WORKFLOW,
            "fixture=memory_ingress_1024;surface=ipc_memory;durability=memory",
        ),
        |b| {
            b.iter(|| {
                if let Some(frame) = ingress_frame.as_ref() {
                    let capacity = queue_capacity(1024);
                    let queue = vb_ipc::MemoryIngress::bounded(capacity);
                    let mut submitted = 0_u16;
                    while submitted < 1024 {
                        let _sent = queue.try_submit(black_box(frame.clone()));
                        submitted = submitted.saturating_add(1);
                    }
                    queue.len()
                } else {
                    0
                }
            })
        },
    );
    group.bench_function(
        metadata(
            "bench_memory_ingress_submit_recv_single_thread",
            SMALL_WORKFLOW,
            "fixture=memory_ingress_pair;surface=ipc_memory;durability=memory",
        ),
        |b| {
            let queue = vb_ipc::MemoryIngress::bounded(queue_capacity(1024));
            b.iter(|| {
                if let Some(frame) = ingress_frame.as_ref() {
                    let _sent = queue.try_submit(black_box(frame.clone()));
                    queue.try_recv()
                } else {
                    Ok(None)
                }
            })
        },
    );
    group.bench_function(
        metadata(
            "bench_memory_ingress_backpressure_full_queue",
            SMALL_WORKFLOW,
            "fixture=memory_ingress_full;surface=ipc_memory;durability=memory",
        ),
        |b| {
            let queue = vb_ipc::MemoryIngress::bounded(queue_capacity(1));
            if let Some(frame) = ingress_frame.as_ref() {
                let _prefill = queue.try_submit(frame.clone());
            }
            b.iter(|| {
                if let Some(frame) = ingress_frame.as_ref() {
                    queue.try_submit(black_box(frame.clone()))
                } else {
                    Err(vb_ipc::IpcError::Disconnected)
                }
            })
        },
    );
    group.bench_function(
        metadata(
            "postcard_encode_event",
            SMALL_WORKFLOW,
            "fixture=run_accepted_event;surface=journal_encode",
        ),
        |b| b.iter(|| postcard::to_allocvec(black_box(&event))),
    );
    group.bench_function(
        metadata(
            "postcard_decode_event",
            SMALL_WORKFLOW,
            "fixture=run_accepted_event;surface=journal_decode",
        ),
        |b| {
            b.iter(|| {
                if let Ok(bytes) = encoded_event.as_ref() {
                    let decoded: Result<(vb_storage::RecordEnvelope, JournalEvent), _> =
                        vb_storage::decode_record(
                            black_box(bytes.as_slice()),
                            vb_storage::MAGIC_JOURNAL_EVENT,
                            vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
                        );
                    Some(decoded)
                } else {
                    None
                }
            })
        },
    );
    group.bench_function(
        metadata(
            "ipc_frame_encode",
            SMALL_WORKFLOW,
            "fixture=submit_run_payload;surface=ipc_encode",
        ),
        |b| {
            b.iter(|| {
                if let Ok(bytes) = encoded_payload.as_ref() {
                    Some(vb_ipc::frame::encode_frame(
                        vb_ipc::IpcCommand::SubmitRun,
                        0,
                        7,
                        black_box(bytes.bytes()),
                    ))
                } else {
                    None
                }
            })
        },
    );
    group.bench_function(
        metadata(
            "ipc_frame_decode",
            SMALL_WORKFLOW,
            "fixture=submit_run_payload;surface=ipc_decode",
        ),
        |b| {
            b.iter(|| {
                if let Some(frame) = frame_bytes.as_ref() {
                    decode_ipc_frame(black_box(frame.as_slice()))
                } else {
                    Err(vb_ipc::IpcError::HeaderDecodeFailed)
                }
            })
        },
    );
    group.bench_function(
        metadata(
            "bench_fjall_append_run_accepted_no_persist",
            SMALL_WORKFLOW,
            "fixture=fjall_run_events;surface=journal_append;durability=journaled",
        ),
        |b| {
            let mut seq = 0_u64;
            b.iter(|| {
                if let Some(journal) = journal.as_ref() {
                    let event = bench_event(42, seq);
                    seq = seq.saturating_add(1);
                    journal.append_journaled(black_box(&event))
                } else {
                    Err(vb_storage::JournalError::KeyCapacity)
                }
            })
        },
    );
    group.bench_function(
        metadata(
            "bench_replay_ordered_journal_1000_events",
            SMALL_WORKFLOW,
            "fixture=fjall_run_events_1000;surface=journal_replay;durability=journaled",
        ),
        |b| {
            b.iter(|| {
                if let Some(journal) = replay_journal.as_ref() {
                    journal.events_for_run(black_box(RunId::new(43)))
                } else {
                    Err(vb_storage::JournalError::KeyCapacity)
                }
            })
        },
    );
    group.finish();
}

fn bench_run_workflow(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    workflow: &Option<CompiledWorkflow>,
    budget: u64,
    extra: &str,
) {
    group.bench_function(metadata(name, name.as_bytes(), extra), |b| {
        b.iter(|| {
            if let Some(plan) = workflow.as_ref() {
                let mut frame = vb_core::new_run_frame(RunId::new(6), plan);
                let mut store = vb_core::ValueStore::new();
                if let Ok(run) = frame.as_mut() {
                    let _signal = vb_core::run_until_blocked(
                        black_box(plan),
                        run,
                        StepBudget::new(budget),
                        &mut store,
                    );
                }
                Some(frame)
            } else {
                None
            }
        })
    });
}

fn save_chain_workflow(count: u16) -> Option<CompiledWorkflow> {
    let mut nodes = Vec::with_capacity(usize::from(count).saturating_add(1));
    let mut step = 0_u16;
    while step < count {
        nodes.push(CompiledNode {
            id: StepIdx::new(step),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(step.saturating_add(1))),
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
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    });
    compiled_from_nodes(
        "bench_save_chain",
        nodes,
        Box::from([vb_core::ConstValue::I64(1)]),
    )
}

fn choose_slot_workflow(condition: bool) -> Option<CompiledWorkflow> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            kind: CompiledNodeKind::ChooseSlot {
                branches: Box::from([SlotBranch {
                    condition: SlotIdx::new(0),
                    target: StepIdx::new(2),
                }]),
                otherwise: Some(StepIdx::new(3)),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(4)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(4)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(1),
            },
        },
    ];
    compiled_from_nodes(
        "bench_choose_slot",
        nodes,
        Box::from([
            vb_core::ConstValue::Bool(condition),
            vb_core::ConstValue::Bool(true),
            vb_core::ConstValue::Bool(false),
        ]),
    )
}

fn finish_workflow() -> Option<CompiledWorkflow> {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];
    compiled_from_nodes("bench_finish_only", nodes, Box::from([]))
}

fn choose_100_workflow() -> Option<CompiledWorkflow> {
    let mut nodes = Vec::with_capacity(103);
    nodes.push(CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    });
    let mut branches = Vec::with_capacity(100);
    for i in 0..100 {
        let target = if i == 0 { 101 } else { 102 };
        branches.push(SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(target),
        });
    }
    nodes.push(CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        kind: CompiledNodeKind::ChooseSlot {
            branches: branches.into_boxed_slice(),
            otherwise: Some(StepIdx::new(102)),
        },
    });
    nodes.push(CompiledNode {
        id: StepIdx::new(102),
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(103)),
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(1),
        },
    });
    nodes.push(CompiledNode {
        id: StepIdx::new(103),
        output: None,
        next: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    });
    let constants = vec![
        vb_core::ConstValue::Bool(true),
        vb_core::ConstValue::I64(42),
    ];
    compiled_from_nodes(
        "bench_choose_100",
        nodes,
        constants.into_boxed_slice(),
    )
}

fn expression_workflow() -> Option<CompiledWorkflow> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            kind: CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(1),
            },
        },
    ];
    let constants = vec![
        vb_core::ConstValue::I64(10),
        vb_core::ConstValue::I64(3),
        vb_core::ConstValue::I64(7),
    ];
    compiled_from_nodes(
        "bench_expr",
        nodes,
        constants.into_boxed_slice(),
    )
}

fn compiled_from_nodes(
    name: &str,
    nodes: Vec<CompiledNode>,
    constants: Box<[vb_core::ConstValue]>,
) -> Option<CompiledWorkflow> {
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from(name),
        digest: WorkflowDigest::from_bytes([0x33; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants,
        slot_count: 2,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
    })
    .ok()
}

fn queue_capacity(value: usize) -> vb_ipc::QueueCapacity {
    let capacity = match NonZeroUsize::new(value) {
        Some(value) => value,
        None => NonZeroUsize::MIN,
    };
    vb_ipc::QueueCapacity::new(capacity)
}

fn sample_ingress_frame() -> Option<vb_ipc::IngressFrame> {
    vb_ipc::IngressFrame::new(
        RunId::new(9),
        WorkflowDigest::from_bytes([0x44; 32]),
        Bytes::from_static(b"bench-input"),
        vb_ipc::MaxPayloadBytes::DEFAULT,
    )
    .ok()
}

fn generated_benches(c: &mut Criterion) {
    let workflow = vb_compile::compile_workflow(CHOOSE_WORKFLOW);
    let generated_source = match workflow.as_ref() {
        Ok(plan) => vb_codegen::emit_rust_workflow(plan).ok(),
        Err(_) => None,
    };
    let mut group = c.benchmark_group("generated_mode");
    group.bench_function(
        metadata(
            "codegen_emit_choose_workflow",
            CHOOSE_WORKFLOW,
            "fixture=choose_workflow;surface=codegen_emit",
        ),
        |b| {
            b.iter(|| {
                if let Ok(plan) = workflow.as_ref() {
                    Some(vb_codegen::emit_rust_workflow(black_box(plan)))
                } else {
                    None
                }
            })
        },
    );
    group.bench_function(
        metadata(
            "codegen_compare_generated_to_ir_choose",
            CHOOSE_WORKFLOW,
            "fixture=choose_workflow;surface=codegen_compare",
        ),
        |b| {
            b.iter(|| match (workflow.as_ref(), generated_source.as_ref()) {
                (Ok(plan), Some(source)) => Some(vb_codegen::compare_generated_to_ir(
                    black_box(source.as_str()),
                    black_box(plan),
                )),
                _ => None,
            })
        },
    );
    group.finish();
}

fn ir_vs_generated_benches(c: &mut Criterion) {
    let finish_1_workflow = finish_workflow();
    let save_chain_1000 = save_chain_workflow(1000);
    let choose_100_workflow = choose_100_workflow();
    let expr_workflow = expression_workflow();

    let gen_finish = finish_1_workflow
        .as_ref()
        .and_then(|w| GeneratedBinary::compile(w, "finish_1"));
    let gen_chain_1000 = save_chain_1000
        .as_ref()
        .and_then(|w| GeneratedBinary::compile(w, "save_chain_1000"));
    let gen_choose_100 = choose_100_workflow
        .as_ref()
        .and_then(|w| GeneratedBinary::compile(w, "choose_100"));
    let gen_expr = expr_workflow
        .as_ref()
        .and_then(|w| GeneratedBinary::compile(w, "expr"));

    let mut ir_group = c.benchmark_group("ir_vs_generated");
    ir_group.measurement_time(std::time::Duration::from_secs(5));
    ir_group.sample_size(100);

    ir_group.bench_function(
        metadata(
            "ir_execution_1_step",
            b"finish_1",
            "fixture=finish_1;surface=ir_exec",
        ),
        |b| {
            b.iter(|| {
                if let Some(plan) = finish_1_workflow.as_ref() {
                    let mut frame = vb_core::new_run_frame(RunId::new(100), plan);
                    let mut store = vb_core::ValueStore::new();
                    black_box(if let Ok(run) = frame.as_mut() {
                        Some(vb_core::run_until_blocked(plan, run, StepBudget::MAX, &mut store))
                    } else {
                        None
                    })
                } else {
                    None
                }
            })
        },
    );

    ir_group.bench_function(
        metadata(
            "ir_execution_1000_steps",
            b"save_chain_1000",
            "fixture=save_chain_1000;surface=ir_exec",
        ),
        |b| {
            b.iter(|| {
                if let Some(plan) = save_chain_1000.as_ref() {
                    let mut frame = vb_core::new_run_frame(RunId::new(101), plan);
                    let mut store = vb_core::ValueStore::new();
                    black_box(if let Ok(run) = frame.as_mut() {
                        Some(vb_core::run_until_blocked(plan, run, StepBudget::MAX, &mut store))
                    } else {
                        None
                    })
                } else {
                    None
                }
            })
        },
    );

    ir_group.bench_function(
        metadata(
            "ir_execution_choose_100",
            b"choose_100",
            "fixture=choose_100;surface=ir_exec",
        ),
        |b| {
            b.iter(|| {
                if let Some(plan) = choose_100_workflow.as_ref() {
                    let mut frame = vb_core::new_run_frame(RunId::new(102), plan);
                    let mut store = vb_core::ValueStore::new();
                    black_box(if let Ok(run) = frame.as_mut() {
                        Some(vb_core::run_until_blocked(plan, run, StepBudget::MAX, &mut store))
                    } else {
                        None
                    })
                } else {
                    None
                }
            })
        },
    );

    ir_group.bench_function(
        metadata(
            "ir_execution_expr",
            b"expression",
            "fixture=expression_workflow;surface=ir_exec",
        ),
        |b| {
            b.iter(|| {
                if let Some(plan) = expr_workflow.as_ref() {
                    let mut frame = vb_core::new_run_frame(RunId::new(103), plan);
                    let mut store = vb_core::ValueStore::new();
                    black_box(if let Ok(run) = frame.as_mut() {
                        Some(vb_core::run_until_blocked(plan, run, StepBudget::MAX, &mut store))
                    } else {
                        None
                    })
                } else {
                    None
                }
            })
        },
    );

    ir_group.finish();

    let mut gen_group = c.benchmark_group("generated_execution");
    gen_group.measurement_time(std::time::Duration::from_secs(5));
    gen_group.sample_size(100);

    if let Some(ref gen_bin) = gen_finish {
        gen_group.bench_function(
            metadata(
                "generated_execution_1_step",
                b"finish_1",
                "fixture=finish_1;surface=generated_exec",
            ),
            |b| {
                let bin_path = gen_bin.path.clone();
                b.iter(|| {
                    let start = Instant::now();
                    #[allow(clippy::let_underscore_must_use)]
                    let _ = Command::new(&bin_path).output();
                    black_box(start.elapsed())
                })
            },
        );
    }

    if let Some(ref gen_bin) = gen_chain_1000 {
        gen_group.bench_function(
            metadata(
                "generated_execution_1000_steps",
                b"save_chain_1000",
                "fixture=save_chain_1000;surface=generated_exec",
            ),
            |b| {
                let bin_path = gen_bin.path.clone();
                b.iter(|| {
                    let start = Instant::now();
                    #[allow(clippy::let_underscore_must_use)]
                    let _ = Command::new(&bin_path).output();
                    black_box(start.elapsed())
                })
            },
        );
    }

    if let Some(ref gen_bin) = gen_choose_100 {
        gen_group.bench_function(
            metadata(
                "generated_execution_choose_100",
                b"choose_100",
                "fixture=choose_100;surface=generated_exec",
            ),
            |b| {
                let bin_path = gen_bin.path.clone();
                b.iter(|| {
                    let start = Instant::now();
                    #[allow(clippy::let_underscore_must_use)]
                    let _ = Command::new(&bin_path).output();
                    black_box(start.elapsed())
                })
            },
        );
    }

    if let Some(ref gen_bin) = gen_expr {
        gen_group.bench_function(
            metadata(
                "generated_execution_expr",
                b"expression",
                "fixture=expression_workflow;surface=generated_exec",
            ),
            |b| {
                let bin_path = gen_bin.path.clone();
                b.iter(|| {
                    let start = Instant::now();
                    #[allow(clippy::let_underscore_must_use)]
                    let _ = Command::new(&bin_path).output();
                    black_box(start.elapsed())
                })
            },
        );
    }

    gen_group.finish();

    let mut ratio_group = c.benchmark_group("ir_vs_generated_ratio");
    ratio_group.measurement_time(std::time::Duration::from_secs(10));
    ratio_group.sample_size(50);

    if let Some(ref gen_bin) = gen_finish {
        ratio_group.bench_function(
            metadata(
                "ir_vs_generated_1",
                b"finish_1",
                "fixture=finish_1;surface=ratio",
            ),
            |b| {
                let bin_path = gen_bin.path.clone();
                b.iter(|| {
                    let ir_start = Instant::now();
                    if let Some(plan) = finish_1_workflow.as_ref() {
                        let mut frame = vb_core::new_run_frame(RunId::new(200), plan);
                        let mut store = vb_core::ValueStore::new();
                        if let Ok(run) = frame.as_mut() {
                            #[allow(clippy::let_underscore_must_use)]
                            let _ = vb_core::run_until_blocked(plan, run, StepBudget::MAX, &mut store);
                        }
                    }
                    let ir_ns = ir_start.elapsed().as_nanos();

                    let gen_start = Instant::now();
                    #[allow(clippy::let_underscore_must_use)]
                    let _ = Command::new(&bin_path).output();
                    let gen_ns = gen_start.elapsed().as_nanos();

                    #[allow(clippy::as_conversions)]
                    black_box((ir_ns as f64) / (gen_ns as f64))
                })
            },
        );
    }

    if let Some(ref gen_bin) = gen_chain_1000 {
        ratio_group.bench_function(
            metadata(
                "ir_vs_generated_1000",
                b"save_chain_1000",
                "fixture=save_chain_1000;surface=ratio",
            ),
            |b| {
                let bin_path = gen_bin.path.clone();
                b.iter(|| {
                    let ir_start = Instant::now();
                    if let Some(plan) = save_chain_1000.as_ref() {
                        let mut frame = vb_core::new_run_frame(RunId::new(201), plan);
                        let mut store = vb_core::ValueStore::new();
                        if let Ok(run) = frame.as_mut() {
                            #[allow(clippy::let_underscore_must_use)]
                            let _ = vb_core::run_until_blocked(plan, run, StepBudget::MAX, &mut store);
                        }
                    }
                    let ir_ns = ir_start.elapsed().as_nanos();

                    let gen_start = Instant::now();
                    #[allow(clippy::let_underscore_must_use)]
                    let _ = Command::new(&bin_path).output();
                    let gen_ns = gen_start.elapsed().as_nanos();

                    #[allow(clippy::as_conversions)]
                    black_box((ir_ns as f64) / (gen_ns as f64))
                })
            },
        );
    }

    ratio_group.finish();
}

fn bench_expr(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    expr: &str,
) {
    group.bench_function(metadata(name, expr.as_bytes(), "fixture=expression"), |b| {
        b.iter(|| {
            let tokens = vb_expr::lexer::lex_expr(black_box(expr));
            if let Ok(tokens) = tokens.as_ref() {
                let ast = vb_expr::parser::parse_expr(tokens);
                if let Ok(ast) = ast.as_ref() {
                    let mut constants = Vec::new();
                    let program = vb_expr::bytecode::compile_expr_with_pool(ast, &mut constants);
                    if let Ok(program) = program.as_ref() {
                        let _evaluated = vb_expr::eval::eval_expr_program(program, &[], &constants);
                    }
                    program.map(|_| constants)
                } else {
                    ast.map(|_| Vec::new())
                }
            } else {
                tokens.map(|_| Vec::new())
            }
        })
    });
}

fn decode_ipc_frame(frame: &[u8]) -> Result<vb_ipc::IpcPayload, vb_ipc::IpcError> {
    if frame.len() < vb_ipc::IPC_HEADER_LEN {
        return Err(vb_ipc::IpcError::HeaderDecodeFailed);
    }
    let mut header = [0_u8; vb_ipc::IPC_HEADER_LEN];
    let Some(header_bytes) = frame.get(..vb_ipc::IPC_HEADER_LEN) else {
        return Err(vb_ipc::IpcError::HeaderDecodeFailed);
    };
    header.copy_from_slice(header_bytes);
    let payload = match frame.get(vb_ipc::IPC_HEADER_LEN..) {
        Some(bytes) => Bytes::copy_from_slice(bytes),
        None => Bytes::new(),
    };
    let max_payload = vb_ipc::MaxPayloadBytes::DEFAULT;
    let decoded = vb_ipc::decode_frame(&header, payload, max_payload)?;
    vb_ipc::decode_payload(decoded.payload())
}

fn metadata(name: &str, fixture: &[u8], extra: &str) -> String {
    let digest = blake3::hash(fixture);
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={};fixture_digest={digest}",
        fixture.len()
    )
}

fn bench_event(run: u64, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run: RunId::new(run),
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0x11; 32]),
    }
}

fn seed_journal(
    journal: &vb_storage::FjallJournal,
    run: RunId,
    count: u64,
) -> Result<(), vb_storage::JournalError> {
    let mut seq = 0_u64;
    while seq < count {
        let event = bench_event(run.as_u64(), seq);
        journal.append_journaled(&event)?;
        seq = seq.saturating_add(1);
    }
    Ok(())
}

fn one_mb_workflow() -> String {
    let mut source = String::from(
        "version: velvet-ballastics/v1\nname: parse_1mb\nwhen:\n  manual: {}\nnotes:\n",
    );
    while source.len() < 1_048_576 {
        source.push_str("  - fixture-line-for-yaml-parser-throughput\n");
    }
    source.push_str("steps:\n  - id: done\n    finish:\n      result: 0\n");
    source
}

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

criterion_group!(
    benches,
    parse_yaml_benches,
    compile_and_validate_benches,
    expression_benches,
    slot_and_transition_benches,
    storage_and_ipc_benches,
    generated_benches,
    ir_vs_generated_benches
);
criterion_main!(benches);
