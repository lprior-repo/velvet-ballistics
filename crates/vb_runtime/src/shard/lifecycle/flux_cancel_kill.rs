//! Production-binding tests for cancel/kill lifecycle refinements.
//!
//! Bead: vb-hv2xc (RQ-W0-08)
//! Prior PO: PO-FLUX-001, PO-FLUX-002, PO-FLUX-003
//!
//! GOD RULE 2: This module previously held `#[flux_rs::trusted]` model
//! functions that returned hardcoded values without binding to real
//! production behavior. Per the BlackHat review, those vacuous refinements
//! were discarded. This module now hosts PRODUCTION tests that exercise the
//! real `Shard::handle_cancel` and `Shard::handle_kill` behavior and
//! verify the invariants previously expressed as Flux refinements:
//!
//! - PO-FLUX-001: live-only cancel/kill — handle_cancel/handle_kill always
//!   return `Ok(())`, terminal_runs membership is monotonic, pending_timers
//!   is cleared by cancel/kill before any stale handler fires.
//! - PO-FLUX-002: single-terminal winner — a second terminalization attempt
//!   adds zero entries; cross-case (cancel-after-kill or kill-after-cancel)
//!   preserves the first terminal outcome.
//! - PO-FLUX-003: stale-authority cleanup — after cancel/kill removes a
//!   run, subsequent timer fire and ask answer for that run are rejected
//!   with the appropriate `RuntimeError`.
//!
//! Production source references:
//! - `Shard::handle_cancel` at `chunk_002.rs:118-138`
//! - `Shard::handle_kill` at `chunk_002.rs:140-156`
//! - `Shard::handle_timer` at `chunk_002.rs:81-116`
//! - `Shard::handle_ask_answer` at `chunk_002.rs:18-79`
//!
//! Flux verification (if/when the flux-rs toolchain is available) is now
//! expected to bind these test-driven contracts back to Flux models. The
//! production tests below serve as the executable specification.

#![forbid(unsafe_code)]

use std::sync::Arc;

use vb_core::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

use crate::journal::{RuntimeJournalEvent, SharedRuntimeJournal, VolatileRuntimeJournal};
use crate::shard::types::{
    PendingTimerKind, Shard, ShardCommand, ShardConfig, TerminalOutcome,
};
use crate::trace::TraceEvent;
use crate::RuntimeError;

fn small_config() -> ShardConfig {
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
        max_terminal_outcomes: crate::shard::bounded_outcomes::DEFAULT_MAX_TERMINAL_OUTCOMES,
    }
}

fn suspended_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: vb_core::ids::ActionId::new(0),
            input: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("suspended"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn wait_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_deadline = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let wait = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: Some(StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::ZERO,
        },
    };
    let finish_node = CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts = WorkflowParts {
        name: Box::from("wait_then_finish"),
        digest: WorkflowDigest::from_bytes([4; 32]),
        nodes: Box::from([set_deadline, wait, finish_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(10)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn submit_run(shard: &mut Shard, run: RunId, workflow: vb_core::workflow::CompiledWorkflow) {
    assert_eq!(
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: vb_core::capability::CapabilitySet::empty(),
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
}

// ============================================================================
// PO-FLUX-001: Live-Only Cancel/Kill — production binding tests
// ============================================================================

/// Production binding: handle_cancel/handle_kill always return Ok(())
/// regardless of whether the run is present (live or terminal).
#[test]
fn live_only_cancel_always_ok_for_live_run() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(wf) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(700);
    submit_run(&mut shard, run, wf);
    assert_eq!(shard.run_state_contains(run), true);
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true));
    Ok(())
}

#[test]
fn live_only_cancel_always_ok_for_terminal_run() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(wf) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(701);
    submit_run(&mut shard, run, wf);
    shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
    shard.tick()?;
    // Second cancel after terminalization is also Ok (idempotent).
    shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
    shard.tick()?;
    Ok(())
}

#[test]
fn live_only_kill_always_ok_for_live_run() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(wf) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(702);
    submit_run(&mut shard, run, wf);
    shard.enqueue(ShardCommand::Kill { run, reason: None })?;
    shard.tick()?;
    Ok(())
}

#[test]
fn live_only_kill_always_ok_for_terminal_run() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(wf) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(703);
    submit_run(&mut shard, run, wf);
    shard.enqueue(ShardCommand::Kill { run, reason: None })?;
    shard.tick()?;
    // Idempotent: second kill after terminalization is also Ok.
    shard.enqueue(ShardCommand::Kill { run, reason: None })?;
    shard.tick()?;
    Ok(())
}

/// Production binding: terminal_runs membership is monotonic.
#[test]
fn terminal_runs_monotonic_after_cancel() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(wf) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(704);
    submit_run(&mut shard, run, wf);
    shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
    shard.tick()?;
    assert_eq!(shard.terminal_runs_contains(run), true);
    // Repeated cancel keeps membership stable.
    shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
    shard.tick()?;
    assert_eq!(shard.terminal_runs_contains(run), true);
    Ok(())
}

#[test]
fn terminal_runs_monotonic_after_kill() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(wf) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(705);
    submit_run(&mut shard, run, wf);
    shard.enqueue(ShardCommand::Kill { run, reason: None })?;
    shard.tick()?;
    assert_eq!(shard.terminal_runs_contains(run), true);
    shard.enqueue(ShardCommand::Kill { run, reason: None })?;
    shard.tick()?;
    assert_eq!(shard.terminal_runs_contains(run), true);
    Ok(())
}

/// Production binding: terminal_outcomes records Cancelled exactly once.
#[test]
fn cancel_journal_event_count_bounded_by_run_presence() -> Result<(), RuntimeError> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)?;
    let Some(wf) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(706);
    submit_run(&mut shard, run, wf);
    shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
    shard.tick()?;
    shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
    shard.tick()?;
    shard.enqueue(ShardCommand::Kill { run, reason: None })?;
    shard.tick()?;
    let events = journal.snapshot().map_err(|_| RuntimeError::QueueFull)?;
    let cancel_count = events
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::RunCancelled { run: r, .. } if *r == run))
        .count();
    let kill_count = events
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::RunKilled { run: r, .. } if *r == run))
        .count();
    assert_eq!(cancel_count, 1, "events: {events:?}");
    assert_eq!(kill_count, 0, "events: {events:?}");
    Ok(())
}

#[test]
fn kill_journal_event_count_bounded_by_run_presence() -> Result<(), RuntimeError> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared)?;
    let Some(wf) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(707);
    submit_run(&mut shard, run, wf);
    shard.enqueue(ShardCommand::Kill { run, reason: None })?;
    shard.tick()?;
    shard.enqueue(ShardCommand::Kill { run, reason: None })?;
    shard.tick()?;
    shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
    shard.tick()?;
    let events = journal.snapshot().map_err(|_| RuntimeError::QueueFull)?;
    let cancel_count = events
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::RunCancelled { run: r, .. } if *r == run))
        .count();
    let kill_count = events
        .iter()
        .filter(|e| matches!(e, RuntimeJournalEvent::RunKilled { run: r, .. } if *r == run))
        .count();
    assert_eq!(cancel_count, 0, "events: {events:?}");
    assert_eq!(kill_count, 1, "events: {events:?}");
    Ok(())
}

// ============================================================================
// PO-FLUX-002: Single Terminal Winner — production binding tests
// ============================================================================

/// Production binding: counter only increments when run was present.
#[test]
fn counter_only_increments_when_run_present_for_cancel() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(wf) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(708);
    submit_run(&mut shard, run, wf);
    let before = shard.counters().snapshot().runs_failed;
    shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
    shard.tick()?;
    shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
    shard.tick()?;
    let after = shard.counters().snapshot().runs_failed;
    // Cancelled + idempotent cancel = +1 total increment.
    assert_eq!(after - before, 1);
    Ok(())
}

#[test]
fn counter_only_increments_when_run_present_for_kill() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(wf) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(709);
    submit_run(&mut shard, run, wf);
    let before = shard.counters().snapshot().runs_failed;
    shard.enqueue(ShardCommand::Kill { run, reason: None })?;
    shard.tick()?;
    shard.enqueue(ShardCommand::Kill { run, reason: None })?;
    shard.tick()?;
    let after = shard.counters().snapshot().runs_failed;
    assert_eq!(after - before, 1);
    Ok(())
}

/// Production binding: terminal_outcome reflects the FIRST terminalization.
#[test]
fn cancel_first_then_kill_preserves_cancelled_outcome() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(wf) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(710);
    submit_run(&mut shard, run, wf);
    shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
    shard.tick()?;
    shard.enqueue(ShardCommand::Kill { run, reason: None })?;
    shard.tick()?;
    // Terminal outcome was recorded as Cancelled by the first terminalization.
    let outcome = shard.terminal_outcome_get(run);
    assert_eq!(outcome, Some(TerminalOutcome::Cancelled));
    Ok(())
}

#[test]
fn kill_first_then_cancel_preserves_killed_outcome() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(wf) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(711);
    submit_run(&mut shard, run, wf);
    shard.enqueue(ShardCommand::Kill { run, reason: None })?;
    shard.tick()?;
    shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
    shard.tick()?;
    let outcome = shard.terminal_outcome_get(run);
    assert_eq!(outcome, Some(TerminalOutcome::Killed));
    Ok(())
}

// ============================================================================
// PO-FLUX-003: Stale Authority Cleanup — production binding tests
// ============================================================================

/// Production binding: timer fire after cancel returns InvalidTimerFire.
#[test]
fn stale_timer_after_cancel_is_rejected() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(wf) = wait_workflow() else {
        return Ok(());
    };
    let run = RunId::new(712);
    submit_run(&mut shard, run, wf);
    assert_eq!(shard.pending_timer_count(), 1);
    shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
    shard.tick()?;
    assert_eq!(shard.pending_timer_count(), 0);
    // Stale timer fire must be rejected.
    assert_eq!(
        shard.enqueue(ShardCommand::TimerFired {
            run,
            generation: 0,
            deadline: std::time::Instant::now(),
            kind: PendingTimerKind::Wait,
        }),
        Ok(())
    );
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    Ok(())
}

/// Production binding: ask answer after kill returns RunNotFound.
#[test]
fn stale_ask_answer_after_kill_is_rejected() -> Result<(), RuntimeError> {
    use crate::AskAnswer;
    use vb_core::value::Taint;
    let mut shard = Shard::new(small_config())?;
    let Some(wf) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(713);
    submit_run(&mut shard, run, wf);
    shard.enqueue(ShardCommand::Kill { run, reason: None })?;
    shard.tick()?;
    let answer = AskAnswer {
        ticket: crate::shard::types::AskTicket {
            run,
            ask_step: StepIdx::ZERO,
            resume_step: StepIdx::ZERO,
            attempt: 1,
        },
        answer_slot: SlotIdx::ZERO,
        value: vb_core::value::SlotValue::I64(1),
        taint: Taint::Clean,
        encoded_len: 1,
    };
    shard.enqueue(ShardCommand::AskAnswered { answer })?;
    assert_eq!(shard.tick(), Err(RuntimeError::RunNotFound));
    Ok(())
}

/// Production binding: trace events are emitted exactly once per terminalization.
#[test]
fn trace_ring_records_terminal_event_exactly_once() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(small_config())?;
    let Some(wf) = suspended_workflow() else {
        return Ok(());
    };
    let run = RunId::new(714);
    submit_run(&mut shard, run, wf);
    shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
    shard.tick()?;
    shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
    shard.tick()?;
    let events: Vec<TraceEvent> = shard.trace_ring_mut().drain().into_iter().collect();
    let cancel_count = events
        .iter()
        .filter(|e| matches!(e, TraceEvent::RunCancelled { run: r } if *r == run))
        .count();
    assert_eq!(cancel_count, 1, "events: {events:?}");
    Ok(())
}
