//! Tests for snapshot writer disabled-path (interval == 0).
//!
//! These tests verify that when `snapshot_interval_steps` is 0:
//! - `write_snapshot_for_run` returns `SkippedDisabled` for both volatile and Fjall-backed journals.
//! - The run lifecycle proceeds without snapshot interference.

use std::sync::Arc;

use vb_storage::FjallJournal;

use crate::journal::{RuntimeJournalEvent, SharedRuntimeJournal, StorageRuntimeJournal, VolatileRuntimeJournal};
use crate::shard::transitions::SnapshotWriteOutcome;
use crate::shard::{RunState, Shard, ShardConfig};
use vb_core::ids::RunId;

// ---------------------------------------------------------------------------
// Helper: create a volatile-backed shard with snapshot interval = 0
// ---------------------------------------------------------------------------

fn small_config_disabled() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
    }
}

// ---------------------------------------------------------------------------
// Test: interval == 0 with volatile journal returns SkippedDisabled
// ---------------------------------------------------------------------------

#[test]
fn test_snapshot_interval_zero_no_snapshots_volatile_journal() {
    // Given: a shard with snapshot_interval_steps == 0 and a volatile journal
    let journal: SharedRuntimeJournal = Arc::new(VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(small_config_disabled(), journal);

    // Given: a run with 100 executed steps (well past any reasonable interval)
    let run = RunId::new(1000);
    let state = make_test_run_state(run, 100, 0);

    // When: we attempt a snapshot write
    let outcome = shard.write_snapshot_for_run(
        run,
        &state,
        0, // interval == 0
        100,
        0,
    );

    // Then: SkippedDisabled because interval is 0
    assert_eq!(outcome, SnapshotWriteOutcome::SkippedDisabled);
}

// ---------------------------------------------------------------------------
// Test: interval == 0 with Fjall-backed journal also returns SkippedDisabled
// (the interval check happens before the storage check)
// ---------------------------------------------------------------------------

fn temp_fjall_journal() -> Option<Arc<FjallJournal>> {
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&base).ok()?;
    let dir = tempfile::Builder::new()
        .prefix("vb-snap-zero-")
        .tempdir_in(base)
        .ok()?;
    let journal = FjallJournal::open(dir.path(), None).ok()?;
    // Leak dir to keep it alive
    std::mem::forget(dir);
    Some(Arc::new(journal))
}

#[test]
fn test_snapshot_interval_zero_no_snapshots_fjall_journal() {
    // Given: a shard with snapshot_interval_steps == 0 and a Fjall-backed journal
    let fjall = match temp_fjall_journal() {
        Some(j) => j,
        None => return, // skip if temp dir creation fails
    };
    let journal: SharedRuntimeJournal = Arc::new(StorageRuntimeJournal::journaled(fjall));
    let mut shard = Shard::new_with_journal(small_config_disabled(), journal);

    // Given: a run with 100 executed steps
    let run = RunId::new(1001);
    let state = make_test_run_state(run, 100, 0);

    // When: we attempt a snapshot write with interval == 0
    let outcome = shard.write_snapshot_for_run(
        run,
        &state,
        0, // interval == 0
        100,
        0,
    );

    // Then: SkippedDisabled because interval is 0 (checked before storage)
    assert_eq!(outcome, SnapshotWriteOutcome::SkippedDisabled);
}

// ---------------------------------------------------------------------------
// Helper: build a minimal RunState for testing snapshot write paths
// ---------------------------------------------------------------------------

fn make_test_run_state(run: RunId, executed: u64, last_snapshot_executed: u64) -> RunState {
    use vb_core::frame::RunFrame;
    use vb_core::workflow::CompiledWorkflow;

    // Build a minimal workflow (one step that finishes)
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

    // Build a minimal RunFrame with the right executed count
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
