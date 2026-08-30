//! Observation export and autonomous scheduler overhead benchmarks.
//!
//! Covers:
//! - Observation export: journal snapshot + postcard serialization, event
//!   encoding/decoding, blake3 hashing of events and workflows.
//! - Scheduler overhead: seed generation, step scheduling via shard helpers,
//!   timer wheel operations, and command queue dispatch.

#![allow(missing_docs)]

use std::time::Instant;
use vb_core::{
    ActionId, Capability, CompiledWorkflow, RunId, SlotIdx, SlotValue, StepIdx, Taint,
    WorkflowDigest,
};
use vb_runtime::journal::{RuntimeJournal, RuntimeJournalEvent, VolatileRuntimeJournal};
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::{InspectSnapshot, PendingTimerKind, ShardCommand};
use vb_storage::records::RecordKind;
use vb_storage::JournalEvent;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

fn any_workflow_cap() -> Capability {
    Capability::new("".into(), ActionId::new(0))
}

const SMALL_WORKFLOW: &[u8] = b"version: velvet-ballistics/v1\nname: bench_minimal\nwhen:\n  manual: {}\nsteps:\n  - id: save_value\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n";

fn metadata(name: &str, fixture: &[u8], extra: &str) -> String {
    let digest = blake3::hash(fixture);
    format!(
        "{name};tool=criterion;mode=bench;fixture_bytes={};fixture_digest={digest};{extra}",
        fixture.len()
    )
}

fn build_runtime_events(count: u16) -> Vec<RuntimeJournalEvent> {
    let mut events = Vec::with_capacity(usize::from(count));
    let mut i = 0_u16;
    while i < count {
        let run = RunId::new(u64::from(i));
        let event = if i % 5 == 0 {
            RuntimeJournalEvent::RunSubmitted {
                run,
                workflow: WorkflowDigest::from_bytes([0x11; 32]),
            }
        } else if i % 5 == 1 {
            RuntimeJournalEvent::StepStarted {
                run,
                step: StepIdx::new(0),
            }
        } else if i % 5 == 2 {
            RuntimeJournalEvent::SlotWritten {
                run,
                slot: SlotIdx::new(0),
                value: vec![],
                taint: Taint::Clean,
                extra: None,
            }
        } else if i % 5 == 3 {
            RuntimeJournalEvent::StepSucceeded {
                run,
                step: StepIdx::new(0),
                output: SlotIdx::new(0),
                attempt: 1,
            }
        } else {
            RuntimeJournalEvent::RunFinished {
                run,
                result: SlotIdx::new(0),
            }
        };
        events.push(event);
        i = i.saturating_add(1);
    }
    events
}

// ===== Observation export: snapshot + serialization =====

fn observation_export_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("observation_export");

    // Snapshot and postcard-serialize 100 events
    group.bench_function(
        metadata(
            "snapshot_serialize_100_events",
            b"obs_export_100",
            "fixture=volatile_journal_100;surface=observation_export",
        ),
        |b| {
            let journal = VolatileRuntimeJournal::new();
            let events = build_runtime_events(100);
            for event in &events {
                drop(journal.append(event.clone()));
            }
            checked_iter(b, "snapshot_serialize_100_events", || {
                let snap = journal.snapshot().ok();
                black_box(snap.map(|e| postcard::to_allocvec(&e).map(|v| v.len())))
            })
        },
    );

    // Snapshot and postcard-serialize 1000 events
    group.bench_function(
        metadata(
            "snapshot_serialize_1000_events",
            b"obs_export_1000",
            "fixture=volatile_journal_1000;surface=observation_export",
        ),
        |b| {
            let journal = VolatileRuntimeJournal::new();
            let events = build_runtime_events(1000);
            for event in &events {
                drop(journal.append(event.clone()));
            }
            checked_iter(b, "snapshot_serialize_1000_events", || {
                let snap = journal.snapshot().ok();
                black_box(snap.map(|e| postcard::to_allocvec(&e).map(|v| v.len())))
            })
        },
    );

    // Serialize individual event variants (RunSubmitted)
    let submitted_event: RuntimeJournalEvent = RuntimeJournalEvent::RunSubmitted {
        run: RunId::new(1),
        workflow: WorkflowDigest::from_bytes([0x22; 32]),
    };
    group.bench_function(
        metadata(
            "serialize_run_submitted_event",
            b"obs_event_submitted",
            "fixture=single_event;surface=observation_serialize",
        ),
        |b| {
            checked_iter(b, "serialize_run_submitted_event", || {
                black_box(postcard::to_allocvec(&submitted_event).map(|v| v.len()))
            })
        },
    );

    // Serialize individual event variants (ActionCompletedEnvelope)
    let action_abi = WorkflowDigest::from_bytes([0x33; 32]);
    let action_event: RuntimeJournalEvent =
        RuntimeJournalEvent::ActionCompletedEnvelope {
            ticket: vb_core::action::ActionTicket {
                run: RunId::new(42),
                step: StepIdx::new(3),
                seq: vb_core::ids::SeqNo::new(5),
                action: ActionId::new(7),
                attempt: 1,
                idempotency_key: vb_core::action::compute_action_idempotency_key(
                    RunId::new(42),
                    vb_core::ids::SeqNo::new(5),
                    ActionId::new(7),
                ),
                capacity: 3,
            },
            output: SlotIdx::new(0),
            value: vec![1, 2, 3, 4, 5],
            encoded_len: 5,
            taint: Taint::Clean,
            value_digest: blake3::hash(&[1, 2, 3, 4, 5]).into(),
            action_abi_digest: action_abi,
        };
    group.bench_function(
        metadata(
            "serialize_action_completed_envelope_event",
            b"obs_event_action_envelope",
            "fixture=single_action_envelope;surface=observation_serialize",
        ),
        |b| {
            checked_iter(
                b,
                "serialize_action_completed_envelope_event",
                || {
                    black_box(
                        postcard::to_allocvec(&action_event)
                            .map(|v| v.len()),
                    )
                },
            )
        },
    );

    // Serialize RunCancelled (larger event with optional reason string)
    let cancelled_event: RuntimeJournalEvent = RuntimeJournalEvent::RunCancelled {
        run: RunId::new(99),
        reason: Some("timeout exceeded".to_string()),
    };
    group.bench_function(
        metadata(
            "serialize_run_cancelled_event",
            b"obs_event_cancelled",
            "fixture=single_cancelled;surface=observation_serialize",
        ),
        |b| {
            checked_iter(b, "serialize_run_cancelled_event", || {
                black_box(postcard::to_allocvec(&cancelled_event).map(|v| v.len()))
            })
        },
    );

    group.finish();
}

// ===== Observation export: event encoding and decoding =====

fn observation_codec_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("observation_codec");

    let sample_event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: vb_storage::EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0x11; 32]),
    };
    let encoded = vb_storage::encode_record(
        vb_storage::MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        sample_event.seq().get(),
        &sample_event,
        vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );

    // Encode a single JournalEvent
    group.bench_function(
        metadata(
            "encode_journal_event",
            b"codec_encode",
            "fixture=run_accepted_event;surface=storage_encode",
        ),
        |b| {
            let event = JournalEvent::RunAccepted {
                run: RunId::new(10),
                seq: vb_storage::EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([0x11; 32]),
            };
            checked_iter(b, "encode_journal_event", || {
                black_box(vb_storage::encode_record(
                    vb_storage::MAGIC_JOURNAL_EVENT,
                    RecordKind::RunAccepted,
                    event.seq().get(),
                    &event,
                    vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
                ))
            })
        },
    );

    // Decode a single JournalEvent
    group.bench_function(
        metadata(
            "decode_journal_event",
            b"codec_decode",
            "fixture=encoded_event;surface=storage_decode",
        ),
        |b| {
            checked_iter(b, "decode_journal_event", || {
                if let Ok(bytes) = encoded.as_ref() {
                    black_box(vb_storage::decode_record::<JournalEvent>(
                        bytes.as_slice(),
                        vb_storage::MAGIC_JOURNAL_EVENT,
                        vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
                    ))
                } else {
                    Err(vb_storage::JournalError::KeyCapacity)
                }
            })
        },
    );

    // Encode + decode round-trip for 100 events
    group.bench_function(
        metadata(
            "encode_decode_100_events_roundtrip",
            b"codec_roundtrip_100",
            "fixture=100_events;surface=storage_roundtrip",
        ),
        |b| {
            checked_iter(
                b,
                "encode_decode_100_events_roundtrip",
                || {
                    let mut encoded_batch = Vec::new();
                    let mut seq = 0_u64;
                    while seq < 100 {
                        let event = JournalEvent::RunAccepted {
                            run: RunId::new(50),
                            seq: vb_storage::EventSeq::new(seq),
                            workflow: WorkflowDigest::from_bytes([0x11; 32]),
                        };
                        if let Ok(rec) = vb_storage::encode_record(
                            vb_storage::MAGIC_JOURNAL_EVENT,
                            RecordKind::RunAccepted,
                            event.seq().get(),
                            &event,
                            vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
                        ) {
                            encoded_batch.push(rec);
                        }
                        seq = seq.saturating_add(1);
                    }
                    let mut decoded_count = 0_u64;
                    let mut offset = 0_usize;
                    while offset < encoded_batch.len() && decoded_count < 100 {
                        if let Ok((_, _event)) = vb_storage::decode_record::<JournalEvent>(
                            encoded_batch[offset].as_slice(),
                            vb_storage::MAGIC_JOURNAL_EVENT,
                            vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
                        ) {
                            decoded_count = decoded_count.saturating_add(1);
                        }
                        offset = offset.saturating_add(1);
                    }
                    black_box(decoded_count)
                },
            )
        },
    );

    group.finish();
}

// ===== Observation export: blake3 hashing =====

fn observation_hashing_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("observation_hashing");

    // Hash a serialized workflow parts
    let workflow = vb_compile::compile_workflow(SMALL_WORKFLOW).ok();
    group.bench_function(
        metadata(
            "hash_workflow_parts",
            SMALL_WORKFLOW,
            "fixture=small_workflow;surface=blake3_hash",
        ),
        |b| {
            checked_iter(b, "hash_workflow_parts", || {
                let mut result = [0u8; 32];
                if let Some(ref wf) = workflow {
                    let parts_bytes = postcard::to_allocvec(&wf.to_parts()).ok();
                    if let Some(ref bytes) = parts_bytes {
                        result = blake3::hash(bytes).into();
                    }
                }
                black_box(result)
            })
        },
    );

    // Hash a serialized runtime journal snapshot (100 events)
    group.bench_function(
        metadata(
            "hash_snapshot_100_events",
            b"hash_snapshot_100",
            "fixture=volatile_journal_100;surface=blake3_hash",
        ),
        |b| {
            checked_iter(b, "hash_snapshot_100_events", || {
                let journal = VolatileRuntimeJournal::new();
                let events = build_runtime_events(100);
                for event in &events {
                    drop(journal.append(event.clone()));
                }
                let mut result = [0u8; 32];
                if let Ok(snapshot) = journal.snapshot() {
                    let serialized = postcard::to_allocvec(&snapshot).ok();
                    if let Some(ref bytes) = serialized {
                        result = blake3::hash(bytes).into();
                    }
                }
                black_box(result)
            })
        },
    );

    // Hash a serialized runtime journal snapshot (1000 events)
    group.bench_function(
        metadata(
            "hash_snapshot_1000_events",
            b"hash_snapshot_1000",
            "fixture=volatile_journal_1000;surface=blake3_hash",
        ),
        |b| {
            checked_iter(b, "hash_snapshot_1000_events", || {
                let journal = VolatileRuntimeJournal::new();
                let events = build_runtime_events(1000);
                for event in &events {
                    drop(journal.append(event.clone()));
                }
                let mut result = [0u8; 32];
                if let Ok(snapshot) = journal.snapshot() {
                    let serialized = postcard::to_allocvec(&snapshot).ok();
                    if let Some(ref bytes) = serialized {
                        result = blake3::hash(bytes).into();
                    }
                }
                black_box(result)
            })
        },
    );

    // Hash raw event bytes (postcard-serialized RunSubmitted)
    let submitted_event: RuntimeJournalEvent = RuntimeJournalEvent::RunSubmitted {
        run: RunId::new(1),
        workflow: WorkflowDigest::from_bytes([0x11; 32]),
    };
    let submitted_bytes = postcard::to_allocvec(&submitted_event).unwrap();
    group.bench_function(
        metadata(
            "hash_serialized_event",
            &submitted_bytes,
            "fixture=serialized_event;surface=blake3_hash",
        ),
        |b| {
            checked_iter(b, "hash_serialized_event", || {
                black_box(blake3::hash(&submitted_bytes))
            })
        },
    );

    group.finish();
}

// ===== Scheduler overhead: seed generation =====

fn scheduler_seed_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_seed");

    // Seed input slots for a small workflow run
    let workflow = vb_compile::compile_workflow(SMALL_WORKFLOW).ok();
    let inputs: Vec<(SlotIdx, SlotValue)> = vec![
        (SlotIdx::new(0), SlotValue::I64(1)),
        (SlotIdx::new(1), SlotValue::I64(2)),
        (SlotIdx::new(2), SlotValue::I64(3)),
    ];

    group.bench_function(
        metadata(
            "seed_input_slots_small_run",
            b"seed_small",
            "fixture=small_run;surface=scheduler_seed",
        ),
        |b| {
            checked_iter(b, "seed_input_slots_small_run", || {
                let mut ok = false;
                if let Some(ref wf) = workflow {
                    let frame = vb_core::RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 4);
                    if let Ok(mut f) = frame {
                        ok = vb_runtime::shard::helpers::seed_input_slots(
                            &mut f,
                            &inputs,
                            wf,
                        ).is_ok();
                    }
                }
                black_box(ok)
            })
        },
    );

    // Seed input slots with many values
    let large_inputs: Vec<(SlotIdx, SlotValue)> =
        (0..50).map(|i| (SlotIdx::new(i), SlotValue::I64(i as i64))).collect();
    group.bench_function(
        metadata(
            "seed_input_slots_large_run",
            b"seed_large",
            "fixture=large_run;surface=scheduler_seed",
        ),
        |b| {
            checked_iter(b, "seed_input_slots_large_run", || {
                let mut ok = false;
                if let Some(ref wf) = workflow {
                    let frame = vb_core::RunFrame::new(RunId::new(2), StepIdx::new(0), 2, 52);
                    if let Ok(mut f) = frame {
                        ok = vb_runtime::shard::helpers::seed_input_slots(
                            &mut f,
                            &large_inputs,
                            wf,
                        ).is_ok();
                    }
                }
                black_box(ok)
            })
        },
    );

    // Seed with empty inputs
    let empty_inputs: Vec<(SlotIdx, SlotValue)> = Vec::new();
    group.bench_function(
        metadata(
            "seed_input_slots_empty",
            b"seed_empty",
            "fixture=empty_inputs;surface=scheduler_seed",
        ),
        |b| {
            checked_iter(b, "seed_input_slots_empty", || {
                let mut ok = false;
                if let Some(ref wf) = workflow {
                    let frame = vb_core::RunFrame::new(RunId::new(3), StepIdx::new(0), 2, 2);
                    if let Ok(mut f) = frame {
                        ok = vb_runtime::shard::helpers::seed_input_slots(
                            &mut f,
                            &empty_inputs,
                            wf,
                        ).is_ok();
                    }
                }
                black_box(ok)
            })
        },
    );

    group.finish();
}

// ===== Scheduler overhead: step scheduling and action normalization =====

fn scheduler_step_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_step");

    let action_caps = vb_core::capability::CapabilitySet::from_grants(Box::new([
        any_workflow_cap(),
        cap(ActionId::new(1)),
        cap(ActionId::new(2)),
        cap(ActionId::new(3)),
    ]));

    let workflow = vb_compile::compile_workflow(SMALL_WORKFLOW).ok();
    let run_state = workflow.as_ref().map(|wf| {
        vb_runtime::shard::helpers::make_run_state(wf.clone(), RunId::new(1))
    });

    // normalize_scheduled_ticket with varying attempt numbers
    let ticket1 = vb_core::action::ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: vb_core::ids::SeqNo::new(1),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: vb_core::action::compute_action_idempotency_key(
            RunId::new(1),
            vb_core::ids::SeqNo::new(1),
            ActionId::new(1),
        ),
        capacity: 5,
    };

    group.bench_function(
        metadata(
            "normalize_scheduled_ticket_attempt_1",
            b"norm_attempt_1",
            "fixture=simple_ticket;surface=scheduler_normalize",
        ),
        |b| {
            checked_iter(b, "normalize_scheduled_ticket_attempt_1", || {
                let mut ok = false;
                if let Some(wf) = workflow.as_ref() {
                    if let Some(state) = vb_runtime::shard::helpers::make_run_state(wf.clone(), RunId::new(1)) {
                        ok = vb_runtime::shard::helpers::normalize_scheduled_ticket(
                            &state, ticket1,
                        ).is_ok();
                    }
                }
                black_box(ok)
            })
        },
    );

    // record_scheduled_attempt
    let ticket2 = vb_core::action::ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: vb_core::ids::SeqNo::new(2),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: vb_core::action::compute_action_idempotency_key(
            RunId::new(1),
            vb_core::ids::SeqNo::new(2),
            ActionId::new(1),
        ),
        capacity: 5,
    };

    group.bench_function(
        metadata(
            "record_scheduled_attempt",
            b"record_scheduled",
            "fixture=simple_ticket;surface=scheduler_record",
        ),
        |b| {
            checked_iter(b, "record_scheduled_attempt", || {
                let mut frame_id = 0_u64;
                if let Some(wf) = workflow.as_ref() {
                    let mut owned_state = vb_runtime::shard::helpers::make_run_state(
                        wf.clone(), RunId::new(1),
                    );
                    if let Some(ref mut s) = owned_state {
                        vb_runtime::shard::helpers::record_scheduled_attempt(s, ticket2);
                        frame_id = s.frame.run_id().get();
                    }
                }
                black_box(frame_id)
            })
        },
    );

    // new_action_attempts allocation
    group.bench_function(
        metadata(
            "new_action_attempts_100_steps",
            b"new_action_100",
            "fixture=100_steps;surface=scheduler_init",
        ),
        |b| {
            checked_iter(b, "new_action_attempts_100_steps", || {
                let attempts = vb_runtime::shard::helpers::new_action_attempts(100);
                black_box(attempts.len() == 100)
            })
        },
    );

    // snapshot_from_state
    group.bench_function(
        metadata(
            "snapshot_from_state",
            SMALL_WORKFLOW,
            "fixture=small_run;surface=scheduler_snapshot",
        ),
        |b| {
            checked_iter(b, "snapshot_from_state", || {
                let snap = if let Some(wf) = workflow.as_ref() {
                    if let Some(state) = vb_runtime::shard::helpers::make_run_state(wf.clone(), RunId::new(1)) {
                        let s = vb_runtime::shard::helpers::snapshot_from_state(
                            RunId::new(1), 0, &state,
                        );
                        s.pc.as_usize()
                    } else {
                        0
                    }
                } else {
                    0
                };
                black_box(snap)
            })
        },
    );

    group.finish();
}

fn cap(action: ActionId) -> Capability {
    Capability::new("".into(), action)
}

// ===== Scheduler overhead: timer wheel operations =====

fn scheduler_timer_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_timer");

    // Timer wheel insert and fire (single entry)
    group.bench_function(
        metadata(
            "timer_wheel_insert_fire_single",
            b"timer_single",
            "fixture=single_timer;surface=scheduler_timer",
        ),
        |b| {
            checked_iter(b, "timer_wheel_insert_fire_single", || {
                let mut wheel = TimerWheel::new();
                let now = std::time::Instant::now();
                let deadline = now + std::time::Duration::from_millis(100);
                drop(wheel.insert(
                    RunId::new(1),
                    deadline,
                    PendingTimerKind::Wait,
                ));
                let fired = wheel.fire_expired(now + std::time::Duration::from_millis(200));
                black_box(fired.len())
            })
        },
    );

    // Timer wheel insert and fire (batch)
    group.bench_function(
        metadata(
            "timer_wheel_insert_fire_batch_100",
            b"timer_batch_100",
            "fixture=batch_100;surface=scheduler_timer",
        ),
        |b| {
            checked_iter(b, "timer_wheel_insert_fire_batch_100", || {
                let mut wheel = TimerWheel::new();
                let now = std::time::Instant::now();
                let mut i = 0_u64;
                while i < 100 {
                    let deadline =
                        now + std::time::Duration::from_millis(i.saturating_mul(10));
                    drop(wheel.insert(
                        RunId::new(i),
                        deadline,
                        PendingTimerKind::Wait,
                    ));
                    i = i.saturating_add(1);
                }
                let fired = wheel.fire_expired(now + std::time::Duration::from_secs(60));
                black_box(fired.len())
            })
        },
    );

    // Timer wheel cancel
    group.bench_function(
        metadata(
            "timer_wheel_cancel",
            b"timer_cancel",
            "fixture=single_timer;surface=scheduler_cancel",
        ),
        |b| {
            checked_iter(b, "timer_wheel_cancel", || {
                let mut wheel = TimerWheel::new();
                let now = std::time::Instant::now();
                let deadline = now + std::time::Duration::from_millis(100);
                drop(wheel.insert(
                    RunId::new(5),
                    deadline,
                    PendingTimerKind::Wait,
                ));
                black_box(wheel.cancel(RunId::new(5)))
            })
        },
    );

    // next_deadline query
    group.bench_function(
        metadata(
            "timer_wheel_next_deadline",
            b"timer_deadline",
            "fixture=single_timer;surface=scheduler_next_deadline",
        ),
        |b| {
            let mut wheel = TimerWheel::new();
            let now = std::time::Instant::now();
            let deadline = now + std::time::Duration::from_millis(50);
            drop(wheel.insert(
                RunId::new(1),
                deadline,
                PendingTimerKind::Wait,
            ));
            checked_iter(b, "timer_wheel_next_deadline", || {
                black_box(wheel.next_deadline())
            })
        },
    );

    group.finish();
}

// ===== Scheduler overhead: command queue dispatch =====

fn scheduler_queue_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_queue");

    // Enqueue and dequeue shard commands
    let cmd = ShardCommand::Inspect {
        run: RunId::new(1),
        correlation: 42,
    };
    group.bench_function(
        metadata(
            "enqueue_dequeue_shard_command",
            b"queue_cmd",
            "fixture=single_command;surface=scheduler_queue",
        ),
        |b| {
            checked_iter(b, "enqueue_dequeue_shard_command", || {
                let queue =
                    vb_runtime::shard::types::ShardCommandQueue::new(1024).ok();
                if let Some(ref q) = queue {
                    drop(q.enqueue(cmd.clone()));
                    let popped = q.pop();
                    black_box(popped.is_some())
                } else {
                    black_box(false)
                }
            })
        },
    );

    // Batch enqueue (100 commands)
    group.bench_function(
        metadata(
            "enqueue_batch_100_commands",
            b"queue_batch_100",
            "fixture=batch_100_commands;surface=scheduler_queue",
        ),
        |b| {
            checked_iter(b, "enqueue_batch_100_commands", || {
                let queue =
                    vb_runtime::shard::types::ShardCommandQueue::new(1024).ok();
                if let Some(ref q) = queue {
                    let mut i = 0_u64;
                    while i < 100 {
                        let cmd = ShardCommand::Inspect {
                            run: RunId::new(i),
                            correlation: i,
                        };
                        drop(q.enqueue(cmd));
                        i = i.saturating_add(1);
                    }
                    black_box(q.len() == 100)
                } else {
                    black_box(false)
                }
            })
        },
    );

    group.finish();
}

// ===== Scheduler overhead: run state creation and inspection =====

fn scheduler_state_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_state");

    let workflow = vb_compile::compile_workflow(SMALL_WORKFLOW).ok();

    // make_run_state with full slot population
    group.bench_function(
        metadata(
            "make_run_state_small",
            b"state_small",
            "fixture=small_run;surface=scheduler_state",
        ),
        |b| {
            checked_iter(b, "make_run_state_small", || {
                let created = if let Some(ref wf) = workflow {
                    let state =
                        vb_runtime::shard::helpers::make_run_state(wf.clone(), RunId::new(1));
                    state.is_some()
                } else {
                    false
                };
                black_box(created)
            })
        },
    );

    // retry_metadata_exists check
    group.bench_function(
        metadata(
            "retry_metadata_exists_check",
            b"retry_check",
            "fixture=run_state;surface=scheduler_retry",
        ),
        |b| {
            let wf = if let Some(wf) = workflow.as_ref() {
                Some(wf.clone())
            } else {
                None
            };
            checked_iter(b, "retry_metadata_exists_check", || {
                let state = if let Some(ref wf) = wf {
                    vb_runtime::shard::types::RunState {
                        frame: vb_core::RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1)
                            .unwrap(),
                        workflow: wf.clone(),
                        store: vb_core::ValueStore::new(),
                        action_attempts: Box::new([1_u16; 16]),
                        admission: None,
                        collect_states: vb_runtime::primitives::collect::CollectStates::new(),
                        action_contracts: Box::new([]),
                    }
                } else {
                    vb_runtime::shard::types::RunState {
                        frame: vb_core::RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1)
                            .unwrap(),
                        workflow: vb_compile::compile_workflow(SMALL_WORKFLOW).unwrap(),
                        store: vb_core::ValueStore::new(),
                        action_attempts: Box::new([1_u16; 16]),
                        admission: None,
                        collect_states: vb_runtime::primitives::collect::CollectStates::new(),
                        action_contracts: Box::new([]),
                    }
                };
                let has_retry =
                    vb_runtime::shard::helpers::retry_metadata_exists(
                        &state,
                        StepIdx::new(0),
                    );
                black_box(has_retry)
            })
        },
    );

    group.finish();
}

// ===== Combined: full observation export pipeline =====

fn observation_pipeline_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("observation_pipeline");

    group.bench_function(
        metadata(
            "full_pipeline_100_events",
            b"pipeline_100",
            "fixture=volatile_journal_100;surface=full_pipeline",
        ),
        |b| {
            checked_iter(b, "full_pipeline_100_events", || {
                let journal = VolatileRuntimeJournal::new();
                // Append events
                let events = build_runtime_events(100);
                for event in &events {
                    drop(journal.append(event.clone()));
                }
                // Snapshot
                if let Ok(snapshot) = journal.snapshot() {
                    // Serialize
                    let serialized = postcard::to_allocvec(&snapshot).ok();
                    if let Some(ref bytes) = serialized {
                        // Hash
                        let hash = blake3::hash(bytes);
                        // Encode as journal records
                        let mut encoded_count = 0_u64;
                        for evt in &snapshot {
                            let _rec_bytes = postcard::to_allocvec(evt);
                            encoded_count = encoded_count.saturating_add(1);
                        }
                        let len = serialized.as_ref().map(|b| b.len()).unwrap_or(0);
                        let hash_bytes: [u8; 32] = hash.into();
                        black_box((hash_bytes, len, encoded_count))
                    } else {
                        let hash = blake3::hash(&[]);
                        let hash_bytes: [u8; 32] = hash.into();
                        black_box((hash_bytes, 0, 0_u64))
                    }
                } else {
                    let hash = blake3::hash(&[]);
                    let hash_bytes: [u8; 32] = hash.into();
                    black_box((hash_bytes, 0, 0_u64))
                }
            })
        },
    );

    group.finish();
}

// ===== Autonomy scheduler overhead: scheduling loop simulation =====

fn scheduler_loop_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_loop");

    let workflow = vb_compile::compile_workflow(SMALL_WORKFLOW).ok();

    group.bench_function(
        metadata(
            "scheduler_step_overhead",
            b"step_overhead",
            "fixture=single_step;surface=scheduler_loop",
        ),
        |b| {
            checked_iter(b, "scheduler_step_overhead", || {
                if let Some(ref wf) = workflow {
                    let state = vb_runtime::shard::helpers::make_run_state(
                        wf.clone(),
                        RunId::new(1),
                    );
                    if let Some(ref s) = state {
                        let snap = vb_runtime::shard::helpers::snapshot_from_state(
                            RunId::new(1), 0, s,
                        );
                        let _retry =
                            vb_runtime::shard::helpers::retry_metadata_exists(
                                s,
                                StepIdx::new(0),
                            );
                        let _pc = snap.pc;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
        },
    );

    // Simulate multiple scheduling decisions
    group.bench_function(
        metadata(
            "scheduler_step_batch_50",
            b"step_batch_50",
            "fixture=50_steps;surface=scheduler_loop",
        ),
        |b| {
            checked_iter(b, "scheduler_step_batch_50", || {
                if let Some(ref wf) = workflow {
                    let mut processed = 0_u64;
                    let mut i = 0_u64;
                    while i < 50 {
                        let state = vb_runtime::shard::helpers::make_run_state(
                            wf.clone(),
                            RunId::new(i),
                        );
                        if state.is_some() {
                            processed = processed.saturating_add(1);
                        }
                        i = i.saturating_add(1);
                    }
                    processed == 50
                } else {
                    false
                }
            })
        },
    );

    group.finish();
}

// ===== Latency budget helpers =====

fn bench_latency_budget_us() -> u64 {
    match std::env::var("VB_BENCH_LATENCY_BUDGET_US") {
        Ok(raw) => raw.parse().unwrap_or(100_000),
        Err(_) => 100_000,
    }
}

fn latency_within_budget(elapsed: std::time::Duration, budget_us: u64) -> bool {
    budget_us > 0 && elapsed.as_micros() <= u128::from(budget_us)
}

fn budget_failure_message(
    benchmark: &str,
    elapsed: std::time::Duration,
    budget_us: u64,
) -> String {
    format!(
        "benchmark latency budget exceeded: benchmark={benchmark}; elapsed_us={}; budget_us={budget_us}",
        elapsed.as_micros()
    )
}

fn assert_latency_within_budget(
    benchmark: &str,
    elapsed: std::time::Duration,
    budget_us: u64,
) {
    assert!(
        latency_within_budget(elapsed, budget_us),
        "{}",
        budget_failure_message(benchmark, elapsed, budget_us)
    );
}

fn checked_iter<T, F>(bencher: &mut criterion::Bencher<'_>, benchmark: &str, mut work: F)
where
    F: FnMut() -> T,
{
    bencher.iter_custom(|iterations| {
        let budget_us = bench_latency_budget_us();
        let (total, max_elapsed) = (0..iterations).fold(
            (std::time::Duration::ZERO, std::time::Duration::ZERO),
            |(total, max_elapsed), _| {
                let start = Instant::now();
                let result = work();
                let elapsed = start.elapsed();
                assert_latency_within_budget(benchmark, elapsed, budget_us);
                black_box(result);
                (
                    total.saturating_add(elapsed),
                    std::time::Duration::max(max_elapsed, elapsed),
                )
            },
        );
        eprintln!(
            "latency budget ok: benchmark={benchmark}; max_iteration_us={}; budget_us={}",
            max_elapsed.as_micros(),
            budget_us
        );
        total
    });
}

criterion_group!(
    benches,
    observation_export_benches,
    observation_codec_benches,
    observation_hashing_benches,
    scheduler_seed_benches,
    scheduler_step_benches,
    scheduler_timer_benches,
    scheduler_queue_benches,
    scheduler_state_benches,
    observation_pipeline_benches,
    scheduler_loop_benches,
);
criterion_main!(benches);
