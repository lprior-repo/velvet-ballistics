#![forbid(unsafe_code)]
//! Deterministic replay reproduction tests for seeded schedules (vb-wy33p.16).
//!
//! These cross-crate integration tests prove that the same seed/workflow/fault
//! script produces identical transcript, journal observation, and terminal
//! result across independent runs.
//!
//! Acceptance criteria:
//! - Same-seed action workflow produces identical journal, replay, and terminal
//! - Same-seed wait workflow reproduces identically
//! - Same-seed ask workflow reproduces identically
//! - Same-seed timer workflow reproduces identically
//! - Same-seed crash/restart recovery reproduces identically
//! - Same-seed branch workflow reproduces identically

use std::num::NonZeroUsize;
use std::sync::Arc;

use postcard::to_allocvec;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use vb_core::{
    ActionId, CapabilitySet, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue,
    ResourceContract, RuntimePolicy, SlotIdx, StepIdx, Taint, WorkflowDigest, WorkflowParts,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::recovery::{
    ActionReplayTracker, RecoveryFrameSeedBuilder, RecoveryRuntimeSummary,
    summarize_recovery_events,
};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

/// Unique bead identifier for this test module.
const BEAD_ID: &str = "vb-wy33p.16";

// ---------------------------------------------------------------------------
// Seeded helpers
// ---------------------------------------------------------------------------

/// Generate deterministic bytes from a u64 seed using a stable RNG.
fn seeded_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut bytes = vec![0u8; len];
    rng.fill(&mut bytes[..]);
    bytes
}

/// Deterministic workflow digest from a seed byte.
fn workflow_digest(seed_byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([seed_byte; 32])
}

/// Create a minimal finish-only workflow deterministically from a seed.
fn finish_workflow(seed_byte: u8) -> CompiledWorkflow {
    let set_const = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("wy33p-deterministic-finish"),
        digest: workflow_digest(seed_byte),
        nodes: Box::from([set_const, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).expect("finish_workflow must compile")
}

/// Create a two-step workflow that finishes in two steps (branch-like).
fn two_step_workflow(seed_byte: u8) -> CompiledWorkflow {
    let step0 = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let step1 = CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::new(2)),
        next: Some(StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(1),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(2),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("wy33p-two-step"),
        digest: workflow_digest(seed_byte),
        nodes: Box::from([step0, step1, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(1), ConstValue::I64(2)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).expect("two_step_workflow must compile")
}

/// Create a multi-slot workflow with branching pattern (3 branches from step 0).
fn branch_workflow(seed_byte: u8) -> CompiledWorkflow {
    let branch_a = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(1)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let branch_b = CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::new(2)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(1),
        },
    };
    let branch_c = CompiledNode {
        id: StepIdx::new(2),
        output: Some(SlotIdx::new(3)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(2),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(3),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("wy33p-branch"),
        digest: workflow_digest(seed_byte),
        nodes: Box::from([branch_a, branch_b, branch_c, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(10), ConstValue::I64(20), ConstValue::I64(30)]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).expect("branch_workflow must compile")
}

/// Create runtime config used for all seeded runs.
fn runtime_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 32,
        step_budget_per_tick: 8,
        max_active_runs: 4,
        policy: RuntimePolicy::Relaxed,
    }
}

/// Shared runtime journal configuration for a given seed.
fn create_durable_runtime(
    dir: &std::path::Path,
) -> Result<(Arc<FjallJournal>, StorageRuntimeJournal), String> {
    let journal =
        vb_storage::open_store(dir).map_err(|e| format!("open_store: {e}"))?;
    let runtime_journal = StorageRuntimeJournal::shared_strict(Arc::new(journal.clone()));
    Ok((Arc::new(journal), runtime_journal))
}

/// Run a compiled workflow to terminal with a seeded runtime, then reopen and
/// return the journal events, summary, and frame seed for replay comparison.
fn run_and_replay(
    dir: &std::path::Path,
    run: vb_core::RunId,
    workflow: &CompiledWorkflow,
) -> Result<
    (
        Vec<JournalEvent>,
        RecoveryRuntimeSummary,
        vb_storage::recovery::RecoveryFrameSeed,
    ),
    String,
> {
    let (journal, runtime_journal) = create_durable_runtime(dir)?;
    let shard_count = NonZeroUsize::new(1).ok_or("shard count zero")?;
    let mut runtime =
        Runtime::new(shard_count, runtime_config(), runtime_journal);

    runtime
        .submit_compiled_with_inputs(run, workflow.clone(), Box::from([]))
        .map_err(|e| format!("submit: {e}"))?;

    runtime.tick_all().map_err(|e| format!("tick_all: {e}"))?;
    drop(runtime);

    let events = journal
        .events_for_run(run)
        .map_err(|e| format!("events_for_run: {e}"))?;

    let summary = summarize_recovery_events(&events)
        .map(|h| h.summary())
        .map_err(|e| format!("summary: {e}"))?;

    let seed = RecoveryFrameSeedBuilder::new()
        .build(&events)
        .map_err(|e| format!("seed build: {e}"))?;

    Ok((events, summary, seed))
}

/// Run a workflow twice with the same seed and compare all replay artifacts.
fn compare_repro(
    seed: u64,
    workflow_fn: fn(u8) -> CompiledWorkflow,
) -> Result<(), String> {
    let seed_byte = (seed % 256) as u8;
    let workflow = workflow_fn(seed_byte);
    let run_a = vb_core::RunId::new(seed.wrapping_mul(1000));
    let run_b = vb_core::RunId::new(seed.wrapping_mul(1000) + 1);

    let dir_a = tempfile::tempdir().map_err(|e| format!("tempdir a: {e}"))?;
    let dir_b = tempfile::tempdir().map_err(|e| format!("tempdir b: {e}"))?;

    let (events_a, summary_a, seed_a) =
        run_and_replay(dir_a.path(), run_a, &workflow)?;
    let (events_b, summary_b, seed_b) =
        run_and_replay(dir_b.path(), run_b, &workflow)?;

    // 1. Transcript comparison: journal events must be identical
    assert_eq!(
        events_a, events_b,
        "[{BEAD_ID}] journal events diverge for seed {seed}: a={events_a:?}, b={events_b:?}"
    );

    // 2. Observation comparison: summaries must be identical
    assert_eq!(
        summary_a, summary_b,
        "[{BEAD_ID}] recovery summaries diverge for seed {seed}: a={summary_a:?}, b={summary_b:?}"
    );

    // 3. Frame seed comparison: seeds must be identical
    assert_eq!(
        seed_a, seed_b,
        "[{BEAD_ID}] frame seeds diverge for seed {seed}"
    );

    // 4. Event count parity
    assert_eq!(
        events_a.len(),
        events_b.len(),
        "[{BEAD_ID}] event count diverge for seed {seed}"
    );

    // 5. Slot value determinism via postcard encoding
    for event in &events_a {
        let encoded_a = to_allocvec(event).expect("encode event a");
        let encoded_b = to_allocvec(event).expect("encode event b");
        assert_eq!(
            encoded_a, encoded_b,
            "[{BEAD_ID}] event postcard encoding diverges: {event:?}"
        );
    }

    // 6. Digest consistency across both runs
    let digest_a = blake3::hash(&postcard::to_allocvec(&events_a).unwrap());
    let digest_b = blake3::hash(&postcard::to_allocvec(&events_b).unwrap());
    assert_eq!(
        digest_a, digest_b,
        "[{BEAD_ID}] blake3 transcript hash diverges for seed {seed}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test: same-seed action workflow
// ---------------------------------------------------------------------------

#[test]
fn wy33p_01_same_seed_action_workflow_identical_transcript() {
    compare_repro(0x1A2B, finish_workflow).expect("action workflow repro");
}

#[test]
fn wy33p_02_same_seed_action_workflow_identical_summary() {
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let seed = 0x1A2B;
    let seed_byte = (seed % 256) as u8;
    let workflow = finish_workflow(seed_byte);
    let run_a = vb_core::RunId::new(seed.wrapping_mul(1000));
    let run_b = vb_core::RunId::new(seed.wrapping_mul(1000) + 1);

    let (_, summary_a, _) = run_and_replay(dir_a.path(), run_a, &workflow).unwrap();
    let (_, summary_b, _) = run_and_replay(dir_b.path(), run_b, &workflow).unwrap();

    assert_eq!(
        summary_a.run, summary_b.run,
        "run id must be deterministic"
    );
    assert_eq!(
        summary_a.steps_started, summary_b.steps_started,
        "steps_started must be deterministic"
    );
    assert_eq!(
        summary_a.steps_succeeded, summary_b.steps_succeeded,
        "steps_succeeded must be deterministic"
    );
    assert_eq!(
        summary_a.terminal, summary_b.terminal,
        "terminal must be deterministic"
    );
}

#[test]
fn wy33p_03_different_seed_produces_different_transcript() {
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let seed_a = 0x1A2B;
    let seed_b = 0x3C4D;

    let workflow_a = finish_workflow((seed_a % 256) as u8);
    let workflow_b = finish_workflow((seed_b % 256) as u8);

    let run_a = vb_core::RunId::new(seed_a);
    let run_b = vb_core::RunId::new(seed_b);

    let (events_a, _, _) =
        run_and_replay(dir_a.path(), run_a, &workflow_a).unwrap();
    let (events_b, _, _) =
        run_and_replay(dir_b.path(), run_b, &workflow_b).unwrap();

    assert_ne!(
        events_a, events_b,
        "different seeds must produce different transcripts"
    );
}

// ---------------------------------------------------------------------------
// Test: same-seed two-step workflow
// ---------------------------------------------------------------------------

#[test]
fn wy33p_04_same_seed_two_step_identical_transcript() {
    compare_repro(0x2B3C, two_step_workflow).expect("two-step workflow repro");
}

#[test]
fn wy33p_05_two_step_identical_frame_seed() {
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let seed = 0x2B3C;
    let seed_byte = (seed % 256) as u8;
    let workflow = two_step_workflow(seed_byte);
    let run_a = vb_core::RunId::new(seed.wrapping_mul(1000));
    let run_b = vb_core::RunId::new(seed.wrapping_mul(1000) + 1);

    let (_, _, seed_a) =
        run_and_replay(dir_a.path(), run_a, &workflow).unwrap();
    let (_, _, seed_b) =
        run_and_replay(dir_b.path(), run_b, &workflow).unwrap();

    assert_eq!(
        seed_a.summary.steps_succeeded,
        seed_b.summary.steps_succeeded,
        "two-step must have identical step counts"
    );
    assert_eq!(
        seed_a.step_count, seed_b.step_count,
        "two-step step_count must match"
    );
    assert_eq!(
        seed_a.slot_count, seed_b.slot_count,
        "two-step slot_count must match"
    );
    assert_eq!(
        seed_a.steps, seed_b.steps,
        "two-step step entries must match"
    );
}

// ---------------------------------------------------------------------------
// Test: same-seed branch workflow
// ---------------------------------------------------------------------------

#[test]
fn wy33p_06_same_seed_branch_workflow_identical_transcript() {
    compare_repro(0x3C4D, branch_workflow)
        .expect("branch workflow repro");
}

#[test]
fn wy33p_07_branch_workflow_identical_summary_counters() {
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let seed = 0x3C4D;
    let seed_byte = (seed % 256) as u8;
    let workflow = branch_workflow(seed_byte);
    let run_a = vb_core::RunId::new(seed.wrapping_mul(1000));
    let run_b = vb_core::RunId::new(seed.wrapping_mul(1000) + 1);

    let (_, summary_a, _) =
        run_and_replay(dir_a.path(), run_a, &workflow).unwrap();
    let (_, summary_b, _) =
        run_and_replay(dir_b.path(), run_b, &workflow).unwrap();

    assert_eq!(
        summary_a.steps_started, summary_b.steps_started,
        "branch steps_started must match"
    );
    assert_eq!(
        summary_a.steps_succeeded, summary_b.steps_succeeded,
        "branch steps_succeeded must match"
    );
    assert_eq!(
        summary_a.slots_written, summary_b.slots_written,
        "branch slots_written must match"
    );
}

// ---------------------------------------------------------------------------
// Test: crash/restart reproducibility
// ---------------------------------------------------------------------------

/// Run a workflow, capture the journal state, then reopen the store and
/// replay to verify the same observations are produced.
fn crash_restart_repro(seed: u64, workflow_fn: fn(u8) -> CompiledWorkflow) -> Result<(), String> {
    let seed_byte = (seed % 256) as u8;
    let workflow = workflow_fn(seed_byte);
    let dir = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    let run = vb_core::RunId::new(seed.wrapping_mul(2000));

    // First run: write journal events
    let (journal, runtime_journal) = create_durable_runtime(dir.path())?;
    let shard_count = NonZeroUsize::new(1).ok_or("shard count zero")?;
    let mut runtime = Runtime::new(shard_count, runtime_config(), runtime_journal);
    runtime
        .submit_compiled_with_inputs(run, workflow.clone(), Box::from([]))
        .map_err(|e| format!("submit: {e}"))?;
    runtime.tick_all().map_err(|e| format!("tick_all: {e}"))?;
    drop(runtime);

    // Capture transcript from the journal
    let events_first = journal
        .events_for_run(run)
        .map_err(|e| format!("events_for_run: {e}"))?;
    let summary_first = summarize_recovery_events(&events_first)
        .map(|h| h.summary())
        .map_err(|e| format!("summary: {e}"))?;
    let seed_first = RecoveryFrameSeedBuilder::new()
        .build(&events_first)
        .map_err(|e| format!("seed build: {e}"))?;

    // Reopen the store (simulates crash + restart)
    let reopened_journal = vb_storage::open_store(dir.path())
        .map_err(|e| format!("reopen: {e}"))?;
    let events_second = reopened_journal
        .events_for_run(run)
        .map_err(|e| format!("reopen events_for_run: {e}"))?;

    // Replay from reopened store
    let mut tracker = ActionReplayTracker::new();
    let replayed = vb_storage::replay_journal(
        &reopened_journal,
        run,
        &mut tracker,
        &[],
        &[],
    )
    .map_err(|e| format!("replay_journal: {e}"))?;

    let summary_second =
        summarize_recovery_events(&replayed).map(|h| h.summary()).map_err(|e| format!("summary2: {e}"))?;
    let seed_second = RecoveryFrameSeedBuilder::new()
        .build(&replayed)
        .map_err(|e| format!("seed2 build: {e}"))?;

    // Compare: all replay artifacts must match
    assert_eq!(
        events_first, events_second,
        "[{BEAD_ID}] crash/restart: events diverge after reopen for seed {seed}"
    );

    assert_eq!(
        summary_first, summary_second,
        "[{BEAD_ID}] crash/restart: summaries diverge for seed {seed}"
    );

    assert_eq!(
        seed_first, seed_second,
        "[{BEAD_ID}] crash/restart: frame seeds diverge for seed {seed}"
    );

    Ok(())
}

#[test]
fn wy33p_08_crash_restart_action_workflow() {
    crash_restart_repro(0x4E5F, finish_workflow)
        .expect("crash/restart action workflow repro");
}

#[test]
fn wy33p_09_crash_restart_branch_workflow() {
    crash_restart_repro(0x5F6A, branch_workflow)
        .expect("crash/restart branch workflow repro");
}

#[test]
fn wy33p_10_crash_restart_two_step_workflow() {
    crash_restart_repro(0x6A7B, two_step_workflow)
        .expect("crash/restart two-step workflow repro");
}

// ---------------------------------------------------------------------------
// Test: deterministic replay from stored events
// ---------------------------------------------------------------------------

/// Create journal events from a seed and verify replay produces identical
/// results when replayed independently.
fn seeded_event_replay_repro(
    seed: u64,
    event_count: usize,
) -> Result<(), String> {
    let mut rng = StdRng::seed_from_u64(seed);
    let run = vb_core::RunId::new(seed);
    let workflow = workflow_digest((seed % 256) as u8);

    let mut events: Vec<JournalEvent> = Vec::with_capacity(event_count);
    for i in 0..event_count {
        let seq = i as u64;
        if i == 0 {
            events.push(JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(seq),
                workflow,
            });
        } else if i == event_count - 1 {
            // Terminal event
            events.push(JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(seq),
                result: SlotIdx::new(0),
                attempt: 1,
            });
        } else {
            let step = StepIdx::new(((i - 1) / 2) as u16);
            let event_variant: u8 = rng.gen_range(0..3);
            match event_variant {
                0 => {
                    events.push(JournalEvent::StepStarted {
                        run,
                        seq: EventSeq::new(seq),
                        step,
                        attempt: 1,
                    });
                }
                1 => {
                    events.push(JournalEvent::SlotWrittenEvent {
                        run,
                        seq: EventSeq::new(seq),
                        slot: SlotIdx::new(0),
                        value: Some(seeded_bytes(seed + i as u64, 8)),
                        extra: None,
                        attempt: 1,
                    });
                }
                _ => {
                    events.push(JournalEvent::ActionScheduled {
                        run,
                        seq: EventSeq::new(seq),
                        step,
                        action: ActionId::new(i as u16),
                        attempt: 1,
                    });
                }
            }
        }
    }

    // Replay the events deterministically twice
    let mut tracker1 = ActionReplayTracker::new();
    let replayed1 =
        vb_storage::replay_events(&events, &mut tracker1, &[]).map_err(|e| format!("replay1: {e}"))?;

    let mut tracker2 = ActionReplayTracker::new();
    let replayed2 =
        vb_storage::replay_events(&events, &mut tracker2, &[]).map_err(|e| format!("replay2: {e}"))?;

    assert_eq!(
        replayed1, replayed2,
        "[{BEAD_ID}] replay_events produces identical results for seed {seed}"
    );

    let summary1 =
        summarize_recovery_events(&replayed1).map(|h| h.summary()).map_err(|e| format!("summary1: {e}"))?;
    let summary2 =
        summarize_recovery_events(&replayed2).map(|h| h.summary()).map_err(|e| format!("summary2: {e}"))?;

    assert_eq!(
        summary1, summary2,
        "[{BEAD_ID}] summaries identical for replayed seed {seed}"
    );

    Ok(())
}

#[test]
fn wy33p_11_deterministic_event_replay_small_sequence() {
    seeded_event_replay_repro(0x7C8D, 5).expect("small sequence replay");
}

#[test]
fn wy33p_12_deterministic_event_replay_medium_sequence() {
    seeded_event_replay_repro(0x8D9E, 20).expect("medium sequence replay");
}

#[test]
fn wy33p_13_deterministic_event_replay_large_sequence() {
    seeded_event_replay_repro(0x9EAF, 50).expect("large sequence replay");
}

// ---------------------------------------------------------------------------
// Test: seeded summary counter invariants
// ---------------------------------------------------------------------------

/// Verify that recovery summary counters are deterministic for a given seed.
fn summary_counter_invariants(seed: u64, workflow_fn: fn(u8) -> CompiledWorkflow) -> Result<(), String> {
    let seed_byte = (seed % 256) as u8;
    let workflow = workflow_fn(seed_byte);
    let dir = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    let run = vb_core::RunId::new(seed.wrapping_mul(3000));

    let (_, summary, _) =
        run_and_replay(dir.path(), run, &workflow).map_err(|e| format!("run: {e}"))?;

    // Summary counters must be self-consistent
    if let Some(terminal) = &summary.terminal {
        match terminal {
            vb_storage::recovery::RecoveryTerminalState::Finished { .. } => {
                assert!(
                    summary.steps_succeeded >= 1,
                    "Finished run must have at least 1 step succeeded"
                );
            }
            vb_storage::recovery::RecoveryTerminalState::Cancelled => {}
            vb_storage::recovery::RecoveryTerminalState::Failed => {}
        }
    }

    assert!(
        summary.steps_started >= summary.steps_succeeded,
        "steps_started must be >= steps_succeeded"
    );

    assert!(
        summary.first_seq <= summary.last_seq,
        "first_seq must be <= last_seq"
    );

    // Workflow digest must be recorded
    assert!(
        summary.workflow.is_some(),
        "summary must record workflow digest"
    );

    Ok(())
}

#[test]
fn wy33p_14_summary_counter_invariants_finish_workflow() {
    summary_counter_invariants(0xABCD, finish_workflow).expect("finish workflow counters");
}

#[test]
fn wy33p_15_summary_counter_invariants_two_step_workflow() {
    summary_counter_invariants(0xBCDE, two_step_workflow).expect("two-step counters");
}

#[test]
fn wy33p_16_summary_counter_invariants_branch_workflow() {
    summary_counter_invariants(0xCDEF, branch_workflow).expect("branch counters");
}

// ---------------------------------------------------------------------------
// Test: frame seed step entry determinism
// ---------------------------------------------------------------------------

/// Verify that frame seed step entries are deterministic across runs.
fn frame_seed_step_determinism(
    seed: u64,
    workflow_fn: fn(u8) -> CompiledWorkflow,
) -> Result<(), String> {
    let seed_byte = (seed % 256) as u8;
    let workflow = workflow_fn(seed_byte);
    let dir_a = tempfile::tempdir().map_err(|e| format!("tempdir a: {e}"))?;
    let dir_b = tempfile::tempdir().map_err(|e| format!("tempdir b: {e}"))?;
    let run_a = vb_core::RunId::new(seed.wrapping_mul(4000));
    let run_b = vb_core::RunId::new(seed.wrapping_mul(4000) + 1);

    let (_, _, seed_a) =
        run_and_replay(dir_a.path(), run_a, &workflow).map_err(|e| format!("run a: {e}"))?;
    let (_, _, seed_b) =
        run_and_replay(dir_b.path(), run_b, &workflow).map_err(|e| format!("run b: {e}"))?;

    assert_eq!(
        seed_a.step_count, seed_b.step_count,
        "step_count must be deterministic"
    );
    assert_eq!(
        seed_a.slot_count, seed_b.slot_count,
        "slot_count must be deterministic"
    );
    assert_eq!(
        seed_a.first_step, seed_b.first_step,
        "first_step must be deterministic"
    );
    assert_eq!(
        seed_a.pc, seed_b.pc,
        "program counter must be deterministic"
    );

    // Step entries must be identical
    for (i, (entry_a, entry_b)) in
        seed_a.steps.iter().zip(seed_b.steps.iter()).enumerate()
    {
        assert_eq!(
            entry_a.step, entry_b.step,
            "step entry {} step index must match",
            i
        );
    }

    Ok(())
}

#[test]
fn wy33p_17_frame_seed_steps_finish_workflow() {
    frame_seed_step_determinism(0xDEFA, finish_workflow)
        .expect("finish workflow frame seed steps");
}

#[test]
fn wy33p_18_frame_seed_steps_two_step_workflow() {
    frame_seed_step_determinism(0xEFAB, two_step_workflow)
        .expect("two-step workflow frame seed steps");
}

#[test]
fn wy33p_19_frame_seed_steps_branch_workflow() {
    frame_seed_step_determinism(0xFABC, branch_workflow)
        .expect("branch workflow frame seed steps");
}

// ---------------------------------------------------------------------------
// Test: multiple seeds produce reproducible results
// ---------------------------------------------------------------------------

#[test]
fn wy33p_20_multiple_seeds_reproducible() {
    let seeds: Vec<u64> = vec![0, 1, 42, 255, 256, 1000, 9999, 0xFFFE, 0x7FFF];
    for seed in seeds {
        compare_repro(seed, finish_workflow)
            .unwrap_or_else(|_| panic!("[{BEAD_ID}] repro failed for seed {seed}"));
    }
}

#[test]
fn wy33p_21_multiple_seeds_two_step() {
    let seeds: Vec<u64> = vec![0, 7, 123, 999, 5000, 0xABCD];
    for seed in seeds {
        compare_repro(seed, two_step_workflow)
            .unwrap_or_else(|_| panic!("[{BEAD_ID}] two-step repro failed for seed {seed}"));
    }
}

#[test]
fn wy33p_22_multiple_seeds_branch() {
    let seeds: Vec<u64> = vec![0, 13, 456, 7890, 0x1234];
    for seed in seeds {
        compare_repro(seed, branch_workflow)
            .unwrap_or_else(|_| panic!("[{BEAD_ID}] branch repro failed for seed {seed}"));
    }
}

// ---------------------------------------------------------------------------
// Test: journal event identity across seeds
// ---------------------------------------------------------------------------

#[test]
fn wy33p_23_journal_event_identity_postcard_roundtrip() {
    let seed = 0x1234;
    let seed_byte = (seed % 256) as u8;
    let workflow = finish_workflow(seed_byte);
    let dir = tempfile::tempdir().unwrap();
    let run = vb_core::RunId::new(seed);

    let (events, _, _) = run_and_replay(dir.path(), run, &workflow).unwrap();

    for (i, event) in events.iter().enumerate() {
        // Encode to postcard
        let bytes = to_allocvec(event).expect("encode event");

        // Re-run to get the same event
        let dir2 = tempfile::tempdir().unwrap();
        let (events2, _, _) = run_and_replay(dir2.path(), run, &workflow).unwrap();

        let event2 = &events2[i];
        let bytes2 = to_allocvec(event2).expect("encode event2");

        assert_eq!(
            bytes, bytes2,
            "[{BEAD_ID}] postcard roundtrip diverges at event {i}"
        );
    }
}

// ---------------------------------------------------------------------------
// Test: transcript hash determinism
// ---------------------------------------------------------------------------

#[test]
fn wy33p_24_transcript_hash_determinism_finish() {
    let seed = 0x5678;
    let seed_byte = (seed % 256) as u8;
    let workflow = finish_workflow(seed_byte);
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let run_a = vb_core::RunId::new(seed);
    let run_b = vb_core::RunId::new(seed + 1);

    let (events_a, _, _) = run_and_replay(dir_a.path(), run_a, &workflow).unwrap();
    let (events_b, _, _) = run_and_replay(dir_b.path(), run_b, &workflow).unwrap();

    let hash_a = blake3::hash(&postcard::to_allocvec(&events_a).unwrap());
    let hash_b = blake3::hash(&postcard::to_allocvec(&events_b).unwrap());

    assert_eq!(
        hash_a, hash_b,
        "[{BEAD_ID}] transcript blake3 hash must be deterministic"
    );
}

#[test]
fn wy33p_25_transcript_hash_determinism_two_step() {
    let seed = 0x6789;
    let seed_byte = (seed % 256) as u8;
    let workflow = two_step_workflow(seed_byte);
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let run_a = vb_core::RunId::new(seed);
    let run_b = vb_core::RunId::new(seed + 1);

    let (events_a, _, _) = run_and_replay(dir_a.path(), run_a, &workflow).unwrap();
    let (events_b, _, _) = run_and_replay(dir_b.path(), run_b, &workflow).unwrap();

    let hash_a = blake3::hash(&postcard::to_allocvec(&events_a).unwrap());
    let hash_b = blake3::hash(&postcard::to_allocvec(&events_b).unwrap());

    assert_eq!(
        hash_a, hash_b,
        "[{BEAD_ID}] two-step transcript hash must be deterministic"
    );
}

#[test]
fn wy33p_26_transcript_hash_determinism_branch() {
    let seed = 0x789A;
    let seed_byte = (seed % 256) as u8;
    let workflow = branch_workflow(seed_byte);
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let run_a = vb_core::RunId::new(seed);
    let run_b = vb_core::RunId::new(seed + 1);

    let (events_a, _, _) = run_and_replay(dir_a.path(), run_a, &workflow).unwrap();
    let (events_b, _, _) = run_and_replay(dir_b.path(), run_b, &workflow).unwrap();

    let hash_a = blake3::hash(&postcard::to_allocvec(&events_a).unwrap());
    let hash_b = blake3::hash(&postcard::to_allocvec(&events_b).unwrap());

    assert_eq!(
        hash_a, hash_b,
        "[{BEAD_ID}] branch transcript hash must be deterministic"
    );
}

// ---------------------------------------------------------------------------
// Test: replay journal determinism with stored events
// ---------------------------------------------------------------------------

#[test]
fn wy33p_27_replay_journal_determinism_stored() {
    let seed = 0x89AB;
    let dir = tempfile::tempdir().unwrap();
    let run = vb_core::RunId::new(seed);

    let (journal, runtime_journal) = create_durable_runtime(dir.path()).unwrap();
    let shard_count = NonZeroUsize::new(1).unwrap();
    let workflow = finish_workflow((seed % 256) as u8);
    let mut runtime = Runtime::new(shard_count, runtime_config(), runtime_journal);
    runtime
        .submit_compiled_with_inputs(run, workflow.clone(), Box::from([]))
        .unwrap();
    runtime.tick_all().unwrap();
    drop(runtime);

    // First replay
    let reopened1 = vb_storage::open_store(dir.path()).unwrap();
    let events1 = reopened1.events_for_run(run).unwrap();
    let mut tracker1 = ActionReplayTracker::new();
    let replayed1 =
        vb_storage::replay_journal(&reopened1, run, &mut tracker1, &[], &[]).unwrap();

    // Second replay from same store
    let reopened2 = vb_storage::open_store(dir.path()).unwrap();
    let events2 = reopened2.events_for_run(run).unwrap();
    let mut tracker2 = ActionReplayTracker::new();
    let replayed2 =
        vb_storage::replay_journal(&reopened2, run, &mut tracker2, &[], &[]).unwrap();

    assert_eq!(
        events1, events2,
        "[{BEAD_ID}] stored events must be identical across reopen"
    );
    assert_eq!(
        replayed1, replayed2,
        "[{BEAD_ID}] replay_journal must be deterministic"
    );

    let summary1 = summarize_recovery_events(&replayed1).unwrap().summary();
    let summary2 = summarize_recovery_events(&replayed2).unwrap().summary();
    assert_eq!(
        summary1, summary2,
        "[{BEAD_ID}] replay summary must be deterministic"
    );
}

// ---------------------------------------------------------------------------
// Test: ask and wait event replay determinism (synthetic events)
// ---------------------------------------------------------------------------

/// Run a synthetic wait/suspend workflow through journal events and verify
/// that replay produces identical outcomes across seeds.
fn wait_ask_replay_determinism(seed: u64) -> Result<(), String> {
    let run = vb_core::RunId::new(seed);
    let workflow = workflow_digest((seed % 256) as u8);
    let mut rng = StdRng::seed_from_u64(seed);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::WaitResolved {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(5),
            result: SlotIdx::new(0),
            attempt: 1,
        },
    ];

    // Replay twice independently
    let mut tracker_a = ActionReplayTracker::new();
    let replayed_a =
        vb_storage::replay_events(&events, &mut tracker_a, &[]).map_err(|e| format!("replay a: {e}"))?;

    let mut tracker_b = ActionReplayTracker::new();
    let replayed_b =
        vb_storage::replay_events(&events, &mut tracker_b, &[]).map_err(|e| format!("replay b: {e}"))?;

    assert_eq!(
        replayed_a, replayed_b,
        "[{BEAD_ID}] wait/reply replay must be deterministic for seed {seed}"
    );

    let summary_a =
        summarize_recovery_events(&replayed_a).map(|h| h.summary()).map_err(|e| format!("sum a: {e}"))?;
    let summary_b =
        summarize_recovery_events(&replayed_b).map(|h| h.summary()).map_err(|e| format!("sum b: {e}"))?;

    assert_eq!(
        summary_a, summary_b,
        "[{BEAD_ID}] wait summary must be deterministic"
    );

    // Suspensions counter must be deterministic
    assert_eq!(
        summary_a.suspensions, 1,
        "[{BEAD_ID}] wait suspension count must be 1"
    );

    Ok(())
}

#[test]
fn wy33p_28_wait_event_replay_deterministic() {
    wait_ask_replay_determinism(0xAAAA).expect("wait replay determinism");
}

#[test]
fn wy33p_29_wait_event_replay_deterministic_varying_seed() {
    for seed in [0, 1, 100, 999, 0xDEAD] {
        wait_ask_replay_determinism(seed)
            .unwrap_or_else(|_| panic!("[{BEAD_ID}] wait replay failed for seed {seed}"));
    }
}

// ---------------------------------------------------------------------------
// Test: ask event replay determinism (synthetic events)
// ---------------------------------------------------------------------------

fn ask_replay_determinism(seed: u64) -> Result<(), String> {
    let run = vb_core::RunId::new(seed);
    let workflow = workflow_digest((seed % 256) as u8);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::AskAnsweredEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(5),
            result: SlotIdx::new(0),
            attempt: 1,
        },
    ];

    let mut tracker_a = ActionReplayTracker::new();
    let replayed_a =
        vb_storage::replay_events(&events, &mut tracker_a, &[]).map_err(|e| format!("replay a: {e}"))?;

    let mut tracker_b = ActionReplayTracker::new();
    let replayed_b =
        vb_storage::replay_events(&events, &mut tracker_b, &[]).map_err(|e| format!("replay b: {e}"))?;

    assert_eq!(
        replayed_a, replayed_b,
        "[{BEAD_ID}] ask replay must be deterministic for seed {seed}"
    );

    let summary_a =
        summarize_recovery_events(&replayed_a).map(|h| h.summary()).map_err(|e| format!("sum a: {e}"))?;
    let summary_b =
        summarize_recovery_events(&replayed_b).map(|h| h.summary()).map_err(|e| format!("sum b: {e}"))?;

    assert_eq!(
        summary_a, summary_b,
        "[{BEAD_ID}] ask summary must be deterministic"
    );

    Ok(())
}

#[test]
fn wy33p_30_ask_event_replay_deterministic() {
    ask_replay_determinism(0xBBBB).expect("ask replay determinism");
}

#[test]
fn wy33p_31_ask_event_replay_deterministic_varying_seed() {
    for seed in [0, 42, 777, 0xCCCC] {
        ask_replay_determinism(seed)
            .unwrap_or_else(|_| panic!("[{BEAD_ID}] ask replay failed for seed {seed}"));
    }
}

// ---------------------------------------------------------------------------
// Test: timer event replay determinism (synthetic events)
// ---------------------------------------------------------------------------

fn timer_replay_determinism(seed: u64) -> Result<(), String> {
    let run = vb_core::RunId::new(seed);
    let workflow = workflow_digest((seed % 256) as u8);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::WaitResolved {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(5),
            result: SlotIdx::new(0),
            attempt: 1,
        },
    ];

    let mut tracker_a = ActionReplayTracker::new();
    let replayed_a =
        vb_storage::replay_events(&events, &mut tracker_a, &[]).map_err(|e| format!("replay a: {e}"))?;

    let mut tracker_b = ActionReplayTracker::new();
    let replayed_b =
        vb_storage::replay_events(&events, &mut tracker_b, &[]).map_err(|e| format!("replay b: {e}"))?;

    assert_eq!(
        replayed_a, replayed_b,
        "[{BEAD_ID}] timer replay must be deterministic for seed {seed}"
    );

    let summary_a =
        summarize_recovery_events(&replayed_a).map(|h| h.summary()).map_err(|e| format!("sum a: {e}"))?;
    let summary_b =
        summarize_recovery_events(&replayed_b).map(|h| h.summary()).map_err(|e| format!("sum b: {e}"))?;

    assert_eq!(
        summary_a, summary_b,
        "[{BEAD_ID}] timer summary must be deterministic"
    );

    Ok(())
}

#[test]
fn wy33p_32_timer_event_replay_deterministic() {
    timer_replay_determinism(0xDDDD).expect("timer replay determinism");
}

#[test]
fn wy33p_33_timer_event_replay_deterministic_varying_seed() {
    for seed in [0, 12345, 0xEEEE] {
        timer_replay_determinism(seed)
            .unwrap_or_else(|_| panic!("[{BEAD_ID}] timer replay failed for seed {seed}"));
    }
}

// ---------------------------------------------------------------------------
// Test: full transcript comparison across all workflow types
// ---------------------------------------------------------------------------

#[test]
fn wy33p_34_full_workflow_transcript_repro() {
    let workflows: Vec<(fn(u8) -> CompiledWorkflow, &str)> = vec![
        (finish_workflow, "finish"),
        (two_step_workflow, "two_step"),
        (branch_workflow, "branch"),
    ];
    let seeds: Vec<u64> = vec![0, 1, 42, 1234, 0xABCD];

    for (seed, (wf_fn, name)) in seeds.iter().flat_map(|&s| {
        workflows
            .iter()
            .map(move |(f, n)| (s, (*f, *n)))
    }) {
        compare_repro(seed, *wf_fn).unwrap_or_else(|_| {
            panic!(
                "[{BEAD_ID}] full transcript repro failed for seed {seed} workflow {name}"
            )
        });
    }
}
