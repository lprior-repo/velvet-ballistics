use crate::ids::StepIdx;

/// Parallelism mode of a checked concurrent group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConcurrentMode {
    /// Ordered for-each lanes driven by one reusable lane template.
    ForEachOrdered,
    /// Ordered fan-out with one lane per branch.
    TogetherOrdered,
    /// First successful branch wins; the remaining lanes are canceled.
    FirstSuccess,
}

/// Cold validation cache for a concurrent group region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConcurrentGroupRegion {
    /// Checked owner group step ID.
    pub(crate) owner: StepIdx,
    /// Checked join step ID.
    pub(crate) join: StepIdx,
    /// Owning group for nesting; None for root groups.
    pub(crate) parent: Option<StepIdx>,
    /// Parallelism mode.
    pub(crate) mode: ConcurrentMode,
    /// Effective physical child count (foreach: min(limit,at_once)).
    pub(crate) capacity: u16,
    /// One reusable foreach template, otherwise one lane per branch.
    pub(crate) lanes: Box<[ConcurrentLaneRegion]>,
}

/// Cold validation cache for one concurrent lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConcurrentLaneRegion {
    /// Checked lane entry step ID.
    pub(crate) entry: StepIdx,
    /// First step outside this lane (checked).
    pub(crate) exclusive_end: StepIdx,
    /// Actual ConcurrentLaneEnd steps reached in this lane.
    pub(crate) ends: Box<[StepIdx]>,
    /// Direct nested group owners checked in this lane.
    pub(crate) children: Box<[StepIdx]>,
}
