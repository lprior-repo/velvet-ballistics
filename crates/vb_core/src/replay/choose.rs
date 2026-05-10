#![forbid(unsafe_code)]
//! Choice-related replay step handlers.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::StepIdx;
use crate::value::SlotValue;

use super::{ReplayAction, ReplayError, eval_expr_for_replay, slot_to_replay_err};

/// Replays a ChooseSlot node which selects a branch based on boolean slot values.
pub fn replay_choose_slot(
    run: &mut RunFrame,
    branches: &[crate::workflow::SlotBranch],
    otherwise: Option<StepIdx>,
) -> Result<ReplayAction, ReplayError> {
    let mut index = 0usize;
    while index < branches.len() {
        let branch = branches.get(index).ok_or(ReplayError::Internal {
            reason: "choose_slot branch index checked by loop bound",
        })?;
        let value = run.read_slot(branch.condition).map_err(|e| match e {
            EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
            EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
            _ => ReplayError::Internal {
                reason: "unexpected error reading choose_slot condition",
            },
        })?;
        match value {
            SlotValue::Bool(true) => {
                run.set_pc(branch.target).map_err(slot_to_replay_err)?;
                run.increment_executed()
                    .map_err(|_| ReplayError::Internal {
                        reason: "executed counter overflow",
                    })?;
                return Ok(ReplayAction::Continue(branch.target));
            }
            SlotValue::Bool(false) => {}
            _ => {
                return Err(ReplayError::Internal {
                    reason: "choose_slot condition is not boolean",
                });
            }
        }
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "choose_slot branch index overflow",
        })?;
    }
    let target = otherwise.ok_or(ReplayError::Internal {
        reason: "choose_slot no branch matched and no otherwise",
    })?;
    run.set_pc(target).map_err(slot_to_replay_err)?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(ReplayAction::Continue(target))
}

/// Replays a ChooseExpr node which selects a branch based on evaluated expressions.
pub fn replay_choose_expr(
    plan: &crate::workflow::CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut crate::value_store::ValueStore,
    branches: &[crate::workflow::ExprBranch],
    otherwise: Option<StepIdx>,
) -> Result<ReplayAction, ReplayError> {
    let mut index = 0usize;
    while index < branches.len() {
        let branch = branches.get(index).ok_or(ReplayError::Internal {
            reason: "choose_expr branch index checked by loop bound",
        })?;
        let (value, _taint) = eval_expr_for_replay(plan, run, store, branch.condition)
            .map_err(|_| ReplayError::ExpressionEvalFailed { step: run.pc() })?;
        match value {
            SlotValue::Bool(true) => {
                run.set_pc(branch.target).map_err(slot_to_replay_err)?;
                run.increment_executed()
                    .map_err(|_| ReplayError::Internal {
                        reason: "executed counter overflow",
                    })?;
                return Ok(ReplayAction::Continue(branch.target));
            }
            SlotValue::Bool(false) => {}
            _ => {
                return Err(ReplayError::Internal {
                    reason: "choose_expr condition is not boolean",
                });
            }
        }
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "choose_expr branch index overflow",
        })?;
    }
    let target = otherwise.ok_or(ReplayError::Internal {
        reason: "choose_expr no branch matched and no otherwise",
    })?;
    run.set_pc(target).map_err(slot_to_replay_err)?;
    run.increment_executed()
        .map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })?;
    Ok(ReplayAction::Continue(target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::RunFrame;
    use crate::ids::{RunId, SlotIdx, StepIdx, WorkflowDigest};
    use crate::value::SlotValue;
    use crate::workflow::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprOp, ExprProgram,
        ResourceContract, SlotBranch,
    };
    use crate::value_store::ValueStore;

    fn test_run_frame(slot_count: u16) -> Result<RunFrame, String> {
        RunFrame::new(RunId::new(1), StepIdx::new(0), 10, slot_count).map_err(|e| e.to_string())
    }

    // ===== replay_choose_slot happy path tests =====

    #[test]
    fn replay_choose_slot_takes_first_true_branch() -> Result<(), String> {
        let mut run = test_run_frame(3)?;
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
            .map_err(|e| e.to_string())?;

        let branches = vec![
            SlotBranch {
                condition: SlotIdx::new(0),
                target: StepIdx::new(5),
            },
            SlotBranch {
                condition: SlotIdx::new(1),
                target: StepIdx::new(7),
            },
        ];

        let result =
            replay_choose_slot(&mut run, &branches, Some(StepIdx::new(9))).map_err(|e| e.to_string())?;

        match result {
            ReplayAction::Continue(target) if target == StepIdx::new(5) => Ok(()),
            ReplayAction::Continue(other) => Err(format!("expected target 5, got {}", other.get())),
            ReplayAction::Finished => Err(String::from("expected Continue, got Finished")),
            ReplayAction::Suspended { .. } => Err(String::from("expected Continue, got Suspended")),
        }
    }

    #[test]
    fn replay_choose_slot_skips_false_branches_takes_second() -> Result<(), String> {
        let mut run = test_run_frame(3)?;
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))
            .map_err(|e| e.to_string())?;
        run.write_slot(SlotIdx::new(1), SlotValue::Bool(true))
            .map_err(|e| e.to_string())?;

        let branches = vec![
            SlotBranch {
                condition: SlotIdx::new(0),
                target: StepIdx::new(5),
            },
            SlotBranch {
                condition: SlotIdx::new(1),
                target: StepIdx::new(7),
            },
        ];

        let result =
            replay_choose_slot(&mut run, &branches, Some(StepIdx::new(9))).map_err(|e| e.to_string())?;

        match result {
            ReplayAction::Continue(target) if target == StepIdx::new(7) => Ok(()),
            ReplayAction::Continue(other) => Err(format!("expected target 7, got {}", other.get())),
            ReplayAction::Finished => Err(String::from("expected Continue, got Finished")),
            ReplayAction::Suspended { .. } => Err(String::from("expected Continue, got Suspended")),
        }
    }

    #[test]
    fn replay_choose_slot_falls_through_to_otherwise() -> Result<(), String> {
        let mut run = test_run_frame(3)?;
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))
            .map_err(|e| e.to_string())?;
        run.write_slot(SlotIdx::new(1), SlotValue::Bool(false))
            .map_err(|e| e.to_string())?;

        let branches = vec![
            SlotBranch {
                condition: SlotIdx::new(0),
                target: StepIdx::new(5),
            },
            SlotBranch {
                condition: SlotIdx::new(1),
                target: StepIdx::new(7),
            },
        ];

        let result =
            replay_choose_slot(&mut run, &branches, Some(StepIdx::new(3))).map_err(|e| e.to_string())?;

        match result {
            ReplayAction::Continue(target) if target == StepIdx::new(3) => Ok(()),
            ReplayAction::Continue(other) => Err(format!("expected target 3, got {}", other.get())),
            ReplayAction::Finished => Err(String::from("expected Continue, got Finished")),
            ReplayAction::Suspended { .. } => Err(String::from("expected Continue, got Suspended")),
        }
    }

    // ===== replay_choose_slot error path tests =====

    #[test]
    fn replay_choose_slot_error_slot_not_available() -> Result<(), String> {
        let mut run = test_run_frame(1)?;

        let branches = vec![SlotBranch {
            condition: SlotIdx::new(99),
            target: StepIdx::new(5),
        }];

        let result = replay_choose_slot(&mut run, &branches, Some(StepIdx::new(3)));

        match result {
            Err(ReplayError::SlotNotAvailable { slot }) if slot == SlotIdx::new(99) => Ok(()),
            Err(ReplayError::SlotNotAvailable { slot }) => {
                Err(format!("expected slot 99, got {:?}", slot))
            }
            Err(ReplayError::Internal { reason }) => {
                Err(format!("expected SlotNotAvailable, got Internal: {}", reason))
            }
            Ok(ReplayAction::Continue(target)) => {
                Err(format!("expected error, got Continue({})", target.get()))
            }
            Ok(ReplayAction::Finished) => Err(String::from("expected error, got Finished")),
            Ok(ReplayAction::Suspended { .. }) => Err(String::from("expected error, got Suspended")),
        }
    }

    #[test]
    fn replay_choose_slot_error_non_bool_value() -> Result<(), String> {
        let mut run = test_run_frame(1)?;
        run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
            .map_err(|e| e.to_string())?;

        let branches = vec![SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(5),
        }];

        let result = replay_choose_slot(&mut run, &branches, Some(StepIdx::new(3)));

        match result {
            Err(ReplayError::Internal { reason }) if reason.contains("not boolean") => Ok(()),
            Err(ReplayError::Internal { reason }) => {
                Err(format!("expected 'not boolean', got '{}'", reason))
            }
            Err(ReplayError::SlotNotAvailable { slot }) => {
                Err(format!("expected Internal error, got SlotNotAvailable({})", slot.get()))
            }
            Ok(_) => Err(String::from("expected error, got Ok")),
        }
    }

    #[test]
    fn replay_choose_slot_error_no_matching_and_no_otherwise() -> Result<(), String> {
        let mut run = test_run_frame(1)?;
        run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))
            .map_err(|e| e.to_string())?;

        let branches = vec![SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(5),
        }];

        let result = replay_choose_slot(&mut run, &branches, None);

        match result {
            Err(ReplayError::Internal { reason }) if reason.contains("no branch matched and no otherwise") => {
                Ok(())
            }
            Err(ReplayError::Internal { reason }) => {
                Err(format!("expected specific error message, got '{}'", reason))
            }
            Err(ReplayError::SlotNotAvailable { slot }) => {
                Err(format!("expected Internal error, got SlotNotAvailable({})", slot.get()))
            }
            Ok(_) => Err(String::from("expected error, got Ok")),
        }
    }

    // ===== replay_choose_expr happy path tests =====

    fn minimal_expr_workflow_with_const(
        idx: usize,
        const_val: crate::value::ConstValue,
    ) -> Result<CompiledWorkflow, String> {
        let expr = ExprProgram::try_from_ops(
            vec![ExprOp::LoadConst(crate::ids::ConstIdx::new(idx))].into_boxed_slice(),
        )
        .map_err(|e| crate::WorkflowError::Expression(e))
        .map_err(|e| e.to_string())?;

        CompiledWorkflow::try_from_parts(crate::workflow::WorkflowParts {
            name: Box::<str>::from("replay_choose_expr_test"),
            digest: WorkflowDigest::from_bytes([0xAA; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
            ]
            .into_boxed_slice(),
            expressions: vec![expr].into_boxed_slice(),
            accessors: Box::new([]),
            constants: vec![const_val].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
        .map_err(|e| e.to_string())
    }

    #[test]
    fn replay_choose_expr_takes_first_true_branch() -> Result<(), String> {
        let plan = minimal_expr_workflow_with_const(0, crate::value::ConstValue::Bool(true))?;
        let mut run = test_run_frame(1)?;
        let mut store = ValueStore::new();

        let branches = vec![ExprBranch {
            condition: crate::ids::ExprIdx::new(0),
            target: StepIdx::new(1),
        }];

        let result = replay_choose_expr(
            &plan,
            &mut run,
            &mut store,
            &branches,
            Some(StepIdx::new(2)),
        )
        .map_err(|e| e.to_string())?;

        match result {
            ReplayAction::Continue(target) if target == StepIdx::new(1) => Ok(()),
            ReplayAction::Continue(other) => Err(format!("expected target 1, got {}", other.get())),
            ReplayAction::Finished => Err(String::from("expected Continue, got Finished")),
            ReplayAction::Suspended { .. } => Err(String::from("expected Continue, got Suspended")),
        }
    }

    #[test]
    fn replay_choose_expr_all_false_takes_otherwise() -> Result<(), String> {
        let plan = minimal_expr_workflow_with_const(0, crate::value::ConstValue::Bool(false))?;
        let mut run = test_run_frame(1)?;
        let mut store = ValueStore::new();

        let branches = vec![ExprBranch {
            condition: crate::ids::ExprIdx::new(0),
            target: StepIdx::new(1),
        }];

        let result = replay_choose_expr(
            &plan,
            &mut run,
            &mut store,
            &branches,
            Some(StepIdx::new(2)),
        )
        .map_err(|e| e.to_string())?;

        match result {
            ReplayAction::Continue(target) if target == StepIdx::new(2) => Ok(()),
            ReplayAction::Continue(other) => Err(format!("expected target 2, got {}", other.get())),
            ReplayAction::Finished => Err(String::from("expected Continue, got Finished")),
            ReplayAction::Suspended { .. } => Err(String::from("expected Continue, got Suspended")),
        }
    }

    #[test]
    fn replay_choose_expr_empty_branches_takes_otherwise() -> Result<(), String> {
        let plan = minimal_expr_workflow_with_const(0, crate::value::ConstValue::Bool(true))?;
        let mut run = test_run_frame(1)?;
        let mut store = ValueStore::new();

        let branches: Vec<ExprBranch> = vec![];

        let result = replay_choose_expr(
            &plan,
            &mut run,
            &mut store,
            &branches,
            Some(StepIdx::new(5)),
        )
        .map_err(|e| e.to_string())?;

        match result {
            ReplayAction::Continue(target) if target == StepIdx::new(5) => Ok(()),
            ReplayAction::Continue(other) => Err(format!("expected target 5, got {}", other.get())),
            ReplayAction::Finished => Err(String::from("expected Continue, got Finished")),
            ReplayAction::Suspended { .. } => Err(String::from("expected Continue, got Suspended")),
        }
    }

    // ===== replay_choose_expr error path tests =====

    #[test]
    fn replay_choose_expr_error_expression_eval_failed() -> Result<(), String> {
        // Expression index out of bounds causes ExpressionEvalFailed
        let plan = minimal_expr_workflow_with_const(0, crate::value::ConstValue::Bool(true))?;
        let mut run = test_run_frame(1)?;
        let mut store = ValueStore::new();

        let branches = vec![ExprBranch {
            condition: crate::ids::ExprIdx::new(99),
            target: StepIdx::new(1),
        }];

        let result = replay_choose_expr(
            &plan,
            &mut run,
            &mut store,
            &branches,
            Some(StepIdx::new(2)),
        );

        match result {
            Err(ReplayError::ExpressionEvalFailed { step }) if step == StepIdx::new(0) => Ok(()),
            Err(ReplayError::ExpressionEvalFailed { step }) => {
                Err(format!("expected step 0, got {:?}", step.get()))
            }
            Err(ReplayError::Internal { reason }) => {
                Err(format!("expected ExpressionEvalFailed, got Internal: {}", reason))
            }
            Ok(_) => Err(String::from("expected error, got Ok")),
        }
    }

    #[test]
    fn replay_choose_expr_error_non_bool_value() -> Result<(), String> {
        let plan = minimal_expr_workflow_with_const(0, crate::value::ConstValue::I64(42))?;
        let mut run = test_run_frame(1)?;
        let mut store = ValueStore::new();

        let branches = vec![ExprBranch {
            condition: crate::ids::ExprIdx::new(0),
            target: StepIdx::new(1),
        }];

        let result = replay_choose_expr(
            &plan,
            &mut run,
            &mut store,
            &branches,
            Some(StepIdx::new(2)),
        );

        match result {
            Err(ReplayError::Internal { reason }) if reason.contains("not boolean") => Ok(()),
            Err(ReplayError::Internal { reason }) => {
                Err(format!("expected 'not boolean', got '{}'", reason))
            }
            Err(ReplayError::ExpressionEvalFailed { step }) => {
                Err(format!("expected Internal error, got ExpressionEvalFailed at step {}", step.get()))
            }
            Ok(_) => Err(String::from("expected error, got Ok")),
        }
    }
}
