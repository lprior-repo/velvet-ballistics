#![forbid(unsafe_code)]

use super::{StepIdx, WorkflowError, WorkflowParts};

// Existing validation boundaries below retain their original stubs; only merge_slots is newly mirrored as executable production logic.

// Production `crates/vb_core/src/workflow/loop_regions.rs:12-19`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledLoopRegion {
    pub start: StepIdx,
    pub body: StepIdx,
    pub control: StepIdx,
    pub done: StepIdx,
}

// Production `crates/vb_core/src/workflow/loop_regions.rs:21-36`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopRegionSlot {
    Empty,
    Region(CompiledLoopRegion),
    NestedEntry,
    RegionAndNested(CompiledLoopRegion),
}

// Production `crates/vb_core/src/workflow/loop_regions.rs:38-56`
impl LoopRegionSlot {
    #[must_use]
    pub const fn region(&self) -> Option<&CompiledLoopRegion> {
        match self {
            Self::Empty | Self::NestedEntry => None,
            Self::Region(region) | Self::RegionAndNested(region) => Some(region),
        }
    }
    #[must_use]
    pub const fn is_nested_entry(&self) -> bool {
        match self {
            Self::Empty | Self::Region(_) => false,
            Self::NestedEntry | Self::RegionAndNested(_) => true,
        }
    }
}

// Production `crates/vb_core/src/workflow/loop_regions.rs:122-140`
pub fn merge_slots(regions: &[Option<CompiledLoopRegion>], nested: &[bool]) -> Vec<LoopRegionSlot> {
    regions
        .iter()
        .zip(nested.iter())
        .map(|(region, blocked)| match (region, *blocked) {
            (Some(region), true) => LoopRegionSlot::RegionAndNested(*region),
            (Some(region), false) => LoopRegionSlot::Region(*region),
            (None, true) => LoopRegionSlot::NestedEntry,
            (None, false) => LoopRegionSlot::Empty,
        })
        .collect()
}

/// Stub for production `loop_regions::validate`: builds a
/// control-indexed cache without changing serialized workflow parts.
pub fn validate(_parts: &WorkflowParts) -> Result<Vec<Option<CompiledLoopRegion>>, WorkflowError> {
    Ok(Vec::new())
}

/// Stub for production `loop_regions::nested_activation_map`: derives
/// the nested activation entry map from validated loop regions.
pub fn nested_activation_map(
    _parts: &WorkflowParts,
    _regions: &[Option<CompiledLoopRegion>],
) -> Result<Vec<bool>, WorkflowError> {
    Ok(Vec::new())
}
