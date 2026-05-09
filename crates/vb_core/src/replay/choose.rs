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
