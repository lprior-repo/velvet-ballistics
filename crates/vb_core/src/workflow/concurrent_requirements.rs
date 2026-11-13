#![forbid(unsafe_code)]

use super::CompiledWorkflow;

/// Checked static bounds for concurrent execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConcurrentRequirements {
    frames: u16,
    groups: u16,
}

impl ConcurrentRequirements {
    pub(super) const fn new(frames: u16, groups: u16) -> Self {
        Self { frames, groups }
    }

    /// Total frames including root and waiting parent.
    #[must_use]
    pub const fn frames(self) -> u16 {
        self.frames
    }

    /// Peak simultaneous groups.
    #[must_use]
    pub const fn groups(self) -> u16 {
        self.groups
    }
}

impl CompiledWorkflow {
    /// None for legacy or unvalidated paths.
    #[must_use]
    pub fn concurrent_requirements(&self) -> Option<ConcurrentRequirements> {
        self.concurrent_requirements
    }
}
