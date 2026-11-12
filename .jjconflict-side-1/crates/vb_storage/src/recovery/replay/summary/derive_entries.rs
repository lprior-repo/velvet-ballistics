#![forbid(unsafe_code)]
//! Recovered entry assembly and observed-dimension counting for frame-seed
//! derivation in [`super::derive`].

use std::collections::{HashMap, HashSet};

use vb_core::{ActionId, RunId, SlotIdx, StepIdx};

use crate::recovery::types::{
    RecoveredPendingAction, RecoveredStepEntry, RecoveredStepState, RecoveryError, RecoveryResult,
};

pub(crate) trait RecoveryIndex {
    fn index(self) -> u16;
}

impl RecoveryIndex for StepIdx {
    fn index(self) -> u16 {
        self.get()
    }
}

impl RecoveryIndex for SlotIdx {
    fn index(self) -> u16 {
        self.get()
    }
}

pub(crate) fn dimension_count<T: RecoveryIndex>(max: Option<T>, run: RunId) -> RecoveryResult<u16> {
    max.map(|value| {
        value
            .index()
            .checked_add(1)
            .ok_or(RecoveryError::FrameDimensionOverflow { run })
    })
    .map_or(Ok(0), |result| result)
}

pub(crate) fn recovered_steps(
    step_states: HashMap<StepIdx, RecoveredStepState>,
) -> Vec<RecoveredStepEntry> {
    let mut entries: Vec<RecoveredStepEntry> = step_states
        .into_iter()
        .map(|(step, state)| RecoveredStepEntry { step, state })
        .collect();
    entries.sort_by_key(|entry| entry.step);
    entries
}

pub(crate) fn recovered_pending_actions(
    pending_actions: HashSet<(ActionId, StepIdx)>,
) -> Vec<RecoveredPendingAction> {
    let mut entries: Vec<RecoveredPendingAction> = pending_actions
        .into_iter()
        .map(|(action, step)| RecoveredPendingAction { step, action })
        .collect();
    entries.sort_by_key(|entry| (entry.step, entry.action));
    entries
}
