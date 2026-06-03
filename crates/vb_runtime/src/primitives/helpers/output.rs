#![forbid(unsafe_code)]
//! Output slot helper functions.

use vb_core::errors::EngineError;
use vb_core::ids::{SlotIdx, StepIdx};

/// Requires that an output slot be present.
pub(crate) fn require_output(
    output: Option<SlotIdx>,
    step: StepIdx,
) -> Result<SlotIdx, EngineError> {
    output.ok_or(EngineError::MissingOutputSlot { step })
}