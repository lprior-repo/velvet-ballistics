#![forbid(unsafe_code)]
//! Kani choose-slot harnesses for the replay module.

use crate::frame::RunFrame;
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::{CompiledNode, CompiledNodeKind, SlotBranch};

use super::{ReplayAction, step::replay_step};
use super::kani_harnesses_plan::make_minimal_plan;

/// Proves that `replay_choose_slot` never panics for any boolean
/// combination of two slot conditions, with and without an otherwise
/// branch. This covers the full branch-and-fallback state space
/// (2^2 × 2 = 8 concrete states).
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
    )
    .expect("plan construction failed");

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)
        .expect("frame construction failed");
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(slot_a))
        .expect("write slot a failed");
    run.write_slot(SlotIdx::new(1), SlotValue::Bool(slot_b))
        .expect("write slot b failed");

    let mut store = ValueStore::new();
    let node = plan.node(StepIdx::new(0)).expect("node 0 missing");
    let _result = replay_step(node, &mut run, &mut store, &plan);

    if !slot_a && !slot_b && !has_otherwise {
        assert!(_result.is_err());
    } else {
        assert!(_result.is_ok());
    }
}

/// Proves that choose slot always selects a target from the input set
/// {branch targets} ∪ {otherwise}. Uses symbolic boolean slot values
/// to cover all concrete input combinations.
#[kani::proof]
fn verify_choose_slot_output_in_input_set() {
    let slot_a: bool = kani::any();
    let slot_b: bool = kani::any();

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
                    otherwise: Some(StepIdx::new(3)),
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
    )
    .expect("plan construction failed");

    let step_count = plan.node_count();
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)
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
            panic!("unexpected action variant");
        }
        Err(_) => {
            panic!("unexpected error for input with otherwise");
        }
    }
}
