//! Pure delta computation functions.
//!
//! Each function compares two slices and returns only the entries that differ.

use super::delta_types::{PcDelta, SlotDelta, StateDelta, TaintDelta};
use vb_core::frame::StepState;
use vb_core::ids::StepIdx;
use vb_core::value::{SlotValue, Taint};

/// Compute which slots changed between `slots_before` and `slots_after`.
///
/// Only slots whose values differ appear in the result. The output length
/// is bounded by `min(len(before), len(after))`.
pub(crate) fn compute_slot_deltas(
    slots_before: &[Option<SlotValue>],
    slots_after: &[Option<SlotValue>],
) -> Vec<SlotDelta> {
    slots_before
        .iter()
        .zip(slots_after.iter())
        .enumerate()
        .filter_map(|(i, (before, after))| {
            if before != after {
                let Ok(slot) = u16::try_from(i) else {
                    return None;
                };
                Some(SlotDelta {
                    slot,
                    before: *before,
                    after: *after,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Compute which taint flags changed between `taint_before` and `taint_after`.
///
/// Only slots whose taint differs appear in the result.
pub(crate) fn compute_taint_deltas(
    taint_before: &[Taint],
    taint_after: &[Taint],
) -> Vec<TaintDelta> {
    taint_before
        .iter()
        .zip(taint_after.iter())
        .enumerate()
        .filter_map(|(i, (before, after))| {
            if before != after {
                let Ok(slot) = u16::try_from(i) else {
                    return None;
                };
                Some(TaintDelta {
                    slot,
                    before: *before,
                    after: *after,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Compute which step states changed between `states_before` and `states_after`.
///
/// Only steps whose state differs appear in the result.
pub(crate) fn compute_state_deltas(
    states_before: &[StepState],
    states_after: &[StepState],
) -> Vec<StateDelta> {
    states_before
        .iter()
        .zip(states_after.iter())
        .enumerate()
        .filter_map(|(i, (before, after))| {
            if before != after {
                let Ok(step) = u16::try_from(i) else {
                    return None;
                };
                Some(StateDelta {
                    step,
                    before: *before,
                    after: *after,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Compute a program-counter delta between two step indices.
pub(crate) fn compute_pc_delta(before: StepIdx, after: StepIdx) -> PcDelta {
    PcDelta { before, after }
}
