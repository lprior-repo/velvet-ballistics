#![forbid(unsafe_code)]
//! Nop step handler.

use crate::frame::RunFrame;

use super::shared;
use super::{ReplayAction, ReplayError};

/// Executes a Nop node: advance PC to the next step.
pub(super) fn replay_nop(
    node: &crate::workflow::CompiledNode,
    run: &mut RunFrame,
) -> Result<ReplayAction, ReplayError> {
    let next = node.next.ok_or(ReplayError::Internal {
        reason: "Nop node missing next step",
    })?;
    run.set_pc(next).map_err(shared::slot_to_replay_err)?;
    shared::increment_replay_executed(run)?;
    Ok(ReplayAction::Continue(next))
}
