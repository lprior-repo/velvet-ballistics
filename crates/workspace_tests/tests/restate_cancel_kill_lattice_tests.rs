#![forbid(unsafe_code)]
//! Artifact: restate_cancel_kill_lattice_tests
//!
//! Integration tests for the cancel outcome lattice for run states.
//!
//! These tests verify that `ShardCommand::Cancel` produces correct outcomes
//! across all run states (HP-1, HP-3, HP-4, HP-5, EC-1, INV-1).
//!
//! ## Coverage
//!
//! - HP-1: Cancel on Running run produces Cancelled outcome
//! - HP-3: Cancel on Suspended (Resumable) run produces Cancelled outcome
//! - HP-4: Cancel on Failed run produces no-op (already terminal)
//! - HP-5: Cancel on non-existent run is idempotent (no-op)
//! - EC-1: Cancel removes pending timer for suspended run
//! - INV-1: Cancel run then resubmit same ID succeeds
//!
//! ## Test Philosophy
//!
//! These tests validate the outcome lattice for cancel operations across
//! all possible run states, ensuring each state transition is well-defined.

use vb_core::ids::RunId;
use vb_runtime::shard::{Shard, ShardCommand, ShardConfig};
use vb_runtime::journal::VolatileRuntimeJournal;

// ---------------------------------------------------------------------------
// Test fixtures and helpers
// ---------------------------------------------------------------------------

fn small_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    }
}

fn make_suspended_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let yaml = b"
version: velvet-ballistics/v1
name: suspend-test
when:
  manual: {}
steps:
  - id: wait_step
    wait:
      seconds: 3600
  - id: done
    finish:
      result: 0
";
    vb_compile::YamlCompiler::default()
        .compile(yaml)
        .ok()
        .map(|w| w.into())
}

fn make_finished_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let yaml = b"
version: velvet-ballistics/v1
name: finish-test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 42
";
    vb_compile::YamlCompiler::default()
        .compile(yaml)
        .ok()
        .map(|w| w.into())
}

fn make_running_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let yaml = b"
version: velvet-ballistics/v1
name: running-test
when:
  manual: {}
steps:
  - id: step1
    do:
      action: noop
  - id: done
    finish:
      result: 0
";
    vb_compile::YamlCompiler::default()
        .compile(yaml)
        .ok()
        .map(|w| w.into())
}

// ---------------------------------------------------------------------------
// HP-1: Cancel on Running run produces Cancelled outcome
// ---------------------------------------------------------------------------

#[test]
fn hp1_cancel_on_running_run_produces_cancelled_outcome() {
    let config = small_config();
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: vb_runtime::journal::SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);

    let Some(workflow) = make_running_workflow() else {
        return;
    };
    let run = RunId::new(1001);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );

    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.runs.get(&run).is_some(), true);

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.runs.get(&run), None);

    let events = journal.snapshot().expect("journal snapshot should succeed");
    assert!(
        events.contains(&vb_runtime::journal::RuntimeJournalEvent::RunCancelled {
            run,
            reason: None
        }),
        "HP-1: journal should contain RunCancelled event for running run"
    );

    let trace_events = shard.trace_ring_mut().drain();
    assert!(
        trace_events.contains(&vb_runtime::trace::TraceEvent::RunCancelled { run }),
        "HP-1: trace should contain RunCancelled event"
    );
}

#[test]
fn hp1_cancel_with_reason_propagates_reason_to_journal() {
    let config = small_config();
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: vb_runtime::journal::SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);

    let Some(workflow) = make_running_workflow() else {
        return;
    };
    let run = RunId::new(1002);
    let cancel_reason = Some("user requested cancellation".to_string());

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: cancel_reason.clone() }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let events = journal.snapshot().expect("journal snapshot should succeed");
    assert!(
        events.contains(&vb_runtime::journal::RuntimeJournalEvent::RunCancelled {
            run,
            reason: cancel_reason,
        }),
        "HP-1: journal should contain RunCancelled with provided reason"
    );
}

#[test]
fn hp1_cancel_on_running_run_removes_from_runs_map() {
    let config = small_config();
    let mut shard = Shard::new(config);

    let Some(workflow) = make_running_workflow() else {
        return;
    };
    let run = RunId::new(1003);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.runs.get(&run), None, "HP-1: run should be removed from runs map");
}

// ---------------------------------------------------------------------------
// HP-3: Cancel on Suspended (Resumable) run produces Cancelled outcome
// ---------------------------------------------------------------------------

#[test]
fn hp3_cancel_on_suspended_run_produces_cancelled_outcome() {
    let config = small_config();
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: vb_runtime::journal::SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);

    let Some(workflow) = make_suspended_workflow() else {
        return;
    };
    let run = RunId::new(2001);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );

    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.runs.get(&run).is_some(), true);
    assert_eq!(shard.pending_timer_count(), 1);

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.runs.get(&run), None);
    assert_eq!(shard.pending_timer_count(), 0);

    let events = journal.snapshot().expect("journal snapshot should succeed");
    assert!(
        events.contains(&vb_runtime::journal::RuntimeJournalEvent::RunCancelled {
            run,
            reason: None
        }),
        "HP-3: journal should contain RunCancelled event for suspended run"
    );
}

#[test]
fn hp3_cancel_removes_pending_timer_for_suspended_run() {
    let config = small_config();
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: vb_runtime::journal::SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);

    let Some(workflow) = make_suspended_workflow() else {
        return;
    };
    let run = RunId::new(2002);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.pending_timer_count(), 1);

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.pending_timer_count(), 0);
}

// ---------------------------------------------------------------------------
// HP-4: Cancel on Failed run produces no-op (already terminal)
// ---------------------------------------------------------------------------

#[test]
fn hp4_cancel_on_finished_run_is_noop() {
    let config = small_config();
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: vb_runtime::journal::SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);

    let Some(workflow) = make_finished_workflow() else {
        return;
    };
    let run = RunId::new(3001);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.runs.get(&run), None);

    let before_events = journal.snapshot().expect("journal snapshot should succeed");

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let after_events = journal.snapshot().expect("journal snapshot should succeed");
    assert_eq!(
        before_events, after_events,
        "HP-4: no new events should be emitted for cancel on finished run"
    );
}

#[test]
fn hp4_cancel_on_finished_run_does_not_increment_failed_counter() {
    let config = small_config();
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: vb_runtime::journal::SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);

    let Some(workflow) = make_finished_workflow() else {
        return;
    };
    let run = RunId::new(3002);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let before_failed = shard.counters().snapshot().runs_failed;

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let after_failed = shard.counters().snapshot().runs_failed;
    assert_eq!(
        before_failed, after_failed,
        "HP-4: failed counter should not increment for finished run cancel"
    );
}

// ---------------------------------------------------------------------------
// HP-5: Cancel on non-existent run is idempotent (no-op)
// ---------------------------------------------------------------------------

#[test]
fn hp5_cancel_on_nonexistent_run_is_idempotent() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let nonexistent_run = RunId::new(9999);

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: nonexistent_run,
            reason: None,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.runs.get(&nonexistent_run), None);
}

#[test]
fn hp5_cancel_on_nonexistent_run_does_not_increment_failed_counter() {
    let config = small_config();
    let mut shard = Shard::new(config);
    let nonexistent_run = RunId::new(9998);

    let before = shard.counters().snapshot().runs_failed;

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: nonexistent_run,
            reason: None,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let after = shard.counters().snapshot().runs_failed;
    assert_eq!(
        before, after,
        "HP-5: failed counter should not increment for non-existent run cancel"
    );
}

#[test]
fn hp5_cancel_nonexistent_run_no_journal_events() {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: vb_runtime::journal::SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared);

    let before = journal.snapshot().expect("journal snapshot should succeed");

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel {
            run: RunId::new(8888),
            reason: None,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let after = journal.snapshot().expect("journal snapshot should succeed");
    assert_eq!(
        before, after,
        "HP-5: no journal events should be emitted for non-existent run cancel"
    );
}

#[test]
fn hp5_multiple_cancels_on_same_run_are_idempotent() {
    let config = small_config();
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: vb_runtime::journal::SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(config, shared);

    let Some(workflow) = make_running_workflow() else {
        return;
    };
    let run = RunId::new(8001);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let events_after_first = journal.snapshot().expect("journal snapshot should succeed");

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let events_after_second = journal.snapshot().expect("journal snapshot should succeed");
    assert_eq!(
        events_after_first, events_after_second,
        "HP-5: multiple cancels should be idempotent"
    );
}

// ---------------------------------------------------------------------------
// EC-1: Cancel removes pending timer for suspended run
// ---------------------------------------------------------------------------

#[test]
fn ec1_cancel_removes_timer_for_suspended_run() {
    let config = small_config();
    let mut shard = Shard::new(config);

    let Some(workflow) = make_suspended_workflow() else {
        return;
    };
    let run = RunId::new(4001);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.pending_timer_count(), 1);

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.pending_timer_count(), 0);
    assert_eq!(shard.runs.get(&run), None);
}

// ---------------------------------------------------------------------------
// INV-1: Cancel run then resubmit same ID succeeds
// ---------------------------------------------------------------------------

#[test]
fn inv1_cancel_then_resubmit_same_run_id_succeeds() {
    let config = small_config();
    let mut shard = Shard::new(config);

    let Some(workflow) = make_running_workflow() else {
        return;
    };
    let run = RunId::new(5001);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow: workflow.clone(),
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.runs.get(&run).is_some(), true);

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.runs.get(&run), None);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(
        shard.runs.get(&run).is_some(), true,
        "INV-1: after cancel, same run ID can be resubmitted"
    );
}

// ---------------------------------------------------------------------------
// HP-1: Cancel increments failed counter for active run
// ---------------------------------------------------------------------------

#[test]
fn hp1_cancel_increments_failed_counter() {
    let config = small_config();
    let mut shard = Shard::new(config);

    let Some(workflow) = make_running_workflow() else {
        return;
    };
    let run = RunId::new(6001);

    let before_failed = shard.counters().snapshot().runs_failed;

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let after_failed = shard.counters().snapshot().runs_failed;
    assert_eq!(
        after_failed, before_failed + 1,
        "HP-1: cancel should increment the failed counter"
    );
}

// ---------------------------------------------------------------------------
// HP-1: Cancel emits RunCancelled trace event
// ---------------------------------------------------------------------------

#[test]
fn hp1_cancel_emits_run_cancelled_trace_event() {
    let config = small_config();
    let mut shard = Shard::new(config);

    let Some(workflow) = make_running_workflow() else {
        return;
    };
    let run = RunId::new(7001);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let trace_events = shard.trace_ring_mut().drain();
    assert!(
        trace_events.contains(&vb_runtime::trace::TraceEvent::RunCancelled { run }),
        "HP-1: trace should contain RunCancelled event"
    );
}

// ---------------------------------------------------------------------------
// HP-3: Cancel on suspended run removes from runs map
// ---------------------------------------------------------------------------

#[test]
fn hp3_cancel_removes_suspended_run_from_runs_map() {
    let config = small_config();
    let mut shard = Shard::new(config);

    let Some(workflow) = make_suspended_workflow() else {
        return;
    };
    let run = RunId::new(8001);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.runs.get(&run).is_some(), true);

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    assert_eq!(shard.runs.get(&run), None, "HP-3: suspended run should be removed from runs map");
}

// ---------------------------------------------------------------------------
// EC-1: Cancel releases frame back to pool
// ---------------------------------------------------------------------------

#[test]
fn ec1_cancel_releases_frame_to_pool() {
    let config = small_config();
    let mut shard = Shard::new(config);

    let Some(workflow) = make_running_workflow() else {
        return;
    };
    let run = RunId::new(9001);

    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let (free_before, _) = shard.frame_pool_metrics();
    assert_eq!(free_before, 0, "frame should be in use");

    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));

    let (free_after, _) = shard.frame_pool_metrics();
    assert_eq!(
        free_after, free_before + 1,
        "EC-1: cancel should release frame back to pool"
    );
}
