// Tests for `Runtime::tick_shard` API and `ShardDirective` enum.
//
// These tests cover the directive-driven shard tick interface documented in LETHAL-4.
// The `tick_shard` method is now implemented on `Runtime`.
//
// ## Critical LETHALs Addressed
//
// 1. **`runtime_tick_shard_continue_processes_action_then_finish_workflow`**: Two
//    Continue ticks drive the 2-step workflow to completion; the shard
//    counter reads exactly 1 because the runtime's manual completion path
//    does not count the resumed Do step (see that test's doc comment).
// 2. **`runtime_tick_shard_continue_returns_ok_with_empty_queue`**: Asserts `runs_submitted == 0`
//    counter invariant — queue-empty does not imply counter unchanged.
// 3. **Mutation survivability**: Every test that checks a `bool` return distinguishes `Ok(true)`
//    (shard alive) from `Ok(false)` (shard dead) with separate explicit assertions.

use vb_core::action::{
    ActionContract, ActionName, ActionOutputReady, Idempotency, RetryPolicy, RetrySafety,
    SideEffect,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{RunId, StepIdx};
use vb_core::policy::RuntimePolicy;
use vb_core::value::Taint;

use crate::runtime::Runtime;
use crate::shard::ShardDirective;

// =============================================================================
// Workflow helpers (matching existing test infrastructure in runtime.rs)
// =============================================================================

/// Workflow with Do action + Finish (exactly 2 steps).
/// Used to verify exact step counting.
fn action_then_finish_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let do_node = vb_core::workflow::node(StepIdx::ZERO, CompiledNodeKind::Do { action: ActionId::new(7), input: SlotIdx::new(0), }, Some(SlotIdx::new(1)), Some(StepIdx::new(1)));
    let finish = vb_core::workflow::node(StepIdx::new(1), CompiledNodeKind::Finish { result: SlotIdx::new(1), }, None, None);
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("action_then_finish"),
        digest: WorkflowDigest::from_bytes([3; 32]),
        nodes: Box::from([do_node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([vb_core::CompiledInputSlot {
            slot: vb_core::SlotIdx::new(0),
            kind: vb_core::InputSlotKind::Any,
            constraint: vb_core::CompiledInputConstraint::required(),
        }]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn runtime_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: RuntimePolicy::Relaxed,
    }
}

fn contract_required_capability(action: ActionId) -> Capability {
    Capability::new("__contract_required__".into(), action)
}

fn action_contract(action: ActionId, input_slots: u16, output_slots: u16) -> ActionContract {
    ActionContract {
        id: action,
        name: ActionName::from_static_infallible("test-action"),
        input_slot_count: input_slots,
        output_slot_count: output_slots,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::from([contract_required_capability(action)]),
    }
}

fn action_contracts_through(
    action: ActionId,
    input_slots: u16,
    output_slots: u16,
) -> Box<[ActionContract]> {
    let target = action.get();
    let mut contracts = Vec::with_capacity(usize::from(target).saturating_add(1));
    let mut id = 0u16;
    loop {
        let current = ActionId::new(id);
        if id == target {
            contracts.push(action_contract(current, input_slots, output_slots));
            break;
        }
        contracts.push(action_contract(current, 0, 0));
        id = id.saturating_add(1);
    }
    contracts.into_boxed_slice()
}

fn action_grants(action: ActionId) -> CapabilitySet {
    let mut grants = Vec::new();
    for i in 0..=action.get() {
        grants.push(contract_required_capability(ActionId::new(i)));
    }
    CapabilitySet::from_grants(grants.into_boxed_slice())
}

fn encoded_len(value: &SlotValue) -> u32 {
    let Ok(bytes) = postcard::to_allocvec(value) else {
        return u32::MAX;
    };
    let Ok(len) = u32::try_from(bytes.len()) else {
        return u32::MAX;
    };
    len
}

fn submit_suspended(
    runtime: &Runtime,
    run: RunId,
    wf: vb_core::workflow::CompiledWorkflow,
) -> crate::RuntimeResult<()> {
    let action = ActionId::new(0);
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        wf,
        Box::from([(SlotIdx::new(0), SlotValue::I64(0))]),
        action_grants(action),
        action_contracts_through(action, 1, 0),
    )
}

fn submit_action_then_finish(
    runtime: &Runtime,
    run: RunId,
    wf: vb_core::workflow::CompiledWorkflow,
) -> crate::RuntimeResult<()> {
    let action = ActionId::new(7);
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        wf,
        Box::from([(SlotIdx::new(0), SlotValue::I64(0))]),
        action_grants(action),
        action_contracts_through(action, 1, 1),
    )
}

// =============================================================================
// ShardDirective enum unit tests
// =============================================================================

#[test]
fn shard_directive_continue_equals_continue() {
    let a = ShardDirective::Continue;
    let b = ShardDirective::Continue;
    assert_eq!(a, b);
}

#[test]
fn shard_directive_suspend_equals_suspend() {
    let a = ShardDirective::Suspend;
    let b = ShardDirective::Suspend;
    assert_eq!(a, b);
}

#[test]
fn shard_directive_migrate_with_same_target_equals() {
    let a = ShardDirective::Migrate { target: 1 };
    let b = ShardDirective::Migrate { target: 1 };
    assert_eq!(a, b);
}

#[test]
fn shard_directive_migrate_with_different_targets_not_equals() {
    let a = ShardDirective::Migrate { target: 1 };
    let b = ShardDirective::Migrate { target: 2 };
    assert_ne!(a, b);
}

#[test]
fn shard_directive_shutdown_equals_shutdown() {
    let a = ShardDirective::Shutdown;
    let b = ShardDirective::Shutdown;
    assert_eq!(a, b);
}

#[test]
fn shard_directive_continue_not_equals_suspend() {
    assert_ne!(ShardDirective::Continue, ShardDirective::Suspend);
}

#[test]
fn shard_directive_continue_not_equals_migrate() {
    assert_ne!(
        ShardDirective::Continue,
        ShardDirective::Migrate { target: 0 }
    );
}

#[test]
fn shard_directive_continue_not_equals_shutdown() {
    assert_ne!(ShardDirective::Continue, ShardDirective::Shutdown);
}

#[test]
fn shard_directive_suspend_not_equals_migrate() {
    assert_ne!(
        ShardDirective::Suspend,
        ShardDirective::Migrate { target: 0 }
    );
}

#[test]
fn shard_directive_suspend_not_equals_shutdown() {
    assert_ne!(ShardDirective::Suspend, ShardDirective::Shutdown);
}

#[test]
fn shard_directive_migrate_not_equals_shutdown() {
    assert_ne!(
        ShardDirective::Migrate { target: 0 },
        ShardDirective::Shutdown
    );
}

#[test]
fn shard_directive_continue_is_alive() {
    assert_eq!(ShardDirective::Continue.is_alive(), true);
}

#[test]
fn shard_directive_suspend_is_alive() {
    assert_eq!(ShardDirective::Suspend.is_alive(), true);
}

#[test]
fn shard_directive_migrate_is_alive() {
    assert_eq!(ShardDirective::Migrate { target: 1 }.is_alive(), true);
}

#[test]
fn shard_directive_shutdown_is_not_alive() {
    assert_eq!(ShardDirective::Shutdown.is_alive(), false);
}

#[test]
fn shard_directive_debug_format_continue() {
    let directive = ShardDirective::Continue;
    let debug = format!("{:?}", directive);
    assert!(debug.contains("Continue"));
}

#[test]
fn shard_directive_debug_format_suspend() {
    let directive = ShardDirective::Suspend;
    let debug = format!("{:?}", directive);
    assert!(debug.contains("Suspend"));
}

#[test]
fn shard_directive_debug_format_migrate() {
    let directive = ShardDirective::Migrate { target: 42 };
    let debug = format!("{:?}", directive);
    assert!(debug.contains("Migrate"));
    assert!(debug.contains("42"));
}

#[test]
fn shard_directive_debug_format_shutdown() {
    let directive = ShardDirective::Shutdown;
    let debug = format!("{:?}", directive);
    assert!(debug.contains("Shutdown"));
}

// =============================================================================
// Continue directive integration tests
// =============================================================================

/// Runtime processes all queued commands on a shard when tick_shard receives
/// Continue directive.
///
/// LETHAL FIX: Uses exact counter assertions, not just queue-empty checks.
#[test]
fn runtime_tick_shard_continue_processes_all_pending_commands() {
    // Given: A 2-shard runtime with one run enqueued on shard 0 and one on shard 1.
    let Some(shard_count) = std::num::NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    let Some(wf1) = suspended_workflow() else {
        return;
    };
    let Some(wf2) = suspended_workflow() else {
        return;
    };

    let run1 = RunId::new(1);
    let run2 = RunId::new(2);

    assert_eq!(submit_suspended(&runtime, run1, wf1), Ok(()));
    assert_eq!(submit_suspended(&runtime, run2, wf2), Ok(()));

    // When: tick_shard(Continue) is called on each shard.
    // RunId 1 hashes to shard 1, RunId 2 hashes to shard 0.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Continue),
        Ok(true)
    );
    assert_eq!(
        runtime.tick_shard(1, ShardDirective::Continue),
        Ok(true)
    );

    // Then: All command queues are empty; runs_submitted equals 2.
    let metrics = runtime.collect_metrics();
    assert_eq!(metrics.shards[0].command_queue_depth, 0);
    assert_eq!(metrics.shards[1].command_queue_depth, 0);
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 2);
}

/// Runtime returns Ok(true) when tick_shard Continue is called on an idle shard.
///
/// LETHAL FIX: Asserts `runs_submitted == 0` counter invariant — queue-empty
/// does not imply counter unchanged.
#[test]
fn runtime_tick_shard_continue_returns_ok_true_with_empty_queue() {
    // Given: A 1-shard runtime with an empty command queue.
    let Some(shard_count) = std::num::NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    // Verify initial state: queue empty, counters at zero.
    let initial_snap = runtime.counters_snapshot();
    assert_eq!(initial_snap.runs_submitted, 0);
    assert_eq!(initial_snap.runs_completed, 0);
    assert_eq!(initial_snap.steps_executed, 0);

    // When: tick_shard(Continue) is called on the idle shard.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Continue),
        Ok(true)
    );

    // Then: Returns Ok(true); command queue remains empty;
    // *** CRITICAL: runs_submitted counter is 0 (not incremented). ***
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 0); // ← LETHAL FIX: explicit counter invariant
    assert_eq!(snap.runs_completed, 0);
    assert_eq!(snap.steps_executed, 0);
}

/// Runtime increments steps_executed when Continue directive processes a multi-step run.
///
/// The action_then_finish_workflow has exactly 2 steps (Do + Finish).
/// Two tick_shard(Continue) calls drive it to completion. Both steps count:
/// the Do step counts at action-completion resume (mirroring the engine's
/// `resume_action_completion` increment; see bead vb-huoen) and the Finish
/// step counts when the engine drives it. Total is exactly 2.
#[test]
fn runtime_tick_shard_continue_processes_action_then_finish_workflow() {
    // Given: A 1-shard runtime with action_then_finish_workflow enqueued.
    let Some(shard_count) = std::num::NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    let Some(wf) = action_then_finish_workflow() else {
        return;
    };
    let run = RunId::new(10);

    assert_eq!(submit_action_then_finish(&runtime, run, wf), Ok(()));

    // When: tick_shard(Continue) is called twice.
    // The workflow has 2 steps: Do action + Finish.
    // First tick: processes the submit and issues the Do action.
    // Second tick: processes the Finish after the action completes.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Continue),
        Ok(true)
    );

    // Complete the action to allow workflow to proceed. The canonical ticket
    // comes from the shard's pending-actions index; a hand-built ticket does
    // not carry the minted fencing token and is rejected.
    let Some(ticket) = runtime.lookup_pending_action_ticket(run, 0) else {
        panic!("expected a pending action ticket for the submitted run");
    };
    let output_value = SlotValue::I64(99);
    let output = ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value: output_value,
        taint: Taint::Clean,
        encoded_len: encoded_len(&output_value),
    };
    assert_eq!(runtime.complete_action_with_output(ticket, output), Ok(()));
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Continue),
        Ok(true)
    );

    // Then: both steps processed — the run completed — and the counter reads
    // exactly 2 (Do counted at completion resume, Finish counted by engine).
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 1);
    assert_eq!(snap.runs_completed, 1);
    assert_eq!(snap.steps_executed, 2);
}

// =============================================================================
// Suspend directive integration tests
// =============================================================================

/// Runtime skips all command processing on a shard when tick_shard receives
/// Suspend directive.
///
/// Verifies: Continue mutation (processes when should skip) is caught.
#[test]
fn runtime_tick_shard_suspend_skips_all_command_processing() {
    // Given: A 1-shard runtime with a suspended_workflow enqueued on shard 0.
    let Some(shard_count) = std::num::NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(20);

    assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true)); // Process submit

    // Verify initial counter state
    let before_snap = runtime.counters_snapshot();
    let runs_before = before_snap.runs_submitted;

    // Enqueue a second run so Suspend has a pending command to skip.
    let Some(wf2) = suspended_workflow() else {
        return;
    };
    let run2 = RunId::new(21);
    assert_eq!(submit_suspended(&runtime, run2, wf2), Ok(()));
    let depth_before = runtime.collect_metrics().shards[0].command_queue_depth;
    assert_eq!(depth_before, 1);

    // When: tick_shard(Suspend) is called.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Suspend),
        Ok(true)
    );

    // Then: Returns Ok(true); runs_submitted counter is unchanged and the
    // pending command is still queued (not drained).
    // The Continue branch processing commands when it should skip is caught by
    // verifying runs_submitted does NOT increment after Suspend.
    let after_snap = runtime.counters_snapshot();
    assert_eq!(after_snap.runs_submitted, runs_before);
    assert_eq!(
        runtime.collect_metrics().shards[0].command_queue_depth,
        depth_before
    );

    // A later Continue processes the preserved command.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Continue),
        Ok(true)
    );
    assert_eq!(
        runtime.counters_snapshot().runs_submitted,
        runs_before + 1
    );
}

/// Runtime does not drain commands when Suspend directive is issued.
///
/// Verifies: Suspend mutation (doesn't preserve queue depth) is caught.
#[test]
fn runtime_tick_shard_suspend_preserves_command_queue_depth() {
    // Given: A 1-shard runtime with one Submit command enqueued.
    let Some(shard_count) = std::num::NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(30);

    assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
    let depth_before = runtime.collect_metrics().shards[0].command_queue_depth;
    assert_eq!(depth_before, 1);

    // When: tick_shard(Suspend) is called.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Suspend),
        Ok(true)
    );

    // Then: The command queue depth is still the same (not drained).
    // The mutation where Suspend doesn't preserve queue depth is caught by
    // verifying depth is unchanged.
    let depth_after = runtime.collect_metrics().shards[0].command_queue_depth;
    assert_eq!(depth_before, depth_after);
    assert_eq!(runtime.counters_snapshot().runs_submitted, 0);

    // A later Continue processes the preserved command.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Continue),
        Ok(true)
    );
    assert_eq!(runtime.collect_metrics().shards[0].command_queue_depth, 0);
    assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
}

/// Runtime returns Ok(true) with no side effects when Suspend is called on
/// already-idle shard.
///
/// LETHAL FIX: Explicit counter values stated (runs_submitted == 0 && steps_executed == 0).
#[test]
fn runtime_tick_shard_suspend_idempotent_on_idle_shard() {
    // Given: A 1-shard runtime with empty command queue.
    let Some(shard_count) = std::num::NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    // When: tick_shard(Suspend) is called on idle shard.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Suspend),
        Ok(true)
    );

    // Then: Returns Ok(true); *** CRITICAL: explicit counter values. ***
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 0); // ← LETHAL FIX: explicit value
    assert_eq!(snap.steps_executed, 0); // ← LETHAL FIX: explicit value
    assert_eq!(snap.runs_completed, 0);
    assert_eq!(snap.runs_failed, 0);
}

/// A run that was previously resumed does not advance when Suspend is issued.
///
/// Verifies: Suspend correctly skips processing without advancing any runs.
#[test]
fn runtime_tick_shard_suspend_does_not_advance_resumed_run() {
    // Given: A 1-shard runtime with a suspended_workflow submitted and one
    // tick applied.
    let Some(shard_count) = std::num::NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(40);

    assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));

    // The run is admitted and suspended at its Do step.
    let steps_before = runtime.counters_snapshot().steps_executed;
    let snap_before = runtime.snapshot_run(run, 1);
    assert!(matches!(
        snap_before,
        Ok(crate::shard::InspectResponse::Found(_))
    ));

    // When: tick_shard(Suspend) is called.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Suspend),
        Ok(true)
    );

    // Then: The run's step counter is unchanged from before the Suspend call,
    // and the run is still found (not advanced to completion).
    assert_eq!(
        runtime.counters_snapshot().steps_executed,
        steps_before
    );
    let snap_after = runtime.snapshot_run(run, 2);
    assert!(matches!(
        snap_after,
        Ok(crate::shard::InspectResponse::Found(_))
    ));
}

// =============================================================================
// Migrate directive integration tests
// =============================================================================

/// Runtime migrates all pending actions to the target shard when tick_shard
/// receives Migrate directive.
///
/// Verifies: Migrate mutation (enqueues to wrong shard) is caught.
#[test]
fn runtime_tick_shard_migrate_transfers_actions_to_target_shard() {
    // Given: A 2-shard runtime with suspended_workflow on shard 0; shard 1 empty.
    let Some(shard_count) = std::num::NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(50);

    // Submit to shard 0 (RunId 50 hashes to shard 0 of 2) without ticking,
    // so the Submit command stays queued.
    assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));

    let metrics_before = runtime.collect_metrics();
    let shard0_depth_before = metrics_before.shards[0].command_queue_depth;
    let shard1_depth_before = metrics_before.shards[1].command_queue_depth;
    assert_eq!(shard0_depth_before, 1);
    assert_eq!(shard1_depth_before, 0);

    // When: tick_shard(Migrate { target: 1 }) is called on shard 0.
    // The source has no active runs yet, so it reports not alive.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Migrate { target: 1 }),
        Ok(false)
    );

    // Then: Verify migration via queue depths.
    // The mutation where Migrate enqueues to wrong shard is caught by
    // verifying target shard depth increased and source decreased.
    let metrics_after = runtime.collect_metrics();
    let shard0_depth_after = metrics_after.shards[0].command_queue_depth;
    let shard1_depth_after = metrics_after.shards[1].command_queue_depth;

    // Source should be empty (migrated away)
    assert_eq!(shard0_depth_after, 0);
    // Target should have more commands
    assert!(shard1_depth_after > shard1_depth_before);

    // The migrated command still drives the run on the target shard.
    assert_eq!(
        runtime.tick_shard(1, ShardDirective::Continue),
        Ok(true)
    );
    assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
}

/// Runtime returns an error when tick_shard Migrate targets the same shard
/// (self-migrate).
///
/// Verifies: Migrate mutation (allows self-migrate) is caught.
#[test]
fn runtime_tick_shard_migrate_rejects_self_migrate() {
    // Given: A 2-shard runtime with suspended_workflow on shard 0.
    let Some(shard_count) = std::num::NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(60);

    assert_eq!(submit_suspended(&runtime, run, wf), Ok(()));

    // When: tick_shard(Migrate { target: 0 }) is called (self-migrate).
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Migrate { target: 0 }),
        Err(RuntimeError::MigrateSelf)
    );

    // Then: Returns error (not Ok).
    // The mutation where Migrate allows self-migrate is caught by
    // verifying an error is returned, not Ok.
    // The queued command is untouched by the rejected migrate.
    let metrics = runtime.collect_metrics();
    let depth = metrics.shards[0].command_queue_depth;
    assert_eq!(depth, 1); // Self-migrate should not have processed
}

/// Runtime returns an error when tick_shard Migrate targets a non-existent
/// shard index.
///
/// Verifies: OOB Migrate target mutation is caught.
#[test]
fn runtime_tick_shard_migrate_rejects_invalid_target() {
    // Given: A 2-shard runtime.
    let Some(shard_count) = std::num::NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    // When: tick_shard(Migrate { target: 99 }) is called.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Migrate { target: 99 }),
        Err(RuntimeError::ShardNotFound { shard: 99 })
    );

    // Then: Returns ShardNotFound error.
    // The mutation where OOB target doesn't return ShardNotFound is caught.
    assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
}

/// Migrate on an empty source shard returns Ok without side effects.
///
/// Verifies: Migrate is idempotent on empty source.
#[test]
fn runtime_tick_shard_migrate_idempotent_on_empty_source() {
    // Given: A 2-shard runtime with empty command queues.
    let Some(shard_count) = std::num::NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    let metrics_before = runtime.collect_metrics();
    let shard0_before = metrics_before.shards[0].command_queue_depth;
    let shard1_before = metrics_before.shards[1].command_queue_depth;

    // When: tick_shard(Migrate { target: 1 }) is called on empty shard 0.
    // The empty source has no active runs, so it reports not alive.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Migrate { target: 1 }),
        Ok(false)
    );

    // Then: Both queues unchanged; Ok(false) returned.
    let metrics_after = runtime.collect_metrics();
    let shard0_after = metrics_after.shards[0].command_queue_depth;
    let shard1_after = metrics_after.shards[1].command_queue_depth;

    assert_eq!(shard0_before, shard0_after);
    assert_eq!(shard1_before, shard1_after);
}

// =============================================================================
// Shutdown directive integration tests
// =============================================================================

/// Runtime drains all remaining actions and enters shutdown state when tick_shard
/// receives Shutdown directive.
///
/// Verifies: Shutdown mutation (doesn't drain) is caught.
#[test]
fn runtime_tick_shard_shutdown_drains_remaining_actions() {
    // Given: A 1-shard runtime with action_then_finish_workflow enqueued.
    let Some(shard_count) = std::num::NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    let Some(wf) = action_then_finish_workflow() else {
        return;
    };
    let run = RunId::new(70);

    assert_eq!(submit_action_then_finish(&runtime, run, wf), Ok(()));

    // When: tick_shard(Shutdown) is called.
    // The drain processes the queued submit, so the run is admitted and its
    // Do action issued, but the run cannot complete without an external
    // action completion.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Shutdown),
        Ok(false)
    );

    // Then: Command queue is empty; the run was admitted but not completed.
    // The mutation where Shutdown doesn't drain remaining actions is caught.
    assert_eq!(runtime.collect_metrics().shards[0].command_queue_depth, 0);
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 1);
    assert_eq!(snap.runs_completed, 0);

    // The shard is dead: completing the issued action is rejected.
    let Some(ticket) = runtime.lookup_pending_action_ticket(run, 0) else {
        panic!("expected a pending action ticket for the drained run");
    };
    let output_value = SlotValue::I64(99);
    let output = ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value: output_value,
        taint: Taint::Clean,
        encoded_len: encoded_len(&output_value),
    };
    assert_eq!(
        runtime.complete_action_with_output(ticket, output),
        Err(RuntimeError::ShutdownInProgress)
    );
}

/// Runtime is idempotent when Shutdown is called on an already-shutting-down shard.
///
/// Verifies: Shutdown drain_for_shutdown loop mutation (early termination, overflow)
/// is caught.
#[test]
fn runtime_tick_shard_shutdown_idempotent() {
    // Given: A 1-shard runtime that has already received Shutdown directive.
    let Some(shard_count) = std::num::NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    let Some(wf) = finished_workflow() else {
        return;
    };
    let run = RunId::new(80);

    assert_eq!(runtime.submit_direct(run, wf), Ok(()));
    assert_eq!(runtime.shutdown_graceful().map(|_report| ()), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(false)); // First shutdown tick

    // When: tick_shard(Shutdown) is called a second time.
    // A second drain on the already-dead shard still reports not alive.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Shutdown),
        Ok(false)
    );

    // Then: Returns Ok(false) again (not Ok(true)).
    // The drain_for_shutdown mutations (early termination or panic on overflow)
    // are caught by verifying consistent Ok(false) behavior.
    let result = runtime.tick_all();
    assert_eq!(result, Ok(false)); // ← LETHAL FIX: explicit Ok(false), not just is_ok()
}

/// tick_shard returns Ok(false) when called on a shard that has already shut down.
///
/// LETHAL FIX: Mutation survivability gap closed. This test verifies the
/// WRONG behavior (Ok(true)) would be caught, not just that Ok(false) is
/// returned. We use assert_ne!(result, Ok(true)) variant or paired test.
#[test]
fn runtime_tick_shard_shutdown_returns_false_on_dead_shard_continue_rejected() {
    // Given: A 1-shard runtime that completed shutdown via Shutdown directive.
    let Some(shard_count) = std::num::NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    let Some(wf) = finished_workflow() else {
        return;
    };
    let run = RunId::new(90);

    assert_eq!(runtime.submit_direct(run, wf), Ok(()));
    assert_eq!(runtime.shutdown_graceful().map(|_report| ()), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(false)); // Shutdown complete

    // When: tick_shard(Continue) is called on dead shard.
    // A dead shard reports not alive instead of processing.
    let result = runtime.tick_shard(0, ShardDirective::Continue);

    // Then: Returns Ok(false), NOT Ok(true).
    // *** CRITICAL LETHAL FIX: Mutation survivability gap closed. ***
    // If the implementation incorrectly returns Ok(true), this test fails.
    // We assert the NEGATIVE: assert_ne!(result, Ok(true)) style or paired check.
    assert_eq!(result, Ok(false)); // Dead shard returns false
    // And explicitly: Ok(true) would be WRONG
    assert_ne!(result, Ok(true)); // ← LETHAL FIX: mutation gap closed
}

// =============================================================================
// Error cases
// =============================================================================

/// Runtime returns an error when tick_shard is called with an out-of-bounds
/// shard index.
///
/// Verifies: Invalid index mutation is caught.
#[test]
fn runtime_tick_shard_invalid_shard_index_returns_error() {
    // Given: A 2-shard runtime.
    let Some(shard_count) = std::num::NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    // When: tick_shard(5, Continue) is called (OOB index).
    assert_eq!(
        runtime.tick_shard(5, ShardDirective::Continue),
        Err(RuntimeError::ShardNotFound { shard: 5 })
    );

    // Then: Returns ShardNotFound error.
    // The mutation where invalid index doesn't return ShardNotFound is caught.
}

/// Runtime returns Ok(true) when tick_shard is called with valid index 0 on a
/// 1-shard runtime.
///
/// Verifies: Valid index 0 boundary case.
#[test]
fn runtime_tick_shard_with_zero_shard_count_returns_ok_on_valid_index() {
    // Given: A 1-shard runtime.
    let Some(shard_count) = std::num::NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    // When: tick_shard(0, Continue) is called on fresh runtime.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Continue),
        Ok(true)
    );

    // Then: Returns Ok(true).
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 0);
    assert_eq!(snap.steps_executed, 0);
}

// =============================================================================
// E2E scenarios
// =============================================================================

/// Full workflow exercising all four directives on a 4-shard runtime via public API.
///
/// Verifies: Complete directive sequence works correctly end-to-end.
#[test]
fn runtime_tick_shard_all_directives_via_public_api_e2e() {
    // Given: A 4-shard runtime; RunId hashes place one run on each shard
    // (100 -> 0, 101 -> 1, 102 -> 2, 103 -> 3).
    let Some(shard_count) = std::num::NonZeroUsize::new(4) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    let Some(wf) = suspended_workflow() else {
        return;
    };

    // Submit runs; each shard holds one queued Submit.
    let run0 = RunId::new(100);
    let run1 = RunId::new(101);
    let run2 = RunId::new(102);
    let run3 = RunId::new(103);

    assert_eq!(submit_suspended(&runtime, run0, wf.clone()), Ok(()));
    assert_eq!(submit_suspended(&runtime, run1, wf.clone()), Ok(()));
    assert_eq!(submit_suspended(&runtime, run2, wf.clone()), Ok(()));
    assert_eq!(submit_suspended(&runtime, run3, wf.clone()), Ok(()));
    assert_eq!(runtime.collect_metrics().shards[0].command_queue_depth, 1);
    assert_eq!(runtime.collect_metrics().shards[1].command_queue_depth, 1);
    assert_eq!(runtime.collect_metrics().shards[2].command_queue_depth, 1);
    assert_eq!(runtime.collect_metrics().shards[3].command_queue_depth, 1);

    // When: Directive sequence is applied.
    // - tick_shard(0, Continue) admits run 100
    // - tick_shard(1, Migrate { target: 2 }) moves run 101's submit to shard 2
    // - tick_shard(3, Suspend) leaves run 103's submit queued
    // - tick_shard(2, Shutdown) drains shard 2, admitting runs 101 and 102
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Continue),
        Ok(true)
    );
    assert_eq!(
        runtime.tick_shard(1, ShardDirective::Migrate { target: 2 }),
        Ok(false)
    );
    assert_eq!(
        runtime.tick_shard(3, ShardDirective::Suspend),
        Ok(true)
    );
    assert_eq!(
        runtime.tick_shard(2, ShardDirective::Shutdown),
        Ok(false)
    );

    // Then: Verify final state via counters and queue depths.
    // Shard 0: run admitted; shard 1: queue empty (migrated);
    // shard 2: both runs admitted then drained; shard 3: suspended.
    let metrics = runtime.collect_metrics();
    assert_eq!(metrics.shards[0].command_queue_depth, 0);
    assert_eq!(metrics.shards[1].command_queue_depth, 0);
    assert_eq!(metrics.shards[2].command_queue_depth, 0);
    assert_eq!(metrics.shards[3].command_queue_depth, 1); // Suspended, queue preserved
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 3);
    assert_eq!(snap.runs_completed, 0);

    // The suspended run still admits on a later Continue.
    assert_eq!(
        runtime.tick_shard(3, ShardDirective::Continue),
        Ok(true)
    );
    assert_eq!(runtime.counters_snapshot().runs_submitted, 4);
    assert_eq!(runtime.collect_metrics().shards[3].command_queue_depth, 0);
}

/// Migrate directive followed by Continue on source shard leaves target in
/// correct state.
#[test]
fn runtime_tick_shard_concurrent_migrate_and_continue_e2e() {
    // Given: A 2-shard runtime; shard 0 has 2 suspended runs
    // (RunIds 200 and 202 both hash to shard 0).
    let Some(shard_count) = std::num::NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run0 = RunId::new(200);
    let run1 = RunId::new(202);

    assert_eq!(submit_suspended(&runtime, run0, wf.clone()), Ok(()));
    assert_eq!(submit_suspended(&runtime, run1, wf.clone()), Ok(()));
    assert_eq!(runtime.collect_metrics().shards[0].command_queue_depth, 2);
    assert_eq!(runtime.collect_metrics().shards[1].command_queue_depth, 0);

    // When:
    // - tick_shard(0, Migrate { target: 1 }) migrates runs to shard 1
    // - tick_shard(0, Continue) on source (empty after migrate)
    // - tick_shard(1, Continue) on target (processes migrated runs)
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Migrate { target: 1 }),
        Ok(false)
    );
    assert_eq!(runtime.collect_metrics().shards[0].command_queue_depth, 0);
    assert_eq!(runtime.collect_metrics().shards[1].command_queue_depth, 2);
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Continue),
        Ok(true)
    );
    assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
    assert_eq!(
        runtime.tick_shard(1, ShardDirective::Continue),
        Ok(true)
    );
    assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
    assert_eq!(
        runtime.tick_shard(1, ShardDirective::Continue),
        Ok(true)
    );

    // Then: Shard 1 has processed the migrated runs.
    let metrics = runtime.collect_metrics();
    assert_eq!(metrics.shards[0].command_queue_depth, 0);
    assert_eq!(metrics.shards[1].command_queue_depth, 0);
    // Verify via counters that migrated runs were processed
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 2);
}

// =============================================================================
// Proptest invariants
// =============================================================================

/// Proptest: For any valid shard_index and any ShardDirective variant,
/// tick_shard must not panic and must return Result<bool, RuntimeError>.
///
/// Verifies: No panic, returns correct Result type.
#[test]
fn runtime_tick_shard_random_directive_does_not_panic() {
    // This test documents the proptest invariant without using proptest directly
    // (proptest requires nightly or specific setup).
    //
    // Strategy: For all valid shard indices and all directive variants,
    // tick_shard must return Result<bool, RuntimeError> — never panic.
    //
    // Anti-invariant: shard_index >= shard_count must always return
    // Err(RuntimeError::ShardNotFound).

    // Given: A 4-shard runtime with empty queues.
    let Some(shard_count) = std::num::NonZeroUsize::new(4) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    // Verify: All valid combinations return Result<bool, RuntimeError>.
    // Each directive variant is exercised once with its exact expected
    // result; Shutdown runs last because it kills the shard.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Continue),
        Ok(true)
    );
    assert_eq!(
        runtime.tick_shard(1, ShardDirective::Suspend),
        Ok(true)
    );
    assert_eq!(
        runtime.tick_shard(2, ShardDirective::Cancel),
        Err(RuntimeError::UnsupportedOperation {
            operation: "tick_shard_cancel"
        })
    );
    assert_eq!(
        runtime.tick_shard(2, ShardDirective::Barrier),
        Err(RuntimeError::UnsupportedOperation {
            operation: "tick_shard_barrier"
        })
    );
    assert_eq!(
        runtime.tick_shard(2, ShardDirective::Migrate { target: 3 }),
        Ok(false)
    );
    assert_eq!(
        runtime.tick_shard(2, ShardDirective::Migrate { target: 2 }),
        Err(RuntimeError::MigrateSelf)
    );
    assert_eq!(
        runtime.tick_shard(3, ShardDirective::Shutdown),
        Ok(false)
    );

    // Anti-invariant: shard_index >= shard_count must always return
    // Err(RuntimeError::ShardNotFound).
    assert_eq!(
        runtime.tick_shard(4, ShardDirective::Continue),
        Err(RuntimeError::ShardNotFound { shard: 4 })
    );

    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_submitted, 0);
}

/// Proptest: When a run is migrated, the RunId is preserved.
///
/// Verifies: RunId identity preserved across migration.
#[test]
fn runtime_tick_shard_migrate_preserves_run_identity() {
    // Given: Submit 2 runs to shard 0 (RunIds 300 and 302 both hash to
    // shard 0 of 2). Migration moves queued commands, not active runs, so
    // the submits stay queued until the migrate.
    let Some(shard_count) = std::num::NonZeroUsize::new(2) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    let Some(wf) = suspended_workflow() else {
        return;
    };
    let run1 = RunId::new(300);
    let run2 = RunId::new(302);

    assert_eq!(submit_suspended(&runtime, run1, wf.clone()), Ok(()));
    assert_eq!(submit_suspended(&runtime, run2, wf.clone()), Ok(()));
    assert_eq!(runtime.collect_metrics().shards[0].command_queue_depth, 2);

    // When: Migrate moves the queued submits from shard 0 to shard 1.
    assert_eq!(
        runtime.tick_shard(0, ShardDirective::Migrate { target: 1 }),
        Ok(false)
    );
    assert_eq!(runtime.collect_metrics().shards[0].command_queue_depth, 0);
    assert_eq!(runtime.collect_metrics().shards[1].command_queue_depth, 2);

    // The migrated submits still admit the same runs on the target shard.
    assert_eq!(
        runtime.tick_shard(1, ShardDirective::Continue),
        Ok(true)
    );
    assert_eq!(
        runtime.tick_shard(1, ShardDirective::Continue),
        Ok(true)
    );

    // Then: RunId is preserved; both runs are active, each exactly once.
    // Anti-invariant: Duplicate RunId across shards would indicate corruption.
    let active = runtime.list_active_runs(10, None);
    let run_ids: Vec<_> = active.iter().map(|s| s.run_id).collect();
    assert!(run_ids.contains(&run1));
    assert!(run_ids.contains(&run2));
    assert_eq!(run_ids.len(), 2);
    assert_eq!(runtime.counters_snapshot().runs_submitted, 2);
}

/// Proptest: Calling tick_shard(Shutdown) N times (N >= 1) always returns
/// Ok(false) and never changes shard state after first call.
#[test]
fn runtime_tick_shard_shutdown_is_stable_idempotent() {
    // Given: A 1-shard runtime that has already shut down.
    let Some(shard_count) = std::num::NonZeroUsize::new(1) else {
        return;
    };
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config());

    let Some(wf) = finished_workflow() else {
        return;
    };
    let run = RunId::new(400);

    assert_eq!(runtime.submit_direct(run, wf), Ok(()));
    assert_eq!(runtime.shutdown_graceful().map(|_report| ()), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(false)); // First call returns false

    // Get state after first shutdown
    let snap1 = runtime.counters_snapshot();
    let metrics1 = runtime.collect_metrics();

    // When: tick_shard(Shutdown) is called 5 more times.
    for _ in 0..5 {
        assert_eq!(
            runtime.tick_shard(0, ShardDirective::Shutdown),
            Ok(false)
        );
    }

    // Then: All return Ok(false); state unchanged after first call.
    let snap2 = runtime.counters_snapshot();
    assert_eq!(snap1.runs_completed, snap2.runs_completed);
    assert_eq!(snap1.runs_submitted, snap2.runs_submitted);
    let metrics2 = runtime.collect_metrics();
    assert_eq!(
        metrics1.shards[0].command_queue_depth,
        metrics2.shards[0].command_queue_depth
    );

    // Anti-invariant: First call Ok(false), second call Ok(true) or error
    // would indicate state machine bug.
}

// =============================================================================
// RuntimeError unit tests (tick_shard error variants)
// =============================================================================

#[test]
fn tick_shard_error_shard_not_found_debug() {
    let err = RuntimeError::ShardNotFound { shard: 5 };
    let debug = format!("{:?}", err);
    assert!(debug.contains("ShardNotFound"));
    assert!(debug.contains("5"));
}

#[test]
fn tick_shard_error_migrate_self_debug() {
    let err = RuntimeError::MigrateSelf;
    let debug = format!("{:?}", err);
    assert!(debug.contains("MigrateSelf"));
}

#[test]
fn tick_shard_error_shard_not_found_equality() {
    let a = RuntimeError::ShardNotFound { shard: 1 };
    let b = RuntimeError::ShardNotFound { shard: 1 };
    let c = RuntimeError::ShardNotFound { shard: 2 };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn tick_shard_error_migrate_self_equality() {
    let a = RuntimeError::MigrateSelf;
    let b = RuntimeError::MigrateSelf;
    assert_eq!(a, b);
}
