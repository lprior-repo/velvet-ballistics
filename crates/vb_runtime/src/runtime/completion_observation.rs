//! Opt-in bounded observations of accepted terminal outcomes.
//!
//! A receipt means the terminal append wrapper succeeded and the live result
//! was captured. It does not promise physical durability for a volatile journal.
//! Absence after an execution error does not prove that no event was accepted:
//! journal append can precede a failing sequence update. Neither execution nor
//! observation errors imply that retrying effects is safe.

use core::num::NonZeroU16;
use vb_core::ids::RunId;
use vb_core::value::{FiniteF64, SlotValue, Taint};

use crate::RuntimeResult;
use crate::shard::types::RunState;

/// Owned scalar values that never retain a released frame's arena handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnedCompletionScalar {
    Null,
    Bool(bool),
    I64(i64),
    F64(FiniteF64),
}

/// Clean scalar observation, or a value-free restricted/unsupported outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionValueObservation {
    Available(OwnedCompletionScalar),
    Restricted,
    Unavailable,
}

/// Positive evidence of an accepted terminal outcome, not whole-tick success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionObservation {
    pub run: RunId,
    pub attempt: NonZeroU16,
    pub result: CompletionValueObservation,
}

/// Caller-owned output for one shard. An occupied slot must be cleared by its owner.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CompletionObservationSlot {
    pub first: Option<CompletionObservation>,
    /// A second capture occurred, even for the same run/attempt. The first stays
    /// intact, but callers must not treat it as an unambiguous result.
    pub conflict: bool,
}

/// Rejection before any shard processing occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationPreflightError {
    BufferTooShort { required: usize, provided: usize },
    SlotNotEmpty { index: usize },
}

/// Observer completeness failed without changing the execution result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationConflict {
    MultipleCompletions,
}

/// Independent execution and observation outcomes; receipts survive either error.
#[derive(Debug)]
pub struct ObservedTickOutcome {
    pub execution: RuntimeResult<bool>,
    pub observation: Result<(), ObservationConflict>,
}

#[derive(Default)]
pub(crate) struct CompletionCapture {
    pub(crate) enabled: bool,
    pub(crate) slot: CompletionObservationSlot,
}

impl CompletionCapture {
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn capture_terminal(&mut self, run: RunId, state: &RunState) {
        if !self.enabled {
            return;
        }
        if self.slot.first.is_some() {
            self.slot.conflict = true;
            return;
        }
        self.slot.first = Some(CompletionObservation {
            run,
            attempt: state.run_attempt,
            result: observe_result(state),
        });
    }
}

fn observe_result(state: &RunState) -> CompletionValueObservation {
    let Some(slot) = crate::shard::helpers::result_slot_for_finished_run(state) else {
        return CompletionValueObservation::Unavailable;
    };
    match state.frame.read_taint(slot) {
        Ok(Taint::Clean) => {}
        Ok(_) => return CompletionValueObservation::Restricted,
        Err(_) => return CompletionValueObservation::Unavailable,
    }
    let Ok(value) = state.frame.read_slot(slot) else {
        return CompletionValueObservation::Unavailable;
    };
    let scalar = match *value {
        SlotValue::Null => OwnedCompletionScalar::Null,
        SlotValue::Bool(value) => OwnedCompletionScalar::Bool(value),
        SlotValue::I64(value) => OwnedCompletionScalar::I64(value),
        SlotValue::F64(value) => OwnedCompletionScalar::F64(value),
        _ => return CompletionValueObservation::Unavailable,
    };
    CompletionValueObservation::Available(scalar)
}

pub(super) fn preflight_slots(
    required: usize,
    slots: &mut [CompletionObservationSlot],
) -> Result<&mut [CompletionObservationSlot], ObservationPreflightError> {
    let provided = slots.len();
    let required_slots = slots
        .get_mut(..required)
        .ok_or(ObservationPreflightError::BufferTooShort { required, provided })?;
    for (index, slot) in required_slots.iter().enumerate() {
        if slot.first.is_some() || slot.conflict {
            return Err(ObservationPreflightError::SlotNotEmpty { index });
        }
    }
    Ok(required_slots)
}

#[cfg(test)]
#[path = "completion_observation_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "completion_observation_runtime_tests.rs"]
mod runtime_tests;
