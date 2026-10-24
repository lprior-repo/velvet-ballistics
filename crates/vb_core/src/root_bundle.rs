//! Owned root bundle: validated, immutable root value set bound to a workflow projection.
//!
//! An `OwnedRootBundle` is constructed from a complete set of root values, a
//! checkpointed value store, and a compiled workflow. The constructor validates
//! the entire input set through eight phases before producing the bundle.
//!
//! The bundle is valid only against its retained input projection (digest, frame
//! slot count, input slots, resource contract). It is not artifact authority and
//! does not constitute a full runtime. Downstream consumers must compare the
//! exact projection before use.

mod binding;
pub mod errors;
mod graph;
mod roots;
mod runtime_store;
pub use runtime_store::validate_runtime_store;
mod taint;
mod validation;

pub use errors::{ArenaNode, OwnedRootBundleError, ValueKind};
pub use graph::checked_edge_work;
pub use validation::checked_arena_count;

use crate::errors::CoreError;
use crate::ids::{SlotIdx, WorkflowDigest};
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStoreCheckpointV1;
use crate::workflow::{CompiledInputSlot, CompiledWorkflow, ResourceContract};

use binding::Binding;

/// Immutable owned root bundle.
///
/// Private fields; no Default, no Deserialize, no Clone.
/// Valid only against the retained input projection.
pub struct OwnedRootBundle {
    binding: Binding,
    checkpoint: ValueStoreCheckpointV1,
    roots: Vec<(SlotIdx, SlotValue, Taint)>,
}

impl OwnedRootBundle {
    /// Construct an owned root bundle from roots, checkpoint, and workflow.
    ///
    /// Consumes all inputs. Validates through eight phases in strict order.
    /// On success returns the immutable bundle. On failure returns a typed
    /// error; no partial state is exposed.
    pub fn new(
        roots: Box<[(SlotIdx, SlotValue, Taint)]>,
        checkpoint: ValueStoreCheckpointV1,
        workflow: &CompiledWorkflow,
    ) -> Result<Self, OwnedRootBundleError> {
        validation::preflight(&checkpoint, workflow)?;
        roots::validate(&roots, workflow, &checkpoint)?;
        let graph = graph::validate(&checkpoint, &roots)?;
        taint::validate(&roots, &graph)?;
        drop(graph);

        let binding = Binding::new(workflow)?;

        let canonical = canonicalize(roots, workflow.slot_count())?;

        Ok(Self {
            binding,
            checkpoint,
            roots: canonical,
        })
    }

    /// Workflow digest retained from input projection.
    pub fn digest(&self) -> WorkflowDigest {
        self.binding.workflow_digest()
    }

    /// Frame slot count from workflow.
    pub fn frame_slot_count(&self) -> u16 {
        self.binding.slot_count()
    }

    /// Input slots from workflow.
    pub fn input_slots(&self) -> &[CompiledInputSlot] {
        self.binding.input_slots()
    }

    /// Resource contract from workflow.
    pub fn resource_contract(&self) -> &ResourceContract {
        self.binding.resource_contract()
    }

    /// Checkpoint retained from input.
    pub fn checkpoint(&self) -> &ValueStoreCheckpointV1 {
        &self.checkpoint
    }

    /// Declared input slot count (roots.len()).
    pub fn input_slot_count(&self) -> usize {
        self.roots.len()
    }

    /// Roots as slice of (SlotIdx, SlotValue, Taint).
    pub fn roots(&self) -> &[(SlotIdx, SlotValue, Taint)] {
        &self.roots
    }

    /// Check if a slot index is a declared root.
    pub fn has_root(&self, slot: SlotIdx) -> bool {
        self.root(slot).is_some()
    }

    /// Get root value and taint for a declared root slot.
    ///
    /// Uses binary search on the canonical sorted roots.
    pub fn root(&self, slot: SlotIdx) -> Option<(SlotValue, Taint)> {
        let idx = slot.get();
        let pos = self
            .roots
            .binary_search_by_key(&idx, |(s, _, _)| s.get())
            .ok()?;
        self.roots
            .get(pos)
            .map(|(_, value, taint)| (*value, *taint))
    }

    /// Pure binding comparison: exact digest, frame count, input slots,
    /// and entire ResourceContract equality. No digest alone, no activation.
    pub fn matches_workflow(&self, wf: &CompiledWorkflow) -> bool {
        self.binding.matches(wf)
    }
}

fn canonicalize(
    roots: Box<[(SlotIdx, SlotValue, Taint)]>,
    frame_slot_count: u16,
) -> Result<Vec<(SlotIdx, SlotValue, Taint)>, OwnedRootBundleError> {
    let slot_count = usize::from(frame_slot_count);
    let mut map: Vec<Option<(SlotIdx, SlotValue, Taint)>> = Vec::new();
    map.try_reserve_exact(slot_count)
        .map_err(|_| OwnedRootBundleError::Checkpoint(CoreError::AllocationFailed))?;
    map.resize_with(slot_count, || None);

    fill_slot_map(&mut map, &roots)?;

    let mut canonical = roots.into_vec();
    canonical.clear();
    canonical.extend(map.into_iter().flatten());

    Ok(canonical)
}

fn fill_slot_map(
    map: &mut [Option<(SlotIdx, SlotValue, Taint)>],
    roots: &[(SlotIdx, SlotValue, Taint)],
) -> Result<(), OwnedRootBundleError> {
    for (slot, value, taint) in roots.iter() {
        let idx = usize::from(slot.get());
        let entry = map.get_mut(idx).ok_or(OwnedRootBundleError::Checkpoint(
            CoreError::InternalInvariantViolation {
                reason: "owned_root_canonicalize",
            },
        ))?;
        *entry = Some((*slot, *value, *taint));
    }
    Ok(())
}
