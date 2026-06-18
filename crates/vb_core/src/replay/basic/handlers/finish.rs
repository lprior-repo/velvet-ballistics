#![forbid(unsafe_code)]
//! Finish step handler.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::SlotIdx;

use super::shared;
use super::{ReplayAction, ReplayError};

/// Executes a Finish node: read the result slot and return Finished.
pub(super) fn replay_finish(
    run: &mut RunFrame,
    result: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let _value = *run.read_slot(result).map_err(|e| match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected error reading finish result slot",
        },
    })?;
    shared::increment_replay_executed(run)?;
    Ok(ReplayAction::Finished)
}
