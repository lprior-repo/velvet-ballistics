//! Fixture-backed benchmark suite with explicit metadata in benchmark IDs.

#![allow(missing_docs)]

use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use vb_core::{RunId, SlotIdx, StepBudget, StepIdx, WorkflowDigest};
use vb_storage::{EventSeq, JournalEvent};

const SMALL_WORKFLOW: &[u8] = b"version: velvet-ballastics/v1\nname: bench_minimal\nwhen:\n  manual: {}\nsteps:\n  - id: save_value\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n";
const CHOOSE_WORKFLOW: &[u8] = b"version: velvet-ballastics/v1\nname: bench_choose\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose:\n      condition: true\n      on_true: 1\n      on_false: 1\n  - id: done\n    finish:\n      result: true\n";
const EXPR_EQ_SYMBOL: &str = "$input.value == 7";
const EXPR_NUMBER_COMPARE: &str = "7 > 3";
const EXPR_BOOLEAN_CHAIN: &str = "true && false || true";
const EXPR_ARITHMETIC: &str = "1 + 2 * 3";
const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir-and-generated;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";
const JOURNAL_REPLAY_EVENTS: u64 = 128;

fn parse_yaml_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("yaml_parse");
    let small_meta = metadata("parse_yaml_small", SMALL_WORKFLOW, "fixture=small_workflow");
    group.throughput(Throughput::Bytes(SMALL_WORKFLOW.len() as u64));
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
    group.throughput(Throughput::Bytes(one_mb.len() as u64));
    group.bench_with_input(
        BenchmarkId::from_parameter(large_meta),
        &one_mb,
        |b, input| b.iter(|| vb_yaml::parse_yaml_events(black_box(input.as_str()))),
    );
    group.finish();
}

fn compile_and_validate_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("compile_validate");
    group.throughput(Throughput::Bytes(SMALL_WORKFLOW.len() as u64));
    group.bench_function(
        metadata(
            "validator_compile_and_validate_minimal",
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
            "parser_to_ir_compile_minimal",
            SMALL_WORKFLOW,
            "fixture=small_workflow;surface=compiler",
        ),
        |b| b.iter(|| vb_compile::compile_workflow(black_box(SMALL_WORKFLOW))),
    );

    let many_steps = many_step_workflow(1000);
    group.throughput(Throughput::Bytes(many_steps.len() as u64));
    group.bench_function(
        metadata(
            "parser_to_ir_compile_1000_steps",
            many_steps.as_bytes(),
            "fixture=generated_1000_steps;surface=compiler",
        ),
        |b| b.iter(|| vb_compile::compile_workflow(black_box(many_steps.as_bytes()))),
    );
    group.bench_function(
        metadata(
            "validator_compile_and_validate_1000_steps",
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
    let mut group = c.benchmark_group("runtime_core");
    group.bench_function(
        metadata("slot_write", SMALL_WORKFLOW, "fixture=run_frame_slot"),
        |b| {
            b.iter(|| {
                let mut frame = vb_core::RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
                if let Ok(run) = frame.as_mut() {
                    let _written = run.write_slot(SlotIdx::new(0), vb_core::SlotValue::I64(7));
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
            "engine_step_once_small",
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
            "engine_run_until_blocked_budget_10",
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
        Ok(dir) => vb_storage::FjallJournal::open(dir.path()).ok(),
        Err(_) => None,
    };
    let replay_dir = tempfile::tempdir();
    let replay_journal = match replay_dir.as_ref() {
        Ok(dir) => vb_storage::FjallJournal::open(dir.path()).ok(),
        Err(_) => None,
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

    let mut group = c.benchmark_group("storage_ipc");
    group.bench_function(
        metadata(
            "journal_event_envelope_encode",
            SMALL_WORKFLOW,
            "fixture=run_accepted_event;surface=journal_encode",
        ),
        |b| b.iter(|| postcard::to_allocvec(black_box(&event))),
    );
    group.bench_function(
        metadata(
            "journal_event_envelope_decode",
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
            "ipc_frame_encode_submit_run",
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
            "ipc_frame_decode_submit_run",
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
            "journal_append_fjall_unpersisted",
            SMALL_WORKFLOW,
            "fixture=fjall_run_events;surface=journal_append;durability=append_without_sync",
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
            "journal_replay_fjall_128_events",
            SMALL_WORKFLOW,
            "fixture=fjall_run_events_128;surface=journal_replay",
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
    generated_benches
);
criterion_main!(benches);
