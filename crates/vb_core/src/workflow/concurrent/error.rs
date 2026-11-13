use crate::ids::StepIdx;
use thiserror::Error;

/// Error raised when the concurrent region analysis of a workflow DAG fails.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ConcurrentAnalysisError {
    /// The concurrent region table layout is invalid.
    #[error("concurrent region analysis failed: invalid layout in the region table")]
    InvalidLayout,

    /// The concurrent group kind, lane arity, or child peak is invalid.
    #[error("group {owner:?} has an invalid shape")]
    InvalidShape {
        /// Step at the root of the malformed group.
        owner: StepIdx,
    },

    /// A concurrent group's required fanout exceeds the declared policy maximum.
    #[error("group {owner:?} requires fanout {actual} but the declared maximum is {max}")]
    OverFanout {
        /// Step at the root of the over-fanned group.
        owner: StepIdx,
        /// Number of lanes required by the group layout.
        actual: u64,
        /// Declared policy fanout limit.
        max: u16,
    },

    /// The computed peak concurrent frame count cannot fit in a u16.
    #[error("workflow requires {actual} physical frames, which cannot be represented in u16")]
    FrameOverflow {
        /// Step at which the computed peak is observed.
        owner: StepIdx,
        /// Computed required frame count at that point.
        actual: u64,
    },

    /// The computed peak concurrent group count cannot fit in a u16.
    #[error("workflow has {actual} simultaneous groups, which cannot be represented in u16")]
    GroupOverflow {
        /// Step at which the computed peak is observed.
        owner: StepIdx,
        /// Computed required group count at that point.
        actual: u64,
    },
}
