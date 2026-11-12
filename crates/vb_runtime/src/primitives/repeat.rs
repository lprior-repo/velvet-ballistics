#![forbid(unsafe_code)]
//! Repeat adapters preserving live body-entry policy around the shared numeric kernel.

use super::body_entry::{BodyEntryPolicy, LegacyEntry};
use super::helpers::jump_to;
use vb_core::engine::repeat::{self as kernel, RepeatFlow};
use vb_core::{EngineError, EngineSignal, RunFrame, SlotIdx, StepIdx};

fn apply_flow<P: BodyEntryPolicy>(
    run: &mut RunFrame,
    flow: RepeatFlow,
) -> Result<EngineSignal, EngineError> {
    match flow {
        RepeatFlow::Forward(target) => jump_to(run, target),
        RepeatFlow::Body(target) => P::jump(run, target),
    }
}

/// Initializes the packed attempt slot and enters the body.
pub fn repeat_start(
    run: &mut RunFrame,
    max_attempts: u16,
    body: StepIdx,
    _done: StepIdx,
    output: Option<SlotIdx>,
) -> Result<EngineSignal, EngineError> {
    let flow = kernel::repeat_start(run, max_attempts, body, output)?;
    apply_flow::<LegacyEntry>(run, flow)
}

/// Validates the attempt state and enters the legacy body.
pub fn repeat_attempt(
    run: &mut RunFrame,
    attempt_slot: SlotIdx,
    body: StepIdx,
    _done: StepIdx,
) -> Result<EngineSignal, EngineError> {
    let flow = kernel::repeat_attempt(run, attempt_slot, body)?;
    apply_flow::<LegacyEntry>(run, flow)
}

pub(crate) fn repeat_check_with_policy<P: BodyEntryPolicy>(
    run: &mut RunFrame,
    attempt_slot: SlotIdx,
    done: StepIdx,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<EngineSignal, EngineError> {
    let flow = kernel::repeat_check(run, attempt_slot, done, next, step)?;
    apply_flow::<P>(run, flow)
}

/// Uses the legacy entry-only reset without claiming complete owned-region authority.
pub fn repeat_check(
    run: &mut RunFrame,
    attempt_slot: SlotIdx,
    done: StepIdx,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<EngineSignal, EngineError> {
    repeat_check_with_policy::<LegacyEntry>(run, attempt_slot, done, next, step)
}

/// Copies the result and taint, then advances.
pub fn repeat_finish(
    run: &mut RunFrame,
    result: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    step: StepIdx,
) -> Result<EngineSignal, EngineError> {
    let flow = kernel::repeat_finish(run, result, output, next, step)?;
    apply_flow::<LegacyEntry>(run, flow)
}

#[cfg(test)]
use vb_core::SlotValue;
#[cfg(test)]
use vb_core::engine::repeat::{decode_repeat_state, encode_repeat_state};
#[cfg(test)]
fn invalid_repeat_state() -> EngineError {
    EngineError::InternalInvariantViolation {
        reason: "invalid_repeat_state".into(),
    }
}
#[cfg(test)]
#[path = "repeat/tests.rs"]
mod tests;
