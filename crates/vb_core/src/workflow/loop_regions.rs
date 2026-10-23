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
/// Returns a per-step boolean array where `true` blocks nested Starts and their validated
/// inclusive body/control intervals. The array length matches the node count.
pub(super) fn nested_activation_map(
    parts: &WorkflowParts,
    regions: &[Option<CompiledLoopRegion>],
) -> Result<Box<[bool]>, WorkflowError> {
    nested_entries::derive(parts, regions)
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
        resource: "loop_regions",
    }
}

fn crossing(region: CompiledLoopRegion, from: StepIdx, to: StepIdx) -> WorkflowError {
    WorkflowError::LoopRegionCrossing {
        control: region.control,
        from,
        to,
    }
}
