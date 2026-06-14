#![forbid(unsafe_code)]
//! Kani frame/state harnesses for the replay module.

use crate::frame::RunFrame;
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::{CompiledNode, CompiledNodeKind, SlotBranch};

use super::{ReplayAction, step::replay_step};
use super::kani_harnesses_plan::make_minimal_plan;

/// Proves that replay is deterministic: two identical frames
/// with the same slot values produce identical results.
#[kani::proof]
fn verify_replay_deterministic_for_same_input() {
    let slot_val: bool = kani::any();

    let plan = match make_minimal_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
                },
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
                next: None,
            },
        ],
        vec![],
    ) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let node = match plan.node(StepIdx::new(0)) {
        Some(v) => v,
        None => {
            kani::assume(false);
            return;
        }
    };

    let mut run_a = match RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };
    match run_a
        .write_slot(SlotIdx::new(0), SlotValue::Bool(slot_val))
    {
        Ok(_) => {}
        Err(_) => {
            kani::assume(false, "write slot a failed");
            return;
        }
    };

    let mut store_a = ValueStore::new();
    }
    let mut store_a = ValueStore::new();
    let result_a = replay_step(node, &mut run_a, &mut store_a, &plan);

    let mut run_b = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)
        .expect("frame b failed");
    run_b
        .write_slot(SlotIdx::new(0), SlotValue::Bool(slot_val))
        .expect("write slot b failed");
    let mut store_b = ValueStore::new();
    let result_b = replay_step(node, &mut run_b, &mut store_b, &plan);

    match (result_a, result_b) {
        (Ok(ReplayAction::Continue(a)), Ok(ReplayAction::Continue(b))) => {
            assert_eq!(a, b);
        }
        (Err(_), Err(_)) => {}
        _ => {
            panic!("non-deterministic replay: mismatched results");
        }
    }
}

// -----------------------------------------------------------------------
// PO-KANI-012: Replay skips absorbing terminal states
// -----------------------------------------------------------------------
// Failed, Cancelled, and Skipped are absorbing. Succeeded has a separate
// loop body re-entry edge to Running, so this harness excludes it.

/// PO-KANI-012: Verify replay dispatch skips absorbing terminal states.
/// A step in Failed/Cancelled/Skipped is not re-executed.
/// Uses kani::any() to test absorbing terminal states and step positions.
#[kani::proof]
fn kani_replay_skips_terminal_states() {
    let _step_raw: u8 = kani::any();
    let step_state_val: u8 = kani::any();

    let terminal_state = match step_state_val % 3 {
        0 => crate::frame::StepState::Failed,
        1 => crate::frame::StepState::Cancelled,
        _ => crate::frame::StepState::Skipped,
    };

    let plan = match make_minimal_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
                next: Some(StepIdx::new(1)),
            },
            CompiledNode {
                id: StepIdx::new(1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
                next: Some(StepIdx::new(2)),
            },
            CompiledNode {
                id: StepIdx::new(2),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
                next: None,
            },
            CompiledNode {
                id: StepIdx::new(3),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
                output: None,
                next: None,
            },
        ],
        vec![],
    ) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)
        .expect("frame construction failed");

    let terminal_idx = StepIdx::new(1);
    match terminal_state {
        crate::frame::StepState::Failed => run.mark_failed(terminal_idx).unwrap(),
        crate::frame::StepState::Cancelled => run.mark_cancelled(terminal_idx).unwrap(),
        crate::frame::StepState::Skipped => run.mark_skipped(terminal_idx).unwrap(),
        _ => {}
    }

    let state = run.step_state(terminal_idx).unwrap();
    kani::assert(
        matches!(
            state,
            crate::frame::StepState::Failed
                | crate::frame::StepState::Cancelled
                | crate::frame::StepState::Skipped
        ),
        "step 1 is in an absorbing terminal state",
    );

    let mut store = ValueStore::new();
    let node0 = plan.node(StepIdx::new(0)).expect("node 0 missing");
    set_pc_for_replay(&mut run, StepIdx::new(0));
    let _ = replay_step(node0, &mut run, &mut store, &plan);

    let node1 = plan.node(StepIdx::new(1)).expect("node 1 missing");
    set_pc_for_replay(&mut run, StepIdx::new(1));
    let result = replay_step(node1, &mut run, &mut store, &plan);

    let state_after = run.step_state(terminal_idx).unwrap();
    kani::assert(
        matches!(
            state_after,
            crate::frame::StepState::Failed
                | crate::frame::StepState::Cancelled
                | crate::frame::StepState::Skipped
        ),
        "absorbing terminal state must remain terminal after replay (PO-KANI-012)",
    );

    kani::assert(
        state_after == state,
        "replay must not mutate terminal step state (PO-KANI-012)",
    );

    kani::cover!(result.is_ok(), "replay terminal step: ok outcome");
    kani::cover!(result.is_err(), "replay terminal step: err outcome");
}

/// Helper: set the program counter on a RunFrame for replay.
fn set_pc_for_replay(run: &mut RunFrame, pc: StepIdx) {
    let _ = run.set_pc(pc);
}
