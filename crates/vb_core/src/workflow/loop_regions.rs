#![forbid(unsafe_code)]

//! Cold ownership and edge-closure proofs for repeated body activation.
//!
//! Call only after node identities, reference bounds, reachability and existing
//! forward-edge checks. This cache proves topology, not cleanup or recovery:
//! nested and recovered-error activation still require runtime state proofs.

use super::{WorkflowError, WorkflowParts};
use crate::ids::StepIdx;

/// Immutable, constructor-validated ownership of one loop backedge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledLoopRegion {
    start: StepIdx,
    body: StepIdx,
    control: StepIdx,
    done: StepIdx,
}

/// Total per-step loop cache state.
///
/// Replaces the former parallel `Option<CompiledLoopRegion>` cache and
/// boolean nested-entry mask: every step has exactly one state, so lookups
/// are total with no out-of-range confusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopRegionSlot {
    /// Neither a loop control nor a nested activation entry.
    Empty,
    /// A loop control owning this region.
    Region(CompiledLoopRegion),
    /// A nested activation entry without a loop region.
    NestedEntry,
    /// A loop control whose step is also a nested activation entry.
    RegionAndNested(CompiledLoopRegion),
}

impl LoopRegionSlot {
    /// Loop region, when this step is a loop control.
    #[must_use]
    pub const fn region(&self) -> Option<&CompiledLoopRegion> {
        match self {
            Self::Empty | Self::NestedEntry => None,
            Self::Region(region) | Self::RegionAndNested(region) => Some(region),
        }
    }

    /// Whether this step is a nested activation entry.
    #[must_use]
    pub const fn is_nested_entry(&self) -> bool {
        match self {
            Self::Empty | Self::Region(_) => false,
            Self::NestedEntry | Self::RegionAndNested(_) => true,
        }
    }
}

impl CompiledLoopRegion {
    /// Owning start, excluded from the body activation interval.
    #[must_use]
    pub const fn start(&self) -> StepIdx {
        self.start
    }

    /// First step in the inclusive body activation interval.
    #[must_use]
    pub const fn body(&self) -> StepIdx {
        self.body
    }

    /// Unique latch and last step in the inclusive activation interval.
    #[must_use]
    pub const fn control(&self) -> StepIdx {
        self.control
    }

    /// Owning completion target, excluded from the activation interval.
    #[must_use]
    pub const fn done(&self) -> StepIdx {
        self.done
    }

    fn contains(self, step: StepIdx) -> bool {
        self.body <= step && step <= self.control
    }
}

mod edges;
mod nested_entries;
mod pairing;
mod shapes;

/// Builds a control-indexed cache without changing serialized workflow parts.
/// All scans are bounded by the already validated node/branch tables. Scratch
/// allocation occurs only here, with fallible reservation before growth.
pub(super) fn validate(
    parts: &WorkflowParts,
) -> Result<Box<[Option<CompiledLoopRegion>]>, WorkflowError> {
    let mut regions = empty_regions(parts.nodes.len())?;
    for node in &parts.nodes {
        pairing::cache_control(parts, node, &mut regions)?;
    }
    pairing::validate_region_relations(&regions)?;
    let mut targets = Vec::new();
    for region in regions.iter().flatten().copied() {
        edges::validate_region_edges(parts, region, &regions, &mut targets)?;
    }
    Ok(regions.into_boxed_slice())
}

/// Derives the nested activation entry map from validated loop regions.
///
/// Returns a per-step boolean array where `true` marks nested Starts and their validated
/// inclusive body/control intervals. The array length matches the node count.
pub(super) fn nested_activation_map(
    parts: &WorkflowParts,
    regions: &[Option<CompiledLoopRegion>],
) -> Result<Box<[bool]>, WorkflowError> {
    nested_entries::derive(parts, regions)
}

/// Merges validated regions and the nested-entry mask into total slots.
///
/// Every step maps to exactly one [`LoopRegionSlot`]. The merge is pure and
/// total: inputs already agree on length, so no failure is possible here.
pub(super) fn merge_slots(
    regions: &[Option<CompiledLoopRegion>],
    nested: &[bool],
) -> Box<[LoopRegionSlot]> {
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

fn empty_regions(count: usize) -> Result<Vec<Option<CompiledLoopRegion>>, WorkflowError> {
    let mut regions = Vec::new();
    regions
        .try_reserve_exact(count)
        .map_err(|_| resource_error())?;
    regions.resize(count, None);
    Ok(regions)
}

fn resource_error() -> WorkflowError {
    WorkflowError::ResourceContractExceeded {
        resource: crate::workflow::ContractResource::LoopRegions,
    }
}

fn crossing(region: CompiledLoopRegion, from: StepIdx, to: StepIdx) -> WorkflowError {
    WorkflowError::LoopRegionCrossing {
        control: region.control,
        from,
        to,
    }
}
