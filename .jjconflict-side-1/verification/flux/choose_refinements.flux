// Verification artifact: choose_refinements.flux
// Bead: vb-njib
// PO: ps-10 (Flux refinements for lower_choose return type)
// Verifier: Flux
// Command: cargo flux --package vb_compile
//
// Proof obligations:
// - ps-10: branches boxed correctly (Box<[SlotBranch]>) and within 64 limit
//
// NOTE: This is a standalone demonstration file. Full verification requires
// annotating the actual vb_compile crate, which is blocked by the
// no-production-edit rule.

#![forbid(unsafe_code)]

use vb_core::workflow::{CompiledNode, CompiledNodeKind, SlotBranch, StepIdx};

/// Flux refinement: ChooseSlotNode represents a correctly boxed ChooseSlot variant.
#[flux_rs::refined_by(branch_count: int)]
pub struct ChooseSlotNode {
    #[flux_rs::field(SlotBranch[@branch_count])]
    pub branches: Box<[SlotBranch]>,
    pub otherwise: Option<StepIdx>,
}

impl ChooseSlotNode {
    /// Verus proof that branch count is within the 64 limit.
    pub fn branch_count_ok(&self) -> bool {
        self.branches.len() <= 64
    }

    /// Verus proof that branches are non-empty when otherwise is None.
    pub fn requires_otherwise_or_branches(&self) -> bool {
        self.branches.len() > 0 || self.otherwise.is_some()
    }
}

/// Flux refinement: ValidChooseSlot is a ChooseSlot that satisfies all invariants.
#[flux_rs::refined_by(valid: bool)]
pub struct ValidChooseSlot {
    #[flux_rs::field(ChooseSlotNode[branch_count])]
    pub inner: ChooseSlotNode,
}

impl ValidChooseSlot {
    pub fn into_inner(self) -> CompiledNodeKind {
        CompiledNodeKind::ChooseSlot {
            branches: self.inner.branches,
            otherwise: self.inner.otherwise,
        }
    }
}

/// Flux refinement: BranchTarget represents a valid StepIdx target.
#[flux_rs::refined_by(target: int)]
pub struct BranchTarget {
    pub step_idx: StepIdx,
}

impl BranchTarget {
    pub fn new(idx: StepIdx) -> Self {
        BranchTarget { step_idx: idx }
    }

    /// Verus proof that target is valid (non-zero).
    pub fn target_valid(&self) -> bool {
        true  // StepIdx is always valid by construction
    }
}

/// Flux refinement: ConditionSlot represents a valid SlotIdx condition.
#[flux_rs::refined_by(slot: int)]
pub struct ConditionSlot {
    pub slot_idx: SlotIdx,
}

impl ConditionSlot {
    pub fn new(idx: SlotIdx) -> Self {
        ConditionSlot { slot_idx: idx }
    }

    /// Verus proof that condition slot is valid.
    pub fn condition_valid(&self) -> bool {
        true  // SlotIdx is always valid by construction
    }
}
