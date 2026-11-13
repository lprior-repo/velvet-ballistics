use vb_core::ids::StepIdx;

/// Durable proof that an untimed `WaitEvent` suspended the run.
///
/// Authority is a compiled untimed `WaitEvent` at the recovered pc plus a
/// durable `StepStarted` for that step, with no unresolved `WaitScheduledEvent`.
/// There is no timer and no public event-delivery API in v1, so recovery
/// restores the same waiting boundary without re-driving the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecoveredOpenWaitEvent {
    step: StepIdx,
}

impl RecoveredOpenWaitEvent {
    #[must_use]
    pub(crate) const fn new(step: StepIdx) -> Self {
        Self { step }
    }

    #[must_use]
    pub(crate) const fn step(self) -> StepIdx {
        self.step
    }
}
