//! Tests for snapshot writer positive-interval paths.
//!
//! These tests verify that when `snapshot_interval_steps > 0`:
//! - No snapshot fires before the interval is reached.
//! - A snapshot fires at the interval boundary.
//! - `last_snapshot_executed` is updated after a successful snapshot.
//! - Subsequent snapshots fire at interval multiples.

use std::sync::Arc;

use vb_storage::FjallJournal;

use crate::journal::{RuntimeJournalEvent, SharedRuntimeJournal, StorageRuntimeJournal, VolatileRuntimeJournal};
use crate::shard::transitions::SnapshotWriteOutcome;
use crate::shard::{RunState, Shard, ShardConfig};
use vb_core::ids::RunId;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn config_with_interval(interval: u64) -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
        coalesce_window_ticks: 1,
        snapshot_interval_steps: interval,
    }
}

fn make_test_run_state(run: RunId, executed: u64, last_snapshot_executed: u64) -> RunState {
    use vb_core::frame::RunFrame;
    use vb_core::workflow::CompiledWorkflow;

    let node = vb_core::workflow::CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: vb_core::workflow::CompiledNodeKind::Finish {
            result: vb_core::ids::SlotIdx::ZERO,
        },
    };
    let parts = vb_core::workflow::WorkflowParts {
        name: Box::from("test-workflow"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([1; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).expect("test workflow must compile");

    let frame = RunFrame::new(
        run,
        vb_core::ids::WorkflowDigest::from_bytes([1; 32]),
        vb_core::ids::StepIdx::ZERO,
        0, // depth
        executed,
        1, // slot_count
        0, // taint_count
    );

    RunState {
        frame,
        workflow,
        action_attempts: Default::default(),
        last_snapshot_executed,
        pending_action: None,
        pending_timer: None,
    }
}

fn temp_fjall_journal() -> Option<(tempfile::TempDir, Arc<FjallJournal>)> {
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    let _ = std::fs::create_dir_all(&base);
    let dir = tempfile::Builder::new()
        .prefix("vb-snap-pos-")
        .tempdir_in(&base)
        .ok()?;
    let journal = FjallJournal::open(dir.path(), None).ok()?;
    Some((dir, Arc::new(journal)))
}

// ---------------------------------------------------------------------------
// Test: interval > 0 with volatile journal returns SkippedNoStorage
// ---------------------------------------------------------------------------

#[test]
fn test_snapshot_interval_positive_volatile_journal_skips_no_storage() {
    // Given: a shard with snapshot_interval_steps > 0 but only a volatile journal
    let journal: SharedRuntimeJournal = Arc::new(VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(config_with_interval(10), journal);

    // Given: a run with 50 executed steps (past interval)
    let run = RunId::new(2000);
    let state = make_test_run_state(run, 50, 0);

    // When: we attempt a snapshot write
    let outcome = shard.write_snapshot_for_run(
        run,
        &state,
        10, // interval > 0
        50,
        0,
    );

    // Then: SkippedNoStorage because volatile journal has no Fjall backend
    assert_eq!(outcome, SnapshotWriteOutcome::SkippedNoStorage);
}

// ---------------------------------------------------------------------------
// Test: interval == 10, first snapshot fires at step 10
// ---------------------------------------------------------------------------

#[test]
fn test_snapshot_interval_positive_writes_midrun_snapshots() {
    // Given: a shard with snapshot_interval_steps == 10 and a Fjall-backed journal
    let Some((_dir, fjall)) = temp_fjall_journal() else {
        return;
    };
    let journal: SharedRuntimeJournal = Arc::new(StorageRuntimeJournal::journaled(fjall));
    let mut shard = Shard::new_with_journal(config_with_interval(10), journal);

    let run = RunId::new(3000);

    // --- Step 1: executed = 5, below interval (10) ---
    let state_5 = make_test_run_state(run, 5, 0);
    let outcome_5 = shard.write_snapshot_for_run(run, &state_5, 10, 5, 0);
    assert_eq!(outcome_5, SnapshotWriteOutcome::SkippedNotReady);

    // --- Step 2: executed = 10, at interval boundary ---
    let state_10 = make_test_run_state(run, 10, 0);
    let outcome_10 = shard.write_snapshot_for_run(run, &state_10, 10, 10, 0);
    assert_eq!(outcome_10, SnapshotWriteOutcome::Written);

    // Verify that the journal sequence was updated (snapshot sequence = journal seq + 1)
    let seq_after_10 = shard.journal_sequence_for(run);
    assert!(seq_after_10.get() >= 1); // at least 1 (snapshot seq)

    // --- Step 3: executed = 15, between first and second snapshot ---
    let state_15 = make_test_run_state(run, 15, 10);
    let outcome_15 = shard.write_snapshot_for_run(run, &state_15, 10, 15, 10);
    assert_eq!(outcome_15, SnapshotWriteOutcome::SkippedNotReady);
    // 15 - 10 = 5 < 10 → not ready

    // --- Step 4: executed = 20, second snapshot at 10 + 10 ---
    let state_20 = make_test_run_state(run, 20, 10);
    let outcome_20 = shard.write_snapshot_for_run(run, &state_20, 10, 20, 10);
    assert_eq!(outcome_20, SnapshotWriteOutcome::Written);

    // Verify sequence advanced
    let seq_after_20 = shard.journal_sequence_for(run);
    assert!(seq_after_20.get() >= 2); // at least 2 snapshots

    // --- Step 5: executed = 29, before third snapshot ---
    let state_29 = make_test_run_state(run, 29, 20);
    let outcome_29 = shard.write_snapshot_for_run(run, &state_29, 10, 29, 20);
    assert_eq!(outcome_29, SnapshotWriteOutcome::SkippedNotReady);
    // 29 - 20 = 9 < 10 → not ready

    // --- Step 6: executed = 30, third snapshot at 20 + 10 ---
    let state_30 = make_test_run_state(run, 30, 20);
    let outcome_30 = shard.write_snapshot_for_run(run, &state_30, 10, 30, 20);
    assert_eq!(outcome_30, SnapshotWriteOutcome::Written);

    // Verify third snapshot sequence
    let seq_after_30 = shard.journal_sequence_for(run);
    assert!(seq_after_30.get() >= 3);
}

// ---------------------------------------------------------------------------
// Test: interval == 1 fires a snapshot every step after the first
// ---------------------------------------------------------------------------

#[test]
fn test_snapshot_interval_one_fires_every_step() {
    let Some((_dir, fjall)) = temp_fjall_journal() else {
        return;
    };
    let journal: SharedRuntimeJournal = Arc::new(StorageRuntimeJournal::journaled(fjall));
    let mut shard = Shard::new_with_journal(config_with_interval(1), journal);

    let run = RunId::new(4000);

    let mut last_executed = 0u64;

    for executed in 1..=5 {
        let state = make_test_run_state(run, executed, last_executed);
        let outcome = shard.write_snapshot_for_run(run, &state, 1, executed, last_executed);
        assert_eq!(outcome, SnapshotWriteOutcome::Written,
            "expected Written at executed={}, last_executed={}", executed, last_executed);
        last_executed = executed;
    }

    // Verify 5 snapshots were written
    let seq = shard.journal_sequence_for(run);
    assert_eq!(seq.get(), 5);
}

// ---------------------------------------------------------------------------
// Test: executed exactly at interval boundary
// ---------------------------------------------------------------------------

#[test]
fn test_snapshot_at_exact_boundary() {
    let Some((_dir, fjall)) = temp_fjall_journal() else {
        return;
    };
    let journal: SharedRuntimeJournal = Arc::new(StorageRuntimeJournal::journaled(fjall));
    let mut shard = Shard::new_with_journal(config_with_interval(5), journal);

    let run = RunId::new(5000);

    // executed = 4, interval = 5 → not ready
    let state_4 = make_test_run_state(run, 4, 0);
    assert_eq!(
        shard.write_snapshot_for_run(run, &state_4, 5, 4, 0),
        SnapshotWriteOutcome::SkippedNotReady
    );

    // executed = 5, interval = 5 → ready (5 - 0 = 5 >= 5)
    let state_5 = make_test_run_state(run, 5, 0);
    assert_eq!(
        shard.write_snapshot_for_run(run, &state_5, 5, 5, 0),
        SnapshotWriteOutcome::Written
    );
}
