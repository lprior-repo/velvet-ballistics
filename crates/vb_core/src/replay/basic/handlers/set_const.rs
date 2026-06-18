#![forbid(unsafe_code)]
//! SetConst step handler.

use crate::frame::RunFrame;
use crate::ids::ConstIdx;

use super::shared;
use super::{ReplayAction, ReplayError};

/// Executes a SetConst node: write a constant value into the output slot.
pub(super) fn replay_set_const(
    plan: &crate::workflow::CompiledWorkflow,
    run: &mut RunFrame,
    node: &crate::workflow::CompiledNode,
    value: ConstIdx,
) -> Result<ReplayAction, ReplayError> {
    let constant = plan.constant(value).copied().ok_or(ReplayError::Internal {
        reason: "constant out of bounds",
    })?;
    let slot_value = constant
        .to_slot_value()
        .map_err(|_| ReplayError::Internal {
            reason: "constant to slot value failed",
        })?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "SetConst node missing output slot",
    })?;
    run.write_slot(output, slot_value)
        .map_err(shared::slot_to_replay_err)?;
    let next = shared::advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}
