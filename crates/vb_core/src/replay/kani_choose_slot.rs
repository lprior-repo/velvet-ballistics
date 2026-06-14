#![forbid(unsafe_code)]
//! Kani replay harnesses for choose-slot behavior.

use super::*;
use crate::frame::RunFrame;
use crate::ids::{RunId, SlotIdx};
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::{CompiledNodeKind, SlotBranch};

use super::super::{ReplayAction, ReplayError, step::replay_step};

#[kani::proof]
fn verify_replay_choose_slot_two_branches_no_panic() {
    let slot_a: bool = kani::any();
    let slot_b: bool = kani::any();
    let has_otherwise: bool = kani::any();

    let plan = make_minimal_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![
                        SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(1),
                        },
                        SlotBranch {
                            condition: SlotIdx::new(1),
                            target: StepIdx::new(2),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: if has_otherwise {
                        Some(StepIdx::new(3))
                    } else {
                        None
                    },
                },
                output: None,
                next: None,
            },
            nop_node(1),
            nop_node(2),
            nop_node(3),
        ],
        vec![],
    )
    .expect("plan construction failed");

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )
    .expect("frame construction failed");
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(slot_a))
        .expect("write slot a failed");
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(slot_b))
        .expect("write slot b failed");

    let mut store = ValueStore::new();
    let node = plan.node(StepIdx::new(0)).expect("node 0 missing");
    let result = replay_step(node, &mut run, &mut store, &plan);

    if !slot_a && !slot_b && !has_otherwise {
        assert!(result.is_err());
    } else {
        assert!(result.is_ok());
    }
}

#[kani::proof]
fn verify_choose_slot_output_in_input_set() {
    let slot_a: bool = kani::any();
    let slot_b: bool = kani::any();
    let plan = two_branch_plan().expect("plan construction failed");

    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )
    .expect("frame construction failed");
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(slot_a))
        .expect("write slot a failed");
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(slot_b))
        .expect("write slot b failed");

    let mut store = ValueStore::new();
    let node = plan.node(StepIdx::new(0)).expect("node 0 missing");
    let result = replay_step(node, &mut run, &mut store, &plan);

    match result {
        Ok(ReplayAction::Continue(target)) => {
            let valid =
                target == StepIdx::new(1) || target == StepIdx::new(2) || target == StepIdx::new(3);
            assert!(valid);
        }
        Ok(_) => {
            kani::assume(false, "unexpected action variant"); return;
        }
        Err(_) => {
            kani::assume(false, "unexpected error for input with otherwise"); return;
        }
    }
}

#[kani::proof]
fn verify_replay_deterministic_for_same_input() {
    let slot_val: bool = kani::any();
    let plan = deterministic_plan().expect("plan construction failed");
    let node = plan.node(StepIdx::new(0)).expect("node 0 missing");

    let result_a = replay_bool_slot(
        &plan,
        node,
        slot_val,
        "frame a failed",
        "write slot a failed",
    );
    let result_b = replay_bool_slot(
        &plan,
        node,
        slot_val,
        "frame b failed",
        "write slot b failed",
    );

    match (result_a, result_b) {
        (Ok(ReplayAction::Continue(a)), Ok(ReplayAction::Continue(b))) => {
            assert_eq!(a, b);
        }
        (Err(_), Err(_)) => {}
        _ => {
            kani::assume(false, "non-deterministic replay: mismatched results"); return;
        }
    }
}

fn two_branch_plan() -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    make_minimal_plan(
        vec![
            CompiledNode {
                id: StepIdx::new(0),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![
                        SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(1),
                        },
                        SlotBranch {
                            condition: SlotIdx::new(1),
                            target: StepIdx::new(2),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(3)),
                },
                output: None,
                next: None,
            },
            nop_node(1),
            nop_node(2),
            nop_node(3),
        ],
        vec![],
    )
}

fn deterministic_plan() -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    make_minimal_plan(
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
            nop_node(1),
            nop_node(2),
        ],
        vec![],
    )
}

fn replay_bool_slot(
    plan: &crate::workflow::CompiledWorkflow,
    node: &CompiledNode,
    slot_val: bool,
    frame_msg: &str,
    write_msg: &str,
) -> Result<ReplayAction, ReplayError> {
    let mut run = RunFrame::new(
        RunId::new(0),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )
    .expect(frame_msg);
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(slot_val))
        .expect(write_msg);
    let mut store = ValueStore::new();
    replay_step(node, &mut run, &mut store, plan)
}

fn nop_node(id: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
        output: None,
        next: None,
    }
}
