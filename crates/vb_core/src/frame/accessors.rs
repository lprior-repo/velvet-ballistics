//! Const-fn accessors for `RunFrame` identity and dimensions.

use crate::ids::{RunId, StepIdx};

use super::run_frame::RunFrame;

impl RunFrame {
    /// Run identifier.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }

    /// Current program counter.
    #[must_use]
    pub const fn pc(&self) -> StepIdx {
        self.pc
    }

    /// Number of transitions executed by this frame.
    #[must_use]
    pub const fn executed(&self) -> u64 {
        self.executed
    }

    /// Number of step states allocated in this frame.
    #[must_use]
    pub const fn step_count(&self) -> u16 {
        self.step_count
    }

    /// Number of slots allocated in this frame.
    #[must_use]
    pub const fn slot_count(&self) -> u16 {
        self.slot_count
    }

    /// Maximum allowed parallel in-flight branches for this workflow.
    #[must_use]
    pub const fn max_parallel_in_flight(&self) -> u16 {
        self.max_parallel_in_flight
    }

    /// Current number of parallel in-flight branch executions.
    #[must_use]
    pub const fn parallel_in_flight(&self) -> u16 {
        self.parallel_in_flight
    }
}
