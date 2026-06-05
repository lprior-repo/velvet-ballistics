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
use crate::workflow::{
    CompiledNode, CompiledNodeKind, ResourceContract, SlotBranch, WorkflowParts,
};

use super::{ReplayAction, step::replay_step};

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
        let node = plan.node(StepIdx::new(0)).expect("node 0 missing");

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
        let step_raw: u8 = kani::any();
        let step_state_val: u8 = kani::any();

        // Map to an absorbing terminal state: Failed(3), Cancelled(7), Skipped(4).
        let terminal_state = match step_state_val % 3 {
            0 => crate::frame::StepState::Failed,
            1 => crate::frame::StepState::Cancelled,
            _ => crate::frame::StepState::Skipped,
        };

        // Build a 4-step plan with the terminal step at position 1
        let plan = make_minimal_plan(
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
        )
        .expect("plan construction failed");

        let step_count = plan.node_count();
        let slot_count = plan.slot_count();
        let mut run = RunFrame::new(RunId::new(0), StepIdx::new(0), step_count, slot_count)
            .expect("frame construction failed");

        // Set step 1 (the middle step) to a terminal state
        let terminal_idx = StepIdx::new(1);
        match terminal_state {
            crate::frame::StepState::Failed => run.mark_failed(terminal_idx).unwrap(),
            crate::frame::StepState::Cancelled => run.mark_cancelled(terminal_idx).unwrap(),
            crate::frame::StepState::Skipped => run.mark_skipped(terminal_idx).unwrap(),
            _ => {}
        }

        // Verify step 1 is an absorbing terminal.
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

        // Execute step 0 (Nop) to advance to step 1
        let mut store = ValueStore::new();
        let node0 = plan.node(StepIdx::new(0)).expect("node 0 missing");
        set_pc_for_replay(&mut run, StepIdx::new(0));
        let _ = replay_step(node0, &mut run, &mut store, &plan);

        // Now at step 1 (absorbing terminal). Attempting replay should NOT execute it.
        // The step should either be skipped or produce a Continue action
        // without modifying the step state.
        let node1 = plan.node(StepIdx::new(1)).expect("node 1 missing");
        set_pc_for_replay(&mut run, StepIdx::new(1));
        let result = replay_step(node1, &mut run, &mut store, &plan);

        // After replay attempt, step state must still be absorbing terminal.
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

        // The state must be exactly the same as before
        kani::assert(
            state_after == state,
            "replay must not mutate terminal step state (PO-KANI-012)",
        );

        // Cover both success and error paths
        kani::cover!(result.is_ok(), "replay terminal step: ok outcome");
        kani::cover!(result.is_err(), "replay terminal step: err outcome");
    }

    /// Helper: set the program counter on a RunFrame for replay.
    /// Handles the PC out-of-bounds case by returning early.
    fn set_pc_for_replay(run: &mut RunFrame, pc: StepIdx) {
        // Safe: RunFrame::set_pc validates bounds internally
        let _ = run.set_pc(pc);
    }
}
