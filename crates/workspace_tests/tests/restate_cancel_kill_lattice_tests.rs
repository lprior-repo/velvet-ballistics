#![forbid(unsafe_code)]
//! restate_cancel_kill_lattice_tests: Cancel Lattice State Machine Tests
//!
//! Integration tests verifying cancel behavior across run lifecycles:
//! - HP-1: cancel running run transitions to terminal cancelled state
//! - HP-3: cancel action-suspended (resumable) run removes pending action
//! - HP-4: action after cancel returns error on tick (cannot process terminal run)
//! - EC-1: terminal states don't regress
//! - INV-1: terminal never regresses

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::action::{
    ActionFailure, ActionFailureCode, ActionOutputReady, ActionTicket, Idempotency, RetryPolicy,
    RetrySafety, SideEffect,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, ConstIdx, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::journal::{RuntimeJournalEvent, VolatileRuntimeJournal};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{InspectResponse, ShardConfig};

fn shard_count(value: usize) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(value).ok_or_else(|| format!("expected non-zero shard count, got {value}"))
}

fn test_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 16,
        max_active_runs: 8,
        policy: RuntimePolicy::Relaxed,
    }
}

fn node(id: u16, output: Option<u16>, next: Option<u16>, kind: CompiledNodeKind) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: output.map(SlotIdx::new),
        next: next.map(StepIdx::new),
        on_error: None,
        error_slot: None,
        kind,
    }
}

fn workflow_from_parts(
    name: &str,
    digest_byte: u8,
    nodes: Box<[CompiledNode]>,
    constants: Box<[ConstValue]>,
    slot_count: u16,
) -> Result<CompiledWorkflow, String> {
    let parts = WorkflowParts {
        name: Box::from(name),
        digest: WorkflowDigest::from_bytes([digest_byte; 32]),
        nodes,
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants,
        slot_count,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|err| format!("workflow fixture {name} invalid: {err:?}"))
}

fn finished_workflow() -> Result<CompiledWorkflow, String> {
    workflow_from_parts(
        "finished",
        0xA1,
        Box::from([
            node(
                0,
                Some(0),
                Some(1),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            ),
            node(
                1,
                None,
                None,
                CompiledNodeKind::Finish {
                    result: SlotIdx::ZERO,
                },
            ),
        ]),
        Box::from([ConstValue::Bool(true)]),
        1,
    )
}

fn action_then_finish_workflow() -> Result<CompiledWorkflow, String> {
    workflow_from_parts(
        "action_then_finish",
        0xA3,
        Box::from([
            node(
                0,
                Some(1),
                Some(1),
                CompiledNodeKind::Do {
                    action: ActionId::new(7),
                    input: SlotIdx::ZERO,
                },
            ),
            node(
                1,
                None,
                None,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
            ),
        ]),
        Box::from([]),
        2,
    )
}

fn required_capability(action: ActionId) -> Capability {
    Capability::new(Box::from("test.contract.required"), action)
}

fn action_contract(
    action: ActionId,
    input_slots: u16,
    output_slots: u16,
) -> vb_core::action::ActionContract {
    vb_core::action::ActionContract {
        id: action,
        input_slot_count: input_slots,
        output_slot_count: output_slots,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::from([required_capability(action)]),
    }
}

fn action_contracts_through(
    action: ActionId,
    input_slots: u16,
    output_slots: u16,
) -> Box<[vb_core::action::ActionContract]> {
    let target = action.get();
    let mut contracts = Vec::new();
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
    CapabilitySet::from_grants(Box::from([required_capability(action)]))
}

fn submit_action_then_finish(
    runtime: &Runtime,
    run: RunId,
    workflow: CompiledWorkflow,
) -> vb_runtime::RuntimeResult<()> {
    let action = ActionId::new(7);
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
        action_grants(action),
        action_contracts_through(action, 1, 1),
    )
}

fn action_ticket(run: RunId, action: ActionId) -> ActionTicket {
    ActionTicket {
        run,
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action,
        attempt: 1,
        idempotency_key: 0,
        capacity: 1,
    }
}

fn action_output(value: SlotValue) -> ActionOutputReady {
    ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value,
        taint: Taint::Clean,
        encoded_len: 8,
    }
}

fn tick_count(runtime: &mut Runtime, count: usize) -> Result<(), String> {
    for _ in 0..count {
        assert_eq!(
            runtime.tick_all(),
            Ok(true),
            "tick_all should return true while draining queued commands"
        );
    }
    Ok(())
}

fn make_action_failure(code: ActionFailureCode) -> ActionFailure {
    ActionFailure {
        code,
        retry_policy: RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

// =============================================================================
// HP-1: Cancel Running Run Transitions to Terminal Cancelled
// =============================================================================

/// HP-1a: Cancel running run transitions to Cancelled terminal state
#[test]
fn cancel_running_run_transitions_to_cancelled() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(10001);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_count(&mut runtime, 2)?;

    let counters = runtime.counters_snapshot();
    assert_eq!(
        counters.runs_failed, 1,
        "cancelled run must be counted as failed"
    );

    let events = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;
    assert!(
        events.iter().any(|e| {
            matches!(e, RuntimeJournalEvent::RunCancelled { run: r, .. } if *r == run)
        }),
        "journal must contain RunCancelled event"
    );

    Ok(())
}

/// HP-1b: Cancel produces RunCancelled in journal
#[test]
fn cancel_produces_run_cancelled_journal_event() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(10002);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_count(&mut runtime, 2)?;

    let events = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;
    assert!(
        events.iter().any(|e| {
            matches!(e, RuntimeJournalEvent::RunCancelled { run: r, .. } if *r == run)
        }),
        "journal must contain RunCancelled event"
    );

    Ok(())
}

// =============================================================================
// HP-3: Cancel Action-Suspended Run Removes Pending Action
// =============================================================================

/// HP-3: Cancel action-suspended run records cancellation
#[test]
fn cancel_action_suspended_run_records_cancellation() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(20001);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    let counters_before_cancel = runtime.counters_snapshot();
    assert_eq!(
        counters_before_cancel.runs_submitted, 1,
        "run must be submitted"
    );
    assert_eq!(
        counters_before_cancel.runs_completed, 0,
        "run must NOT be completed (waiting for action)"
    );

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_count(&mut runtime, 2)?;

    let counters = runtime.counters_snapshot();
    assert_eq!(
        counters.runs_failed, 1,
        "cancelled run must be counted as failed"
    );
    assert_eq!(
        counters.runs_completed, 0,
        "cancelled run must NOT be completed"
    );

    Ok(())
}

// =============================================================================
// HP-4: Timer/Action After Cancel Returns Error on Tick
// =============================================================================

/// HP-4a: Action completion after cancel cannot be processed (run is terminal)
#[test]
fn action_completion_after_cancel_returns_error_on_tick() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(30001);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_count(&mut runtime, 2)?;

    let result = runtime.complete_action_with_output(
        action_ticket(run, ActionId::new(7)),
        action_output(SlotValue::I64(42)),
    );
    assert_eq!(
        result,
        Ok(()),
        "action completion enqueue after cancel succeeds"
    );

    let tick_result = runtime.tick_all();
    assert!(
        tick_result.is_err(),
        "tick_all after enqueuing action for cancelled run must fail, got {:?}",
        tick_result
    );

    let counters = runtime.counters_snapshot();
    assert_eq!(counters.runs_completed, 0, "run must NOT be completed");

    Ok(())
}

/// HP-4b: Action failure after cancel cannot be processed (run is terminal)
#[test]
fn action_failure_after_cancel_returns_error_on_tick() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(30002);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_count(&mut runtime, 2)?;

    let ticket = action_ticket(run, ActionId::new(7));
    let failure = make_action_failure(ActionFailureCode::Unknown);
    let result = runtime.fail_action(ticket, failure);
    assert_eq!(
        result,
        Ok(()),
        "action failure enqueue after cancel succeeds"
    );

    let tick_result = runtime.tick_all();
    assert!(
        tick_result.is_err(),
        "tick_all after enqueuing failure for cancelled run must fail, got {:?}",
        tick_result
    );

    Ok(())
}

/// HP-4c: Resume after cancel cannot be processed (run is terminal)
#[test]
fn resume_after_cancel_returns_error_on_tick() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(30003);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_count(&mut runtime, 2)?;

    let result = runtime.resume_run(run);
    assert_eq!(result, Ok(()), "resume enqueue after cancel succeeds");

    let tick_result = runtime.tick_all();
    assert!(
        tick_result.is_err(),
        "tick_all after enqueuing resume for cancelled run must fail, got {:?}",
        tick_result
    );

    Ok(())
}

// =============================================================================
// EC-1: Terminal States Don't Regress
// =============================================================================

/// EC-1a: Completed terminal state is NotFound and doesn't regress
#[test]
fn completed_terminal_state_is_notfound() -> Result<(), String> {
    let mut runtime = Runtime::new(shard_count(1)?, test_config());
    let run = RunId::new(40001);

    assert_eq!(runtime.submit_direct(run, finished_workflow()?), Ok(()));
    tick_count(&mut runtime, 2)?;

    let state = runtime.snapshot_run(run, 1);
    assert_eq!(
        state,
        Ok(InspectResponse::NotFound {
            run,
            correlation: 1
        }),
        "run should be NotFound after completion"
    );

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_count(&mut runtime, 2)?;

    let state_after = runtime.snapshot_run(run, 2);
    assert_eq!(
        state_after,
        Ok(InspectResponse::NotFound {
            run,
            correlation: 2
        }),
        "run should still be NotFound after cancel attempt"
    );

    Ok(())
}

/// EC-1b: Cancelled terminal state is NotFound and doesn't regress on second cancel
#[test]
fn cancelled_terminal_state_is_notfound_and_idempotent() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(40002);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_count(&mut runtime, 2)?;

    let state = runtime.snapshot_run(run, 1);
    assert!(
        matches!(state, Ok(InspectResponse::NotFound { .. })),
        "run should be NotFound after first cancel, got {:?}",
        state
    );

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_count(&mut runtime, 2)?;

    let state_after = runtime.snapshot_run(run, 2);
    assert!(
        matches!(state_after, Ok(InspectResponse::NotFound { .. })),
        "run should still be NotFound after second cancel, got {:?}",
        state_after
    );

    Ok(())
}

// =============================================================================
// INV-1: Terminal Never Regresses (Invariant)
// =============================================================================

/// INV-1a: Cancelled terminal state has correct counters
#[test]
fn cancelled_terminal_state_has_correct_counters() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(50001);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_count(&mut runtime, 2)?;

    let counters = runtime.counters_snapshot();
    assert_eq!(counters.runs_failed, 1, "cancelled run counted as failed");
    assert_eq!(counters.runs_completed, 0, "cancelled run not completed");

    Ok(())
}

/// INV-1b: Cancel of non-existent run is idempotent (returns Ok)
#[test]
fn cancel_of_nonexistent_run_is_idempotent() -> Result<(), String> {
    let runtime = Runtime::new(shard_count(1)?, test_config());
    let run = RunId::new(50002);

    let result1 = runtime.cancel_run(run);
    assert_eq!(
        result1,
        Ok(()),
        "first cancel of nonexistent run must succeed (idempotent)"
    );

    let result2 = runtime.cancel_run(run);
    assert_eq!(
        result2,
        Ok(()),
        "second cancel of nonexistent run must also succeed (idempotent)"
    );

    Ok(())
}

/// INV-1c: Counters invariant - completed + failed = terminal
#[test]
fn counters_invariant_completed_plus_failed_equals_terminal() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());

    let run1 = RunId::new(60001);
    let run2 = RunId::new(60002);
    let run3 = RunId::new(60003);

    assert_eq!(runtime.submit_direct(run1, finished_workflow()?), Ok(()));
    tick_count(&mut runtime, 2)?;

    assert_eq!(
        submit_action_then_finish(&runtime, run2, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run2), Ok(()));
    tick_count(&mut runtime, 2)?;

    assert_eq!(
        submit_action_then_finish(&runtime, run3, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    runtime
        .cancel_run(run3)
        .map_err(|e| format!("cancel_run failed: {e:?}"))?;
    tick_count(&mut runtime, 2)?;

    let counters = runtime.counters_snapshot();
    let total_terminal = counters.runs_completed + counters.runs_failed;
    let total_submitted = counters.runs_submitted;

    assert_eq!(
        total_terminal, total_submitted,
        "invariant: completed({}) + failed({}) = terminal({}) must equal submitted({})",
        counters.runs_completed, counters.runs_failed, total_terminal, total_submitted
    );

    Ok(())
}

/// INV-1d: Cancel is idempotent - counters don't change on second cancel
#[test]
fn cancel_is_idempotent_counters_unchanged() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(60004);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_count(&mut runtime, 2)?;

    let counters_before = runtime.counters_snapshot();

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_count(&mut runtime, 2)?;

    let counters_after = runtime.counters_snapshot();

    assert_eq!(
        counters_before.runs_failed, counters_after.runs_failed,
        "failed counter must not change on second cancel"
    );

    Ok(())
}
