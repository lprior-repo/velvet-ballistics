#![forbid(unsafe_code)]

use vb_core::budget::WholeWorkflowBudget;
use vb_core::ids::{ExprIdx, SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, ExprBranch, ResourceContract, SlotBranch, WorkflowError,
};

fn step(id: u16, kind: CompiledNodeKind) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind,
    }
}

fn step_next(id: u16, next: u16, kind: CompiledNodeKind) -> CompiledNode {
    let mut n = step(id, kind);
    n.next = Some(StepIdx::new(next));
    n
}

#[test]
fn choose_two_branches_converging() -> Result<(), String> {
    let nodes = vec![
        step(
            0,
            CompiledNodeKind::Choose {
                branches: Box::new([
                    ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    ExprBranch {
                        condition: ExprIdx::new(1),
                        target: StepIdx::new(2),
                    },
                ]),
                otherwise: Some(StepIdx::new(3)),
            },
        ),
        step_next(1, 2, CompiledNodeKind::Nop),
        step_next(2, 3, CompiledNodeKind::Nop),
        step(
            3,
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ),
    ];
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &ResourceContract::DEFAULT)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(budget.max_total_steps, 4);
    assert_eq!(budget.max_fanout, 2);
    Ok(())
}

#[test]
fn choose_slot_two_branches_converging() -> Result<(), String> {
    let nodes = vec![
        step(
            0,
            CompiledNodeKind::ChooseSlot {
                branches: Box::new([
                    SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    SlotBranch {
                        condition: SlotIdx::new(1),
                        target: StepIdx::new(2),
                    },
                ]),
                otherwise: Some(StepIdx::new(3)),
            },
        ),
        step_next(1, 2, CompiledNodeKind::Nop),
        step_next(2, 3, CompiledNodeKind::Nop),
        step(
            3,
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ),
    ];
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &ResourceContract::DEFAULT)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(budget.max_total_steps, 4);
    assert_eq!(budget.max_fanout, 2);
    Ok(())
}

#[test]
fn choose_duplicate_targets() -> Result<(), String> {
    let nodes = vec![
        step(
            0,
            CompiledNodeKind::Choose {
                branches: Box::new([
                    ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    ExprBranch {
                        condition: ExprIdx::new(1),
                        target: StepIdx::new(1),
                    },
                ]),
                otherwise: None,
            },
        ),
        step(
            1,
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ),
    ];
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &ResourceContract::DEFAULT)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(budget.max_total_steps, 2);
    assert_eq!(budget.max_fanout, 2);
    Ok(())
}

#[test]
fn choose_back_edge_cycle() {
    let nodes = vec![step(
        0,
        CompiledNodeKind::Choose {
            branches: Box::new([ExprBranch {
                condition: ExprIdx::new(0),
                target: StepIdx::new(0),
            }]),
            otherwise: None,
        },
    )];
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &ResourceContract::DEFAULT);
    assert!(matches!(
        result,
        Err(WorkflowError::JumpCycle {
            step,
            target
        }) if step == StepIdx::new(0) && target == StepIdx::new(0)
    ));
}

#[test]
fn choose_out_of_bounds_target() {
    let nodes = vec![step(
        0,
        CompiledNodeKind::Choose {
            branches: Box::new([ExprBranch {
                condition: ExprIdx::new(0),
                target: StepIdx::new(9),
            }]),
            otherwise: None,
        },
    )];
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &ResourceContract::DEFAULT);
    assert!(matches!(
        result,
        Err(WorkflowError::StepOutOfBounds { step }) if step == StepIdx::new(9)
    ));
}
