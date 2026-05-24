//! Fixture-backed benchmark suite with explicit metadata in benchmark IDs.

#![allow(missing_docs)]

use bytes::Bytes;
use criterion::{Bencher, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};
use vb_core::{
    ActionId, Capability, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ExprIdx,
    ResourceContract, RunId, SlotBranch, SlotIdx, StepBudget,
    StepIdx, WorkflowDigest, WorkflowParts,
};
use vb_runtime::journal::RuntimeJournal;
use vb_storage::{EventSeq, JournalEvent};

fn cap(action: ActionId) -> Capability {
    Capability::new("".into(), action)
}

fn any_workflow_cap() -> Capability {
    Capability::new("".into(), ActionId::new(0))
}

const SMALL_WORKFLOW: &[u8] = b"version: velvet-ballastics/v1\nname: bench_minimal\nwhen:\n  manual: {}\nsteps:\n  - id: save_value\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n";
const CHOOSE_WORKFLOW: &[u8] = b"version: velvet-ballastics/v1\nname: bench_choose\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose:\n      condition: true\n      on_true: 1\n      on_false: 1\n  - id: done\n    finish:\n      result: true\n";
const EXPR_EQ_SYMBOL: &str = "$input.value == 7";
const EXPR_NUMBER_COMPARE: &str = "7 > 3";
const EXPR_BOOLEAN_CHAIN: &str = "true && false || true";
const EXPR_ARITHMETIC: &str = "1 + 2 * 3";
const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";
const JOURNAL_REPLAY_EVENTS: u64 = 1000;
const BENCH_LATENCY_BUDGET_US: u64 = 100_000;
const BENCH_LATENCY_BUDGET_ENV: &str = "VB_BENCH_LATENCY_BUDGET_US";
const BENCH_LATENCY_REPORT_ENV: &str = "VB_BENCH_LATENCY_REPORT";

type WallBencher<'a> = Bencher<'a, criterion::measurement::WallTime>;

fn bench_latency_budget_us() -> u64 {
    match std::env::var(BENCH_LATENCY_BUDGET_ENV) {
        Ok(raw) => match raw.parse::<u64>() {
            Ok(value) => value,
            Err(_) => BENCH_LATENCY_BUDGET_US,
        },
        Err(_) => BENCH_LATENCY_BUDGET_US,
    }
}

#[allow(clippy::arithmetic_side_effects)]
fn budget_utilization_percent(elapsed: Duration, budget_us: u64) -> u128 {
    if budget_us == 0 {
        u128::MAX
    } else {
        elapsed
            .as_micros()
            .saturating_mul(100)
            .saturating_div(u128::from(budget_us))
    }
}

fn latency_within_budget(elapsed: Duration, budget_us: u64) -> bool {
    budget_us > 0 && elapsed.as_micros() <= u128::from(budget_us)
}

fn budget_failure_message(benchmark: &str, elapsed: Duration, budget_us: u64) -> String {
    format!(
        "benchmark latency budget exceeded: benchmark={benchmark}; elapsed_us={}; budget_us={budget_us}; utilization_pct={}",
        elapsed.as_micros(),
        budget_utilization_percent(elapsed, budget_us)
    )
}

fn budget_success_message(benchmark: &str, elapsed: Duration, budget_us: u64) -> String {
    format!(
        "latency budget ok: benchmark={benchmark}; max_iteration_us={}; budget_us={budget_us}; utilization_pct={}",
        elapsed.as_micros(),
        budget_utilization_percent(elapsed, budget_us)
    )
}

fn report_latency_budget_success(benchmark: &str, elapsed: Duration, budget_us: u64) {
    let enabled = match std::env::var(BENCH_LATENCY_REPORT_ENV) {
        Ok(value) => !matches!(value.as_str(), "0" | "false" | "FALSE"),
        Err(_) => true,
    };
    if enabled {
        eprintln!("{}", budget_success_message(benchmark, elapsed, budget_us));
    }
}

fn assert_latency_within_budget(benchmark: &str, elapsed: Duration, budget_us: u64) {
    assert!(
        latency_within_budget(elapsed, budget_us),
        "{}",
        budget_failure_message(benchmark, elapsed, budget_us)
    );
}

fn checked_iter<T, F>(bencher: &mut WallBencher<'_>, benchmark: &str, mut work: F)
where
    F: FnMut() -> T,
{
    bencher.iter_custom(|iterations| {
        let budget_us = bench_latency_budget_us();
        let (total, max_elapsed) = (0..iterations).fold(
            (Duration::ZERO, Duration::ZERO),
            |(total, max_elapsed), _| {
                let start = Instant::now();
                black_box(work());
                let elapsed = start.elapsed();
                assert_latency_within_budget(benchmark, elapsed, budget_us);
                (
                    total.saturating_add(elapsed),
                    std::cmp::max(max_elapsed, elapsed),
                )
            },
        );
        report_latency_budget_success(benchmark, max_elapsed, budget_us);
        total
    });
}

fn bytes_len(bytes: &[u8]) -> u64 {
    u64::try_from(bytes.len()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    #[test]
    fn zero_microsecond_budget_rejects_all_iterations() {
        assert!(!super::latency_within_budget(std::time::Duration::ZERO, 0));
    }

    #[test]
    fn failure_message_names_benchmark_iteration_and_budget() {
        let message =
            super::budget_failure_message("slow_case", std::time::Duration::from_micros(101), 100);
        assert!(message.contains("benchmark=slow_case"));
        assert!(message.contains("elapsed_us=101"));
        assert!(message.contains("budget_us=100"));
        assert!(message.contains("utilization_pct=101"));
    }

    #[test]
    fn success_message_reports_budget_utilization() {
        let message =
            super::budget_success_message("fast_case", std::time::Duration::from_micros(25), 100);
        assert!(message.contains("latency budget ok"));
        assert!(message.contains("benchmark=fast_case"));
        assert!(message.contains("max_iteration_us=25"));
        assert!(message.contains("budget_us=100"));
        assert!(message.contains("utilization_pct=25"));
    }
}

fn metadata(name: &str, _input: impl AsRef<[u8]>, extra: &str) -> String {
    format!("{name};{extra};{BENCH_METADATA}")
}

fn bench_expr(group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>, name: &str, source: &str) {
    group.bench_function(metadata(name, source.as_bytes(), "surface=expression"), |b| {
        checked_iter(b, name, || {
            vb_compile::expression::parse_expression(black_box(source))
        })
    });
}

fn bench_event(run_val: u64, seq_val: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run: RunId::new(run_val),
        seq: EventSeq::new(seq_val),
        workflow: WorkflowDigest::from_bytes([0x42; 32]),
    }
}

fn seed_journal(
    journal: &vb_storage::FjallJournal,
    run: RunId,
    count: u64,
) -> Result<(), vb_storage::JournalError> {
    let mut i = 0_u64;
    while i < count {
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(i),
            workflow: WorkflowDigest::from_bytes([0x77; 32]),
        };
        journal.append_journaled(&event)?;
        i = i.saturating_add(1);
    }
    Ok(())
}

fn decode_ipc_frame(bytes: &[u8]) -> Result<vb_ipc::IpcFrame, vb_ipc::IpcError> {
    let header: &[u8; vb_ipc::IPC_HEADER_LEN] = bytes
        .get(..vb_ipc::IPC_HEADER_LEN)
        .and_then(|h| h.try_into().ok())
        .ok_or(vb_ipc::IpcError::HeaderDecodeFailed)?;
    let payload = Bytes::copy_from_slice(
        bytes
            .get(vb_ipc::IPC_HEADER_LEN..)
            .unwrap_or(&[]),
    );
    vb_ipc::decode_frame(header, payload, vb_ipc::MaxPayloadBytes::DEFAULT)
}

fn one_mb_workflow() -> String {
    let mut s = String::with_capacity(1024 * 1024);
    s.push_str("version: velvet-ballastics/v1\nname: bench_large\nwhen:\n  manual: {}\nsteps:\n");
    let mut i = 0_u32;
    while s.len() < 1024 * 1024 {
        s.push_str("  - id: save_");
        s.push_str(&i.to_string());
        s.push_str("\n    save:\n      value: ");
        s.push_str(&i.to_string());
        s.push('\n');
        i = i.saturating_add(1);
    }
    s
}

fn many_step_workflow(steps: u16) -> String {
    let mut s = String::with_capacity(usize::from(steps).saturating_mul(64));
    s.push_str("version: velvet-ballastics/v1\nname: bench_many_steps\nwhen:\n  manual: {}\nsteps:\n");
    let mut i = 0_u16;
    while i < steps {
        s.push_str("  - id: step_");
        s.push_str(&i.to_string());
        s.push_str("\n    save:\n      value: ");
        s.push_str(&i.to_string());
        s.push('\n');
        i = i.saturating_add(1);
    }
    s.push_str("  - id: done\n    finish:\n      result: 0\n");
    s
}

/// Observer function to force materialization of parse result.
/// Marked no_inline to prevent LLVM from constant-folding the parse.
#[inline(never)]
fn parse_and_observe(input: &str) -> usize {
    vb_yaml::parse_yaml_events(input)
        .map(|e| e.len())
        .unwrap_or(0)
}

fn parse_yaml_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("yaml_parse");
    let small_meta = metadata("parse_yaml_small", SMALL_WORKFLOW, "fixture=small_workflow");
    group.throughput(Throughput::Bytes(bytes_len(SMALL_WORKFLOW)));
    group.bench_with_input(
        BenchmarkId::from_parameter(small_meta),
        SMALL_WORKFLOW,
        |b, input| {
            checked_iter(b, "parse_yaml_small", || {
                let result = match std::str::from_utf8(input) {
                    Ok(text) => vb_yaml::parse_yaml_events(black_box(text)),
                    Err(error) => Err(vb_yaml::YamlError::ParseError {
                        line: 0,
                        reason: error.to_string().into_boxed_str(),
                    }),
                };
                black_box(result.is_ok())
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
        |b, input| {
            // Use a separate observer function to prevent elision.
            // The key insight: criterion measures b.iter() calls, not what's inside.
            // So we must ensure the parse actually happens inside the iter closure.
            checked_iter(b, "parse_yaml_1mb", || parse_and_observe(input.as_str()))
        },
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
            checked_iter(b, "validate_minimal", || {
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
        |b| {
            checked_iter(b, "compile_ir_minimal", || {
                vb_compile::compile_workflow(black_box(SMALL_WORKFLOW))
            })
        },
    );

    let many_steps = many_step_workflow(1000);
    group.throughput(Throughput::Bytes(bytes_len(many_steps.as_bytes())));
    group.bench_function(
        metadata(
            "compile_ir_1000_steps",
            many_steps.as_bytes(),
            "fixture=generated_1000_steps;surface=compiler",
        ),
        |b| {
            checked_iter(b, "compile_ir_1000_steps", || {
                vb_compile::compile_workflow(black_box(many_steps.as_bytes()))
            })
        },
    );
    group.bench_function(
        metadata(
            "validate_1000_steps",
            many_steps.as_bytes(),
            "fixture=generated_1000_steps;surface=validator",
        ),
        |b| {
            checked_iter(b, "validate_1000_steps", || {
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
            checked_iter(b, "bench_engine_numeric_slots_read_write_i64", || {
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
            checked_iter(b, "slot_read", || {
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
            checked_iter(
                b,
                "bench_engine_step_once_save_const_single_transition",
                || {
                    if let Ok(plan) = workflow.as_ref() {
                        let mut frame = vb_core::new_run_frame(RunId::new(2), plan);
                        let mut store = vb_core::ValueStore::new();
                        if let Ok(run) = frame.as_mut() {
                            let signal = vb_core::step_once(black_box(plan), run, &mut store);
                            black_box(signal.is_ok())
                        } else {
                            black_box(false)
                        }
                    } else {
                        black_box(false)
                    }
                },
            )
        },
    );
    group.bench_function(
        metadata(
            "engine_run_until_blocked_budget_10_small_workflow",
            SMALL_WORKFLOW,
            "fixture=small_workflow;surface=engine_run",
        ),
        |b| {
            checked_iter(
                b,
                "engine_run_until_blocked_budget_10_small_workflow",
                || {
                    if let Ok(plan) = workflow.as_ref() {
                        let mut frame = vb_core::new_run_frame(RunId::new(3), plan);
                        let mut store = vb_core::ValueStore::new();
                        if let Ok(run) = frame.as_mut() {
                            let signal = vb_core::run_until_blocked(
                                black_box(plan),
                                run,
                                StepBudget::new(10),
                                &mut store,
                            );
                            black_box(signal.is_ok())
                        } else {
                            black_box(false)
                        }
                    } else {
                        black_box(false)
                    }
                },
            )
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
            checked_iter(b, "bench_memory_ingress_try_submit_capacity_1024", || {
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
            checked_iter(b, "bench_memory_ingress_submit_recv_single_thread", || {
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
            checked_iter(b, "bench_memory_ingress_backpressure_full_queue", || {
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
        |b| {
            checked_iter(b, "postcard_encode_event", || {
                postcard::to_allocvec(black_box(&event))
            })
        },
    );
    group.bench_function(
        metadata(
            "postcard_decode_event",
            SMALL_WORKFLOW,
            "fixture=run_accepted_event;surface=journal_decode",
        ),
        |b| {
            checked_iter(b, "postcard_decode_event", || {
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
            checked_iter(b, "ipc_frame_encode", || {
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
            checked_iter(b, "ipc_frame_decode", || {
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
            checked_iter(b, "bench_fjall_append_run_accepted_no_persist", || {
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
            checked_iter(b, "bench_replay_ordered_journal_1000_events", || {
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
        checked_iter(b, name, || {
            if let Some(plan) = workflow.as_ref() {
                let mut frame = vb_core::new_run_frame(RunId::new(6), plan);
                let mut store = vb_core::ValueStore::new();
                if let Ok(run) = frame.as_mut() {
                    let signal = vb_core::run_until_blocked(
                        black_box(plan),
                        run,
                        StepBudget::new(budget),
                        &mut store,
                    );
                    black_box(signal.is_ok())
                } else {
                    black_box(false)
                }
            } else {
                black_box(false)
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
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
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
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(4)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
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
        on_error: None,
        error_slot: None,
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
        on_error: None,
        error_slot: None,
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
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ChooseSlot {
            branches: branches.into_boxed_slice(),
            otherwise: Some(StepIdx::new(102)),
        },
    });
    nodes.push(CompiledNode {
        id: StepIdx::new(102),
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(103)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(1),
        },
    });
    nodes.push(CompiledNode {
        id: StepIdx::new(103),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    });
    let constants = vec![
        vb_core::ConstValue::Bool(true),
        vb_core::ConstValue::I64(42),
    ];
    compiled_from_nodes("bench_choose_100", nodes, constants.into_boxed_slice())
}

fn expression_workflow() -> Option<CompiledWorkflow> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
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
    compiled_from_nodes("bench_expr", nodes, constants.into_boxed_slice())
}

fn for_each_workflow() -> Option<CompiledWorkflow> {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(2)),
            next: Some(StepIdx::new(3)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildList {
                items: Box::new([SlotIdx::new(0), SlotIdx::new(1)]),
            },
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: Some(SlotIdx::new(4)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(2),
                item_slot: SlotIdx::new(3),
                limit: 2,
                body: StepIdx::new(4),
                done: StepIdx::new(5),
            },
        },
        CompiledNode {
            id: StepIdx::new(4),
            output: Some(SlotIdx::new(3)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(4),
                body: StepIdx::new(5),
                done: StepIdx::new(5),
            },
        },
        CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(3),
            },
        },
    ];
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::from("bench_for_each"),
        digest: WorkflowDigest::from_bytes([0x44; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::ConstValue::I64(1), vb_core::ConstValue::I64(2)]),
        slot_count: 5,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
        symbols_count: 0,
    })
    .ok()
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
        step_names: Box::default(),
        symbols_count: 0,
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

// ===== Evidence chain event accumulation benchmarks =====

fn evidence_chain_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("evidence_chain");

    // Benchmark: accumulate N events into a VolatileRuntimeJournal.
    group.bench_function(
        metadata(
            "evidence_chain_accumulate_100_events",
            b"evidence_100",
            "fixture=volatile_journal_100;surface=event_accumulate",
        ),
        |b| {
            checked_iter(b, "evidence_chain_accumulate_100_events", || {
                let journal = vb_runtime::journal::VolatileRuntimeJournal::new();
                let mut i = 0_u16;
                while i < 100 {
                    let run = RunId::new(u64::from(i));
                    let event = if i.is_multiple_of(5) {
                        vb_runtime::journal::RuntimeJournalEvent::RunSubmitted {
                            run,
                            workflow: WorkflowDigest::from_bytes([0x11; 32]),
                        }
                    } else if i % 5 == 1 {
                        vb_runtime::journal::RuntimeJournalEvent::StepStarted {
                            run,
                            step: StepIdx::new(0),
                        }
                    } else if i % 5 == 2 {
                        vb_runtime::journal::RuntimeJournalEvent::SlotWritten {
                            run,
                            slot: SlotIdx::new(0),
                            value: vec![],
                            taint: vb_core::Taint::Clean,
                            extra: None,
                        }
                    } else if i % 5 == 3 {
                        vb_runtime::journal::RuntimeJournalEvent::StepSucceeded {
                            run,
                            step: StepIdx::new(0),
                            output: SlotIdx::new(0),
                            attempt: 1,
                        }
                    } else {
                        vb_runtime::journal::RuntimeJournalEvent::RunFinished {
                            run,
                            result: SlotIdx::new(0),
                        }
                    };
                    drop(journal.append(black_box(event)));
                    i = i.saturating_add(1);
                }
                black_box(journal.snapshot().map(|e| e.len()))
            })
        },
    );

    // Benchmark: accumulate 1000 events.
    group.bench_function(
        metadata(
            "evidence_chain_accumulate_1000_events",
            b"evidence_1000",
            "fixture=volatile_journal_1000;surface=event_accumulate",
        ),
        |b| {
            checked_iter(b, "evidence_chain_accumulate_1000_events", || {
                let journal = vb_runtime::journal::VolatileRuntimeJournal::new();
                let mut i = 0_u16;
                while i < 1000 {
                    let run = RunId::new(u64::from(i));
                    let event = if i.is_multiple_of(5) {
                        vb_runtime::journal::RuntimeJournalEvent::RunSubmitted {
                            run,
                            workflow: WorkflowDigest::from_bytes([0x11; 32]),
                        }
                    } else if i % 5 == 1 {
                        vb_runtime::journal::RuntimeJournalEvent::StepStarted {
                            run,
                            step: StepIdx::new(0),
                        }
                    } else if i % 5 == 2 {
                        vb_runtime::journal::RuntimeJournalEvent::SlotWritten {
                            run,
                            slot: SlotIdx::new(0),
                            value: vec![],
                            taint: vb_core::Taint::Clean,
                            extra: None,
                        }
                    } else if i % 5 == 3 {
                        vb_runtime::journal::RuntimeJournalEvent::StepSucceeded {
                            run,
                            step: StepIdx::new(0),
                            output: SlotIdx::new(0),
                            attempt: 1,
                        }
                    } else {
                        vb_runtime::journal::RuntimeJournalEvent::RunFinished {
                            run,
                            result: SlotIdx::new(0),
                        }
                    };
                    drop(journal.append(black_box(event)));
                    i = i.saturating_add(1);
                }
                black_box(journal.snapshot().map(|e| e.len()))
            })
        },
    );

    // Benchmark: snapshot read after 100 events.
    group.bench_function(
        metadata(
            "evidence_chain_snapshot_100_events",
            b"evidence_snap_100",
            "fixture=volatile_journal_snapshot_100;surface=event_snapshot",
        ),
        |b| {
            let journal = vb_runtime::journal::VolatileRuntimeJournal::new();
            let mut i = 0_u16;
            while i < 100 {
                let run = RunId::new(u64::from(i));
                let event = vb_runtime::journal::RuntimeJournalEvent::RunSubmitted {
                    run,
                    workflow: WorkflowDigest::from_bytes([0x22; 32]),
                };
                drop(journal.append(event));
                i = i.saturating_add(1);
            }
            checked_iter(b, "evidence_chain_snapshot_100_events", || {
                black_box(journal.snapshot().map(|e| e.len()))
            })
        },
    );

    group.finish();
}

// ===== Admission gate overhead benchmarks =====

fn admission_gate_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("admission_gate");
    let digest = WorkflowDigest::from_bytes([0xAB; 32]);
    let always_present = vb_runtime::admission::AlwaysPresentArtifactStore::shared();
    let any_workflow_caps = vb_core::CapabilitySet::from_grants(Box::new([any_workflow_cap()]));
    let action_caps = vb_core::CapabilitySet::from_grants(Box::new([
        cap(ActionId::new(1)),
        cap(ActionId::new(2)),
        cap(ActionId::new(3)),
    ]));
    let empty_caps = vb_core::CapabilitySet::empty();

    // Relaxed policy — always succeeds, no artifact check.
    group.bench_function(
        metadata(
            "admit_run_relaxed",
            b"admission_relaxed",
            "fixture=always_present;surface=admit_run;policy=relaxed",
        ),
        |b| {
            checked_iter(b, "admit_run_relaxed", || {
                let result = vb_runtime::admission::admit_run(
                    black_box(always_present.as_ref()),
                    black_box(vb_core::RuntimePolicy::Relaxed),
                    black_box(digest),
                    black_box(RunId::new(1)),
                    black_box(any_workflow_caps.clone()),
                );
                black_box(result.is_ok())
            })
        },
    );

    // Strict policy with artifact present.
    group.bench_function(
        metadata(
            "admit_run_strict_artifact_present",
            b"admission_strict",
            "fixture=always_present;surface=admit_run;policy=strict",
        ),
        |b| {
            checked_iter(b, "admit_run_strict_artifact_present", || {
                let result = vb_runtime::admission::admit_run(
                    black_box(always_present.as_ref()),
                    black_box(vb_core::RuntimePolicy::Strict),
                    black_box(digest),
                    black_box(RunId::new(2)),
                    black_box(any_workflow_caps.clone()),
                );
                black_box(result.is_ok())
            })
        },
    );

    // Admission with multiple action capabilities.
    group.bench_function(
        metadata(
            "admit_run_multiple_action_caps",
            b"admission_multi_caps",
            "fixture=always_present;surface=admit_run;policy=strict;caps=3_actions",
        ),
        |b| {
            checked_iter(b, "admit_run_multiple_action_caps", || {
                let result = vb_runtime::admission::admit_run(
                    black_box(always_present.as_ref()),
                    black_box(vb_core::RuntimePolicy::Strict),
                    black_box(digest),
                    black_box(RunId::new(3)),
                    black_box(action_caps.clone()),
                );
                black_box(result.is_ok())
            })
        },
    );

    // Admission with empty capabilities.
    group.bench_function(
        metadata(
            "admit_run_empty_caps",
            b"admission_empty_caps",
            "fixture=always_present;surface=admit_run;policy=relaxed;caps=empty",
        ),
        |b| {
            checked_iter(b, "admit_run_empty_caps", || {
                let result = vb_runtime::admission::admit_run(
                    black_box(always_present.as_ref()),
                    black_box(vb_core::RuntimePolicy::Relaxed),
                    black_box(digest),
                    black_box(RunId::new(4)),
                    black_box(empty_caps.clone()),
                );
                black_box(result.is_ok())
            })
        },
    );

    group.finish();
}

// ===== Capability check benchmarks =====

fn capability_check_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("capability_check");

    let any_workflow_caps = vb_core::CapabilitySet::from_grants(Box::new([any_workflow_cap()]));
    let action_caps = vb_core::CapabilitySet::from_grants(Box::new([
        cap(ActionId::new(1)),
        cap(ActionId::new(2)),
        cap(ActionId::new(3)),
        cap(ActionId::new(4)),
        cap(ActionId::new(5)),
        cap(ActionId::new(6)),
        cap(ActionId::new(7)),
        cap(ActionId::new(8)),
        cap(ActionId::new(9)),
        cap(ActionId::new(10)),
    ]));
    let empty_caps = vb_core::CapabilitySet::empty();
    let mixed_caps = vb_core::CapabilitySet::from_grants(Box::new([
        cap(ActionId::new(1)),
        cap(ActionId::new(2)),
    ]));

    // AnyWorkflow short-circuit.
    group.bench_function(
        metadata(
            "capability_check_any_workflow_grants",
            b"cap_any_workflow",
            "fixture=any_workflow_set;surface=capability_check",
        ),
        |b| {
            checked_iter(b, "capability_check_any_workflow_grants", || {
                let result = any_workflow_caps.grants(black_box(&cap(ActionId::new(99))));
                black_box(result)
            })
        },
    );

    // Action match from 10-element set (first element).
    group.bench_function(
        metadata(
            "capability_check_action_match_first",
            b"cap_action_first",
            "fixture=action_set_10;surface=capability_check",
        ),
        |b| {
            checked_iter(b, "capability_check_action_match_first", || {
                let result = action_caps.grants(black_box(&cap(ActionId::new(1))));
                black_box(result)
            })
        },
    );

    // Action miss from 10-element set.
    group.bench_function(
        metadata(
            "capability_check_action_miss",
            b"cap_action_miss",
            "fixture=action_set_10;surface=capability_check",
        ),
        |b| {
            checked_iter(b, "capability_check_action_miss", || {
                let result = action_caps.grants(black_box(&cap(ActionId::new(99))));
                black_box(result)
            })
        },
    );

    // Empty set denies all.
    group.bench_function(
        metadata(
            "capability_check_empty_denies",
            b"cap_empty",
            "fixture=empty_set;surface=capability_check",
        ),
        |b| {
            checked_iter(b, "capability_check_empty_denies", || {
                let result = empty_caps.grants(black_box(&cap(ActionId::new(1))));
                black_box(result)
            })
        },
    );

    // Mixed capability set check (action + workflow).
    group.bench_function(
        metadata(
            "capability_check_mixed_set",
            b"cap_mixed",
            "fixture=mixed_set;surface=capability_check",
        ),
        |b| {
            checked_iter(b, "capability_check_mixed_set", || {
                let result = mixed_caps.grants(black_box(&cap(ActionId::new(2))));
                black_box(result)
            })
        },
    );

    // Full admission capability check via vb_runtime::admission::check_capability.
    group.bench_function(
        metadata(
            "capability_check_admission_gate",
            b"cap_admission",
            "fixture=action_set_10;surface=admission_check_capability",
        ),
        |b| {
            checked_iter(b, "capability_check_admission_gate", || {
                let result = vb_runtime::admission::check_capability(
                    black_box(ActionId::new(1)),
                    black_box(&cap(ActionId::new(1))),
                    black_box(&action_caps),
                );
                black_box(result.is_ok())
            })
        },
    );

    group.finish();
}

criterion_group!(
    benches,
    parse_yaml_benches,
    compile_and_validate_benches,
    expression_benches,
    slot_and_transition_benches,
    storage_and_ipc_benches,
    evidence_chain_benches,
    admission_gate_benches,
    capability_check_benches
);

criterion_main!(benches);
