type FramePoolKey = (u16, u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PendingTimerKind {
    Wait,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingTimer {
    pub step: StepIdx,
    pub kind: PendingTimerKind,
    pub generation: u64,
    pub deadline: Instant,
}

impl PendingTimer {
    #[must_use]
    pub fn matches_authority(
        self,
        generation: u64,
        deadline: Instant,
        kind: PendingTimerKind,
    ) -> bool {
        self.generation == generation && self.deadline == deadline && self.kind == kind
    }
}
