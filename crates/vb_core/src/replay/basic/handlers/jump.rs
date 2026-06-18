#![forbid(unsafe_code)]
//! Jump step handler.

use crate::frame::RunFrame;
use crate::ids::StepIdx;

use super::shared;
use super::{ReplayAction, ReplayError};

/// Executes a Jump node: advance PC to the target step.
pub(super) fn replay_jump(
    run: &mut RunFrame,
    target: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    run.set_pc(target).map_err(shared::slot_to_replay_err)?;
    shared::increment_replay_executed(run)?;
    Ok(ReplayAction::Continue(target))
}
