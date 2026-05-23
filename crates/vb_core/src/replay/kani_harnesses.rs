#![forbid(unsafe_code)]
//! Kani model-checking harnesses for the replay module.
//!
//! Each harness is gated behind `#[cfg(kani)]` so that `cargo kani`
//! picks it up as a proof obligation while regular `cargo test` skips it.

use crate::errors::CoreError;
use crate::frame::RunFrame;
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{ConstValue, SlotValue};
use crate::value_store::ValueStore;
use crate::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, SlotBranch, WorkflowParts};

use super::{ReplayAction, ReplayError, SuspensionKind, step::replay_step};

fn make_minimal_plan(
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
) -> Result<crate::workflow::CompiledWorkflow, CoreError> {
    crate::workflow::CompiledWorkflow::try_from_parts(WorkflowParts {
        name: "kani_plan".into(),
        digest: crate::ids::WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into(),
        expressions: vec![].into(),
        accessors: vec![].into(),
        constants: constants.into(),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|_| CoreError::InvalidCompiledWorkflow {
        reason: "kani test workflow validation failed",
    })
}

#[cfg(kani)]
mod verification {
    use super::*;

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
        let node = plan
            .node(StepIdx::new(0))
            .expect("node 0 missing");
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
        let node = plan
            .node(StepIdx::new(0))
            .expect("node 0 missing");
        let result = replay_step(node, &mut run, &mut store, &plan);

        match result {
            Ok(ReplayAction::Continue(target)) => {
                let valid = target == StepIdx::new(1)
                    || target == StepIdx::new(2)
                    || target == StepIdx::new(3);
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

    /// Proves that replay is deterministic: two identical frames
    /// with the same slot values produce identical results.
    #[kani::proof]
    fn verify_replay_deterministic_for_same_input() {
        let slot_val: bool = kani::any();

        let plan = make_minimal_plan(
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
        )
        .expect("plan construction failed");

        let step_count = plan.node_count();
        let slot_count = plan.slot_count();
        let node = plan
            .node(StepIdx::new(0))
            .expect("node 0 missing");

        let mut run_a = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)
            .expect("frame a failed");
        run_a
            .write_slot(SlotIdx::new(0), SlotValue::Bool(slot_val))
            .expect("write slot a failed");
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
}
