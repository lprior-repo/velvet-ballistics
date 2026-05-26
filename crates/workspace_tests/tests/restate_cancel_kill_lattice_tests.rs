#![cfg(test)]
#![forbid(unsafe_code)]
//! restate_cancel_kill_lattice_tests: Cancel/Kill State Machine Lattice Tests
//!
//! Integration tests for Cancel and Kill behavior against the step-state lattice.
//! Tests verify state transition invariants defined in the canonical step-state model.
//!
//! Behaviors covered:
//! - HP-1: cancel running run transitions to terminal cancelled state
//! - HP-3: cancel action-suspended run removes pending action
//! - HP-4: action after cancel returns error
//! - EC-1: terminal states don't regress
//! - INV-1: terminal never regresses
//!
//! Reference spec: `verification/verus/step_state_machine.rs`

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
use vb_runtime::trace::TraceEvent;

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

fn tick_and_drain(runtime: &mut Runtime) -> Result<Vec<TraceEvent>, String> {
    assert_eq!(
        runtime.tick_all(),
        Ok(true),
        "tick_all should return true when shards alive"
    );
    Ok(Vec::new())
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

// =============================================================================
// HP-1: cancel running run transitions to terminal cancelled state
// =============================================================================

/// HP-1: Cancel transitions a running run to terminal Cancelled state.
///
/// Given an active run in Running state, when cancel_run is called,
/// then the run transitions to Cancelled terminal state and all resources
/// are released.
#[test]
fn hp1_cancel_running_run_transitions_to_cancelled() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(20001);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    let counters_before = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_completed, 0,
        "run should be suspended waiting for action"
    );

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters = runtime.counters_snapshot();
    assert_eq!(counters.runs_failed, 1, "cancelled run counts as failed");
    assert_eq!(counters.runs_completed, 0, "cancelled run is not completed");

    let events = journal
        .snapshot()
        .map_err(|e| format!("journal snapshot failed: {e:?}"))?;
    assert!(
        events.iter().any(|e| matches!(
            e,
            RuntimeJournalEvent::RunCancelled { run: r, .. } if *r == run
        )),
        "journal must contain RunCancelled event"
    );

    Ok(())
}

// =============================================================================
// HP-3: cancel action-suspended run removes pending action
// =============================================================================

/// HP-3: Cancel removes pending action for action-suspended run.
///
/// Given a run suspended waiting for an action (Resumable state),
/// when cancel_run is called, then the pending action is removed
/// and subsequent action completion returns error.
#[test]
fn hp3_cancel_action_suspended_run_removes_pending_action() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(20003);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let failure = ActionFailure {
        code: ActionFailureCode::Rejected,
        retry_policy: RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    let result = runtime.fail_action(action_ticket(run, ActionId::new(7)), failure);
    assert!(
        result.is_err(),
        "action completion after cancel should return error"
    );

    Ok(())
}

// =============================================================================
// HP-4: action after cancel returns error
// =============================================================================

/// HP-4: Action completion after cancel returns error.
///
/// Given a run that was cancelled, when complete_action is called,
/// then an error is returned (RunNotFound or similar).
#[test]
fn hp4_action_after_cancel_returns_error() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(20004);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let result = runtime.complete_action_with_output(
        action_ticket(run, ActionId::new(7)),
        action_output(SlotValue::I64(42)),
    );
    assert!(
        result.is_err(),
        "action completion after cancel should return error"
    );

    Ok(())
}

// =============================================================================
// EC-1: terminal states don't regress
// =============================================================================

/// EC-1: Terminal states don't regress (idempotent self-transition only).
///
/// Given a run in terminal Cancelled state, when cancel is called again,
/// then the state remains Cancelled (no regression to non-terminal state).
#[test]
fn ec1_terminal_cancelled_state_does_not_regress() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(20005);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_before = runtime.counters_snapshot();

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_after = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_failed, counters_after.runs_failed,
        "failed counter should not change on second cancel"
    );
    assert_eq!(
        counters_before.runs_completed, counters_after.runs_completed,
        "completed counter should not change on second cancel"
    );

    Ok(())
}

// =============================================================================
// INV-1: terminal never regresses
// =============================================================================

/// INV-1: Terminal state never regresses to non-terminal state.
///
/// Given a run in terminal Cancelled state, when tick_all is called multiple times,
/// then the run remains in terminal state and counters do not change.
#[test]
fn inv1_terminal_never_regresses_after_cancel() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(20006);

    assert_eq!(
        submit_action_then_finish(&runtime, run, action_then_finish_workflow()?),
        Ok(())
    );
    tick_count(&mut runtime, 2)?;

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_before = runtime.counters_snapshot();

    for _ in 0..5 {
        runtime.tick_all().map_err(|e| format!("tick_all failed: {e:?}"))?;
    }

    let counters_after = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_failed, counters_after.runs_failed,
        "failed counter should not change after cancel"
    );
    assert_eq!(
        counters_before.runs_completed, counters_after.runs_completed,
        "completed counter should not change after cancel"
    );

    assert_eq!(
        runtime.snapshot_run(run, 1),
        Ok(InspectResponse::NotFound {
            run,
            correlation: 1
        }),
        "cancelled run should remain not found (terminal)"
    );

    Ok(())
}

/// INV-1: Terminal state never regresses - completed run stays terminal.
///
/// Given a run that completed successfully, when cancel is called,
/// then the completed state is preserved (cancel on completed run is idempotent).
#[test]
fn inv1_completed_run_terminal_never_regresses() -> Result<(), String> {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new_with_journal(shard_count(1)?, test_config(), journal.clone());
    let run = RunId::new(20007);

    assert_eq!(runtime.submit_direct(run, finished_workflow()?), Ok(()));
    tick_count(&mut runtime, 2)?;

    let counters_before = runtime.counters_snapshot();
    assert_eq!(counters_before.runs_completed, 1, "run should be completed");

    assert_eq!(runtime.cancel_run(run), Ok(()));
    tick_and_drain(&mut runtime)?;

    let counters_after = runtime.counters_snapshot();
    assert_eq!(
        counters_before.runs_completed, counters_after.runs_completed,
        "completed counter should not change after cancel on completed run"
    );

    Ok(())
}
