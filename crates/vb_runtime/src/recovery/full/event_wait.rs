use vb_core::ids::StepIdx;

/// Durable proof that an untimed `WaitEvent` suspended the run.
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
