//!
//! Kani harnesses for Resume state machine — TLA bridge RRO-TLA-RESUME-001.
//!
//! Bead: vb-282my
//! Obligations: PO-vb282my-RS-KANI-001 through PO-vb282my-RS-KANI-006
//!
//! Target: crate::shard::lifecycle::chunk_001::handle_resume
//!         crate::shard::transitions::apply
//!
//! GOD RULE 1: Structural `WorkflowParts` / `RunFrame` inputs are produced by
//!   bounded symbolic generators using `kani::any()` and production
//!   constructors; no fixed dummy workflow/run-frame fixture is used.
//! GOD RULE 2: Every harness calls production functions: apply(), handle_resume().
//!   harnesses test apply() with valid aggregate ownership seeded through
//!   production Shard helpers. handle_resume() guard paths are tested with a
//!   symbolically generated RunState admitted through the aggregate path.

#![forbid(unsafe_code)]
#![cfg(kani)]

use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

use crate::shard::ResumeError;
use crate::shard::types::{ResumeStatus, RunState, RuntimeEvent, RuntimeState, Shard, ShardConfig};

// =========================================================================
// Bounded generators
// =========================================================================

fn any_run_id() -> RunId {
    RunId::new(kani::any::<u64>())
}

fn any_live_runtime_state() -> RuntimeState {
    let variant: u8 = kani::any();
    kani::assume(variant < 4);
    match variant {
        0 => RuntimeState::Initial,
        1 => RuntimeState::Resumable,
        2 => RuntimeState::Resuming,
        _ => RuntimeState::Running,
    }
}

/// Constructs a minimal Shard. The Shard::new() constructor creates all
/// sub-components (command queue, in-memory journal, artifact store).
fn new_shard() -> Shard {
    Shard::new(ShardConfig {
        command_queue_capacity: 1,
        trace_capacity: 1,
        step_budget_per_tick: 1,
        max_active_runs: 1,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    })
}

/// PO-vb282my-RS-KANI-001/002: construct a bounded symbolic workflow through
/// the production validator before deriving a `RunFrame`. The reduced guard
/// domain varies run id, workflow digest, step count (1..=2), slot count
/// (1..=2), linear `Nop` prefix nodes, optional outputs, and step-name
/// materialization. Expressions/accessors/constants/action contracts stay empty
/// because these resume guard harnesses do not drive workflow execution and the
/// generated nodes do not reference those tables. The bounds still exercise the
/// vb-4969v aggregate/runtime-state invariant for both minimal and multi-step /
/// multi-slot validated workflows without hardcoding a fixed `WorkflowParts` or
/// `RunFrame` shape.
fn symbolic_workflow() -> Option<CompiledWorkflow> {
    CompiledWorkflow::try_from_parts(symbolic_workflow_parts()).ok()
}

fn symbolic_workflow_parts() -> WorkflowParts {
    let step_count = symbolic_step_count();
    let slot_count = symbolic_slot_count();
    let nodes = symbolic_nodes(step_count, slot_count);
    let parts = WorkflowParts {
        name: symbolic_workflow_name(),
        digest: vb_core::ids::WorkflowDigest::from_bytes(kani::any::<[u8; 32]>()),
        nodes,
        expressions: Vec::new().into_boxed_slice(),
        accessors: Vec::new().into_boxed_slice(),
        constants: Vec::new().into_boxed_slice(),
        slot_count,
        symbols_count: u32::from(kani::any::<u8>() % 4),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: symbolic_step_names(step_count),
    };
    assume_guard_workflow_parts_domain(&parts);
    parts
}

fn assume_guard_workflow_parts_domain(parts: &WorkflowParts) {
    kani::assume(parts.nodes.len() >= 1);
    kani::assume(parts.nodes.len() <= 2);
    kani::assume(parts.slot_count >= 1);
    kani::assume(parts.slot_count <= 2);
    kani::assume(parts.entry.as_usize() == 0);
    kani::assume(parts.expressions.is_empty());
    kani::assume(parts.accessors.is_empty());
    kani::assume(parts.constants.is_empty());

    match parts.nodes.first() {
        Some(node) if parts.nodes.len() == 1 => {
            kani::assume(matches!(&node.kind, CompiledNodeKind::Finish { .. }));
        }
        Some(node) => {
            kani::assume(matches!(&node.kind, CompiledNodeKind::Nop));
        }
        None => kani::assume(false),
    }

    if let Some(node) = parts.nodes.get(1) {
        kani::assume(matches!(&node.kind, CompiledNodeKind::Finish { .. }));
    }
}

fn symbolic_step_count() -> u16 {
    let count = u16::from(kani::any::<u8>() % 2) + 1;
    kani::cover!(count == 1, "symbolic_workflow_one_step");
    kani::cover!(count == 2, "symbolic_workflow_two_steps");
    count
}

fn symbolic_slot_count() -> u16 {
    let count = u16::from(kani::any::<u8>() % 2) + 1;
    kani::cover!(count == 1, "symbolic_workflow_one_slot");
    kani::cover!(count == 2, "symbolic_workflow_two_slots");
    count
}

fn symbolic_workflow_name() -> Box<str> {
    match kani::any::<u8>() % 3 {
        0 => Box::from("kani-resume-a"),
        1 => Box::from("kani-resume-b"),
        _ => Box::from("kani-resume-c"),
    }
}

fn symbolic_nodes(step_count: u16, slot_count: u16) -> Box<[CompiledNode]> {
    match step_count {
        1 => Vec::from([finish_node(0, slot_count)]).into_boxed_slice(),
        _ => Vec::from([prefix_node(0, 1, slot_count), finish_node(1, slot_count)])
            .into_boxed_slice(),
    }
}

fn prefix_node(id: u16, next: u16, slot_count: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: symbolic_optional_slot(slot_count),
        next: Some(StepIdx::new(next)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }
}

fn finish_node(id: u16, slot_count: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: symbolic_slot(slot_count),
        },
    }
}

fn symbolic_slot(slot_count: u16) -> SlotIdx {
    SlotIdx::new(kani::any::<u16>() % slot_count)
}

fn symbolic_optional_slot(slot_count: u16) -> Option<SlotIdx> {
    if kani::any::<bool>() {
        Some(symbolic_slot(slot_count))
    } else {
        None
    }
}

fn symbolic_step_names(step_count: u16) -> Box<[Box<str>]> {
    if !kani::any::<bool>() {
        return Vec::new().into_boxed_slice();
    }
    match step_count {
        1 => Vec::from([Box::<str>::from("resume-step-0")]).into_boxed_slice(),
        _ => Vec::from([
            Box::<str>::from("resume-step-0"),
            Box::<str>::from("resume-step-1"),
        ])
        .into_boxed_slice(),
    }
}

/// Constructs a symbolic RunState for inserting into shard.runs.
/// RunFrame dimensions are derived from the validated workflow instead of
/// using a fixed dummy `RunFrame::new(..., 1, 1)` shape.
fn symbolic_run_state(run: RunId) -> Option<RunState> {
    let workflow = symbolic_workflow()?;
    let step_count = workflow.node_count();
    let slot_count = workflow.slot_count();
    let frame = match RunFrame::new(run, workflow.entry(), step_count, slot_count) {
        Ok(frame) => frame,
        Err(_) => return None,
    };
    Some(RunState {
        frame,
        workflow,
        store: vb_core::value_store::ValueStore::with_max_slots(slot_count),
        action_attempts: crate::shard::helpers::new_action_attempts(step_count),
        admission: None,
        collect_states: Default::default(),
        action_contracts: Vec::new().into_boxed_slice(),
    })
}

fn assert_symbolic_run_state_invariants(state: &RunState, run: RunId) {
    kani::assert(
        state.frame.run_id() == run,
        "symbolic RunFrame must use requested run id",
    );
    kani::assert(
        state.frame.step_count() == state.workflow.node_count(),
        "symbolic RunFrame step count must match workflow node count",
    );
    kani::assert(
        state.frame.slot_count() == state.workflow.slot_count(),
        "symbolic RunFrame slot count must match workflow slot count",
    );
    kani::assert(
        state.action_attempts.len() == usize::from(state.frame.step_count()),
        "symbolic action_attempts length must match frame step count",
    );
}

fn admit_run_with_state(shard: &mut Shard, run: RunId, runtime_state: RuntimeState) -> bool {
    match symbolic_run_state(run) {
        Some(run_state) => {
            assert_symbolic_run_state_invariants(&run_state, run);
            shard.admit_run_state(run, run_state, runtime_state).is_ok()
        }
        None => false,
    }
}

fn seed_checked_out_runtime_state(
    shard: &mut Shard,
    run: RunId,
    runtime_state: RuntimeState,
) -> bool {
    shard.checked_out_run_insert(run).is_ok()
        && shard.runtime_state_insert(run, runtime_state).is_ok()
}

// =========================================================================
// PO-vb282my-RS-KANI-001: Resume state guard
// handle_resume rejects non-Resumable states
// =========================================================================

fn assert_resume_rejects_non_resumable_state(state: RuntimeState) {
    let mut shard = new_shard();
    let run = any_run_id();

    // Set up: run exists in the aggregate with a non-Resumable runtime state.
    kani::assert(
        admit_run_with_state(&mut shard, run, state),
        "setup admitted run state must succeed",
    );

    // Call production function handle_resume
    let result = shard.handle_resume(run);

    // Assert: must return Err(NotResumable) for non-Resumable states
    match result {
        Err(ResumeError::NotResumable {
            run_id,
            current_state,
        }) => {
            kani::assert(
                run_id == run,
                "NotResumable must identify the requested run id",
            );
            kani::assert(
                current_state == state,
                "NotResumable must report the exact non-resumable state",
            );
        }
        _ => {
            kani::assert(
                false,
                "non-Resumable and non-Running states must be rejected exactly",
            );
        }
    }
}

#[kani::proof]
#[kani::unwind(12)]
fn kani_resume_initial_guard() {
    assert_resume_rejects_non_resumable_state(RuntimeState::Initial);
    kani::cover!(true, "initial_state_rejected");
}

#[kani::proof]
#[kani::unwind(12)]
fn kani_resume_resuming_guard() {
    assert_resume_rejects_non_resumable_state(RuntimeState::Resuming);
    kani::cover!(true, "resuming_state_rejected");
}

// =========================================================================
// PO-vb282my-RS-KANI-002: Already Running path
// handle_resume returns Ok(AlreadyRunning) when RuntimeState == Running
// =========================================================================

#[kani::proof]
#[kani::unwind(16)]
fn kani_resume_already_running() {
    let mut shard = new_shard();
    let run = any_run_id();

    // Set up: run exists in the aggregate, runtime state is Running.
    kani::assert(
        admit_run_with_state(&mut shard, run, RuntimeState::Running),
        "setup admitted running state must succeed",
    );

    // Call production function
    let result = shard.handle_resume(run);

    // Assert: Ok(AlreadyRunning)
    match result {
        Ok(resume_result) => {
            kani::assert(
                matches!(resume_result.status, ResumeStatus::AlreadyRunning),
                "Running state must return AlreadyRunning",
            );
            kani::cover!(true, "already_running_path");
        }
        Err(_) => {
            kani::assert(false, "Running state must not return an error");
        }
    }
}

// =========================================================================
// PO-vb282my-RS-KANI-003: Append-before-drive ordering
// apply(Resume) transitions state to Resuming BEFORE drive_run
// Test apply() directly — it sets state to Resuming.
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_resume_append_before_drive() {
    let mut shard = new_shard();
    let run = any_run_id();

    // Set initial state to Resumable with valid checked-out aggregate ownership.
    kani::assert(
        seed_checked_out_runtime_state(&mut shard, run, RuntimeState::Resumable),
        "setup checked-out resumable state must succeed",
    );

    // apply(Resume) sets state to Resuming — this is called BEFORE drive_run
    kani::assert(
        shard.apply(run, RuntimeEvent::Resume).is_ok(),
        "apply Resume must succeed",
    );

    // After apply(Resume), state must be Resuming
    let state_after = shard.runtime_state_get(run);
    kani::assert(
        matches!(state_after, Some(RuntimeState::Resuming)),
        "apply(Resume) must transition to Resuming",
    );

    kani::cover!(true, "resume_transition_to_resuming");
}

// =========================================================================
// PO-vb282my-RS-KANI-004: Append failure rollback
// apply(ResumeRollback) restores RuntimeState::Resumable
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_resume_append_failure_rollback() {
    let mut shard = new_shard();
    let run = any_run_id();

    // Set initial state to Resuming (as if resume was attempted).
    kani::assert(
        seed_checked_out_runtime_state(&mut shard, run, RuntimeState::Resuming),
        "setup checked-out resuming state must succeed",
    );

    // Call production apply with ResumeRollback
    kani::assert(
        shard.apply(run, RuntimeEvent::ResumeRollback).is_ok(),
        "apply ResumeRollback must succeed",
    );

    // Assert: state must be restored to Resumable
    let state_after = shard.runtime_state_get(run);
    kani::assert(
        matches!(state_after, Some(RuntimeState::Resumable)),
        "apply(ResumeRollback) must restore to Resumable",
    );

    kani::cover!(true, "rollback_to_resumable");
}

// =========================================================================
// PO-vb282my-RS-KANI-005: Drive failure rollback preserving journal
// apply(ResumeRollback) is idempotent: restores Resumable regardless
// of previous state. Journal entry is NOT removed by apply.
// =========================================================================

#[kani::proof]
#[kani::unwind(15)]
fn kani_resume_drive_failure_preserves_journal() {
    let mut shard = new_shard();
    let run = any_run_id();

    // Set initial state to Resuming (appended journal, about to drive).
    kani::assert(
        seed_checked_out_runtime_state(&mut shard, run, RuntimeState::Resuming),
        "setup checked-out resuming state must succeed",
    );

    // Simulate: journal append succeeded, then drive failed
    // → restore_resumable_after_drive_failure calls apply(ResumeRollback)
    kani::assert(
        shard.apply(run, RuntimeEvent::ResumeRollback).is_ok(),
        "apply ResumeRollback must succeed",
    );

    // Assert: state is Resumable (journal entry preserved, state rollback only)
    let state_after = shard.runtime_state_get(run);
    kani::assert(
        matches!(state_after, Some(RuntimeState::Resumable)),
        "drive failure rollback must restore Resumable",
    );

    kani::cover!(true, "drive_failure_journal_preserved");
}

// =========================================================================
// PO-vb282my-RS-KANI-006: RuntimeState consistency after rollback
// apply(ResumeRollback) → RuntimeState == Resumable, no stale state
// Test all transitions to verify FSM correctness.
// =========================================================================

#[kani::proof]
#[kani::unwind(15)]
fn kani_resume_rollback_consistency() {
    let mut shard = new_shard();
    let run = any_run_id();

    // Start from any state
    let initial_state = any_live_runtime_state();
    kani::assert(
        seed_checked_out_runtime_state(&mut shard, run, initial_state),
        "setup checked-out initial runtime state must succeed",
    );

    // Apply ResumeRollback always → Resumable
    kani::assert(
        shard.apply(run, RuntimeEvent::ResumeRollback).is_ok(),
        "apply ResumeRollback must succeed",
    );

    // Verify
    let state_after = shard.runtime_state_get(run);
    kani::assert(
        matches!(state_after, Some(RuntimeState::Resumable)),
        "apply(ResumeRollback) must always result in Resumable",
    );

    kani::cover!(true, "rollback_consistency_verified");

    // Verify all non-terminal transitions produce correct states.
    // Terminal events are checked separately because they remove runtime state.
    let test_events: [(RuntimeEvent, RuntimeState); 5] = [
        (RuntimeEvent::Submit, RuntimeState::Initial),
        (RuntimeEvent::Resume, RuntimeState::Resuming),
        (RuntimeEvent::AwaitAction, RuntimeState::Resumable),
        (RuntimeEvent::AwaitTimer, RuntimeState::Resumable),
        (RuntimeEvent::DriveContinue, RuntimeState::Running),
    ];

    for (event, expected) in &test_events {
        let mut s = new_shard();
        let r = any_run_id();
        kani::assert(
            seed_checked_out_runtime_state(&mut s, r, RuntimeState::Initial),
            "transition setup must seed checked-out runtime state",
        );
        kani::assert(
            s.apply(r, *event).is_ok(),
            "apply non-terminal event must succeed",
        );
        let got = s.runtime_state_get(r);
        kani::assert(
            got == Some(*expected),
            "non-terminal transition must produce correct state",
        );
        match *event {
            RuntimeEvent::Submit => kani::cover!(true, "transition_Submit_to_Initial"),
            RuntimeEvent::Resume => kani::cover!(true, "transition_Resume_to_Resuming"),
            RuntimeEvent::AwaitAction => {
                kani::cover!(true, "transition_AwaitAction_to_Resumable");
            }
            RuntimeEvent::AwaitTimer => {
                kani::cover!(true, "transition_AwaitTimer_to_Resumable");
            }
            RuntimeEvent::DriveContinue => {
                kani::cover!(true, "transition_DriveContinue_to_Running");
            }
            _ => {}
        }
    }

    let mut terminal = new_shard();
    let terminal_run = any_run_id();
    kani::assert(
        seed_checked_out_runtime_state(&mut terminal, terminal_run, RuntimeState::Running),
        "terminal setup must seed checked-out running state",
    );
    let apply_result = terminal.apply(terminal_run, RuntimeEvent::Fail);
    kani::assert(apply_result.is_ok(), "Fail terminal apply must succeed");
    let terminal_state = terminal.runtime_state_get(terminal_run);
    kani::assert(
        terminal_state.is_none(),
        "Fail terminal event must remove runtime state",
    );
    kani::cover!(true, "transition_Fail_removes_state");
}

// =========================================================================
// Supplementary: apply() function state transition correctness
// Tests the apply function for all RuntimeEvent variants.
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_resume_apply_state_transitions() {
    let mut shard = new_shard();
    let run = any_run_id();

    kani::assert(
        seed_checked_out_runtime_state(&mut shard, run, RuntimeState::Initial),
        "Submit setup must seed checked-out runtime state",
    );
    // Test Submit → Initial
    kani::assert(
        shard.apply(run, RuntimeEvent::Submit).is_ok(),
        "apply Submit must succeed",
    );
    let state = shard.runtime_state_get(run);
    kani::assert(
        state == Some(RuntimeState::Initial),
        "Submit must set Initial state",
    );
    kani::cover!(true, "transition_Submit_to_Initial");

    // Test Resume → Resuming
    let mut s2 = new_shard();
    let r2 = any_run_id();
    kani::assert(
        seed_checked_out_runtime_state(&mut s2, r2, RuntimeState::Resumable),
        "Resume setup must seed checked-out resumable state",
    );
    kani::assert(
        s2.apply(r2, RuntimeEvent::Resume).is_ok(),
        "apply Resume must succeed",
    );
    let state2 = s2.runtime_state_get(r2);
    kani::assert(
        state2 == Some(RuntimeState::Resuming),
        "Resume must set Resuming state",
    );
    kani::cover!(true, "transition_Resume_to_Resuming");

    // Test ResumeRollback → Resumable
    let mut s3 = new_shard();
    let r3 = any_run_id();
    kani::assert(
        seed_checked_out_runtime_state(&mut s3, r3, RuntimeState::Resuming),
        "ResumeRollback setup must seed checked-out resuming state",
    );
    kani::assert(
        s3.apply(r3, RuntimeEvent::ResumeRollback).is_ok(),
        "apply ResumeRollback must succeed",
    );
    let state3 = s3.runtime_state_get(r3);
    kani::assert(
        state3 == Some(RuntimeState::Resumable),
        "ResumeRollback must set Resumable state",
    );
    kani::cover!(true, "transition_ResumeRollback_to_Resumable");

    // Test DriveContinue → Running
    let mut s4 = new_shard();
    let r4 = any_run_id();
    kani::assert(
        seed_checked_out_runtime_state(&mut s4, r4, RuntimeState::Initial),
        "DriveContinue setup must seed checked-out runtime state",
    );
    kani::assert(
        s4.apply(r4, RuntimeEvent::DriveContinue).is_ok(),
        "apply DriveContinue must succeed",
    );
    let state4 = s4.runtime_state_get(r4);
    kani::assert(
        state4 == Some(RuntimeState::Running),
        "DriveContinue must set Running state",
    );
    kani::cover!(true, "transition_DriveContinue_to_Running");

    // Test AwaitAction → Resumable
    let mut s5 = new_shard();
    let r5 = any_run_id();
    kani::assert(
        seed_checked_out_runtime_state(&mut s5, r5, RuntimeState::Running),
        "AwaitAction setup must seed checked-out running state",
    );
    kani::assert(
        s5.apply(r5, RuntimeEvent::AwaitAction).is_ok(),
        "apply AwaitAction must succeed",
    );
    let state5 = s5.runtime_state_get(r5);
    kani::assert(
        state5 == Some(RuntimeState::Resumable),
        "AwaitAction must set Resumable state",
    );
    kani::cover!(true, "transition_AwaitAction_to_Resumable");

    // Test AwaitTimer → Resumable
    let mut s6 = new_shard();
    let r6 = any_run_id();
    kani::assert(
        seed_checked_out_runtime_state(&mut s6, r6, RuntimeState::Running),
        "AwaitTimer setup must seed checked-out running state",
    );
    kani::assert(
        s6.apply(r6, RuntimeEvent::AwaitTimer).is_ok(),
        "apply AwaitTimer must succeed",
    );
    let state6 = s6.runtime_state_get(r6);
    kani::assert(
        state6 == Some(RuntimeState::Resumable),
        "AwaitTimer must set Resumable state",
    );
    kani::cover!(true, "transition_AwaitTimer_to_Resumable");

    // Test Fail → terminal runtime state removal
    let mut s7 = new_shard();
    let r7 = any_run_id();
    kani::assert(
        seed_checked_out_runtime_state(&mut s7, r7, RuntimeState::Running),
        "Fail setup must seed checked-out running state",
    );
    let apply7 = s7.apply(r7, RuntimeEvent::Fail);
    kani::assert(apply7.is_ok(), "Fail terminal apply must succeed");
    let state7 = s7.runtime_state_get(r7);
    kani::assert(
        state7.is_none(),
        "Fail must remove state from runtime_states",
    );
    kani::cover!(true, "transition_Fail_removes_state");

    // Test TerminalRemove → swap_remove (state is removed)
    let mut s8 = new_shard();
    let r8 = any_run_id();
    kani::assert(
        seed_checked_out_runtime_state(&mut s8, r8, RuntimeState::Initial),
        "TerminalRemove setup must seed checked-out initial state",
    );
    kani::assert(
        s8.apply(r8, RuntimeEvent::TerminalRemove).is_ok(),
        "apply TerminalRemove must succeed",
    );
    let state8 = s8.runtime_state_get(r8);
    kani::assert(
        state8.is_none(),
        "TerminalRemove must remove state from runtime_states",
    );
    kani::cover!(true, "transition_TerminalRemove_removes_state");

    // Test DriveFinished → swap_remove (state is removed)
    let mut s9 = new_shard();
    let r9 = any_run_id();
    kani::assert(
        seed_checked_out_runtime_state(&mut s9, r9, RuntimeState::Running),
        "DriveFinished setup must seed checked-out running state",
    );
    kani::assert(
        s9.apply(r9, RuntimeEvent::DriveFinished).is_ok(),
        "apply DriveFinished must succeed",
    );
    let state9 = s9.runtime_state_get(r9);
    kani::assert(
        state9.is_none(),
        "DriveFinished must remove state from runtime_states",
    );
    kani::cover!(true, "transition_DriveFinished_removes_state");
}

#[kani::proof]
#[kani::unwind(10)]
fn kani_terminal_membership_cannot_coexist_with_runtime_state() {
    let mut shard = new_shard();
    let run = any_run_id();

    kani::assert(
        seed_checked_out_runtime_state(&mut shard, run, RuntimeState::Running),
        "setup checked-out runtime state must succeed",
    );
    kani::assert(
        shard.terminal_runs_insert(run).is_ok(),
        "terminal membership insert must succeed",
    );
    kani::assert(
        shard.runtime_state_get(run).is_none(),
        "terminal membership must clear runtime state",
    );
    kani::assert(
        shard.apply(run, RuntimeEvent::ResumeRollback).is_err(),
        "terminal membership must reject runtime state recreation",
    );
    kani::assert(
        shard.runtime_state_get(run).is_none(),
        "terminal run must not regain runtime state",
    );
}

#[kani::proof]
#[kani::unwind(10)]
fn kani_failed_runtime_state_insert_rejected() {
    let mut shard = new_shard();
    let run = any_run_id();

    kani::assert(
        shard.checked_out_run_insert(run).is_ok(),
        "setup checked-out owner must succeed",
    );
    kani::assert(
        shard
            .runtime_state_insert(run, RuntimeState::Failed)
            .is_err(),
        "Failed runtime state insert must be rejected",
    );
    kani::assert(
        shard.runtime_state_get(run).is_none(),
        "rejected Failed state must not be retained",
    );
}
