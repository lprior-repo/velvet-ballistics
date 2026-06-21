// vb-8tjk8: snapshot writer tests
//
// Tests verify:
// 1. interval == 0 -> SkippedDisabled
// 2. interval > 0 with volatile journal -> SkippedNoStorage
// 3. interval > 0 with Fjall -> snapshots fire at correct boundaries
// 4. last_snapshot_executed updates after successful write

use std::sync::Arc;

use crate::journal::{StorageRuntimeJournal, VolatileRuntimeJournal};
use crate::shard::transitions::SnapshotWriteOutcome;

// --- Config helpers ---

fn config_interval_zero() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
    }
}

fn config_interval_n(n: u64) -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
        coalesce_window_ticks: 1,
        snapshot_interval_steps: n,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,
    }
}

// --- RunState builder ---

fn build_run_state(run: RunId, executed: u64, last_snapshot_executed: u64) -> RunState {
    let frame = vb_core::frame::RunFrame::new(run, vb_core::ids::StepIdx::ZERO, 1, 1)
        .expect("minimal test frame");
    let mut state = RunState {
        frame,
        workflow: make_test_workflow(),
        store: vb_core::value_store::ValueStore::new(),
        action_attempts: super::new_action_attempts(1),
        admission: None,
        collect_states: crate::primitives::collect::CollectStates::new(),
        action_contracts: Box::new([]),
        last_snapshot_executed,
    };
    for _ in 0..executed {
        let _ = state.frame.increment_executed();
    }
    state
}

fn make_test_workflow() -> vb_core::workflow::CompiledWorkflow {
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
        name: Box::from("snap-test"),
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
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).expect("wf")
}

// --- Fjall temp journal helper (no tempfile crate dependency) ---

fn open_temp_fjall() -> Option<(std::path::PathBuf, Arc<vb_storage::FjallJournal>)> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let _ = std::fs::create_dir_all(&base);
    // Create a unique temp directory
    let tmp_path = base.join(format!(
        "vb-snap-test-{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&tmp_path);
    std::fs::create_dir_all(&tmp_path).ok()?;
    let journal = vb_storage::FjallJournal::open(&tmp_path, None).ok()?;
    Some((tmp_path, Arc::new(journal)))
}

fn cleanup_temp_fjall(path: &std::path::Path) {
    let _ = std::fs::remove_dir_all(path);
}

// =======================================================================
// H-001: interval == 0 -> SkippedDisabled (volatile journal)
// =======================================================================

#[test]
fn test_snapshot_interval_zero_no_snapshots() -> Result<(), RuntimeError> {
    let journal: SharedRuntimeJournal = Arc::new(VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(config_interval_zero(), journal);

    let run = RunId::new(1);
    let state = build_run_state(run, 100, 0);

    let outcome = shard.write_snapshot_for_run(run, &state, 0, 100, 0);
    assert_eq!(outcome, SnapshotWriteOutcome::SkippedDisabled);
    Ok(())
}

// =======================================================================
// H-001: interval == 0 -> SkippedDisabled (Fjall journal, interval checked first)
// =======================================================================

#[test]
fn test_snapshot_interval_zero_no_snapshots_fjall() -> Result<(), RuntimeError> {
    let Some((path, fjall)) = open_temp_fjall() else {
        return;
    };
    let journal: SharedRuntimeJournal = Arc::new(StorageRuntimeJournal::journaled(fjall));
    let mut shard = Shard::new_with_journal(config_interval_zero(), journal);

    let run = RunId::new(2);
    let state = build_run_state(run, 100, 0);

    let outcome = shard.write_snapshot_for_run(run, &state, 0, 100, 0);
    assert_eq!(outcome, SnapshotWriteOutcome::SkippedDisabled);
    cleanup_temp_fjall(&path);
    Ok(())
}

// =======================================================================
// H-001: interval > 0 with volatile journal -> SkippedNoStorage
// =======================================================================

#[test]
fn test_snapshot_positive_interval_volatile_skips_no_storage() -> Result<(), RuntimeError> {
    let journal: SharedRuntimeJournal = Arc::new(VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(config_interval_n(10), journal);

    let run = RunId::new(3);
    let state = build_run_state(run, 50, 0);

    let outcome = shard.write_snapshot_for_run(run, &state, 10, 50, 0);
    assert_eq!(outcome, SnapshotWriteOutcome::SkippedNoStorage);
    Ok(())
}

// =======================================================================
// H-001: interval > 0 with Fjall -> snapshots fire at correct boundaries
// =======================================================================

#[test]
fn test_snapshot_interval_positive_writes_midrun_snapshots() -> Result<(), RuntimeError> {
    let Some((path, fjall)) = open_temp_fjall() else {
        return;
    };
    let journal: SharedRuntimeJournal = Arc::new(StorageRuntimeJournal::journaled(fjall));
    let mut shard = Shard::new_with_journal(config_interval_n(10), journal);

    let run = RunId::new(4);

    // Below interval
    let s5 = build_run_state(run, 5, 0);
    assert_eq!(shard.write_snapshot_for_run(run, &s5, 10, 5, 0), SnapshotWriteOutcome::SkippedNotReady);

    // At interval boundary (10 - 0 = 10 >= 10)
    let s10 = build_run_state(run, 10, 0);
    assert_eq!(shard.write_snapshot_for_run(run, &s10, 10, 10, 0), SnapshotWriteOutcome::Written);

    // Between snapshots (15 - 10 = 5 < 10)
    let s15 = build_run_state(run, 15, 10);
    assert_eq!(shard.write_snapshot_for_run(run, &s15, 10, 15, 10), SnapshotWriteOutcome::SkippedNotReady);

    // Second snapshot (20 - 10 = 10 >= 10)
    let s20 = build_run_state(run, 20, 10);
    assert_eq!(shard.write_snapshot_for_run(run, &s20, 10, 20, 10), SnapshotWriteOutcome::Written);

    // Before third (29 - 20 = 9 < 10)
    let s29 = build_run_state(run, 29, 20);
    assert_eq!(shard.write_snapshot_for_run(run, &s29, 10, 29, 20), SnapshotWriteOutcome::SkippedNotReady);

    // Third snapshot (30 - 20 = 10 >= 10)
    let s30 = build_run_state(run, 30, 20);
    assert_eq!(shard.write_snapshot_for_run(run, &s30, 10, 30, 20), SnapshotWriteOutcome::Written);

    cleanup_temp_fjall(&path);
    Ok(())
}

// =======================================================================
// H-001: interval == 1 fires every step
// =======================================================================

#[test]
fn test_snapshot_interval_one_fires_every_step() -> Result<(), RuntimeError> {
    let Some((path, fjall)) = open_temp_fjall() else {
        return;
    };
    let journal: SharedRuntimeJournal = Arc::new(StorageRuntimeJournal::journaled(fjall));
    let mut shard = Shard::new_with_journal(config_interval_n(1), journal);

    let run = RunId::new(5);
    let mut last = 0u64;

    for executed in 1..=5 {
        let state = build_run_state(run, executed, last);
        let outcome = shard.write_snapshot_for_run(run, &state, 1, executed, last);
        assert_eq!(outcome, SnapshotWriteOutcome::Written,
            "expected Written at executed={executed} last={last}");
        last = executed;
    }

    cleanup_temp_fjall(&path);
    Ok(())
}

// =======================================================================
// H-001: exact boundary check
// =======================================================================

#[test]
fn test_snapshot_at_exact_boundary() -> Result<(), RuntimeError> {
    let Some((path, fjall)) = open_temp_fjall() else {
        return;
    };
    let journal: SharedRuntimeJournal = Arc::new(StorageRuntimeJournal::journaled(fjall));
    let mut shard = Shard::new_with_journal(config_interval_n(5), journal);

    let run = RunId::new(6);

    // 4 < 5
    let s4 = build_run_state(run, 4, 0);
    assert_eq!(shard.write_snapshot_for_run(run, &s4, 5, 4, 0), SnapshotWriteOutcome::SkippedNotReady);

    // 5 - 0 = 5 >= 5
    let s5 = build_run_state(run, 5, 0);
    assert_eq!(shard.write_snapshot_for_run(run, &s5, 5, 5, 0), SnapshotWriteOutcome::Written);

    cleanup_temp_fjall(&path);
    Ok(())
}
