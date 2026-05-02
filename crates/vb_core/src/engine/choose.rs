//! Choose branch evaluation logic.

use crate::errors::EngineError;
use crate::ids::StepIdx;
use crate::value::SlotValue;
use crate::value_store::ValueStore;
use crate::workflow::{CompiledWorkflow, ExprBranch, SlotBranch};

pub(super) fn choose_expr_branch(
    plan: &CompiledWorkflow,
    run: &mut crate::frame::RunFrame,
    store: &mut ValueStore,
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
) -> Result<crate::EngineSignal, EngineError> {
    let next = choose_expr_target(plan, run, store, branches, otherwise)?;
    jump_to(run, next)
}

fn choose_expr_target(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
) -> Result<StepIdx, EngineError> {
    let mut index = 0usize;
    while index < branches.len() {
        let branch = branches
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "choose expr branch index checked by loop bound",
            })?;
        if let Some(target) = choose_expr_branch_target(plan, run, store, branch)? {
            return Ok(target);
        }
        index = index.checked_add(1).ok_or({
            EngineError::InternalInvariantViolation {
                reason: "choose expr branch index overflow",
            }
        })?;
    }

    otherwise.ok_or(EngineError::MissingNextStep { step: run.pc() })
}

fn choose_expr_branch_target(
    plan: &CompiledWorkflow,
    run: &crate::RunFrame,
    store: &mut ValueStore,
    branch: &ExprBranch,
) -> Result<Option<StepIdx>, EngineError> {
    let (value, _taint) =
        super::expr_eval::eval_expr_with_store(plan, run, store, branch.condition)?;
    match value {
        SlotValue::Bool(true) => Ok(Some(branch.target)),
        SlotValue::Bool(false) => Ok(None),
        other => Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: other.type_name(),
        }),
    }
}

pub(super) fn choose_slot_branch(
    run: &mut crate::frame::RunFrame,
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
) -> Result<crate::EngineSignal, EngineError> {
    let next = choose_slot_target(run, branches, otherwise)?;
    jump_to(run, next)
}

fn choose_slot_target(
    run: &crate::RunFrame,
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
) -> Result<StepIdx, EngineError> {
    let mut index = 0usize;
    while index < branches.len() {
        let branch = branches
            .get(index)
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "choose slot branch index checked by loop bound",
            })?;
        if let Some(target) = choose_slot_branch_target(run, branch)? {
            return Ok(target);
        }
        index = index.checked_add(1).ok_or({
            EngineError::InternalInvariantViolation {
                reason: "choose slot branch index overflow",
            }
        })?;
    }

    otherwise.ok_or(EngineError::MissingNextStep { step: run.pc() })
}

fn choose_slot_branch_target(
    run: &crate::RunFrame,
    branch: &SlotBranch,
) -> Result<Option<StepIdx>, EngineError> {
    match run.read_slot(branch.condition)? {
        SlotValue::Bool(true) => Ok(Some(branch.target)),
        SlotValue::Bool(false) => Ok(None),
        value => Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: value.type_name(),
        }),
    }
}

fn jump_to(
    run: &mut crate::frame::RunFrame,
    target: StepIdx,
) -> Result<crate::EngineSignal, EngineError> {
    run.set_pc(target)?;
    run.increment_executed()?;
    Ok(crate::EngineSignal::Continue)
}
