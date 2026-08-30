// Verification artifact: vb_xi2f_try_from_parts.rs
// PO: PO-008 (Flux refinement for try_from_parts return type)
// Bead: vb-xi2f.4
// Verifier: Flux
// Command: cargo flux --package vb_core
// EVIDENCE: non-closure (downgraded vb-hvxpe; demonstration only)
//
// Proof obligations:
// - PO-008: Flux verifies that try_from_parts returns Err with correct
//   error type for each invalid input class.
//
// NOTE: This is a standalone demonstration file. Full verification requires
// annotating crates/vb_core/src/workflow/mod.rs, which is blocked by the
// no-production-edit rule.
//
// EVIDENCE CLASS: non-closure
// This artifact does NOT satisfy closure evidence requirements. It is a
// shadow-model demonstration that exercises Flux refinement syntax against
// the same external types as production, but it is not #[path]-bound to
// any production source and cannot be cited as proof of production safety.

#![forbid(unsafe_code)]

use vb_core::workflow::{CompiledWorkflow, WorkflowError, WorkflowParts};

// ─────────────────────────────────────────────────────────────────
// Flux: Refined type aliases for try_from_parts
// ─────────────────────────────────────────────────────────────────

/// Flux refinement: A ValidatedCompiledWorkflow carries a proof that
/// it was constructed through try_from_parts (validated == true).
#[flux_rs::refined_by(validated: bool)]
pub struct ValidatedCompiledWorkflow {
    #[flux_rs::field(CompiledWorkflow[validated])]
    inner: CompiledWorkflow,
}

impl ValidatedCompiledWorkflow {
    pub fn into_inner(self) -> CompiledWorkflow {
        self.inner
    }
}

/// Flux refinement: A TypedWorkflowError preserves the specific error
/// variant information from WorkflowError.
#[flux_rs::refined_by(variant: int)]
pub struct TypedWorkflowError {
    #[flux_rs::field(WorkflowError[variant])]
    inner: WorkflowError,
}

impl TypedWorkflowError {
    pub fn into_inner(self) -> WorkflowError {
        self.inner
    }
}

// ─────────────────────────────────────────────────────────────────
// Flux: try_from_parts refinement
// ─────────────────────────────────────────────────────────────────

/// Flux refinement of try_from_parts:
/// - Ok branch: validated == true (workflow passed all checks)
/// - Err branch: variant corresponds to the specific validation failure
#[flux_rs::sig(
    fn(parts: WorkflowParts) -> Result<ValidatedCompiledWorkflow, TypedWorkflowError>
        { validated: true }
)]
pub fn try_from_parts_refined(parts: WorkflowParts) -> Result<ValidatedCompiledWorkflow, TypedWorkflowError> {
    match CompiledWorkflow::try_from_parts(parts) {
        Ok(workflow) => Ok(ValidatedCompiledWorkflow { inner: workflow }),
        Err(error) => Err(TypedWorkflowError { inner: error }),
    }
}

// ─────────────────────────────────────────────────────────────────
// Flux: Error variant refinements
// ─────────────────────────────────────────────────────────────────

/// Flux refinement: EmptyNodes error (variant == 0)
#[flux_rs::sig(fn(e: &TypedWorkflowError) -> bool[e.variant == 0])]
pub fn is_empty_nodes(e: &TypedWorkflowError) -> bool {
    matches!(e.inner, WorkflowError::EmptyNodes)
}

/// Flux refinement: EntryOutOfBounds error (variant == 1)
#[flux_rs::sig(fn(e: &TypedWorkflowError) -> bool[e.variant == 1])]
pub fn is_entry_out_of_bounds(e: &TypedWorkflowError) -> bool {
    matches!(e.inner, WorkflowError::EntryOutOfBounds { .. })
}

/// Flux refinement: StepOutOfBounds error (variant == 2)
#[flux_rs::sig(fn(e: &TypedWorkflowError) -> bool[e.variant == 2])]
pub fn is_step_out_of_bounds(e: &TypedWorkflowError) -> bool {
    matches!(e.inner, WorkflowError::StepOutOfBounds { .. })
}

/// Flux refinement: SlotOutOfBounds error (variant == 3)
#[flux_rs::sig(fn(e: &TypedWorkflowError) -> bool[e.variant == 3])]
pub fn is_slot_out_of_bounds(e: &TypedWorkflowError) -> bool {
    matches!(e.inner, WorkflowError::SlotOutOfBounds { .. })
}

/// Flux refinement: UnreachableNode error (variant == 10)
#[flux_rs::sig(fn(e: &TypedWorkflowError) -> bool[e.variant == 10])]
pub fn is_unreachable_node(e: &TypedWorkflowError) -> bool {
    matches!(e.inner, WorkflowError::UnreachableNode { .. })
}

/// Flux refinement: BackwardEdge error (variant == 11)
#[flux_rs::sig(fn(e: &TypedWorkflowError) -> bool[e.variant == 11])]
pub fn is_backward_edge(e: &TypedWorkflowError) -> bool {
    matches!(e.inner, WorkflowError::BackwardEdge { .. })
}
