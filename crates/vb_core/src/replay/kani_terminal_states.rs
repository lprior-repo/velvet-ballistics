#![forbid(unsafe_code)]
//! PO-KANI-012 replay terminal-state harness.

use super::*;
use crate::frame::RunFrame;
use crate::ids::RunId;
use crate::value_store::ValueStore;
use crate::workflow::CompiledNodeKind;

use super::super::step::replay_step;

#[kani::proof]
fn kani_replay_skips_terminal_states() {
    let step_raw: u8 = kani::any();
    let step_state_val: u8 = kani::any();
    let terminal_state = match step_state_val % 4 {
        0 => crate::frame::StepState::Succeeded,
        1 => crate::frame::StepState::Failed,
        2 => crate::frame::StepState::Cancelled,
        _ => crate::frame::StepState::Skipped,
    };

    let plan = match terminal_plan() {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };
    let mut run = match RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    ) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false);
            return;
        }
    };

    let terminal_idx = StepIdx::new(u16::from(step_raw % 4));
    kani::cover!(
        terminal_idx == StepIdx::new(0),
        "terminal step can be first"
    );
    kani::cover!(
        terminal_idx == StepIdx::new(1),
        "terminal step can be middle 1"
    );
    kani::cover!(
        terminal_idx == StepIdx::new(2),
        "terminal step can be middle 2"
    );
    kani::cover!(terminal_idx == StepIdx::new(3), "terminal step can be last");

    mark_terminal(&mut run, terminal_idx, terminal_state);
    let state = match run.step_state(terminal_idx) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };
    kani::assert(is_terminal(state), "selected step is in a terminal state");

    let mut store = ValueStore::new();
    let node_opt = plan.node(terminal_idx);
    kani::assert(
        node_opt.is_some(),
        "symbolic terminal index maps to a plan node",
    );
    let Some(node) = node_opt else {
        return;
    };
    set_pc_for_replay(&mut run, terminal_idx);
    let result = replay_step(node, &mut run, &mut store, &plan);

    let state_after = match run.step_state(terminal_idx) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };
    kani::assert(
        is_terminal(state_after),
        "terminal state must remain terminal after replay (PO-KANI-012)",
    );
    kani::assert(
        state_after == state,
        "replay must not mutate terminal step state (PO-KANI-012)",
    );
    kani::cover!(result.is_ok(), "replay terminal step: ok outcome");
    kani::cover!(result.is_err(), "replay terminal step: err outcome");
}

fn terminal_plan() -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    make_minimal_plan(
        vec![
            nop_node_with_next(0, Some(StepIdx::new(1))),
            nop_node_with_next(1, Some(StepIdx::new(2))),
            nop_node_with_next(2, None),
            nop_node_with_next(3, None),
        ],
        vec![],
    )
}

fn nop_node_with_next(id: u16, next: Option<StepIdx>) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
        output: None,
        next,
    }
}

fn mark_terminal(run: &mut RunFrame, step: StepIdx, state: crate::frame::StepState) {
    match state {
        crate::frame::StepState::Succeeded => match run.mark_succeeded(step) {
            Ok(v) => v,
            Err(_) => { kani::assume(false); return; }
        },
        crate::frame::StepState::Failed => match run.mark_failed(step) {
            Ok(v) => v,
            Err(_) => { kani::assume(false); return; }
        },
        crate::frame::StepState::Cancelled => match run.mark_cancelled(step) {
            Ok(v) => v,
            Err(_) => { kani::assume(false); return; }
        },
        crate::frame::StepState::Skipped => match run.mark_skipped(step) {
            Ok(v) => v,
            Err(_) => { kani::assume(false); return; }
        },
        _ => {}
    }
}

fn is_terminal(state: crate::frame::StepState) -> bool {
    matches!(
        state,
        crate::frame::StepState::Succeeded
            | crate::frame::StepState::Failed
            | crate::frame::StepState::Cancelled
            | crate::frame::StepState::Skipped
    )
}

fn set_pc_for_replay(run: &mut RunFrame, pc: StepIdx) {
    let _ = run.set_pc(pc);
}
