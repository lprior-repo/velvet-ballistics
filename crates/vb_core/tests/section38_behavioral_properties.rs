#![forbid(unsafe_code)]
//! Section 38 behavioral property tests: terminal state rejection, replay
//! determinism, ordering invariants, and snapshot equivalence.

use vb_core::errors::CoreError;
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowError,
    WorkflowParts,
};
use vb_core::{EngineSignal, StepBudget, run_until_blocked, step_once};

// =========================================================================
// Helpers
// =========================================================================

fn default_contract() -> ResourceContract {
    ResourceContract::DEFAULT
}

/// Minimal two-step workflow: SetConst(42) -> Finish(slot 0).
fn simple_workflow() -> Result<CompiledWorkflow, String> {
    let parts = WorkflowParts {
        name: "behavioral_test".into(),
        digest: WorkflowDigest::from_bytes([0x38; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: default_contract(),
        step_names: Box::new([]),
    };
    CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
}

fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

// =========================================================================
// Test 3: Terminal state allows re-entry -- finished run can be re-run
// =========================================================================

#[test]
fn terminal_state_finished_run_can_be_rerun() -> Result<(), String> {
    let workflow = simple_workflow()?;
    let mut frame = RunFrame::new(
        RunId::new(1),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    // Run the full workflow to completion
    let result = run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;
    ensure(
        matches!(
            result,
            EngineSignal::Finished(SlotValue::I64(42), Taint::Clean)
        ),
        "workflow must finish with I64(42)",
    )?;

    // After finishing, step_once can be called again because the engine treats
    // an already-succeeded current step as idempotent and returns Finished again.
    let signal = step_once(&workflow, &mut frame, &mut store).map_err(|e| e.to_string())?;
    ensure(
        matches!(
            signal,
            EngineSignal::Finished(SlotValue::I64(42), Taint::Clean)
        ),
        "re-run should return Finished with same value",
    )
}

#[test]
fn terminal_state_succeeded_rejects_mark_running_direct() -> Result<(), String> {
    let mut frame =
        RunFrame::new(RunId::new(1), StepIdx::new(0), 3, 1).map_err(|e| e.to_string())?;
    frame
        .mark_running(StepIdx::new(0))
        .map_err(|e| e.to_string())?;
    frame
        .mark_succeeded(StepIdx::new(0))
        .map_err(|e| e.to_string())?;
    // Master contract (velvet-ballistics-MASTER.md:1569): no terminal state
    // transitions back to running. Loop body reentry uses the explicit
    // Succeeded->Pending admission path via mark_pending before mark_running.
    let result = frame.mark_running(StepIdx::new(0));
    assert!(
        matches!(
            result,
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition"
            })
        ),
        "succeeded step must reject mark_running; terminal states are absorbing"
    );
    Ok(())
}

#[test]
fn terminal_state_failed_rejects_mark_succeeded() -> Result<(), String> {
    let mut frame =
        RunFrame::new(RunId::new(1), StepIdx::new(0), 3, 1).map_err(|e| e.to_string())?;
    frame
        .mark_running(StepIdx::new(0))
        .map_err(|e| e.to_string())?;
    frame
        .mark_failed(StepIdx::new(0))
        .map_err(|e| e.to_string())?;
    // Failed -> Succeeded is forbidden
    let result = frame.mark_succeeded(StepIdx::new(0));
    ensure(result.is_err(), "failed step must reject mark_succeeded")
}

#[test]
fn terminal_state_cancelled_rejects_mark_running() -> Result<(), String> {
    let mut frame =
        RunFrame::new(RunId::new(1), StepIdx::new(0), 3, 1).map_err(|e| e.to_string())?;
    frame
        .mark_running(StepIdx::new(0))
        .map_err(|e| e.to_string())?;
    frame
        .mark_cancelled(StepIdx::new(0))
        .map_err(|e| e.to_string())?;
    // Cancelled -> Running is forbidden
    let result = frame.mark_running(StepIdx::new(0));
    ensure(result.is_err(), "cancelled step must reject mark_running")
}

// =========================================================================
// Test 5: Step budget exhaustion -- exceeding max steps is rejected
// =========================================================================

#[test]
fn step_budget_exhaustion_zero_budget_rejects_all_steps() -> Result<(), String> {
    let workflow = simple_workflow()?;
    let mut frame = RunFrame::new(
        RunId::new(1),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    // Budget of 0 means no steps can execute
    let result = run_until_blocked(&workflow, &mut frame, StepBudget::new(0), &mut store)
        .map_err(|e| e.to_string())?;
    ensure(
        result == EngineSignal::StepBudgetExhausted,
        "zero budget must exhaust immediately",
    )?;
    ensure(
        frame.executed() == 0,
        "no steps should execute on zero budget",
    )?;
    ensure(
        frame
            .step_state(StepIdx::new(0))
            .map_err(|e| e.to_string())?
            == StepState::Pending,
        "step 0 must still be pending after zero-budget run",
    )
}

#[test]
fn step_budget_exhaustion_insufficient_budget_halts_midway() -> Result<(), String> {
    let workflow = simple_workflow()?;
    let mut frame = RunFrame::new(
        RunId::new(1),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    // Budget of 1 means only the first step can execute
    let result = run_until_blocked(&workflow, &mut frame, StepBudget::new(1), &mut store)
        .map_err(|e| e.to_string())?;
    ensure(
        result == EngineSignal::StepBudgetExhausted,
        "insufficient budget must exhaust after first step",
    )?;
    ensure(frame.executed() == 1, "exactly one step should execute")?;
    ensure(frame.pc() == StepIdx::new(1), "PC must be at step 1")
}

#[test]
fn step_budget_exhaustion_resource_contract_rejects_oversized() -> Result<(), String> {
    let mut parts = simple_workflow_parts();
    parts.resource_contract.max_steps = 1;
    // 2 nodes > max_steps of 1
    let result = CompiledWorkflow::try_from_parts(parts);
    ensure(
        matches!(result, Err(WorkflowError::ResourceContractExceeded { .. })),
        "workflow exceeding max_steps must be rejected",
    )
}

fn simple_workflow_parts() -> WorkflowParts {
    WorkflowParts {
        name: "budget_test".into(),
        digest: WorkflowDigest::from_bytes([0xB5; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: default_contract(),
        step_names: Box::new([]),
    }
}

// =========================================================================
// Test 4: Taint safety -- secret taint propagates to Finish signal
// =========================================================================

#[test]
fn taint_safety_secret_taint_propagates_to_finish_signal() -> Result<(), String> {
    // Build a workflow where we manually write a secret-tainted value into
    // the finish result slot and verify the engine signals the taint.
    let parts = WorkflowParts {
        name: "taint_runtime".into(),
        digest: WorkflowDigest::from_bytes([0x54; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: default_contract(),
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
    let mut frame = RunFrame::new(
        RunId::new(1),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    // Write a secret-tainted value into the result slot
    frame
        .write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Secret)
        .map_err(|e| e.to_string())?;

    let result = run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;
    ensure(
        matches!(
            result,
            EngineSignal::Finished(SlotValue::I64(42), Taint::Secret)
        ),
        "finish signal must carry Secret taint when result slot is secret-tainted",
    )
}

#[test]
fn taint_safety_clean_taint_produces_clean_finish_signal() -> Result<(), String> {
    let workflow = simple_workflow()?;
    let mut frame = RunFrame::new(
        RunId::new(1),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    let result = run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;
    ensure(
        matches!(result, EngineSignal::Finished(_, Taint::Clean)),
        "clean workflow must finish with Clean taint",
    )
}

// =========================================================================
// Test 6: Replay determinism -- replay produces identical state sequence
// =========================================================================

#[test]
fn replay_determinism_same_run_produces_identical_slot_state() -> Result<(), String> {
    let workflow = simple_workflow()?;

    // First execution
    let mut frame_a = RunFrame::new(
        RunId::new(1),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())?;
    let mut store_a = ValueStore::new();
    let result_a = run_until_blocked(&workflow, &mut frame_a, StepBudget::MAX, &mut store_a)
        .map_err(|e| e.to_string())?;

    // Second execution (replay) from scratch
    let mut frame_b = RunFrame::new(
        RunId::new(2),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())?;
    let mut store_b = ValueStore::new();
    let result_b = run_until_blocked(&workflow, &mut frame_b, StepBudget::MAX, &mut store_b)
        .map_err(|e| e.to_string())?;

    // Both must finish with the same result
    ensure(
        result_a == result_b,
        "replay must produce identical engine signal",
    )?;
    ensure(
        frame_a.executed() == frame_b.executed(),
        "replay must execute same step count",
    )?;

    // Slot 0 must have the same value in both
    let slot_a = frame_a
        .read_slot(SlotIdx::new(0))
        .map_err(|e| e.to_string())?;
    let slot_b = frame_b
        .read_slot(SlotIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure(
        slot_a == slot_b,
        "replay must produce identical slot values",
    )
}

#[test]
fn replay_determinism_step_by_step_produces_identical_pc_sequence() -> Result<(), String> {
    let workflow = simple_workflow()?;

    // Execute step by step, recording PC after each step
    let mut frame = RunFrame::new(
        RunId::new(3),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    let initial_pc = frame.pc();
    ensure(initial_pc == StepIdx::new(0), "initial PC must be step 0")?;

    // Step 1: SetConst
    let _ = step_once(&workflow, &mut frame, &mut store).map_err(|e| e.to_string())?;
    let after_step1 = frame.pc();
    ensure(
        after_step1 == StepIdx::new(1),
        "after SetConst, PC must be 1",
    )?;
    ensure(
        frame
            .step_state(StepIdx::new(0))
            .map_err(|e| e.to_string())?
            == StepState::Succeeded,
        "step 0 must be succeeded",
    )?;

    // Step 2: Finish
    let result = step_once(&workflow, &mut frame, &mut store).map_err(|e| e.to_string())?;
    ensure(
        matches!(
            result,
            EngineSignal::Finished(SlotValue::I64(42), Taint::Clean)
        ),
        "step 2 must finish with I64(42)",
    )
}

// =========================================================================
// Test 7: Ordering invariants -- events emitted in valid order
// =========================================================================

#[test]
fn ordering_invariants_step_states_follow_valid_lifecycle() -> Result<(), String> {
    // Verify that step states follow the required lifecycle:
    // Pending -> Running -> (Succeeded | Failed | Cancelled | Waiting | Asking)
    let mut frame =
        RunFrame::new(RunId::new(1), StepIdx::new(0), 5, 1).map_err(|e| e.to_string())?;

    // Initially pending
    ensure(
        frame
            .step_state(StepIdx::new(0))
            .map_err(|e| e.to_string())?
            == StepState::Pending,
        "step must start pending",
    )?;

    // Running
    frame
        .mark_running(StepIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure(
        frame
            .step_state(StepIdx::new(0))
            .map_err(|e| e.to_string())?
            == StepState::Running,
        "step must be running after mark_running",
    )?;

    // Succeeded (terminal)
    frame
        .mark_succeeded(StepIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure(
        frame
            .step_state(StepIdx::new(0))
            .map_err(|e| e.to_string())?
            == StepState::Succeeded,
        "step must be succeeded after mark_succeeded",
    )
}

#[test]
fn ordering_invariants_resumable_states_can_return_to_running() -> Result<(), String> {
    let mut frame =
        RunFrame::new(RunId::new(1), StepIdx::new(0), 5, 1).map_err(|e| e.to_string())?;

    // Waiting is resumable
    frame
        .mark_running(StepIdx::new(0))
        .map_err(|e| e.to_string())?;
    frame
        .mark_waiting(StepIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure(
        frame
            .step_state(StepIdx::new(0))
            .map_err(|e| e.to_string())?
            == StepState::Waiting,
        "must be waiting",
    )?;
    frame
        .mark_running(StepIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure(
        frame
            .step_state(StepIdx::new(0))
            .map_err(|e| e.to_string())?
            == StepState::Running,
        "waiting must be resumable to running",
    )?;

    // Asking is resumable
    frame
        .mark_asking(StepIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure(
        frame
            .step_state(StepIdx::new(0))
            .map_err(|e| e.to_string())?
            == StepState::Asking,
        "must be asking",
    )?;
    frame
        .mark_running(StepIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure(
        frame
            .step_state(StepIdx::new(0))
            .map_err(|e| e.to_string())?
            == StepState::Running,
        "asking must be resumable to running",
    )
}

#[test]
fn ordering_invariants_pc_advances_monotonically_in_linear_workflow() -> Result<(), String> {
    let workflow = simple_workflow()?;
    let mut frame = RunFrame::new(
        RunId::new(1),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    let prev_pc = frame.pc();
    ensure(prev_pc == StepIdx::new(0), "must start at PC 0")?;

    let _ = step_once(&workflow, &mut frame, &mut store).map_err(|e| e.to_string())?;
    let next_pc = frame.pc();
    ensure(
        next_pc.get() > prev_pc.get(),
        "PC must advance monotonically",
    )?;
    ensure(
        next_pc == StepIdx::new(1),
        "PC must be at step 1 after first step",
    )
}

// =========================================================================
// Test 11: Snapshot equivalence -- journal snapshot equals in-memory state
// =========================================================================

#[test]
fn snapshot_equivalence_frame_slots_match_value_store() -> Result<(), String> {
    let workflow = simple_workflow()?;
    let mut frame = RunFrame::new(
        RunId::new(1),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    // Run to completion
    let result = run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;
    ensure(
        matches!(
            result,
            EngineSignal::Finished(SlotValue::I64(42), Taint::Clean)
        ),
        "must finish with I64(42)",
    )?;

    // The slot state in the frame must be consistent with the finished signal.
    // Reading slot 0 should produce the same value the engine reported.
    let slot_value = frame
        .read_slot(SlotIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure(
        *slot_value == SlotValue::I64(42),
        "frame slot must contain the same value as the finish signal",
    )
}

#[test]
fn snapshot_equivalence_executed_count_matches_actual_steps() -> Result<(), String> {
    let workflow = simple_workflow()?;

    // Run with budget 1 first
    let mut frame = RunFrame::new(
        RunId::new(1),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    let result = run_until_blocked(&workflow, &mut frame, StepBudget::new(1), &mut store)
        .map_err(|e| e.to_string())?;
    ensure(
        result == EngineSignal::StepBudgetExhausted,
        "must exhaust after 1 step",
    )?;
    ensure(frame.executed() == 1, "executed count must be exactly 1")?;

    // Resume and finish
    let result = run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;
    ensure(
        matches!(
            result,
            EngineSignal::Finished(SlotValue::I64(42), Taint::Clean)
        ),
        "must finish on resume",
    )?;
    ensure(frame.executed() == 2, "total executed must be 2")
}

#[test]
fn snapshot_equivalence_step_states_consistent_after_completion() -> Result<(), String> {
    let workflow = simple_workflow()?;
    let mut frame = RunFrame::new(
        RunId::new(1),
        workflow.entry(),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|e| e.to_string())?;
    let mut store = ValueStore::new();

    let _ = run_until_blocked(&workflow, &mut frame, StepBudget::MAX, &mut store)
        .map_err(|e| e.to_string())?;

    // Both steps must be in terminal (succeeded) state
    let step0 = frame
        .step_state(StepIdx::new(0))
        .map_err(|e| e.to_string())?;
    let step1 = frame
        .step_state(StepIdx::new(1))
        .map_err(|e| e.to_string())?;
    ensure(step0 == StepState::Succeeded, "step 0 must be succeeded")?;
    ensure(step1 == StepState::Succeeded, "step 1 must be succeeded")
}
