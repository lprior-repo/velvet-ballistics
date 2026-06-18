#![forbid(unsafe_code)]
//! EvalExpr step handler.

use crate::frame::RunFrame;
use crate::ids::ExprIdx;
use crate::value_store::ValueStore;

use super::shared;
use super::{ReplayAction, ReplayError};

/// Executes an EvalExpr node: evaluate an expression and write the result.
pub(super) fn replay_eval_expr(
    plan: &crate::workflow::CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &crate::workflow::CompiledNode,
    expr: ExprIdx,
) -> Result<ReplayAction, ReplayError> {
    let (value, taint) = crate::replay::eval_expr_for_replay(plan, run, store, expr)
        .map_err(|_| ReplayError::ExpressionEvalFailed { step: node.id })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "EvalExpr node missing output slot",
    })?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(shared::slot_to_replay_err)?;
    let next = shared::advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}
