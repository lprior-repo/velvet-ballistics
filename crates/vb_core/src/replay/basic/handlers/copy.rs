#![forbid(unsafe_code)]
//! Copy step handler.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::SlotIdx;

use super::shared;
use super::{ReplayAction, ReplayError};

/// Executes a Copy node: copy a value (with taint) from source to output slot.
pub(super) fn replay_copy(
    run: &mut RunFrame,
    node: &crate::workflow::CompiledNode,
    source: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let value = *run.read_slot(source).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected error reading copy source slot",
        },
    })?;
    let taint = run.read_taint(source).map_err(shared::slot_to_replay_err)?;
    let output = node.output.ok_or(ReplayError::Internal {
        reason: "Copy node missing output slot",
    })?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(shared::slot_to_replay_err)?;
    let next = shared::advance_to_next(run, node)?;
    Ok(ReplayAction::Continue(next))
}
