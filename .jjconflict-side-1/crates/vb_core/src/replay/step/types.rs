use std::collections::HashMap;

use crate::ids::{ListId, SlotIdx, StepIdx};
use crate::value::Taint;

use super::super::ReplayError;

/// Typed non-deterministic suspension kind observed during replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SuspensionKind {
    /// Action boundary waiting for an external action completion.
    ActionPending,
    /// Ask node waiting for an external answer.
    AskPending,
    /// Wait-until node waiting for a deadline.
    WaitUntil,
    /// Wait-event node waiting for an event or timeout.
    WaitEvent,
}

impl SuspensionKind {
    /// Stable diagnostic name for logs and compatibility assertions.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ActionPending => "Do",
            Self::AskPending => "Ask",
            Self::WaitUntil => "WaitUntil",
            Self::WaitEvent => "WaitEvent",
        }
    }
}

impl core::fmt::Display for SuspensionKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str((*self).as_str())
    }
}

/// Internal action returned by `replay_step`.
#[non_exhaustive]
pub enum ReplayAction {
    /// Continue to the next step.
    Continue(StepIdx),
    /// The run finished.
    Finished,
    /// The run is suspended on a non-deterministic node.
    Suspended { step: StepIdx, kind: SuspensionKind },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ReplayCollectState {
    pub(super) source: ListId,
    pub(super) current_page: ListId,
    pub(super) cursor: usize,
    pub(super) page_size: usize,
    pub(super) item_count: usize,
    pub(super) taint: Taint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ReplayCollectStartArgs {
    pub(super) source: SlotIdx,
    pub(super) limit: u32,
    pub(super) page_size: u32,
    pub(super) body: StepIdx,
    pub(super) done: StepIdx,
}

#[derive(Debug, Default)]
pub struct ReplayCollectStates {
    entries: HashMap<SlotIdx, ReplayCollectState>,
}

impl ReplayCollectStates {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub(super) fn upsert(&mut self, collector: SlotIdx, state: ReplayCollectState) {
        self.entries.insert(collector, state);
    }

    pub(super) fn find(
        &self,
        collector: SlotIdx,
        current_page: ListId,
    ) -> Result<ReplayCollectState, ReplayError> {
        self.entries
            .get(&collector)
            .filter(|state| state.current_page == current_page)
            .copied()
            .ok_or(ReplayError::Internal {
                reason: "collect pagination state missing during replay",
            })
    }

    pub(super) fn remove(&mut self, collector: SlotIdx) {
        self.entries.remove(&collector);
    }
}
