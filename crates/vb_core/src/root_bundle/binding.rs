//! Projection binding and comparison for owned root bundles.
//!
//! A `Binding` retains the exact input projection from a compiled workflow
//! at construction time. It is used to verify that a root bundle remains
//! valid against the same projection it was constructed against.

use crate::errors::CoreError;
use crate::ids::WorkflowDigest;
use crate::root_bundle::errors::OwnedRootBundleError;
use crate::workflow::{CompiledInputSlot, CompiledWorkflow, ResourceContract};

/// Immutable projection binding retained from a compiled workflow.
///
/// Captures the exact input declaration, frame slot count, workflow digest,
/// and resource contract at construction time. Used to verify that a root
/// bundle remains valid against the same projection.
pub(super) struct Binding {
    digest: WorkflowDigest,
    frame_slot_count: u16,
    input_slots: Vec<CompiledInputSlot>,
    resource_contract: ResourceContract,
}

impl Binding {
    /// Construct a binding from a compiled workflow.
    ///
    /// Fallibly copies the workflow's input declaration slice. Allocation
    /// failures are mapped to `OwnedRootBundleError::Checkpoint`.
    pub(super) fn new(wf: &CompiledWorkflow) -> Result<Self, OwnedRootBundleError> {
        let mut input_slots = Vec::new();
        let count = wf.input_slots().len();
        if input_slots.try_reserve_exact(count).is_err() {
            return Err(OwnedRootBundleError::Checkpoint(
                CoreError::AllocationFailed,
            ));
        }
        input_slots.extend_from_slice(wf.input_slots());

        Ok(Self {
            digest: wf.digest(),
            frame_slot_count: wf.slot_count(),
            input_slots,
            resource_contract: wf.resource_contract(),
        })
    }

    /// Workflow digest retained from the input projection.
    pub(super) fn workflow_digest(&self) -> WorkflowDigest {
        self.digest
    }

    /// Frame slot count from the workflow.
    pub(super) fn slot_count(&self) -> u16 {
        self.frame_slot_count
    }

    /// Input slots from the workflow.
    pub(super) fn input_slots(&self) -> &[CompiledInputSlot] {
        &self.input_slots
    }

    /// Resource contract from the workflow.
    pub(super) fn resource_contract(&self) -> &ResourceContract {
        &self.resource_contract
    }

    /// Check if this binding matches the given workflow's projection.
    ///
    /// Compares exact digest, frame slot count, input declaration order and
    /// kinds, and the entire resource contract. No digest-only shortcut.
    pub(super) fn matches(&self, wf: &CompiledWorkflow) -> bool {
        if self.digest != wf.digest() {
            return false;
        }
        if self.frame_slot_count != wf.slot_count() {
            return false;
        }
        if self.input_slots.len() != wf.input_slots().len() {
            return false;
        }
        for (mine, theirs) in self.input_slots.iter().zip(wf.input_slots().iter()) {
            if mine.slot != theirs.slot || mine.kind != theirs.kind {
                return false;
            }
        }
        if self.resource_contract != wf.resource_contract() {
            return false;
        }
        true
    }
}
